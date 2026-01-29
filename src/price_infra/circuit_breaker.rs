use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::events::price::PriceSnapshot;
use crate::error::{Error, Result, CircuitBreakerReason};
use crate::types::price::Price;
use crate::utils::helper::alert_operations_team_critical;

pub struct PriceCircuitBreaker {
    active: Arc<AtomicBool>,
    price_movement_threshold: f64,
    mark_index_deviation_threshold: f64,
    last_price: Option<Price>,
}

impl PriceCircuitBreaker {
    pub fn new() -> Self {
        PriceCircuitBreaker {
            active: Arc::new(AtomicBool::new(false)),
            price_movement_threshold: 0.10,  // 10%
            mark_index_deviation_threshold: 0.05,  // 5%
            last_price: None,
        }
    }

    pub fn check(&mut self, snapshot: &PriceSnapshot) -> Result<()> {
        // Check 1: Price movement
        if let Some(last) = self.last_price {
            let movement = (snapshot.index_price - last).abs() / last.to_i64();
            if movement.to_f64() > self.price_movement_threshold {
                self.trigger(CircuitBreakerReason::PriceMovement(movement.to_f64()))?;
            }
        }

        // Check 2: Mark-index deviation
        let deviation = (snapshot.mark_price - snapshot.index_price).abs() / snapshot.index_price.to_i64();
        if deviation.to_f64() > self.mark_index_deviation_threshold {
            self.trigger(CircuitBreakerReason::MarkIndexDeviation(deviation.to_f64()))?;
        }

        // Check 3: All sources stale
        if snapshot.staleness_flags.iter().all(|&stale| stale) {
            self.trigger(CircuitBreakerReason::AllSourcesStale)?;
        }

        self.last_price = Some(snapshot.index_price);
        Ok(())
    }

    fn trigger(&self, reason: CircuitBreakerReason) -> Result<()> {
        self.active.store(true, Ordering::SeqCst);
        tracing::error!("Price circuit breaker triggered: {:?}", reason);

        // Alert operations team
        alert_operations_team_critical(
            format!("Price circuit breaker triggered: {:?}", reason)
        );

        Err(Error::CircuitBreakerTriggered(reason))
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("Price circuit breaker reset");
    }
}