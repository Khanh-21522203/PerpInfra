use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use lazy_static::lazy_static;
use uuid::Uuid;
use crate::types::ids::UserId;

pub mod events;
pub mod event_log;
pub mod utils;
pub mod types;
pub mod price_infra;
pub mod matching;
pub mod risk;
pub mod funding;
pub mod liquidation;
pub mod settlement;
pub mod invariants;
pub mod replay;
pub mod core;
pub mod error;
pub mod config;
pub mod observability;
pub mod interfaces;

lazy_static! {
    pub static ref KILL_SWITCH: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

// Liquidation engine user ID (system account)
pub const LIQUIDATION_ENGINE_USER_ID: UserId = UserId(Uuid::from_u128(0));

// Snapshot version
pub const SNAPSHOT_VERSION: u32 = 1;

// Funding rate multiplier
pub const FUNDING_RATE_MULTIPLIER: i64 = 10_000_000_000; // 10^10

fn main() {
    println!("Hello, world!");
}
