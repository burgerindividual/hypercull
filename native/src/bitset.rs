use std::hint::assert_unchecked;
use std::ops::{BitAnd, BitOr, Not, Shl, Shr};

pub const fn from_elements_u8(elements: &[u8]) -> u8 {
    let mut combined = 0;

    let mut i = 0;
    while i < elements.len() {
        combined |= elements[i];
        i += 1;
    }

    combined
}

pub const fn contains_u8(bitset: u8, other_bitset: u8) -> bool {
    bitset & other_bitset == other_bitset
}

pub const fn contains_u16(bitset: u16, other_bitset: u16) -> bool {
    bitset & other_bitset == other_bitset
}

pub trait BitSet {
    fn get_bit(self, idx: u8) -> bool;
    fn set_bit(&mut self, idx: u8);
    fn clear_bit(&mut self, idx: u8);
    fn modify_bit(&mut self, idx: u8, value: bool);
    fn or_bit(&mut self, idx: u8, value: bool);
    fn and_bit(&mut self, idx: u8, value: bool);
}

impl<
        T: Copy
            + From<bool>
            + Shr<u8, Output = T>
            + Shl<u8, Output = T>
            + BitAnd<T, Output = T>
            + BitOr<T, Output = T>
            + PartialEq<T>
            + Not<Output = T>,
    > BitSet for T
{
    fn get_bit(self, idx: u8) -> bool {
        unsafe { assert_unchecked(idx < (size_of::<T>() as u8 * 8)) };
        ((self >> idx) & T::from(true)) != T::from(false)
    }

    fn set_bit(&mut self, idx: u8) {
        unsafe { assert_unchecked(idx < (size_of::<T>() as u8 * 8)) };
        *self = *self | (T::from(true) << idx);
    }

    fn clear_bit(&mut self, idx: u8) {
        unsafe { assert_unchecked(idx < (size_of::<T>() as u8 * 8)) };
        *self = *self & !(T::from(true) << idx);
    }

    fn modify_bit(&mut self, idx: u8, value: bool) {
        unsafe { assert_unchecked(idx < (size_of::<T>() as u8 * 8)) };
        *self = (*self & !(T::from(true) << idx)) | (T::from(value) << idx);
    }

    fn or_bit(&mut self, idx: u8, value: bool) {
        unsafe { assert_unchecked(idx < (size_of::<T>() as u8 * 8)) };
        *self = *self | (T::from(value) << idx);
    }

    fn and_bit(&mut self, idx: u8, value: bool) {
        unsafe { assert_unchecked(idx < (size_of::<T>() as u8 * 8)) };
        *self = *self & (T::from(value) << idx);
    }
}
