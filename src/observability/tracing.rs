use tracing::Span;
use crate::types::ids::OrderId;

pub fn trace_order_processing(order_id: &OrderId) -> Span {
    tracing::info_span!(
        "order_processing",
        order_id = ?order_id,
    )
}

pub fn trace_matching(order_id: &OrderId) -> Span {
    tracing::info_span!(
        "matching",
        order_id = ?order_id,
    )
}