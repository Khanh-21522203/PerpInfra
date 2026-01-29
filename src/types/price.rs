use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub, Mul, Div};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Price(i64);  // Fixed-point with 8 decimal places

impl Price {
    const MULTIPLIER: i64 = 100_000_000;  // 10^8

    pub fn from_i64(value: i64) -> Self {
        Price(value)
    }

    pub fn to_i64(&self) -> i64 {
        self.0
    }

    pub fn from_f64(value: f64) -> Self {
        Price((value * Self::MULTIPLIER as f64) as i64)
    }

    pub fn to_f64(&self) -> f64 {
        self.0 as f64 / Self::MULTIPLIER as f64
    }

    pub fn zero() -> Self {
        Price(0)
    }

    pub fn raw_value(&self) -> i64 {
        self.0
    }

    pub fn abs(&self) -> Self {
        Price(self.0.abs())
    }
}

impl Add for Price {
    type Output = Price;
    fn add(self, other: Price) -> Price {
        Price(self.0 + other.0)
    }
}

impl Sub for Price {
    type Output = Price;
    fn sub(self, other: Price) -> Price {
        Price(self.0 - other.0)
    }
}

impl Mul<i64> for Price {
    type Output = Price;
    fn mul(self, scalar: i64) -> Price {
        Price(self.0 * scalar)
    }
}

impl Div<i64> for Price {
    type Output = Price;
    fn div(self, scalar: i64) -> Price {
        Price(self.0 / scalar)
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_f64())
    }
}