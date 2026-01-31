use crate::types::balance::Balance;
use crate::types::price::Price;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Quantity(i64);  // Base units

impl Quantity {
    const MULTIPLIER: i64 = 100_000_000;

    pub fn from_i64(value: i64) -> Self {
        Quantity(value)
    }

    pub fn to_i64(&self) -> i64 {
        self.0
    }

    pub fn from_f64(value: f64) -> Self {
        Quantity((value * Self::MULTIPLIER as f64).round() as i64)
    }

    pub fn to_f64(&self) -> f64 {
        self.0 as f64 / Self::MULTIPLIER as f64
    }

    pub fn zero() -> Self {
        Quantity(0)
    }

    pub fn raw_value(&self) -> i64 {
        self.0
    }

    pub fn abs(&self) -> Self {
        Quantity(self.0.abs())
    }

    pub fn min(self, other: Self) -> Self {
        Quantity(self.0.min(other.0))
    }
}

impl Add for Quantity {
    type Output = Quantity;
    fn add(self, other: Quantity) -> Quantity {
        Quantity(self.0 + other.0)
    }
}

impl Sub for Quantity {
    type Output = Quantity;
    fn sub(self, other: Quantity) -> Quantity {
        Quantity(self.0 - other.0)
    }
}

impl Mul<Price> for Quantity {
    type Output = Balance;
    fn mul(self, price: Price) -> Balance {
        Balance::from_i64(self.0 * price.to_i64())
    }
}

impl Sum for Quantity {
    fn sum<I: Iterator<Item = Quantity>>(iter: I) -> Self {
        iter.fold(Quantity::zero(), |acc, x| acc + x)
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}