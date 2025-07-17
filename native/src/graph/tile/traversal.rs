use super::*;

impl Tile {
    pub fn setup_center_tile(&mut self, section_index: u16) {
        let mut outgoing_dirs = ALL_DIRECTIONS;
        while outgoing_dirs != 0 {
            let outgoing_dir = take_one(&mut outgoing_dirs);
            let sections_outgoing = unsafe {
                self.outgoing_dir_section_sets
                    .get_unchecked_mut(to_index(outgoing_dir))
            };

            let mut incoming_dirs = all_except(outgoing_dir);
            while incoming_dirs != 0 {
                let incoming_dir = take_one(&mut incoming_dirs);

                let connected = get_bit(
                    unsafe {
                        self.connection_section_sets
                            .get_unchecked(connection_index(outgoing_dir, incoming_dir))
                    },
                    section_index,
                );

                or_bit(sections_outgoing, section_index, connected);
            }
        }
    }

    pub fn traverse<const TRAVERSAL_DIRS: u8>(
        &mut self,
        start_sections: u8x64,
        mut incoming_dir_section_sets: [u8x64; DIRECTION_COUNT],
        outward_direction_masks: &[u8x64; DIRECTION_COUNT],
        angle_visibility_masks: &[u8x64; 3],
        visible_sections: &mut u8x64,
    ) {
        // the result of the previous culling stages is used as a mask for the traversal
        let main_visibility_mask = *visible_sections;

        loop {
            let mut incoming_changed = false;

            self.try_traverse_dir::<TRAVERSAL_DIRS, NEG_X>(
                &mut incoming_dir_section_sets,
                outward_direction_masks,
                angle_visibility_masks,
                main_visibility_mask,
                &mut incoming_changed,
            );
            self.try_traverse_dir::<TRAVERSAL_DIRS, NEG_Y>(
                &mut incoming_dir_section_sets,
                outward_direction_masks,
                angle_visibility_masks,
                main_visibility_mask,
                &mut incoming_changed,
            );
            self.try_traverse_dir::<TRAVERSAL_DIRS, NEG_Z>(
                &mut incoming_dir_section_sets,
                outward_direction_masks,
                angle_visibility_masks,
                main_visibility_mask,
                &mut incoming_changed,
            );
            self.try_traverse_dir::<TRAVERSAL_DIRS, POS_X>(
                &mut incoming_dir_section_sets,
                outward_direction_masks,
                angle_visibility_masks,
                main_visibility_mask,
                &mut incoming_changed,
            );
            self.try_traverse_dir::<TRAVERSAL_DIRS, POS_Y>(
                &mut incoming_dir_section_sets,
                outward_direction_masks,
                angle_visibility_masks,
                main_visibility_mask,
                &mut incoming_changed,
            );
            self.try_traverse_dir::<TRAVERSAL_DIRS, POS_Z>(
                &mut incoming_dir_section_sets,
                outward_direction_masks,
                angle_visibility_masks,
                main_visibility_mask,
                &mut incoming_changed,
            );

            if !incoming_changed {
                break;
            }
        }

        *visible_sections = incoming_dir_section_sets
            .iter()
            .fold(start_sections, |a, b| a | b);
    }

    fn try_traverse_dir<const TRAVERSAL_DIRS: u8, const OUTGOING_DIR: u8>(
        &mut self,
        incoming_dir_section_sets: &mut [u8x64; DIRECTION_COUNT],
        outward_direction_masks: &[u8x64; DIRECTION_COUNT],
        angle_visibility_masks: &[u8x64; 3],
        main_visibility_mask: u8x64,
        incoming_changed: &mut bool,
    ) {
        if bitset::contains_u8(TRAVERSAL_DIRS, OUTGOING_DIR) {
            let dir_index = to_index(OUTGOING_DIR);
            let axis_index = index_dir_to_axis(dir_index);
            let opposite_dir_index = to_index(opposite(OUTGOING_DIR));

            self.find_outgoing_connections::<TRAVERSAL_DIRS, OUTGOING_DIR>(
                incoming_dir_section_sets,
                outward_direction_masks[dir_index],
                angle_visibility_masks[axis_index],
            );

            let outgoing_sections = self.outgoing_dir_section_sets[dir_index];
            let shifted_masked = match OUTGOING_DIR {
                NEG_X => shift_neg_x(outgoing_sections),
                NEG_Y => shift_neg_y(outgoing_sections),
                NEG_Z => shift_neg_z(outgoing_sections),
                POS_X => shift_pos_x(outgoing_sections),
                POS_Y => shift_pos_y(outgoing_sections),
                POS_Z => shift_pos_z(outgoing_sections),
                _ => unreachable!(),
            } & main_visibility_mask;

            // TODO: does this have to be an OR? I think the answer is yes
            let previous = incoming_dir_section_sets[opposite_dir_index];
            incoming_dir_section_sets[opposite_dir_index] |= shifted_masked;

            *incoming_changed |= incoming_dir_section_sets[opposite_dir_index] != previous;
        }
    }

    fn find_outgoing_connections<const TRAVERSAL_DIRS: u8, const OUTGOING_DIR: u8>(
        &mut self,
        incoming_dir_section_sets: &[u8x64; DIRECTION_COUNT],
        outward_direction_mask: u8x64,
        angle_visibility_mask: u8x64,
    ) {
        let sections_outgoing = &mut self.outgoing_dir_section_sets[to_index(OUTGOING_DIR)];

        let mut incoming_dirs = opposite(TRAVERSAL_DIRS) & !OUTGOING_DIR;
        while incoming_dirs != 0 {
            let incoming_dir = take_one(&mut incoming_dirs);

            let mut connection_sections =
                self.connection_section_sets[connection_index(OUTGOING_DIR, incoming_dir)];

            if incoming_dir == opposite(OUTGOING_DIR) {
                connection_sections &= angle_visibility_mask;
            }

            *sections_outgoing |=
                incoming_dir_section_sets[to_index(incoming_dir)] & connection_sections;
        }

        let opposing_directions =
            bitset::contains_u8(TRAVERSAL_DIRS, OUTGOING_DIR | opposite(OUTGOING_DIR));

        if opposing_directions {
            *sections_outgoing &= outward_direction_mask;
        }
    }
}

fn shift_neg_x(sections: u8x64) -> u8x64 {
    sections >> 1
}

fn shift_pos_x(sections: u8x64) -> u8x64 {
    sections << 1
}

#[rustfmt::skip]
fn shift_neg_z(sections: u8x64) -> u8x64 {
    // The u8x64 "sections" vector represents an 8x8x8 array of bits, with each
    // bit representing a render section. It is indexed with the pattern
    // YYYZZZXXX. Because of our indexing scheme, we know that each u8 lane
    // in the vector represents a row of sections on the X axis.
    // 
    // The array of indices provided to this swizzle can be read with
    // the following diagram:
    // 
    //     z=0       Z Axis      z=7
    //  y=0|------------------------
    //     |
    //     |
    //  Y  |
    // Axis|
    //     |
    //     |
    // y=7 |
    // 
    // Keep in mind, a swizzle with an array of indices full of only incrementing
    // indices starting at 0 would result in a completely unmodified vector. That
    // array would look like the following:
    //
    // 0,  1,  2,  3,  4,  5,  6,  7, 
    // 8,  9,  10, 11, 12, 13, 14, 15,
    // 16, 17, 18, 19, 20, 21, 22, 23,
    // 24, 25, 26, 27, 28, 29, 30, 31,
    // 32, 33, 34, 35, 36, 37, 38, 39,
    // 40, 41, 42, 43, 44, 45, 46, 47,
    // 48, 49, 50, 51, 52, 53, 54, 55,
    // 56, 57, 58, 59, 60, 61, 62, 63,
    // 
    // By shifting each index in that array to the left by 1, this swizzle
    // operation effectively shifts each X-axis row of sections by -1 on the Z
    // axis. The "64" indices seen in this swizzle are used to fill the empty
    // space that the shift left over with zeroes.
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            1,  2,  3,  4,  5,  6,  7,  64,
            9,  10, 11, 12, 13, 14, 15, 64,
            17, 18, 19, 20, 21, 22, 23, 64,
            25, 26, 27, 28, 29, 30, 31, 64,
            33, 34, 35, 36, 37, 38, 39, 64,
            41, 42, 43, 44, 45, 46, 47, 64,
            49, 50, 51, 52, 53, 54, 55, 64,
            57, 58, 59, 60, 61, 62, 63, 64,
        ]
    )
}

#[rustfmt::skip]
fn shift_pos_z(sections: u8x64) -> u8x64 {
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            64, 0,  1,  2,  3,  4,  5,  6,
            64, 8,  9,  10, 11, 12, 13, 14,
            64, 16, 17, 18, 19, 20, 21, 22,
            64, 24, 25, 26, 27, 28, 29, 30,
            64, 32, 33, 34, 35, 36, 37, 38,
            64, 40, 41, 42, 43, 44, 45, 46,
            64, 48, 49, 50, 51, 52, 53, 54,
            64, 56, 57, 58, 59, 60, 61, 62,
        ]
    )
}

#[rustfmt::skip]
fn shift_neg_y(sections: u8x64) -> u8x64 {
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            8,  9,  10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
            32, 33, 34, 35, 36, 37, 38, 39,
            40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55,
            56, 57, 58, 59, 60, 61, 62, 63,
            64, 65, 66, 67, 68, 69, 70, 71,
        ]
    )
}

#[rustfmt::skip]
fn shift_pos_y(sections: u8x64) -> u8x64 {
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            64, 65, 66, 67, 68, 69, 70, 71,
            0,  1,  2,  3,  4,  5,  6,  7,
            8,  9,  10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
            32, 33, 34, 35, 36, 37, 38, 39,
            40, 41, 42, 43, 44, 45, 46, 47,
            48, 49, 50, 51, 52, 53, 54, 55,
        ]
    )
}

pub fn edge_neg_to_pos_x(sections: u8x64) -> u8x64 {
    sections << 7
}

pub fn edge_pos_to_neg_x(sections: u8x64) -> u8x64 {
    sections >> 7
}

#[rustfmt::skip]
pub fn edge_neg_to_pos_z(sections: u8x64) -> u8x64 {
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            64, 64, 64, 64, 64, 64, 64, 0,
            64, 64, 64, 64, 64, 64, 64, 8,
            64, 64, 64, 64, 64, 64, 64, 16,
            64, 64, 64, 64, 64, 64, 64, 24,
            64, 64, 64, 64, 64, 64, 64, 32,
            64, 64, 64, 64, 64, 64, 64, 40,
            64, 64, 64, 64, 64, 64, 64, 48,
            64, 64, 64, 64, 64, 64, 64, 56,
        ]
    )
}

#[rustfmt::skip]
pub fn edge_pos_to_neg_z(sections: u8x64) -> u8x64 {
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            7,  64, 64, 64, 64, 64, 64, 64,
            15, 64, 64, 64, 64, 64, 64, 64,
            23, 64, 64, 64, 64, 64, 64, 64,
            31, 64, 64, 64, 64, 64, 64, 64,
            39, 64, 64, 64, 64, 64, 64, 64,
            47, 64, 64, 64, 64, 64, 64, 64,
            55, 64, 64, 64, 64, 64, 64, 64,
            63, 64, 64, 64, 64, 64, 64, 64,
        ]
    )
}

#[rustfmt::skip]
pub fn edge_neg_to_pos_y(sections: u8x64) -> u8x64 {
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            64,  65,  66,  67,  68,  69,  70,  71,
            72,  73,  74,  75,  76,  77,  78,  79,
            80,  81,  82,  83,  84,  85,  86,  87,
            88,  89,  90,  91,  92,  93,  94,  95,
            96,  97,  98,  99,  100, 101, 102, 103,
            104, 105, 106, 107, 108, 109, 110, 111,
            112, 113, 114, 115, 116, 117, 118, 119,
            0,   1,   2,   3,   4,   5,   6,   7,
        ]
    )
}

#[rustfmt::skip]
pub fn edge_pos_to_neg_y(sections: u8x64) -> u8x64 {
    simd_swizzle!(
        sections,
        Simd::splat(0),
        [
            56,  57,  58,  59,  60,  61,  62,  63,
            64,  65,  66,  67,  68,  69,  70,  71,
            72,  73,  74,  75,  76,  77,  78,  79,
            80,  81,  82,  83,  84,  85,  86,  87,
            88,  89,  90,  91,  92,  93,  94,  95,
            96,  97,  98,  99,  100, 101, 102, 103,
            104, 105, 106, 107, 108, 109, 110, 111,
            112, 113, 114, 115, 116, 117, 118, 119,
        ]
    )
}

pub fn gen_outward_direction_masks(camera_section_in_tile: u8x3) -> [u8x64; DIRECTION_COUNT] {
    let neg_x_lane = (0b10_u8 << camera_section_in_tile[X]).wrapping_sub(1);
    let neg_x_mask = Simd::splat(neg_x_lane);

    let pos_x_lane = 0xFF << camera_section_in_tile[X];
    let pos_x_mask = Simd::splat(pos_x_lane);

    let neg_y_bitmask = (0b10 << camera_section_in_tile[Y]) - 1;
    let neg_y_mask = mask64x8::from_bitmask(neg_y_bitmask).to_int().to_ne_bytes();

    // Mask is truncated to u8 by from_bitmask
    let pos_y_bitmask = 0xFF << camera_section_in_tile[Y];
    let pos_y_mask = mask64x8::from_bitmask(pos_y_bitmask).to_int().to_ne_bytes();

    // native endianness should be correct here, but it's worth double checking
    let neg_z_bitmask = (0b10 << camera_section_in_tile[Z]) - 1;
    let neg_z_lane = u64::from_ne_bytes(
        mask8x8::from_bitmask(neg_z_bitmask)
            .to_int()
            .to_ne_bytes()
            .to_array(),
    );
    let neg_z_mask = u64x8::splat(neg_z_lane).to_ne_bytes();

    let pos_z_bitmask = 0xFF << camera_section_in_tile[Z];
    let pos_z_lane = u64::from_ne_bytes(
        mask8x8::from_bitmask(pos_z_bitmask)
            .to_int()
            .to_ne_bytes()
            .to_array(),
    );
    let pos_z_mask = u64x8::splat(pos_z_lane).to_ne_bytes();

    [
        neg_x_mask, neg_y_mask, neg_z_mask, pos_x_mask, pos_y_mask, pos_z_mask,
    ]
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;

    use super::*;
    use crate::TESTS_RANDOM_SEED;

    #[test]
    fn edge_move_test() {
        const ITERATIONS: u32 = 10000;
        let mut rand = StdRng::seed_from_u64(TESTS_RANDOM_SEED);

        for _ in 0..ITERATIONS {
            let mut src = u8x64::splat(0);

            rand.fill_bytes(src.as_mut_array());

            {
                let mut dst_sane_neg_to_pos_x = u8x64::splat(0);

                for z in 0..8 {
                    for y in 0..8 {
                        modify_bit(
                            &mut dst_sane_neg_to_pos_x,
                            section_index(Simd::from_xyz(7, y, z)),
                            get_bit(&src, section_index(Simd::from_xyz(0, y, z))),
                        );
                    }
                }

                let dst_test_neg_to_pos_x = edge_neg_to_pos_x(src);

                assert_eq!(dst_sane_neg_to_pos_x, dst_test_neg_to_pos_x);
            }

            {
                let mut dst_sane_pos_to_neg_x = u8x64::splat(0);

                for z in 0..8 {
                    for y in 0..8 {
                        modify_bit(
                            &mut dst_sane_pos_to_neg_x,
                            section_index(Simd::from_xyz(0, y, z)),
                            get_bit(&src, section_index(Simd::from_xyz(7, y, z))),
                        );
                    }
                }

                let dst_test_pos_to_neg_x = edge_pos_to_neg_x(src);

                assert_eq!(dst_sane_pos_to_neg_x, dst_test_pos_to_neg_x);
            }

            {
                let mut dst_sane_neg_to_pos_y = u8x64::splat(0);

                for z in 0..8 {
                    for x in 0..8 {
                        modify_bit(
                            &mut dst_sane_neg_to_pos_y,
                            section_index(Simd::from_xyz(x, 7, z)),
                            get_bit(&src, section_index(Simd::from_xyz(x, 0, z))),
                        );
                    }
                }

                let dst_test_neg_to_pos_y = edge_neg_to_pos_y(src);

                assert_eq!(dst_sane_neg_to_pos_y, dst_test_neg_to_pos_y);
            }

            {
                let mut dst_sane_pos_to_neg_y = u8x64::splat(0);

                for z in 0..8 {
                    for x in 0..8 {
                        modify_bit(
                            &mut dst_sane_pos_to_neg_y,
                            section_index(Simd::from_xyz(x, 0, z)),
                            get_bit(&src, section_index(Simd::from_xyz(x, 7, z))),
                        );
                    }
                }

                let dst_test_pos_to_neg_y = edge_pos_to_neg_y(src);

                assert_eq!(dst_sane_pos_to_neg_y, dst_test_pos_to_neg_y);
            }

            {
                let mut dst_sane_neg_to_pos_z = u8x64::splat(0);

                for y in 0..8 {
                    for x in 0..8 {
                        modify_bit(
                            &mut dst_sane_neg_to_pos_z,
                            section_index(Simd::from_xyz(x, y, 7)),
                            get_bit(&src, section_index(Simd::from_xyz(x, y, 0))),
                        );
                    }
                }

                let dst_test_neg_to_pos_z = edge_neg_to_pos_z(src);

                assert_eq!(dst_sane_neg_to_pos_z, dst_test_neg_to_pos_z);
            }

            {
                let mut dst_sane_pos_to_neg_z = u8x64::splat(0);

                for y in 0..8 {
                    for x in 0..8 {
                        modify_bit(
                            &mut dst_sane_pos_to_neg_z,
                            section_index(Simd::from_xyz(x, y, 0)),
                            get_bit(&src, section_index(Simd::from_xyz(x, y, 7))),
                        );
                    }
                }

                let dst_test_pos_to_neg_z = edge_pos_to_neg_z(src);

                assert_eq!(dst_sane_pos_to_neg_z, dst_test_pos_to_neg_z);
            }
        }
    }

    #[test]
    fn shifts_test() {
        const ITERATIONS: u32 = 10000;
        let mut rand = StdRng::seed_from_u64(TESTS_RANDOM_SEED);

        for _ in 0..ITERATIONS {
            let mut src = u8x64::splat(0);

            rand.fill_bytes(src.as_mut_array());

            {
                let mut dst_sane_neg_x = u8x64::splat(0);
                for z in 0..8 {
                    for y in 0..8 {
                        for x in 1..8 {
                            modify_bit(
                                &mut dst_sane_neg_x,
                                section_index(Simd::from_xyz(x - 1, y, z)),
                                get_bit(&src, section_index(Simd::from_xyz(x, y, z))),
                            );
                        }
                    }
                }

                let dst_test_neg_x = shift_neg_x(src);

                assert_eq!(dst_sane_neg_x, dst_test_neg_x);
            }

            {
                let mut dst_sane_pos_x = u8x64::splat(0);
                for z in 0..8 {
                    for y in 0..8 {
                        for x in 0..7 {
                            modify_bit(
                                &mut dst_sane_pos_x,
                                section_index(Simd::from_xyz(x + 1, y, z)),
                                get_bit(&src, section_index(Simd::from_xyz(x, y, z))),
                            );
                        }
                    }
                }

                let dst_test_pos_x = shift_pos_x(src);

                assert_eq!(dst_sane_pos_x, dst_test_pos_x);
            }

            {
                let mut dst_sane_neg_y = u8x64::splat(0);
                for z in 0..8 {
                    for y in 1..8 {
                        for x in 0..8 {
                            modify_bit(
                                &mut dst_sane_neg_y,
                                section_index(Simd::from_xyz(x, y - 1, z)),
                                get_bit(&src, section_index(Simd::from_xyz(x, y, z))),
                            );
                        }
                    }
                }

                let dst_test_neg_y = shift_neg_y(src);

                assert_eq!(dst_sane_neg_y, dst_test_neg_y);
            }

            {
                let mut dst_sane_pos_y = u8x64::splat(0);
                for z in 0..8 {
                    for y in 0..7 {
                        for x in 0..8 {
                            modify_bit(
                                &mut dst_sane_pos_y,
                                section_index(Simd::from_xyz(x, y + 1, z)),
                                get_bit(&src, section_index(Simd::from_xyz(x, y, z))),
                            );
                        }
                    }
                }

                let dst_test_pos_y = shift_pos_y(src);

                assert_eq!(dst_sane_pos_y, dst_test_pos_y);
            }

            {
                let mut dst_sane_neg_z = u8x64::splat(0);
                for z in 1..8 {
                    for y in 0..8 {
                        for x in 0..8 {
                            modify_bit(
                                &mut dst_sane_neg_z,
                                section_index(Simd::from_xyz(x, y, z - 1)),
                                get_bit(&src, section_index(Simd::from_xyz(x, y, z))),
                            );
                        }
                    }
                }

                let dst_test_neg_z = shift_neg_z(src);

                assert_eq!(dst_sane_neg_z, dst_test_neg_z);
            }

            {
                let mut dst_sane_pos_z = u8x64::splat(0);
                for z in 0..7 {
                    for y in 0..8 {
                        for x in 0..8 {
                            modify_bit(
                                &mut dst_sane_pos_z,
                                section_index(Simd::from_xyz(x, y, z + 1)),
                                get_bit(&src, section_index(Simd::from_xyz(x, y, z))),
                            );
                        }
                    }
                }

                let dst_test_pos_z = shift_pos_z(src);

                assert_eq!(dst_sane_pos_z, dst_test_pos_z);
            }
        }
    }

    #[test]
    fn outward_direction_mask_test() {
        for camera_x in 0..8 {
            for camera_y in 0..8 {
                for camera_z in 0..8 {
                    let camera_section_in_tile = u8x3::from_xyz(camera_x, camera_y, camera_z);

                    let mut sane_camera_direction_masks = [SECTIONS_EMPTY; DIRECTION_COUNT];

                    for tile_x in 0..8 {
                        for tile_y in 0..8 {
                            for tile_z in 0..8 {
                                let other_tile_coords = Simd::from_xyz(tile_x, tile_y, tile_z);

                                let negative = other_tile_coords.simd_le(camera_section_in_tile);
                                let positive = other_tile_coords.simd_ge(camera_section_in_tile);
                                let traversal_directions = negative.to_bitmask() as u8
                                    | ((positive.to_bitmask() as u8) << 3);

                                let section_index = section_index(other_tile_coords);
                                for dir_idx in 0..6 {
                                    modify_bit(
                                        &mut sane_camera_direction_masks[dir_idx as usize],
                                        section_index,
                                        traversal_directions.get_bit(dir_idx),
                                    );
                                }
                            }
                        }
                    }

                    let test_camera_direction_masks =
                        gen_outward_direction_masks(camera_section_in_tile);

                    let mut directions = ALL_DIRECTIONS;
                    while directions != 0 {
                        let direction = take_one(&mut directions);
                        let dir_idx = to_index(direction);
                        assert_eq!(
                            sane_camera_direction_masks[dir_idx],
                            test_camera_direction_masks[dir_idx],
                            "sane != test, Camera Coords: {:?}, Direction: {}",
                            camera_section_in_tile,
                            to_str(direction)
                        );
                    }
                }
            }
        }
    }
}
