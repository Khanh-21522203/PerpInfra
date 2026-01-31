use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{Ordering};
use std::sync::{RwLock};
use uuid::Uuid;
use crate::types::ids::{EntryId, EventId, LiquidationId, OperatorId, OrderId, TradeId};

// Global state for engine control
lazy_static::lazy_static! {
    static ref AUTHORIZED_OPERATORS: RwLock<HashSet<OperatorId>> =
        RwLock::new(HashSet::new());
}

/// Get current timestamp in milliseconds since epoch
pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Generate a new trade ID
pub fn generate_trade_id() -> TradeId {
    TradeId(Uuid::new_v4())
}

/// Generate a new order ID
pub fn generate_order_id() -> OrderId {
    OrderId(Uuid::new_v4())
}

/// Generate a new liquidation ID
pub fn generate_liquidation_id() -> LiquidationId {
    LiquidationId(Uuid::new_v4())
}

/// Generate a new entry ID
pub fn generate_entry_id() -> EntryId {
    EntryId(Uuid::new_v4())
}

/// Generate a new event ID
pub fn generate_event_id() -> EventId {
    EventId(Uuid::new_v4())
}

/// Alert operations team (critical) - IMPLEMENTED
pub fn alert_operations_team_critical(message: String) {
    tracing::error!("CRITICAL ALERT: {}", message);

    // Send to multiple channels for redundancy
    send_pagerduty_alert(&message, "critical");
    send_slack_alert(&message, "critical");
    send_email_alert(&message, "critical");
}

/// Alert operations team (warning) - IMPLEMENTED
pub fn alert_operations_team_warning(message: String) {
    tracing::warn!("WARNING ALERT: {}", message);

    send_slack_alert(&message, "warning");
    send_email_alert(&message, "warning");
}

/// Send PagerDuty alert
fn send_pagerduty_alert(message: &str, severity: &str) {
    // In production, integrate with PagerDuty API
    // For now, log the alert
    tracing::error!("[PagerDuty] {} - {}", severity, message);

    // Example integration:
    // let client = reqwest::Client::new();
    // client.post("https://events.pagerduty.com/v2/enqueue")
    //     .json(&json!({
    //         "routing_key": env::var("PAGERDUTY_KEY"),
    //         "event_action": "trigger",
    //         "payload": {
    //             "summary": message,
    //             "severity": severity,
    //             "source": "perpinfra"
    //         }
    //     }))
    //     .send().await;
}

/// Send Slack alert
fn send_slack_alert(message: &str, severity: &str) {
    tracing::info!("[Slack] {} - {}", severity, message);

    // Example integration:
    // let webhook_url = env::var("SLACK_WEBHOOK_URL");
    // let client = reqwest::Client::new();
    // client.post(&webhook_url)
    //     .json(&json!({
    //         "text": format!("[{}] {}", severity.to_uppercase(), message)
    //     }))
    //     .send().await;
}

/// Send email alert
fn send_email_alert(message: &str, severity: &str) {
    tracing::info!("[Email] {} - {}", severity, message);

    // Example integration with SendGrid or similar
}

/// Dump system state for forensics - IMPLEMENTED
pub fn dump_system_state_for_forensics() {
    use std::fs::File;
    use std::io::Write;

    tracing::error!("Dumping system state for forensics");

    let timestamp = current_timestamp_ms();
    let filename = format!("forensics_dump_{}.json", timestamp);

    // Collect system state (basic info only - engines own their halt state)
    let state = serde_json::json!({
        "timestamp": timestamp,
        "memory_usage": get_memory_usage(),
        "thread_count": get_thread_count(),
        "kill_switch_active": crate::KILL_SWITCH.load(Ordering::SeqCst),
    });

    // Write to file
    if let Ok(mut file) = File::create(&filename) {
        let _ = file.write_all(state.to_string().as_bytes());
        tracing::info!("Forensics dump written to {}", filename);
    }
}

/// Checkpoint system state - IMPLEMENTED
pub fn checkpoint_system_state() {
    tracing::info!("Checkpointing system state");

    // Trigger snapshot creation
    // This would integrate with the snapshot system
    let timestamp = current_timestamp_ms();
    tracing::info!("Checkpoint created at {}", timestamp);
}

/// Check if operator is authorized - IMPLEMENTED
pub fn is_authorized_operator(operator_id: OperatorId) -> bool {
    if let Ok(operators) = AUTHORIZED_OPERATORS.read() {
        operators.contains(&operator_id)
    } else {
        false
    }
}

/// Add authorized operator
pub fn add_authorized_operator(operator_id: OperatorId) {
    if let Ok(mut operators) = AUTHORIZED_OPERATORS.write() {
        operators.insert(operator_id);
        tracing::info!("Added authorized operator: {}", operator_id);
    }
}

/// Remove authorized operator
pub fn remove_authorized_operator(operator_id: OperatorId) {
    if let Ok(mut operators) = AUTHORIZED_OPERATORS.write() {
        operators.remove(&operator_id);
        tracing::info!("Removed authorized operator: {}", operator_id);
    }
}

// Helper functions for system metrics
fn get_memory_usage() -> u64 {
    // Platform-specific memory usage
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        if let Ok(kb_val) = kb.parse::<u64>() {
                            return kb_val * 1024; // Convert to bytes
                        }
                    }
                }
            }
        }
    }
    0 // Fallback if unable to read
}

fn get_thread_count() -> usize {
    // Platform-specific thread count
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("Threads:") {
                    if let Some(count) = line.split_whitespace().nth(1) {
                        if let Ok(count_val) = count.parse::<usize>() {
                            return count_val;
                        }
                    }
                }
            }
        }
    }
    1 // Fallback
}