use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub, Mul, Div, Neg};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Balance(i64);  // Signed balance in base units

impl Balance {
    pub fn from_i64(value: i64) -> Self {
        Balance(value)
    }

    pub fn to_i64(&self) -> i64 {
        self.0
    }

    pub fn from_f64(value: f64) -> Self {
        Balance(value as i64)
    }

    pub fn to_f64(&self) -> f64 {
        self.0 as f64
    }

    pub fn zero() -> Self {
        Balance(0)
    }

    pub fn abs(&self) -> Self {
        Balance(self.0.abs())
    }
}

impl Add for Balance {
    type Output = Balance;
    fn add(self, other: Balance) -> Balance {
        Balance(self.0 + other.0)
    }
}

impl Sub for Balance {
    type Output = Balance;
    fn sub(self, other: Balance) -> Balance {
        Balance(self.0 - other.0)
    }
}

impl Mul<Balance> for Balance {
    type Output = Balance;
    fn mul(self, other: Balance) -> Balance {
        Balance(self.0 * other.0)
    }
}

impl Div<Balance> for Balance {
    type Output = Balance;
    fn div(self, other: Balance) -> Balance {
        Balance(self.0 / other.0)
    }
}

impl fmt::Display for Balance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Neg for Balance {
    type Output = Balance;
    fn neg(self) -> Balance {
        Balance(-self.0)
    }
}