use super::*;

// Code size is bloated when this gets inlined
#[inline(never)]
pub fn gen_visibility_masks(relative_tile_pos: f32x3) -> [u8x64; 3] {
    let offsets = relative_tile_pos.mul_add_fast(Simd::splat(1.0 / 16.0), Simd::splat(0.5));

    let (xy_mask_compressed, yx_mask_compressed) = gen_compressed_mask_pair(offsets[X], offsets[Y]);
    let xy_mask = expand_xy_mask(xy_mask_compressed);
    let yx_mask = expand_xy_mask(yx_mask_compressed);

    let (xz_mask_compressed, zx_mask_compressed) = gen_compressed_mask_pair(offsets[X], offsets[Z]);
    let xz_mask = expand_xz_mask(xz_mask_compressed);
    let zx_mask = expand_xz_mask(zx_mask_compressed);

    let (zy_mask_compressed, yz_mask_compressed) = gen_compressed_mask_pair(offsets[Z], offsets[Y]);
    let zy_mask = expand_zy_mask(zy_mask_compressed);
    let yz_mask = expand_zy_mask(yz_mask_compressed);

    let x_mask = yx_mask & zx_mask;
    let y_mask = xy_mask & zy_mask;
    let z_mask = xz_mask & yz_mask;

    [x_mask, y_mask, z_mask]
}

fn gen_compressed_mask_pair(offset_1: f32, offset_2: f32) -> (u8x8, u8x8) {
    let neg_x_offset = Simd::splat(-offset_1);
    let y_offset = Simd::splat(offset_2);
    let ys = Simd::from_array([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);

    let line_1 = neg_x_offset + y_offset + ys;
    let line_2 = neg_x_offset - y_offset - ys;

    let lower_bound = line_1.simd_min_fast(line_2);
    let upper_bound = line_1.simd_max_fast(line_2);

    let (lower_bound_ceil_clamped, upper_bound_floor, lower_bound_mask, upper_bound_mask) =
        rasterize_rows(lower_bound, upper_bound);
    let combined_mask = lower_bound_mask & upper_bound_mask;

    // Get lowest set bit of the mask if the bound falls on an integer.
    let lowest_bit_mask = lower_bound.simd_eq(lower_bound_ceil_clamped).select(
        lower_bound_mask & lower_bound_mask.wrapping_neg(),
        Simd::splat(0),
    );
    // Get highest set bit of the mask if the bound falls on an integer
    let highest_bit_mask = upper_bound
        .simd_eq(upper_bound_floor)
        .select((upper_bound_mask + Simd::splat(1)) >> 1, Simd::splat(0));
    let reverse_mask = lowest_bit_mask | highest_bit_mask | !combined_mask;

    // cut off upper bits
    (combined_mask.cast::<u8>(), reverse_mask.cast::<u8>())
}

#[rustfmt::skip]
fn expand_xy_mask(compressed_mask: u8x8) -> u8x64 {
    simd_swizzle!(
        compressed_mask,
        [
            0, 0, 0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1, 1, 1,
            2, 2, 2, 2, 2, 2, 2, 2,
            3, 3, 3, 3, 3, 3, 3, 3,
            4, 4, 4, 4, 4, 4, 4, 4,
            5, 5, 5, 5, 5, 5, 5, 5,
            6, 6, 6, 6, 6, 6, 6, 6,
            7, 7, 7, 7, 7, 7, 7, 7,
        ]
    )
}

#[rustfmt::skip]
fn expand_xz_mask(compressed_mask: u8x8) -> u8x64 {
    simd_swizzle!(
        compressed_mask,
        [
            0, 1, 2, 3, 4, 5, 6, 7,
            0, 1, 2, 3, 4, 5, 6, 7,
            0, 1, 2, 3, 4, 5, 6, 7,
            0, 1, 2, 3, 4, 5, 6, 7,
            0, 1, 2, 3, 4, 5, 6, 7,
            0, 1, 2, 3, 4, 5, 6, 7,
            0, 1, 2, 3, 4, 5, 6, 7,
            0, 1, 2, 3, 4, 5, 6, 7,
        ]
    )
}

#[rustfmt::skip]
fn expand_zy_mask(compressed_mask: u8x8) -> u8x64 {
    const MASK: u8x64 = Simd::from_array([
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
        0b1, 0b10, 0b100, 0b1000, 0b10000, 0b100000, 0b1000000, 0b10000000,
    ]);
    (simd_swizzle!(
        compressed_mask,
        [
            0, 0, 0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1, 1, 1,
            2, 2, 2, 2, 2, 2, 2, 2,
            3, 3, 3, 3, 3, 3, 3, 3,
            4, 4, 4, 4, 4, 4, 4, 4,
            5, 5, 5, 5, 5, 5, 5, 5,
            6, 6, 6, 6, 6, 6, 6, 6,
            7, 7, 7, 7, 7, 7, 7, 7,
        ]
    ) & MASK).simd_eq(MASK).to_int().cast()
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;

    use super::*;
    use crate::TESTS_RANDOM_SEED;

    fn gen_visibility_masks_slow(relative_tile_pos: f32x3) -> [u8x64; 3] {
        let mut x_mask = SECTIONS_FILLED;
        let mut y_mask = SECTIONS_FILLED;
        let mut z_mask = SECTIONS_FILLED;

        for y in 0..8_u8 {
            for z in 0..8_u8 {
                for x in 0..8_u8 {
                    let section_coords = Simd::from_xyz(x, y, z);
                    let section_index = section_index(section_coords);
                    let relative_section_center = relative_tile_pos
                        + Simd::splat(8.0)
                        + (section_coords.cast::<f32>() * Simd::splat(16.0));

                    let distances = relative_section_center.abs();

                    if distances[X] > distances[Y] || distances[Z] > distances[Y] {
                        clear_bit(&mut y_mask, section_index)
                    }
                    if distances[X] > distances[Z] || distances[Y] > distances[Z] {
                        clear_bit(&mut z_mask, section_index)
                    }
                    if distances[Y] > distances[X] || distances[Z] > distances[X] {
                        clear_bit(&mut x_mask, section_index)
                    }
                }
            }
        }

        [x_mask, y_mask, z_mask]
    }

    #[test]
    fn angle_visibility_masks_test() {
        const ITERATIONS: u32 = 10000;
        let mut rand = StdRng::seed_from_u64(TESTS_RANDOM_SEED);

        for _ in 0..ITERATIONS {
            let relative_tile_pos = Simd::from_xyz(
                // (rand.random_range(-20_i8..20_i8) as f32) * 16.0,
                // (rand.random_range(-20_i8..20_i8) as f32) * 16.0,
                // (rand.random_range(-20_i8..20_i8) as f32) * 16.0,
                rand.random_range(-300.0_f32..300.0_f32),
                rand.random_range(-300.0_f32..300.0_f32),
                rand.random_range(-300.0_f32..300.0_f32),
            );

            let test_masks = gen_visibility_masks(relative_tile_pos);
            let sane_masks = gen_visibility_masks_slow(relative_tile_pos);

            if sane_masks != test_masks {
                println!("Sane X Mask");
                print_tile(&sane_masks[X]);
                println!();
                println!("Sane Y Mask");
                print_tile(&sane_masks[Y]);
                println!();
                println!("Sane Z Mask");
                print_tile(&sane_masks[Z]);
                println!();
                println!("Test X Mask");
                print_tile(&test_masks[X]);
                println!();
                println!("Test Y Mask");
                print_tile(&test_masks[X]);
                println!();
                println!("Test Z Mask");
                print_tile(&test_masks[X]);
                println!();
                panic!("sane != test, Relative Tile Coords: {relative_tile_pos:?}");
            }
        }
    }
}
