use std::time::Duration;

use tokio::{sync::oneshot, task::JoinHandle, time};

use crate::repository::Repository;

const POLL_INTERVAL: u64 = 5;

pub struct EnqueueAlerts {
    repository: Repository,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl EnqueueAlerts {
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
        let repository = self.repository.clone();

        self.shutdown_tx = Some(shutdown_tx);
        self.join_handle = Some(tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = poll_interval.tick() => {
                        enqueue_overdue_ping_alerts(&repository).await
                    },
                    _msg = &mut shutdown_rx => {
                        break;
                    }
                }
            }
        }));
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            if let Some(tx) = self.shutdown_tx.take() {
                if tx.send(()).is_err() {
                    tracing::error!("failed to send EnqueueAlerts job shutdown signal");
                }
            }
            if let Err(e) = handle.await {
                tracing::error!("failed to wait for EnqueueAlerts job to terminate: {}", e);
            }
        }

        tracing::debug!("finished EnqueueAlerts job");
    }
}

async fn enqueue_overdue_ping_alerts(repository: &Repository) {
    if let Err(e) = repository.check().enqueue_alerts_for_overdue_pings().await {
        tracing::error!("failed to enqueue overdue ping alerts: {:?}", e);
    }
}
