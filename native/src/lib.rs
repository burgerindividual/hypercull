#![feature(portable_simd)]
#![allow(dead_code)]

mod bitset;
mod ffi;
mod graph;
mod math;
mod panic;

#[cfg(test)]
pub const TESTS_RANDOM_SEED: u64 = 0x1c41cf821df0e3a9;
