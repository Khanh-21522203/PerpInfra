use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FundingRate {
    value: i64,  // Rate * 10^8
}

impl FundingRate {
    const DECIMALS: u32 = 10;
    const MULTIPLIER: i64 = 10_000_000_000;

    pub fn from_i64(value: i64) -> Self {
        FundingRate { value }
    }

    pub fn to_i64(&self) -> i64 {
        self.value
    }

    pub fn from_f64(value: f64) -> Self {
        FundingRate {
            value: (value * Self::MULTIPLIER as f64).round() as i64
        }
    }

    pub fn to_f64(&self) -> f64 {
        self.value as f64 / Self::MULTIPLIER as f64
    }

    pub fn zero() -> Self {
        FundingRate { value: 0 }
    }

    pub fn clamp(self, min: FundingRate, max: FundingRate) -> Self {
        FundingRate {
            value: self.value.clamp(min.value, max.value)
        }
    }
}