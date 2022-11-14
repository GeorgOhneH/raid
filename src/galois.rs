//! Implementation of GF(2^8): the finite field with 2^8 elements.

use core::mem;
use std::fmt::{Display, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

include!(concat!(env!("OUT_DIR"), "/table.rs"));

/// The field GF(2^8).
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Galois(u8);

impl Galois {
    pub fn new(v: u8) -> Self {
        Self(v)
    }
    pub fn zero() -> Self {
        Self(0)
    }
    pub fn one() -> Self {
        Self(1)
    }

    pub fn pow(self, n: usize) -> Self {
        Self(exp(self.0, n))
    }
}

pub fn zeros<const X: usize>() -> Box<[Galois; X]> {
    let zero = Box::new_zeroed();
    unsafe { zero.assume_init() }
}

pub fn zeros_raw<const X: usize>() -> Box<[u8; X]> {
    let zero = Box::new_zeroed();
    unsafe { zero.assume_init() }
}

pub fn from_fn<const X: usize, F>(mut cb: F) -> Box<[Galois; X]>
where
    F: FnMut(usize) -> Galois,
{
    let mut data: Box<[MaybeUninit<Galois>; X]> = unsafe { Box::new_uninit().assume_init() };
    for (i, elem) in (&mut data[..]).iter_mut().enumerate() {
        elem.write(cb(i));
    }
    unsafe { mem::transmute::<_, Box<[Galois; X]>>(data) }
}

pub fn from_fn_raw<const X: usize, F>(mut cb: F) -> Box<[u8; X]>
where
    F: FnMut(usize) -> u8,
{
    let mut data: Box<[MaybeUninit<u8>; X]> = unsafe { Box::new_uninit().assume_init() };
    for (i, elem) in (&mut data[..]).iter_mut().enumerate() {
        elem.write(cb(i));
    }
    unsafe { mem::transmute::<_, Box<[u8; X]>>(data) }
}

pub fn from_slice<const X: usize>(slice: &[Galois; X]) -> Box<[Galois; X]> {
    let mut data: Box<[MaybeUninit<Galois>; X]> = unsafe { Box::new_uninit().assume_init() };
    for (i, elem) in (&mut data[..]).iter_mut().enumerate() {
        elem.write(slice[i]);
    }
    unsafe { mem::transmute::<_, Box<[Galois; X]>>(data) }
}

pub fn from_slice_raw<const X: usize>(slice: &[u8; X]) -> Box<[Galois; X]> {
    let mut data: Box<[MaybeUninit<Galois>; X]> = unsafe { Box::new_uninit().assume_init() };
    for (i, elem) in (&mut data[..]).iter_mut().enumerate() {
        elem.write(Galois::new(slice[i]));
    }
    unsafe { mem::transmute::<_, Box<[Galois; X]>>(data) }
}

pub fn from_bytes<const X: usize>(bytes: Box<[u8; X]>) -> Box<[Galois; X]> {
    unsafe { core::mem::transmute(bytes) }
}

pub fn from_bytes_ref<const X: usize>(bytes: &[u8; X]) -> &[Galois; X] {
    unsafe { core::mem::transmute(bytes) }
}

pub fn as_bytes<const X: usize>(galois_slice: Box<[Galois; X]>) -> Box<[u8; X]> {
    unsafe { core::mem::transmute(galois_slice) }
}

pub fn as_bytes_ref<const X: usize>(galois_slice: &[Galois; X]) -> &[u8; X] {
    unsafe { core::mem::transmute(galois_slice) }
}

macro_rules! add_impl {
    ($($t:ty)*) => ($(
        impl Add for $t {
            type Output = $t;

            #[inline]
            fn add(self, other: $t) -> $t { Self(add(self.0, other.0)) }
        }

        forward_ref_binop! { impl Add, add for $t, $t }
    )*)
}

macro_rules! sub_impl {
    ($($t:ty)*) => ($(
        impl Sub for $t {
            type Output = $t;

            #[inline]
            fn sub(self, other: $t) -> $t { Self(sub(self.0, other.0)) }
        }

        forward_ref_binop! { impl Sub, sub for $t, $t }
    )*)
}

macro_rules! mul_impl {
    ($($t:ty)*) => ($(
        impl Mul for $t {
            type Output = $t;

            #[inline]
            fn mul(self, other: $t) -> $t { Self(mul(self.0, other.0)) }
        }

        forward_ref_binop! { impl Mul, mul for $t, $t }
    )*)
}

macro_rules! div_impl {
    ($($t:ty)*) => ($(
        impl Div for $t {
            type Output = $t;

            #[inline]
            fn div(self, other: $t) -> $t { Self(div(self.0, other.0)) }
        }

        forward_ref_binop! { impl Div, div for $t, $t }
    )*)
}

macro_rules! forward_ref_binop {
    (impl $imp:ident, $method:ident for $t:ty, $u:ty) => {
        impl<'a> $imp<$u> for &'a $t {
            type Output = <$t as $imp<$u>>::Output;

            #[inline]
            fn $method(self, other: $u) -> <$t as $imp<$u>>::Output {
                $imp::$method(*self, other)
            }
        }

        impl<'a> $imp<&'a $u> for $t {
            type Output = <$t as $imp<$u>>::Output;

            #[inline]
            fn $method(self, other: &'a $u) -> <$t as $imp<$u>>::Output {
                $imp::$method(self, *other)
            }
        }

        impl<'a, 'b> $imp<&'a $u> for &'b $t {
            type Output = <$t as $imp<$u>>::Output;

            #[inline]
            fn $method(self, other: &'a $u) -> <$t as $imp<$u>>::Output {
                $imp::$method(*self, *other)
            }
        }
        impl<'a> $imp<$u> for &'a mut $t {
            type Output = <$t as $imp<$u>>::Output;

            #[inline]
            fn $method(self, other: $u) -> <$t as $imp<$u>>::Output {
                $imp::$method(*self, other)
            }
        }

        impl<'a, 'b> $imp<&'a $u> for &'b mut $t {
            type Output = <$t as $imp<$u>>::Output;

            #[inline]
            fn $method(self, other: &'a $u) -> <$t as $imp<$u>>::Output {
                $imp::$method(*self, *other)
            }
        }
    };
}

add_impl!(Galois);
sub_impl!(Galois);
mul_impl!(Galois);
div_impl!(Galois);

macro_rules! add_assign_impl {
    ($($t:ty)+) => ($(
        impl AddAssign for $t {
            #[inline]
            fn add_assign(&mut self, other: $t) { *self = *self + other }
        }

        forward_ref_op_assign! { impl AddAssign, add_assign for $t, $t }
    )+)
}

macro_rules! sub_assign_impl {
    ($($t:ty)+) => ($(
        impl SubAssign for $t {
            #[inline]
            fn sub_assign(&mut self, other: $t) { *self = *self - other }
        }

        forward_ref_op_assign! { impl SubAssign, sub_assign for $t, $t }
    )+)
}

macro_rules! mul_assign_impl {
    ($($t:ty)+) => ($(
        impl MulAssign for $t {
            #[inline]
            fn mul_assign(&mut self, other: $t) { *self = *self * other }
        }

        forward_ref_op_assign! { impl MulAssign, mul_assign for $t, $t }
    )+)
}

macro_rules! div_assign_impl {
    ($($t:ty)+) => ($(
        impl DivAssign for $t {
            #[inline]
            fn div_assign(&mut self, other: $t) { *self = *self / other }
        }

        forward_ref_op_assign! { impl DivAssign, div_assign for $t, $t }
    )+)
}

macro_rules! forward_ref_op_assign {
    (impl $imp:ident, $method:ident for $t:ty, $u:ty) => {
        impl $imp<&$u> for $t {
            #[inline]
            fn $method(&mut self, other: &$u) {
                $imp::$method(self, *other);
            }
        }
    };
}

add_assign_impl!(Galois);
sub_assign_impl!(Galois);
mul_assign_impl!(Galois);
div_assign_impl!(Galois);

impl Display for Galois {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// Add two elements.
pub fn add(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Subtract `b` from `a`.
pub fn sub(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Multiply two elements.
pub fn mul(a: u8, b: u8) -> u8 {
    MUL_TABLE[a as usize][b as usize]
}

/// Divide one element by another. `b`, the divisor, may not be 0.
pub fn div(a: u8, b: u8) -> u8 {
    if a == 0 {
        0
    } else if b == 0 {
        panic!("Divisor is 0")
    } else {
        let log_a = LOG_TABLE[a as usize];
        let log_b = LOG_TABLE[b as usize];
        let mut log_result = log_a as isize - log_b as isize;
        if log_result < 0 {
            log_result += 255;
        }
        EXP_TABLE[log_result as usize]
    }
}

/// Compute a^n.
pub fn exp(a: u8, n: usize) -> u8 {
    if n == 0 {
        1
    } else if a == 0 {
        0
    } else {
        let log_a = LOG_TABLE[a as usize];
        let mut log_result = log_a as usize * n;
        while 255 <= log_result {
            log_result -= 255;
        }
        EXP_TABLE[log_result]
    }
}
