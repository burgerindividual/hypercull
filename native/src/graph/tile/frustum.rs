use std::array;

use crate::bitset;
use crate::graph::context::CombinedTestResults;
use crate::graph::coords::RelativeBoundingBox;
use crate::math::prelude::*;

const MAX_PLANES: usize = 16;

#[derive(Clone, Copy)]
pub struct FrustumPlane {
    pub normal: f32x4,
    pub axis_bb_offsets: f32x3,
}

/// When using this, it is expected that coordinates are relative to the camera
/// rather than the world origin.
pub struct Frustum {
    planes: Vec<FrustumPlane>,

    // Plane normals ordered component-wise rather than plane-wise. The contents are transposed
    // from the existing plane normals.
    plane_normals_cw: [Simd<f32, MAX_PLANES>; 4],
}

impl Frustum {
    pub fn new(plane_normals: Vec<f32x4>) -> Self {
        let plane_count = plane_normals.len();
        assert!(plane_count <= MAX_PLANES);

        let planes: Vec<FrustumPlane> = plane_normals
            .iter()
            .map(|&normal| FrustumPlane {
                normal,
                axis_bb_offsets: Self::gen_axis_bb_offsets(
                    normal,
                    RelativeBoundingBox::BOUNDING_BOX_EXTENSION,
                ),
            })
            .collect();

        let mut plane_normals_cw = [f32x16::splat(0.0); 4];

        for (plane_idx, &plane_normal) in plane_normals.iter().enumerate() {
            plane_normals_cw[X][plane_idx] = plane_normal[X];
            plane_normals_cw[Y][plane_idx] = plane_normal[Y];
            plane_normals_cw[Z][plane_idx] = plane_normal[Z];
            plane_normals_cw[W][plane_idx] = plane_normal[W];
        }

        Frustum {
            planes,
            plane_normals_cw,
        }
    }

    fn gen_axis_bb_offsets(plane_normal: f32x4, bounds_extension: f32) -> f32x3 {
        plane_normal
            .resize(Default::default())
            .is_sign_negative_fast()
            .select(
                Simd::splat(-bounds_extension),
                Simd::splat(16.0 + bounds_extension),
            )
    }

    #[no_mangle]
    #[inline(never)]
    pub fn test_box(&self, bb: RelativeBoundingBox, results: &mut CombinedTestResults) {
        let plane_count = self.planes.len();
        let planes_bitmask = (1_u16 << plane_count).wrapping_sub(1);

        if plane_count <= 8 {
            self.test_box_inner::<8>(bb, results, planes_bitmask);
        } else {
            self.test_box_inner::<16>(bb, results, planes_bitmask);
        }
    }

    /// This function determines frustum visibility by doing two dot products
    /// per plane: one between the plane and the furthest p
    ///
    /// The equations are derived from [this](https://old.cescg.org/CESCG-2002/DSykoraJJelinek/)
    /// paper, with the following differences:
    ///
    /// The dot product includes the addition of the plane normals' w component,
    /// which allows the comparison to be simpler (a simple check of if the
    /// result is negative)
    ///
    /// The computation is vectorized planewise, where each plane represents a
    /// lane in the component vectors.
    fn test_box_inner<const LANES: usize>(
        &self,
        bb: RelativeBoundingBox,
        results: &mut CombinedTestResults,
        planes_bitmask: u16,
    ) {
        let normal_xs = self.plane_normals_cw[X].resize::<LANES>(0.0);
        let normal_ys = self.plane_normals_cw[Y].resize::<LANES>(0.0);
        let normal_zs = self.plane_normals_cw[Z].resize::<LANES>(0.0);
        let normal_ws = self.plane_normals_cw[W].resize::<LANES>(0.0);

        // This is faster than doing a float comparison because we can ignore special
        // float values like infinity, and because it can hint to the compiler to use
        // vblendvps on x86.
        let is_neg_x = normal_xs.is_sign_negative_fast();
        let is_neg_y = normal_ys.is_sign_negative_fast();
        let is_neg_z = normal_zs.is_sign_negative_fast();

        // n and p vertices named from Table 1 in the linked paper.

        let bb_min_x = Simd::splat(bb.min[X]);
        let bb_max_x = Simd::splat(bb.max[X]);
        let p_vertices_x = is_neg_x.select(bb_min_x, bb_max_x);

        let bb_min_y = Simd::splat(bb.min[Y]);
        let bb_max_y = Simd::splat(bb.max[Y]);
        let p_vertices_y = is_neg_y.select(bb_min_y, bb_max_y);

        let bb_min_z = Simd::splat(bb.min[Z]);
        let bb_max_z = Simd::splat(bb.max[Z]);
        let p_vertices_z = is_neg_z.select(bb_min_z, bb_max_z);

        let p_dots = normal_xs.mul_add_fast(
            p_vertices_x,
            normal_ys.mul_add_fast(
                p_vertices_y,
                normal_zs.mul_add_fast(p_vertices_z, normal_ws),
            ),
        );

        let any_outside = p_dots.is_sign_negative_fast().any();

        if any_outside {
            // early exit
            results.set_outside();
            return;
        }

        let n_vertices_x = is_neg_x.select(bb_max_x, bb_min_x);
        let n_vertices_y = is_neg_y.select(bb_max_y, bb_min_y);
        let n_vertices_z = is_neg_z.select(bb_max_z, bb_min_z);

        let n_dots = normal_xs.mul_add_fast(
            n_vertices_x,
            normal_ys.mul_add_fast(
                n_vertices_y,
                normal_zs.mul_add_fast(n_vertices_z, normal_ws),
            ),
        );

        let intersecting_planes =
            n_dots.is_sign_negative_fast().to_bitmask() as u16 & planes_bitmask;

        results.intersecting_planes = intersecting_planes;
    }

    // The inlining of this was pretty aggressive. It's not really necessary and
    // likely helps the code cache this way.
    #[inline(never)]
    pub fn voxelize_planes(
        &self,
        mut planes: u16,
        relative_tile_pos: f32x3,
        visible_sections: &mut u8x64,
    ) {
        while planes != 0 {
            let plane_idx = bitset::to_index_u16(bitset::take_one_u16(&mut planes));

            let sections_in_plane = voxelize_plane(relative_tile_pos, self.planes[plane_idx]);

            *visible_sections &= sections_in_plane;
        }
    }
}

/// This function voxelizes one of the planes that make up the frustum,
/// producing a 1 bit if the associated section is inside the plane (with a
/// small offset to ensure no false negatives), and a 0 bit if the associated
/// section is outside of the plane.
///
/// The `axis_bb_offsets` vector will offset each bounding box axis depending on
/// the direction of the plane on that axis, and includes the small offset to
/// avoid false negatives.
///
/// This function works by solving the plane equation for the X intercept on
/// each X-axis row of 8 sections. The intercept is then turned into a bitmask,
/// which fills all bits between index 0 and the index of the intercept. That
/// bitmask is optionally flipped depending on the sign of the X value of the
/// plane, which determines the direction the plane is pointing.
///
/// We vectorize this process with 8 lanes across the Z axis, and we do this
/// operation 8 times for each plane of sections on the Y axis. We extract as
/// much work as possible outside of the Y-axis loop, and specific optimzations
/// regarding the mask generation are implemented for x86 machines with AVX2.
fn voxelize_plane(relative_tile_pos: f32x3, plane: FrustumPlane) -> u8x64 {
    // These increments are scaled 16x because sections are cubes with side lengths
    // of 16 blocks.
    const SECTION_INCREMENTS: f32x8 =
        Simd::from_array([0.0, 16.0, 32.0, 48.0, 64.0, 80.0, 96.0, 112.0]);

    let tile_bb_origin = relative_tile_pos + plane.axis_bb_offsets;
    let mut section_bb_y_offset = tile_bb_origin[Y];

    // To simultaneously find 8 X intercepts at once, we vectorize across the Z
    // axis. These offsets let us find the result at different section Z values.
    let section_bb_zs = SECTION_INCREMENTS + Simd::splat(tile_bb_origin[Z]);

    // cz + ax + d
    let partial_intercept_setup = section_bb_zs.mul_add_fast(
        Simd::splat(plane.normal[Z]),
        Simd::splat(tile_bb_origin[X].mul_add_fast(plane.normal[X], plane.normal[W])),
    );

    // -16a
    let plane_x_scaled = Simd::splat(plane.normal[X] * -16.0);

    let tile_x_intercepts_expanded = i32x64::from_slice(
        array::from_fn::<_, 8, _>(|_y| {
            // (by + (cz + ax + d)) / (-16a)
            let tile_x_intercepts = Simd::splat(section_bb_y_offset)
                .mul_add_fast(Simd::splat(plane.normal[Y]), partial_intercept_setup)
                / plane_x_scaled;

            // Increment Y by length of section in blocks after usage of offsets
            section_bb_y_offset += 16.0;

            // SAFETY: We make sure the value going into the conversion is no larger than
            // 7.0. For values under 0.0, we mask out the poison values before using the
            // output.
            let tile_x_intercepts_upper_bounded = unsafe {
                tile_x_intercepts
                    .simd_min_fast(Simd::splat(7.0))
                    .to_int_unchecked::<i32>()
            };

            // Fill lane with 1-bits if the intercept is negative. A lane with all 1-bits
            // will result in a value of 0 in the generated mask.
            let tile_x_intercepts_clamped = tile_x_intercepts_upper_bounded
                | tile_x_intercepts.is_sign_negative_fast().to_simd();

            tile_x_intercepts_clamped.to_array()
        })
        .as_flattened(),
    );

    // Do a signed saturating cast, x86 has specific instructions for this.
    let tile_x_intercepts = tile_x_intercepts_expanded
        .simd_clamp(Simd::splat(i8::MIN).cast(), Simd::splat(i8::MAX).cast())
        .cast::<u8>();

    #[cfg(target_feature = "avx2")]
    let tile_x_masks = unsafe {
        use std::arch::x86_64::*;

        use crate::math::concat_swizzle_pattern;

        let intercepts_halves: [u8x32; 2] = [
            tile_x_intercepts.extract::<0, 32>(),
            tile_x_intercepts.extract::<32, 32>(),
        ];

        let mask_table = _mm256_set1_epi64x(i64::from_le_bytes([
            0b1, 0b11, 0b111, 0b1111, 0b11111, 0b111111, 0b1111111, 0b11111111,
        ]));
        let shuffled_masks_halves: [u8x32; 2] = intercepts_halves
            .map(|intercepts| _mm256_shuffle_epi8(mask_table, intercepts.into()).into());

        simd_swizzle!(
            shuffled_masks_halves[0],
            shuffled_masks_halves[1],
            concat_swizzle_pattern::<64>()
        )
    };

    #[cfg(not(target_feature = "avx2"))]
    let tile_x_masks = {
        let in_bounds_masks = (Simd::splat(0b10) << tile_x_intercepts) - Simd::splat(1);
        tile_x_intercepts
            .simd_lt(Simd::splat(8))
            .to_simd()
            .cast::<u8>()
            & in_bounds_masks
    };

    // If plane[X] is positive, this will be all 1 bits. if plane[X] is negative,
    // this will be all 0 bits. This is used to reverse the direction of the mask
    // when plane[X] is positive.
    let plane_x_positive_mask = Simd::splat(!(plane.normal[X].to_bits() as i32 >> 31) as u8);

    tile_x_masks ^ plane_x_positive_mask
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use rand::prelude::*;

    use super::*;
    use crate::graph::tile;
    use crate::TESTS_RANDOM_SEED;

    fn voxelize_plane_slow(relative_tile_pos: f32x3, plane: f32x4, bounds_extension: f32) -> u8x64 {
        let mut visible_sections = tile::SECTIONS_EMPTY;

        for y in 0..8 {
            for z in 0..8 {
                for x in 0..8 {
                    let section_coords = Simd::from_xyz(x, y, z);
                    let section_index = tile::section_index(section_coords);

                    let relative_section_pos = section_coords
                        .cast::<f32>()
                        .mul_add_fast(Simd::splat(16.0), relative_tile_pos);
                    let bb = RelativeBoundingBox::new(
                        relative_section_pos - Simd::splat(bounds_extension),
                        relative_section_pos + Simd::splat(16.0 + bounds_extension),
                    );

                    let not_outside = plane[X]
                        * (if plane[X] < 0.0 { bb.min[X] } else { bb.max[X] })
                        + plane[Y] * (if plane[Y] < 0.0 { bb.min[Y] } else { bb.max[Y] })
                        + plane[Z] * (if plane[Z] < 0.0 { bb.min[Z] } else { bb.max[Z] })
                        >= -plane[W];

                    tile::modify_bit(&mut visible_sections, section_index, not_outside);
                }
            }
        }

        visible_sections
    }

    #[test]
    fn plane_voxelization_test() {
        const ITERATIONS: u32 = 10000;
        let mut rand = StdRng::seed_from_u64(TESTS_RANDOM_SEED);

        for _ in 0..ITERATIONS {
            // generate random plane from random unit vector and random W component.
            // based off of this math stackexchange answer: https://math.stackexchange.com/a/44691
            let theta = rand.random_range(0.0..TAU);
            let z: f32 = rand.random_range(-1.0..1.0);
            let w: f32 = rand.random_range(-10.0..1000.0);

            let z_modified = (1.0 - (z * z)).sqrt();
            let x = z_modified * theta.cos();
            let y = z_modified * theta.sin();

            let plane_normal = Simd::from_array([x, y, z, w]);
            let plane_bb_offsets = Frustum::gen_axis_bb_offsets(
                plane_normal,
                RelativeBoundingBox::BOUNDING_BOX_EXTENSION,
            );

            let relative_tile_pos = Simd::from_xyz(
                rand.random_range(-3000.0_f32..3000.0_f32),
                rand.random_range(-3000.0_f32..3000.0_f32),
                rand.random_range(-3000.0_f32..3000.0_f32),
            );

            let sane_visible_sections_min = voxelize_plane_slow(
                relative_tile_pos,
                plane_normal,
                RelativeBoundingBox::BOUNDING_BOX_EXTENSION_MIN,
            );
            let sane_visible_sections_max = voxelize_plane_slow(
                relative_tile_pos,
                plane_normal,
                RelativeBoundingBox::BOUNDING_BOX_EXTENSION_MAX,
            );
            let test_visible_sections = voxelize_plane(
                relative_tile_pos,
                FrustumPlane {
                    normal: plane_normal,
                    axis_bb_offsets: plane_bb_offsets,
                },
            );

            if !tile::test_minimum_maximum(
                &sane_visible_sections_min,
                &sane_visible_sections_max,
                &test_visible_sections,
            ) {
                panic!(
                    "Test results don't fit in sane bounds. Relative Tile Coords: {relative_tile_pos:?}, Plane: {plane_normal:?}",
                );
            }
        }
    }
}
