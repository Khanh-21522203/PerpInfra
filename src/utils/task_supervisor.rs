use tokio::task::JoinHandle;
use std::collections::HashMap;
use crate::error::{Error, Result};
use tracing::{info, error, warn};

/// Task Supervisor - Monitors background tasks and detects failures
///
/// ## Purpose
/// Tracks all spawned background tasks and provides health monitoring.
/// Detects task panics or unexpected terminations and reports them.
///
/// ## Usage
/// ```rust
/// let mut supervisor = TaskSupervisor::new();
///
/// // Spawn and register tasks
/// supervisor.spawn("price_aggregation", async move {
///     // task logic
/// });
///
/// // Periodically check health
/// if let Err(e) = supervisor.check_health().await {
///     error!("Task failure detected: {:?}", e);
/// }
/// ```
pub struct TaskSupervisor {
    tasks: HashMap<String, JoinHandle<()>>,
}

impl TaskSupervisor {
    pub fn new() -> Self {
        TaskSupervisor {
            tasks: HashMap::new(),
        }
    }

    /// Spawn a new background task and register it for monitoring
    pub fn spawn<F>(&mut self, name: impl Into<String>, future: F) -> &mut Self
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        let handle = tokio::spawn(future);

        info!("Spawned background task: {}", name);
        self.tasks.insert(name, handle);
        self
    }

    /// Check health of all registered tasks
    /// Returns error if any task has terminated unexpectedly
    pub async fn check_health(&mut self) -> Result<()> {
        let mut failed_tasks = Vec::new();

        for (name, handle) in &self.tasks {
            if handle.is_finished() {
                failed_tasks.push(name.clone());
            }
        }

        if !failed_tasks.is_empty() {
            let error_msg = format!("Tasks terminated unexpectedly: {:?}", failed_tasks);
            error!("{}", error_msg);

            // Remove failed tasks from tracking
            for name in &failed_tasks {
                self.tasks.remove(name);
            }

            return Err(Error::ConfigError(error_msg));
        }

        Ok(())
    }

    /// Get count of active tasks
    pub fn active_task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Gracefully shutdown all tasks
    pub async fn shutdown_all(&mut self) {
        info!("Shutting down {} background tasks", self.tasks.len());

        for (name, handle) in self.tasks.drain() {
            handle.abort();
            info!("Aborted task: {}", name);
        }
    }

    /// Wait for a specific task to complete
    pub async fn wait_for_task(&mut self, name: &str) -> Result<()> {
        if let Some(handle) = self.tasks.remove(name) {
            handle.await
                .map_err(|e| Error::ConfigError(format!("Task {} failed: {:?}", name, e)))?;
            info!("Task {} completed", name);
            Ok(())
        } else {
            Err(Error::ConfigError(format!("Task {} not found", name)))
        }
    }
}

impl Default for TaskSupervisor {
    fn default() -> Self {
        Self::new()
    }
}