// Directions and Direction Sets are represented as raw u8s to work seamlessly
// with the current state of const generics.

use std::num::NonZero;

pub const NEG_X: u8 = 0b000001;
pub const NEG_Y: u8 = 0b000010;
pub const NEG_Z: u8 = 0b000100;
pub const POS_X: u8 = 0b001000;
pub const POS_Y: u8 = 0b010000;
pub const POS_Z: u8 = 0b100000;
pub const ALL_DIRECTIONS: u8 = 0b111111;
pub const DIRECTION_COUNT: usize = 6;

pub const fn to_index(direction: u8) -> usize {
    unsafe { NonZero::new_unchecked(direction) }.trailing_zeros() as usize
}

pub const fn index_dir_to_axis(dir_index: usize) -> usize {
    dir_index % 3
}

pub const fn all_except(direction_set: u8) -> u8 {
    ALL_DIRECTIONS & !direction_set
}

pub const fn opposite(direction_set: u8) -> u8 {
    ((direction_set & 0b111) << 3) | (direction_set >> 3)
}

/// Removes a direction from the direction set, and returns it
pub const fn take_one(direction_set: &mut u8) -> u8 {
    let prev_set = *direction_set;

    // removes the lowest bit in the bit set
    *direction_set &= *direction_set - 1;

    // the difference between the old set and the new set is the removed bit.
    // we return that bit.
    *direction_set ^ prev_set
}

#[cfg(test)]
pub const fn to_str(direction: u8) -> &'static str {
    match direction {
        POS_X => "+X",
        POS_Y => "+Y",
        POS_Z => "+Z",
        NEG_X => "-X",
        NEG_Y => "-Y",
        NEG_Z => "-Z",
        _ => unreachable!(),
    }
}
