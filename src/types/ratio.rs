use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub, Mul, Div};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Ratio {
    value: i64,  // Ratio * 10^8
}

const RATIO_DECIMALS: u32 = 8;
const RATIO_MULTIPLIER: i64 = 100_000_000;  // 10^8

impl Ratio {
    /// Create from floating-point value (for configuration/initialization only)
    /// Uses banker's rounding for determinism
    pub fn from_f64(value: f64) -> Self {
        Ratio {
            value: (value * RATIO_MULTIPLIER as f64).round() as i64,
        }
    }

    /// Create from raw fixed-point value
    pub fn from_raw(value: i64) -> Self {
        Ratio { value }
    }

    /// Get raw fixed-point value
    pub fn raw_value(&self) -> i64 {
        self.value
    }

    /// Convert to f64 for display purposes only
    pub fn to_f64(&self) -> f64 {
        self.value as f64 / RATIO_MULTIPLIER as f64
    }

    pub fn zero() -> Self {
        Ratio { value: 0 }
    }

    pub fn one() -> Self {
        Ratio { value: RATIO_MULTIPLIER }
    }

    /// Check if ratio is less than 1.0 (for liquidation checks)
    pub fn is_below_one(&self) -> bool {
        self.value < RATIO_MULTIPLIER
    }
}

impl From<f64> for Ratio {
    fn from(value: f64) -> Self {
        Ratio::from_f64(value)
    }
}

impl Add for Ratio {
    type Output = Ratio;
    fn add(self, other: Ratio) -> Ratio {
        Ratio { value: self.value + other.value }
    }
}

impl Sub for Ratio {
    type Output = Ratio;
    fn sub(self, other: Ratio) -> Ratio {
        Ratio { value: self.value - other.value }
    }
}

impl Mul for Ratio {
    type Output = Ratio;
    /// Multiplication with proper scaling to maintain precision
    fn mul(self, other: Ratio) -> Ratio {
        // Use i128 to prevent overflow during multiplication
        let result = (self.value as i128 * other.value as i128) / RATIO_MULTIPLIER as i128;
        Ratio { value: result as i64 }
    }
}

impl Div for Ratio {
    type Output = Ratio;
    /// Division with proper scaling to maintain precision
    fn div(self, other: Ratio) -> Ratio {
        if other.value == 0 {
            panic!("Division by zero in Ratio");
        }
        // Scale numerator first to maintain precision
        let result = (self.value as i128 * RATIO_MULTIPLIER as i128) / other.value as i128;
        Ratio { value: result as i64 }
    }
}