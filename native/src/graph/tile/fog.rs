use super::*;
use crate::graph::coords::RelativeBoundingBox;

// based on this algorithm
// https://github.com/CaffeineMC/sodium-fabric/blob/dd25399c139004e863beb8a2195b9d80b847d95c/common/src/main/java/net/caffeinemc/mods/sodium/client/render/chunk/occlusion/OcclusionCuller.java#L153
pub fn test_box(bb: RelativeBoundingBox, fog_distance: f32, results: &mut CombinedTestResults) {
    // find closest to (0,0) because the bounding box coordinates are relative to
    // the camera
    let closest_in_chunk = f32x3::splat(0.0).simd_clamp_fast(bb.min, bb.max);

    let furthest_in_chunk = bb.min.abs().simd_gt(bb.max.abs()).select(bb.min, bb.max);

    // combine operations and single out the XZ lanes on both extrema from here.
    // also, we don't have to subtract from the camera pos because the bounds are
    // already relative to it
    let xz_distances = simd_swizzle!(closest_in_chunk, furthest_in_chunk, [X, Z, X + 3, Z + 3]);
    let xz_distances_squared = xz_distances * xz_distances;

    // add Xs and Zs
    let combined_distances_squared =
        simd_swizzle!(xz_distances_squared, [0, 2]) + simd_swizzle!(xz_distances_squared, [1, 3]);

    let y_distances = simd_swizzle!(closest_in_chunk, furthest_in_chunk, [Y, Y + 3]);
    let y_distances_abs = y_distances.abs();

    let outside_fog_mask = combined_distances_squared
        .simd_ge(Simd::splat(fog_distance * fog_distance))
        | y_distances_abs.simd_ge(Simd::splat(fog_distance));

    if outside_fog_mask.test(0) {
        // early exit
        *results = CombinedTestResults::OUTSIDE;
        return;
    }

    results.set_partial::<{ CombinedTestResults::FOG_BIT }>(outside_fog_mask.test(1));
}

pub fn voxelize_cylinder(relative_tile_pos: f32x3, fog_distance: f32) -> u8x64 {
    const BB_EXTENSION: f32 = RelativeBoundingBox::BOUNDING_BOX_EXTENSION;
    const BB_EXTENSION_SCALED: f32 = BB_EXTENSION / 16.0;

    let section_zs = (f32x8::from_array([0.0, 16.0, 32.0, 48.0, 64.0, 80.0, 96.0, 112.0])
        - Simd::splat(BB_EXTENSION))
        + Simd::splat(relative_tile_pos[Z]);

    let distance_zs = Simd::splat(0.0)
        .simd_max_fast(section_zs)
        .simd_min_fast(section_zs + Simd::splat(16.0 + (BB_EXTENSION * 2.0)));

    let c_squared =
        distance_zs.mul_add_fast(-distance_zs, Simd::splat(fog_distance * fog_distance));
    let c = c_squared.sqrt();

    let upper_bound = (c - Simd::splat(relative_tile_pos[X]))
        .mul_add_fast(Simd::splat(1.0 / 16.0), Simd::splat(BB_EXTENSION_SCALED));
    let lower_bound = (c + Simd::splat(relative_tile_pos[X])).mul_add_fast(
        Simd::splat(-1.0 / 16.0),
        Simd::splat(-1.0 - BB_EXTENSION_SCALED),
    );

    let (.., lower_bound_mask, upper_bound_mask) = rasterize_rows(lower_bound, upper_bound);
    let out_of_bounds_mask = c_squared.is_sign_positive_fast().to_int().cast::<u32>();
    let combined_mask = (lower_bound_mask & upper_bound_mask & out_of_bounds_mask).cast::<u8>();

    let zx_mask = u64x8::splat(u64::from_ne_bytes(combined_mask.to_array())).to_ne_bytes();

    let y_lower_bound_mask = (0xFF_u32
        << unsafe {
            (-fog_distance - relative_tile_pos[Y])
                .mul_add_fast(1.0 / 16.0, -BB_EXTENSION_SCALED)
                .floor()
                .to_int_unchecked::<i32>()
                .clamp(0, 8)
        }) as u8;
    let y_upper_bound_mask = (0xFF_u32
        >> unsafe {
            8 - (fog_distance - relative_tile_pos[Y])
                .mul_add_fast(1.0 / 16.0, BB_EXTENSION_SCALED)
                .ceil()
                .to_int_unchecked::<i32>()
                .clamp(0, 8)
        }) as u8;
    let y_mask = y_lower_bound_mask & y_upper_bound_mask;
    let y_mask_expanded = mask64x8::from_bitmask(y_mask as u64).to_int().to_ne_bytes();

    zx_mask & y_mask_expanded
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;

    use super::*;
    use crate::TESTS_RANDOM_SEED;

    fn voxelize_cylinder_slow(
        relative_tile_pos: f32x3,
        fog_distance: f32,
        bounds_extension: f32,
    ) -> u8x64 {
        let mut visible_sections = SECTIONS_EMPTY;

        for y in 0..8 {
            for z in 0..8 {
                for x in 0..8 {
                    let section_coords = Simd::from_xyz(x, y, z);
                    let section_index = section_index(section_coords);

                    let relative_section_pos = section_coords
                        .cast::<f32>()
                        .mul_add_fast(Simd::splat(16.0), relative_tile_pos);
                    let bb = RelativeBoundingBox::new(
                        relative_section_pos - Simd::splat(bounds_extension),
                        relative_section_pos + Simd::splat(16.0 + bounds_extension),
                    );

                    let closest_in_chunk = f32x3::splat(0.0).simd_max(bb.min).simd_min(bb.max);

                    let distances_squared = closest_in_chunk * closest_in_chunk;

                    let inside_fog = (distances_squared[X] + distances_squared[Z])
                        < (fog_distance * fog_distance)
                        && closest_in_chunk[Y].abs() < fog_distance;

                    modify_bit(&mut visible_sections, section_index, inside_fog);
                }
            }
        }

        visible_sections
    }

    #[test]
    fn fog_voxelization_test() {
        const ITERATIONS: u32 = 10000;
        let mut rand = StdRng::seed_from_u64(TESTS_RANDOM_SEED);

        for _ in 0..ITERATIONS {
            let relative_tile_pos = Simd::from_xyz(
                // (rand.random_range(-20_i8..20_i8) as f32) * 16.0,
                // (rand.random_range(-20_i8..20_i8) as f32) * 16.0,
                // (rand.random_range(-20_i8..20_i8) as f32) * 16.0,
                rand.random_range(-3000.0_f32..3000.0_f32),
                rand.random_range(-3000.0_f32..3000.0_f32),
                rand.random_range(-3000.0_f32..3000.0_f32),
            );
            let fog_distance = rand.random_range(0.0_f32..900.0_f32);

            let sane_visible_sections_min = voxelize_cylinder_slow(
                relative_tile_pos,
                fog_distance,
                RelativeBoundingBox::BOUNDING_BOX_EXTENSION_MIN,
            );
            let sane_visible_sections_max = voxelize_cylinder_slow(
                relative_tile_pos,
                fog_distance,
                RelativeBoundingBox::BOUNDING_BOX_EXTENSION_MAX,
            );
            let test_visible_sections = voxelize_cylinder(relative_tile_pos, fog_distance);

            if !test_minimum_maximum(
                &sane_visible_sections_min,
                &sane_visible_sections_max,
                &test_visible_sections,
            ) {
                panic!(
                    "Test results don't fit in sane bounds. Relative Tile Coords: {relative_tile_pos:?}, Fog Distance: {fog_distance}",
                );
            }
        }
    }
}
