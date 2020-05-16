#![allow(clippy::suspicious_arithmetic_impl)]

use num_traits::{Float, One, Zero};
/// A fully self contained 8 bit float
use std::ops::{Add, Mul, Neg, Sub};
use std::{cmp::Ordering};

/// How much is the exponent for an F8 biased by?
/// Heavily favoring representing numbers closer to 0
pub const BIAS: u8 = 2;

/// 8 bit floating point number
/// Repr: 1(sign) | 3(exp) | 4(significand)
/// 1 = neg, 0 = pos | exp - BIAS | significand
/// Magnitude = 2^(exp - BIAS) * significand
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct F8(u8);

const SIGN_MASK: u8 = 0b1000_0000;
const EXP_MASK: u8 = 0b0111_0000;
const SIGNIF_MASK: u8 = 0b0000_1111;

fn normalize(mut exp: u8, mut signif: u8) -> (u8, u8) {
  if exp >= 0b111 {
    // infinity
    return (0b1111, 0);
  }
  while signif > 0b1111 {
    if exp == 0 {
      return (0b1111, 1);
    }
    exp -= 1;
    signif >>= 1;
  }
  (exp, signif)
}

impl Zero for F8 {
  #[inline]
  fn zero() -> Self { F8(0) }
  #[inline]
  fn is_zero(&self) -> bool { self.0 == 0 }
}

const F8_ONE: F8 = F8::new(0, BIAS, 1);
impl One for F8 {
  #[inline]
  fn one() -> Self { F8_ONE }
  #[inline]
  fn is_one(&self) -> bool { self.0 == F8_ONE.0 }
}

impl Add for F8 {
  type Output = Self;
  fn add(self, o: Self) -> Self::Output {
    let mut e0 = self.exponent();
    let mut e1 = o.exponent();
    let mut m0 = self.significand();
    let mut m1 = o.significand();
    match e0.cmp(&e1) {
      Ordering::Equal => (),
      Ordering::Less => while e0 < e1 {
        m0 <<= 1;
        e0 += 1;
      },
      Ordering::Greater => while e1 < e0 {
        m1 <<= 1;
        e1 += 1;
      },
    }
    assert_eq!(e0, e1, "Exponents not equal");
    match (self.is_sign_positive(), o.is_sign_positive()) {
      (true, true) => {
        let (exp, signif) = normalize(e0, m0 + m1);
        F8::new(0, exp, signif)
      },
      (false, false) => {
        let (exp, signif) = normalize(e0, m0 + m1);
        F8::new(1, exp, signif)
      },
      // self is positive, other is negative
      (true, false) => match m0.cmp(&m1) {
        Ordering::Equal => F8(0),
        Ordering::Greater => {
          let (exp, signif) = normalize(e0, m0 - m1);
          F8::new(0, exp, signif)
        },
        Ordering::Less => {
          let (exp, signif) = normalize(e0, m1 - m0);
          F8::new(1, exp, signif)
        },
      },
      (false, true) => match m1.cmp(&m0) {
        Ordering::Equal => F8(0),
        Ordering::Greater => {
          let (exp, signif) = normalize(e0, m1 - m0);
          F8::new(0, exp, signif)
        },
        Ordering::Less => {
          let (exp, signif) = normalize(e0, m0 - m1);
          F8::new(1, exp, signif)
        },
      },
    }
  }
}

impl Neg for F8 {
  type Output = F8;
  fn neg(self) -> Self::Output { F8(self.0 ^ SIGN_MASK) }
}

impl Sub for F8 {
  type Output = F8;
  #[inline]
  fn sub(self, rhs: Self) -> Self::Output { self + (-rhs) }
}

impl Mul for F8 {
  type Output = F8;
  #[inline]
  fn mul(self, rhs: Self) -> Self::Output {
    let sign = (self.is_sign_negative() ^ rhs.is_sign_negative()) as u8;
    let exp = self.exponent() + rhs.exponent() - BIAS;
    let signif = self.significand() * rhs.significand();
    let (exp, signif) = normalize(exp, signif);
    F8::new(sign, exp, signif)
  }
}
impl F8 {
  pub const fn new(sign: u8, exp: u8, signif: u8) -> Self {
    F8(sign << 7 | ((exp << 4) & EXP_MASK) | (signif & SIGNIF_MASK))
  }
  pub const fn is_sign_positive(self) -> bool { self.0 & SIGN_MASK == 0 }
  pub const fn is_sign_negative(self) -> bool { self.0 & SIGN_MASK != 0 }
  pub const fn exponent(self) -> u8 { (self.0 & EXP_MASK) >> 4 }
  pub const fn significand(self) -> u8 { self.0 & SIGNIF_MASK }
  pub fn signum(self) -> i8 {
    if self.significand() == 0 {
      return 0;
    }
    if self.is_sign_positive() {
      1
    } else {
      -1
    }
  }
  pub fn v(self) -> f32 {
    let pos = self.is_sign_positive();
    let v = 2f32.powi(self.exponent() as i32 - BIAS as i32) * (self.significand() as f32);
    if pos {
      v
    } else {
      -v
    }
  }
  pub fn integer_decode(self) -> (u8, i8, i8) {
    (
      self.significand(),
      self.exponent() as i8 - BIAS as i8,
      self.signum(),
    )
  }
  pub fn try_from(f: f32) -> Option<Self> {
    let (mut signif, mut exp, _) = f.integer_decode();
    let sign = f.is_sign_negative() as u8;
    while signif & 1 != 1 {
      signif >>= 1;
      exp += 1;
    }
    let exp = exp + (BIAS as i16);
    if exp < 0 {
      return None;
    }
    let (exp, signif) = normalize(exp as u8, signif as u8);
    Some(F8::new(sign, exp, signif))
  }
  pub fn approx_from(f: f32) -> Self {
    let (mut signif, mut exp, _) = f.integer_decode();
    let sign = f.is_sign_negative() as u8;
    while exp < -(BIAS as i16) {
      signif >>= 1;
      exp += 1;
    }
    let (exp, signif) = normalize((exp + (BIAS as i16)) as u8, signif as u8);
    F8::new(sign, exp, signif)
  }
}

impl From<F8> for f32 {
  fn from(f8: F8) -> f32 { f8.v() }
}
