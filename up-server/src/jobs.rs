use std::time::Duration;
use tokio::{sync::oneshot, task::JoinHandle, time};

use crate::database::Database;
use crate::repository::dto::Check;
use crate::repository::{dto, Repository};

const POLL_INTERVAL: u64 = 5;

pub struct PollChecks {
    repository: Repository,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl PollChecks {
    pub fn with_repository(repository: Repository) -> Self {
        Self {
            repository,
            shutdown_tx: None,
            join_handle: None,
        }
    }

    pub async fn spawn(&mut self) {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let mut poll_interval = time::interval(Duration::from_secs(POLL_INTERVAL));
        let task_repository = self.repository.clone();

        self.shutdown_tx = Some(shutdown_tx);
        self.join_handle = Some(tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = poll_interval.tick() => {
                        tracing::info!("polling for check statuses");
                        if let Ok(checks) = task_repository.check().read_overdue_checks().await {
                            for check in checks {
                                tracing::info!("overdue: {:?} (status={}, last_pinged_at={:?})", check.uuid.unwrap(), check.status.unwrap().to_string(), check.last_ping_at);
                            }
                        }
                    },
                    msg = &mut shutdown_rx => {
                        break;
                    }
                }
            }
        }));
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            if let Some(tx) = self.shutdown_tx.take() {
                if let Err(_) = tx.send(()) {
                    tracing::error!("failed to send PollChecks job shutdown signal");
                }
            }
            if let Err(e) = handle.await {
                tracing::error!("failed to wait for PollChecks job to terminate: {}", e);
            }
        }
    }
}
