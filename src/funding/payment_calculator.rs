use crate::events::funding::FundingPayment;
use crate::types::balance::Balance;
use crate::types::funding_rate::FundingRate;
use crate::types::position::Position;
use crate::types::price::Price;
use crate::types::quantity::Quantity;

pub struct FundingPaymentCalculator;

impl FundingPaymentCalculator {
    /// Calculate funding payment for a position
    /// Payment = position_size * mark_price * funding_rate
    /// Positive = receive, Negative = pay
    pub fn calculate_payment(
        position: &Position,
        mark_price: Price,
        funding_rate: FundingRate,
    ) -> Balance {
        if position.is_flat() {
            return Balance::zero();
        }

        let notional = Quantity::from_i64(position.size.abs()) * mark_price;
        let payment = notional.to_f64() * funding_rate.to_f64();

        // Long positions pay when rate is positive, receive when negative
        // Short positions receive when rate is positive, pay when negative
        let signed_payment = if position.is_long() {
            -payment
        } else {
            payment
        };

        Balance::from_f64(signed_payment)
    }

    /// Calculate all funding payments for a market
    pub fn calculate_all_payments(
        positions: &[Position],
        mark_price: Price,
        funding_rate: FundingRate,
    ) -> Vec<FundingPayment> {
        positions.iter()
            .filter(|p| !p.is_flat())
            .map(|p| FundingPayment {
                user_id: p.user_id,
                position_size: Quantity::from_i64(p.size),
                payment: Self::calculate_payment(p, mark_price, funding_rate),
            })
            .collect()
    }

    /// Verify zero-sum property
    pub fn verify_zero_sum(payments: &[FundingPayment]) -> bool {
        let sum: i64 = payments.iter()
            .map(|p| p.payment.to_i64())
            .sum();

        // Allow small rounding error (< 1 unit)
        sum.abs() < 1
    }
}