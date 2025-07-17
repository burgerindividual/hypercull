#![allow(non_camel_case_types)]

use core_simd::simd::prelude::*;
use core_simd::simd::*;
use std_float::StdFloat;

pub const X: usize = 0;
pub const Y: usize = 1;
pub const Z: usize = 2;
pub const W: usize = 3;

pub const F32_SIGN_BIT: u32 = 1 << 31;

// the most common non-po2 length we use is 3, so we create shorthands for it
pub type i8x3 = Simd<i8, 3>;
pub type i16x3 = Simd<i16, 3>;
pub type i32x3 = Simd<i32, 3>;
pub type u32x3 = Simd<u32, 3>;

pub type u8x3 = Simd<u8, 3>;
pub type u16x3 = Simd<u16, 3>;

pub type f32x3 = Simd<f32, 3>;
pub type f64x3 = Simd<f64, 3>;

pub trait Coords3<T> {
    fn from_xyz(x: T, y: T, z: T) -> Self;
}

impl<T> Coords3<T> for Simd<T, 3>
where
    T: SimdElement,
{
    fn from_xyz(x: T, y: T, z: T) -> Self {
        Simd::from_array([x, y, z])
    }
}

impl<T> Coords3<bool> for Mask<T, 3>
where
    T: MaskElement,
{
    fn from_xyz(x: bool, y: bool, z: bool) -> Self {
        Mask::from_array([x, y, z])
    }
}

pub trait MulAddFast {
    fn mul_add_fast(self, mul: Self, add: Self) -> Self;
}

impl<const LANES: usize> MulAddFast for Simd<f32, LANES>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn mul_add_fast(self, mul: Self, add: Self) -> Self {
        // this could probably have better detection
        if cfg!(target_feature = "fma") || cfg!(target_feature = "neon") {
            self.mul_add(mul, add)
        } else {
            self * mul + add
        }
    }
}

impl<const LANES: usize> MulAddFast for Simd<f64, LANES>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn mul_add_fast(self, mul: Self, add: Self) -> Self {
        // this could probably have better detection
        if cfg!(target_feature = "fma") || cfg!(target_feature = "neon") {
            self.mul_add(mul, add)
        } else {
            self * mul + add
        }
    }
}

impl MulAddFast for f32 {
    fn mul_add_fast(self, mul: Self, add: Self) -> Self {
        // this could probably have better detection
        if cfg!(target_feature = "fma") || cfg!(target_feature = "neon") {
            self.mul_add(mul, add)
        } else {
            self * mul + add
        }
    }
}

impl MulAddFast for f64 {
    fn mul_add_fast(self, mul: Self, add: Self) -> Self {
        // this could probably have better detection
        if cfg!(target_feature = "fma") || cfg!(target_feature = "neon") {
            self.mul_add(mul, add)
        } else {
            self * mul + add
        }
    }
}

pub trait SignFast: SimdFloat {
    fn is_sign_positive_fast(self) -> Self::Mask;
    fn is_sign_negative_fast(self) -> Self::Mask;
}

impl<const LANES: usize> SignFast for Simd<f32, LANES>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn is_sign_positive_fast(self) -> Self::Mask {
        (self.to_bits() & Simd::splat(F32_SIGN_BIT)).simd_eq(Simd::splat(0))
    }

    fn is_sign_negative_fast(self) -> Self::Mask {
        (self.to_bits() & Simd::splat(F32_SIGN_BIT)).simd_eq(Simd::splat(F32_SIGN_BIT))
    }
}

pub trait SimdOrdFast {
    fn simd_min_fast(self, other: Self) -> Self;
    fn simd_max_fast(self, other: Self) -> Self;
    fn simd_clamp_fast(self, min: Self, max: Self) -> Self;
}

impl<const LANES: usize> SimdOrdFast for Simd<f32, LANES>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn simd_min_fast(self, other: Self) -> Self {
        self.simd_lt(other).select(self, other)
    }

    fn simd_max_fast(self, other: Self) -> Self {
        self.simd_gt(other).select(self, other)
    }

    fn simd_clamp_fast(self, min: Self, max: Self) -> Self {
        let mut x = self;
        x = x.simd_lt(min).select(min, x);
        x = x.simd_gt(max).select(max, x);
        x
    }
}

pub trait RemEuclid {
    fn rem_euclid(self, rhs: Self) -> Self;
}

impl<const LANES: usize> RemEuclid for Simd<i32, LANES>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    fn rem_euclid(self, rhs: Self) -> Self {
        let lhs_f = self.cast::<f64>();
        let rhs_f = rhs.cast::<f64>();
        let div = lhs_f / rhs_f;
        let floor = div.floor();
        let mod_f = lhs_f - (floor * rhs_f);
        unsafe { mod_f.to_int_unchecked() }
    }
}

pub const fn concat_swizzle_pattern<const LEN: usize>() -> [usize; LEN] {
    let mut array = [0; LEN];

    let mut i = 0;
    while i < LEN {
        array[i] = i;
        i += 1;
    }

    array
}
