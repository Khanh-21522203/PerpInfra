use crate::types::ids::{EventId, UserId};
use crate::types::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: Timestamp,
    pub event_id: EventId,
    pub event_type: String,
    pub user_id: Option<UserId>,
    pub action: String,
    pub details: String,
}

pub struct AuditTrail {
    entries: Vec<AuditEntry>,
}

impl AuditTrail {
    pub fn new() -> Self {
        AuditTrail {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, entry: AuditEntry) {
        self.entries.push(entry);
    }

    pub fn get_entries_for_user(&self, user_id: UserId) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| e.user_id == Some(user_id))
            .collect()
    }

    pub fn export_to_csv(&self) -> String {
        let mut csv = String::from("timestamp,event_id,event_type,user_id,action,details\n");

        for entry in &self.entries {
            csv.push_str(&format!(
                "{:?},{:?},{},{:?},{},{}\n",
                entry.timestamp,
                entry.event_id,
                entry.event_type,
                entry.user_id,
                entry.action,
                entry.details
            ));
        }

        csv
    }
}