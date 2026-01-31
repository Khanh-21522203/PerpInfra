use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::types::ids::OperatorId;

pub struct KillSwitch {
    active: Arc<AtomicBool>,
}

impl KillSwitch {
    pub fn new() -> Self {
        KillSwitch {
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn activate(&self, reason: String) {
        if self.active.swap(true, Ordering::SeqCst) {
            // Already active
            return;
        }

        tracing::error!("KILL SWITCH ACTIVATED: {}", reason);

        // Alert operations team
        crate::utils::helper::alert_operations_team_critical(
            format!("KILL SWITCH ACTIVATED: {}", reason)
        );

        // Dump state for forensics
        crate::utils::helper::dump_system_state_for_forensics();
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn deactivate(&self, operator_id: OperatorId) {
        if !crate::utils::helper::is_authorized_operator(operator_id) {
            tracing::error!("Unauthorized kill switch deactivation attempt");
            return;
        }

        self.active.store(false, Ordering::SeqCst);
        tracing::warn!("Kill switch deactivated by operator {:?}", operator_id);
    }
}