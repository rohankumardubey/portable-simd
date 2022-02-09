//! Masks that take up full SIMD vector registers.

use super::MaskElement;
use crate::simd::intrinsics;
use crate::simd::{LaneCount, Simd, SupportedLaneCount};

#[repr(transparent)]
pub struct Mask<T, const LANES: usize>(Simd<T, LANES>)
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount;

impl<T, const LANES: usize> Copy for Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
}

impl<T, const LANES: usize> Clone for Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, const LANES: usize> PartialEq for Mask<T, LANES>
where
    T: MaskElement + PartialEq,
    LaneCount<LANES>: SupportedLaneCount,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T, const LANES: usize> PartialOrd for Mask<T, LANES>
where
    T: MaskElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T, const LANES: usize> Eq for Mask<T, LANES>
where
    T: MaskElement + Eq,
    LaneCount<LANES>: SupportedLaneCount,
{
}

impl<T, const LANES: usize> Ord for Mask<T, LANES>
where
    T: MaskElement + Ord,
    LaneCount<LANES>: SupportedLaneCount,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

// Used for bitmask bit order workaround
pub(crate) trait ReverseBits {
    fn reverse_bits(self) -> Self;
}

macro_rules! impl_reverse_bits {
    { $($int:ty),* } => {
        $(
        impl ReverseBits for $int {
            fn reverse_bits(self) -> Self { <$int>::reverse_bits(self) }
        }
        )*
    }
}

impl_reverse_bits! { u8, u16, u32, u64 }

impl<T, const LANES: usize> Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    pub fn splat(value: bool) -> Self {
        Self(Simd::splat(if value { T::TRUE } else { T::FALSE }))
    }

    #[inline]
    #[must_use = "method returns a new bool and does not mutate the original value"]
    pub unsafe fn test_unchecked(&self, lane: usize) -> bool {
        T::eq(self.0[lane], T::TRUE)
    }

    #[inline]
    pub unsafe fn set_unchecked(&mut self, lane: usize, value: bool) {
        self.0[lane] = if value { T::TRUE } else { T::FALSE }
    }

    #[inline]
    #[must_use = "method returns a new vector and does not mutate the original value"]
    pub fn to_int(self) -> Simd<T, LANES> {
        self.0
    }

    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    pub unsafe fn from_int_unchecked(value: Simd<T, LANES>) -> Self {
        Self(value)
    }

    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    pub fn convert<U>(self) -> Mask<U, LANES>
    where
        U: MaskElement,
    {
        unsafe { Mask(intrinsics::simd_cast(self.0)) }
    }

    // Safety: N must be the exact number of bytes required to hold the bitmask for this mask
    #[inline]
    #[must_use = "method returns a new array and does not mutate the original value"]
    pub unsafe fn to_bitmask_array<const N: usize>(self) -> [u8; N] {
        unsafe {
            let mut bitmask: [u8; N] = intrinsics::simd_bitmask(self.0);

            // There is a bug where LLVM appears to implement this operation with the wrong
            // bit order.
            // TODO fix this in a better way
            if cfg!(target_endian = "big") {
                for x in bitmask.as_mut() {
                    *x = x.reverse_bits();
                }
            }

            bitmask
        }
    }

    // Safety: N must be the exact number of bytes required to hold the bitmask for this mask
    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    pub unsafe fn from_bitmask_array<const N: usize>(mut bitmask: [u8; N]) -> Self {
        unsafe {
            // There is a bug where LLVM appears to implement this operation with the wrong
            // bit order.
            // TODO fix this in a better way
            if cfg!(target_endian = "big") {
                for x in bitmask.as_mut() {
                    *x = x.reverse_bits();
                }
            }

            Self::from_int_unchecked(intrinsics::simd_select_bitmask(
                bitmask,
                Self::splat(true).to_int(),
                Self::splat(false).to_int(),
            ))
        }
    }

    // Safety: U must be the integer with the exact number of bits required to hold the bitmask for
    // this mask
    #[inline]
    pub(crate) unsafe fn to_bitmask_integer<U: ReverseBits>(self) -> U {
        // Safety: caller must only return bitmask types
        let bitmask: U = unsafe { intrinsics::simd_bitmask(self.0) };

        // There is a bug where LLVM appears to implement this operation with the wrong
        // bit order.
        // TODO fix this in a better way
        if cfg!(target_endian = "big") {
            bitmask.reverse_bits()
        } else {
            bitmask
        }
    }

    // Safety: U must be the integer with the exact number of bits required to hold the bitmask for
    // this mask
    #[inline]
    pub(crate) unsafe fn from_bitmask_integer<U: ReverseBits>(bitmask: U) -> Self {
        // There is a bug where LLVM appears to implement this operation with the wrong
        // bit order.
        // TODO fix this in a better way
        let bitmask = if cfg!(target_endian = "big") {
            bitmask.reverse_bits()
        } else {
            bitmask
        };

        // Safety: caller must only pass bitmask types
        unsafe {
            Self::from_int_unchecked(intrinsics::simd_select_bitmask(
                bitmask,
                Self::splat(true).to_int(),
                Self::splat(false).to_int(),
            ))
        }
    }

    #[inline]
    #[must_use = "method returns a new bool and does not mutate the original value"]
    pub fn any(self) -> bool {
        unsafe { intrinsics::simd_reduce_any(self.to_int()) }
    }

    #[inline]
    #[must_use = "method returns a new vector and does not mutate the original value"]
    pub fn all(self) -> bool {
        unsafe { intrinsics::simd_reduce_all(self.to_int()) }
    }
}

impl<T, const LANES: usize> core::convert::From<Mask<T, LANES>> for Simd<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    fn from(value: Mask<T, LANES>) -> Self {
        value.0
    }
}

impl<T, const LANES: usize> core::ops::BitAnd for Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    type Output = Self;
    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    fn bitand(self, rhs: Self) -> Self {
        unsafe { Self(intrinsics::simd_and(self.0, rhs.0)) }
    }
}

impl<T, const LANES: usize> core::ops::BitOr for Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    type Output = Self;
    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    fn bitor(self, rhs: Self) -> Self {
        unsafe { Self(intrinsics::simd_or(self.0, rhs.0)) }
    }
}

impl<T, const LANES: usize> core::ops::BitXor for Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    type Output = Self;
    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    fn bitxor(self, rhs: Self) -> Self {
        unsafe { Self(intrinsics::simd_xor(self.0, rhs.0)) }
    }
}

impl<T, const LANES: usize> core::ops::Not for Mask<T, LANES>
where
    T: MaskElement,
    LaneCount<LANES>: SupportedLaneCount,
{
    type Output = Self;
    #[inline]
    #[must_use = "method returns a new mask and does not mutate the original value"]
    fn not(self) -> Self::Output {
        Self::splat(true) ^ self
    }
}
