use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;

lazy_static! {
    static ref ORDER_PROCESSOR_HALTED: AtomicBool = AtomicBool::new(false);
    static ref LIQUIDATION_ENGINE_HALTED: AtomicBool = AtomicBool::new(false);
    static ref FUNDING_ENGINE_HALTED: AtomicBool = AtomicBool::new(false);
}

pub fn halt_order_processor() {
    ORDER_PROCESSOR_HALTED.store(true, Ordering::SeqCst);
    tracing::warn!("Order processor HALTED");
}

pub fn halt_liquidation_engine() {
    LIQUIDATION_ENGINE_HALTED.store(true, Ordering::SeqCst);
    tracing::warn!("Liquidation engine HALTED");
}

pub fn halt_funding_engine() {
    FUNDING_ENGINE_HALTED.store(true, Ordering::SeqCst);
    tracing::warn!("Funding engine HALTED");
}

pub fn resume_order_processor() {
    ORDER_PROCESSOR_HALTED.store(false, Ordering::SeqCst);
    tracing::info!("Order processor RESUMED");
}

pub fn resume_liquidation_engine() {
    LIQUIDATION_ENGINE_HALTED.store(false, Ordering::SeqCst);
    tracing::info!("Liquidation engine RESUMED");
}

pub fn resume_funding_engine() {
    FUNDING_ENGINE_HALTED.store(false, Ordering::SeqCst);
    tracing::info!("Funding engine RESUMED");
}

pub fn is_order_processor_halted() -> bool {
    ORDER_PROCESSOR_HALTED.load(Ordering::SeqCst)
}

pub fn is_liquidation_engine_halted() -> bool {
    LIQUIDATION_ENGINE_HALTED.load(Ordering::SeqCst)
}

pub fn is_funding_engine_halted() -> bool {
    FUNDING_ENGINE_HALTED.load(Ordering::SeqCst)
}