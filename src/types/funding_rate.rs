use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FundingRate(f64);

impl FundingRate {
    pub fn from_f64(value: f64) -> Self {
        FundingRate(value)
    }

    pub fn to_f64(&self) -> f64 {
        self.0
    }

    pub fn to_fixed_point(&self) -> i64 {
        (self.0 * crate::FUNDING_RATE_MULTIPLIER as f64) as i64
    }

    pub fn from_fixed_point(value: i64) -> Self {
        FundingRate(value as f64 / crate::FUNDING_RATE_MULTIPLIER as f64)
    }
}