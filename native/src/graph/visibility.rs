use crate::graph::direction::*;

pub const UNIQUE_CONNECTION_COUNT: usize = 15;

// Minecraft's Direction enum uses the following order:
// -Y, +Y, -Z, +Z, -X, +X
//
// This clashes with our direction format. because of this, it makes sense
// to just provide all 15 possible direction combinations as constants,
// calculated with the following:
// smaller_dir_ordinal * 8 + larger_dir_ordinal
pub const ARRAY_TO_BIT_IDX: [u8; UNIQUE_CONNECTION_COUNT] = [
    4,  // NEG_Y <-> NEG_X
    20, // NEG_Z <-> NEG_X
    2,  // NEG_Z <-> NEG_Y
    37, // POS_X <-> NEG_X
    5,  // POS_X <-> NEG_Y
    21, // POS_X <-> NEG_Z
    12, // POS_Y <-> NEG_X
    1,  // POS_Y <-> NEG_Y
    10, // POS_Y <-> NEG_Z
    13, // POS_Y <-> POS_X
    28, // POS_Z <-> NEG_X
    3,  // POS_Z <-> NEG_Y
    19, // POS_Z <-> NEG_Z
    29, // POS_Z <-> POS_X
    11, // POS_Z <-> POS_Y
];

// Returns the array index for the mutual connection between dir_1 and dir_2.
// The result of this function is undefined for dir_1 == dir_2.
// The layout of connections to indices can be found here:
// http://tinyurl.com/sodium-vis-triangle
pub const fn connection_index(dir_1: u8, dir_2: u8) -> usize {
    debug_assert!(dir_1 != dir_2);

    let dir_1_idx = to_index(dir_1);
    let dir_2_idx = to_index(dir_2);

    let (large_idx, small_idx) = if dir_1 > dir_2 {
        (dir_1_idx, dir_2_idx)
    } else {
        (dir_2_idx, dir_1_idx)
    };

    (large_idx * 4) + small_idx + (0b1100 >> large_idx) - 10
}
