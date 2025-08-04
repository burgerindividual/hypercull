// Include the core_simd prelude, as this prelude is mainly an extension of that
pub use core_simd::simd::prelude::*;
// Include StdFloat because it goes hand-in-hand with the core_simd functions
pub use std_float::StdFloat;

// the most common non-po2 length we use is 3, so we create shorthands for it
pub type i8x3 = Simd<i8, 3>;
pub type i16x3 = Simd<i16, 3>;
pub type i32x3 = Simd<i32, 3>;
pub type u32x3 = Simd<u32, 3>;

pub type u8x3 = Simd<u8, 3>;
pub type u16x3 = Simd<u16, 3>;

pub type f32x3 = Simd<f32, 3>;
pub type f64x3 = Simd<f64, 3>;

pub use super::{Coords3, MulAddFast, RemEuclid, SignFast, SimdOrdFast, W, X, Y, Z};
