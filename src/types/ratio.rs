use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub, Mul, Div};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Ratio(f64);

impl Ratio {
    pub fn from(value: f64) -> Self {
        Ratio(value)
    }

    pub fn to_f64(&self) -> f64 {
        self.0
    }

    pub fn zero() -> Self {
        Ratio(0.0)
    }
}

impl From<f64> for Ratio {
    fn from(value: f64) -> Self {
        Ratio(value)
    }
}

impl PartialEq for Ratio {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Add for Ratio {
    type Output = Ratio;
    fn add(self, other: Ratio) -> Ratio {
        Ratio(self.0 + other.0)
    }
}

impl Sub for Ratio {
    type Output = Ratio;
    fn sub(self, other: Ratio) -> Ratio {
        Ratio(self.0 - other.0)
    }
}

impl Mul for Ratio {
    type Output = Ratio;
    fn mul(self, other: Ratio) -> Ratio {
        Ratio(self.0 * other.0)
    }
}

impl Div for Ratio {
    type Output = Ratio;
    fn div(self, other: Ratio) -> Ratio {
        Ratio(self.0 / other.0)
    }
}