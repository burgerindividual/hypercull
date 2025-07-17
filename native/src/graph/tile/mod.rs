pub mod angle;
pub mod fog;
pub mod frustum;
pub mod height;
pub mod traversal;

use core_simd::simd::prelude::*;
use core_simd::simd::ToBytes;
use std_float::StdFloat;

use super::visibility::*;
use super::{connection_index, u8x3, *};
use crate::bitset;
use crate::bitset::BitSet;

pub const SECTIONS_EMPTY: u8x64 = Simd::splat(0);
pub const SECTIONS_FILLED: u8x64 = Simd::splat(!0);

pub const OUT_OF_BOUNDS_BELOW_INCOMING_SECTIONS: u8x64 = Simd::from_array([
    255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0,
]);

pub fn section_index(coords: u8x3) -> u16 {
    debug_assert!(coords.simd_lt(Simd::splat(8)).all());

    ((coords[Y] as u16) << 6) | ((coords[Z] as u16) << 3) | (coords[X] as u16)
}

pub fn get_bit(sections: &u8x64, index: u16) -> bool {
    let array_idx = index as usize >> 3;
    let bit_idx = index as u8 & 0b111;
    let byte = unsafe { *sections.as_array().get_unchecked(array_idx) };
    byte.get_bit(bit_idx)
}

pub fn set_bit(sections: &mut u8x64, index: u16) {
    let array_idx = index as usize >> 3;
    let bit_idx = index as u8 & 0b111;
    let byte = unsafe { sections.as_mut_array().get_unchecked_mut(array_idx) };
    byte.set_bit(bit_idx);
}

pub fn clear_bit(sections: &mut u8x64, index: u16) {
    let array_idx = index as usize >> 3;
    let bit_idx = index as u8 & 0b111;
    let byte = unsafe { sections.as_mut_array().get_unchecked_mut(array_idx) };
    byte.clear_bit(bit_idx);
}

pub fn modify_bit(sections: &mut u8x64, index: u16, value: bool) {
    let array_idx = index as usize >> 3;
    let bit_idx = index as u8 & 0b111;
    let byte = unsafe { sections.as_mut_array().get_unchecked_mut(array_idx) };
    byte.modify_bit(bit_idx, value);
}

pub fn or_bit(sections: &mut u8x64, index: u16, value: bool) {
    let array_idx = index as usize >> 3;
    let bit_idx = index as u8 & 0b111;
    let byte = unsafe { sections.as_mut_array().get_unchecked_mut(array_idx) };
    byte.or_bit(bit_idx, value);
}

pub fn rasterize_rows(lower_bound: f32x8, upper_bound: f32x8) -> (f32x8, f32x8, u32x8, u32x8) {
    let lower_bound_ceil_clamped = lower_bound
        .ceil()
        .simd_clamp(Simd::splat(0.0), Simd::splat(8.0));
    let upper_bound_floor = upper_bound.floor();

    let lower_bound_shifts = unsafe {
        lower_bound_ceil_clamped
            .to_int_unchecked::<i32>()
            .cast::<u32>()
    };
    let upper_bound_shifts = unsafe {
        (upper_bound_floor.to_int_unchecked::<i32>() + Simd::splat(1))
            .simd_clamp(Simd::splat(0), Simd::splat(9))
            .cast::<u32>()
    };

    let lower_bound_mask = Simd::splat(!0) << lower_bound_shifts;
    let upper_bound_mask = !(Simd::splat(!0) << upper_bound_shifts);

    (
        lower_bound_ceil_clamped,
        upper_bound_floor,
        lower_bound_mask,
        upper_bound_mask,
    )
}

#[cfg(test)]
pub fn print_tile(sections: &u8x64) {
    for y in 0..8 {
        println!("↓Y{y}");
        for z in 0..8 {
            for x in 0..8 {
                print!(
                    "{}",
                    if get_bit(sections, section_index(Simd::from_xyz(x, y, z))) {
                        1_u8
                    } else {
                        0_u8
                    }
                );
            }
            println!(" Z{z}");
        }
    }
}

#[cfg(test)]
pub fn test_minimum_maximum(
    sane_visible_sections_min: &u8x64,
    sane_visible_sections_max: &u8x64,
    test_visible_sections: &u8x64,
) -> bool {
    let sections_outside_minimum =
        (test_visible_sections & sane_visible_sections_min) ^ sane_visible_sections_min;
    let sections_outside_maximum =
        (test_visible_sections | sane_visible_sections_max) ^ sane_visible_sections_max;

    let mut passed = true;

    if sections_outside_minimum != SECTIONS_EMPTY {
        println!("-------------- Below minimum");
        print_tile(&sections_outside_minimum);

        println!("-------------- Minimum");
        print_tile(&sane_visible_sections_min);

        passed = false;
    }

    if sections_outside_maximum != SECTIONS_EMPTY {
        println!("-------------- Outside maximum");
        print_tile(&sections_outside_maximum);

        println!("-------------- Maximum");
        print_tile(&sane_visible_sections_max);

        passed = false;
    }

    if !passed {
        println!("-------------- Test results");
        print_tile(&test_visible_sections);
    }

    passed
}

#[derive(Debug)]
pub struct Tile {
    // Only changes on section update
    pub connection_section_sets: [u8x64; UNIQUE_CONNECTION_COUNT],
    // Changes every time tile is processed
    pub outgoing_dir_section_sets: [u8x64; DIRECTION_COUNT],
    // visible_sections can be added back here to do visibility tests. for now, this is not
    // necessary
    #[cfg(debug_assertions)]
    pub processed: bool,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            // fully untraversable by default
            connection_section_sets: [SECTIONS_EMPTY; UNIQUE_CONNECTION_COUNT],
            outgoing_dir_section_sets: [SECTIONS_EMPTY; DIRECTION_COUNT],
            #[cfg(debug_assertions)]
            processed: false,
        }
    }
}

impl Tile {
    pub fn set_empty(&mut self) {
        self.outgoing_dir_section_sets = [SECTIONS_EMPTY; DIRECTION_COUNT];
    }
}
