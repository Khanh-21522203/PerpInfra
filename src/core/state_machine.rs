use crate::invariants::kill_switch::{KillSwitch, KillSwitchReason};
use crate::invariants::monitor::InvariantMonitor;

pub struct StateMachine {
    state: SystemState,
    invariant_monitor: InvariantMonitor,
    kill_switch: KillSwitch,
}

impl StateMachine {
    pub fn new(state: SystemState) -> Self {
        StateMachine {
            state,
            invariant_monitor: InvariantMonitor::new(),
            kill_switch: KillSwitch::new(),
        }
    }

    pub fn process_event(&mut self, event: Event) -> Result<Vec<Event>> {
        // Check kill switch
        if self.kill_switch.is_active() {
            return Err(Error::KillSwitchActive);
        }

        // Apply event to state
        let output_events = self.state.apply_event(event)?;

        // Check invariants
        if let Err(e) = self.invariant_monitor.check_all(&self.state) {
            self.kill_switch.trigger(KillSwitchReason::InvariantViolation(
                format!("{:?}", e)
            ));
            return Err(e);
        }

        Ok(output_events)
    }
}