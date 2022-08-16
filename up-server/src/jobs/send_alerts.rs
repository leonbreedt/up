use std::time::Duration;

use tokio::{sync::oneshot, task::JoinHandle, time};

use crate::{notifier::Notifier, repository::Repository};

const POLL_INTERVAL: u64 = 5;

pub struct SendAlerts {
    repository: Repository,
    notifier: Notifier,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl SendAlerts {
    pub fn with_repository(repository: Repository, notifier: Notifier) -> Self {
        Self {
            repository,
            notifier,
            shutdown_tx: None,
            join_handle: None,
        }
    }

    pub async fn spawn(&mut self) {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let mut poll_interval = time::interval(Duration::from_secs(POLL_INTERVAL));
        let repository = self.repository.clone();
        let notifier = self.notifier.clone();

        self.shutdown_tx = Some(shutdown_tx);
        self.join_handle = Some(tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = poll_interval.tick() => {
                        send_alerts(&repository, &notifier).await
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
                    tracing::error!("failed to send SendAlerts job shutdown signal");
                }
            }
            if let Err(e) = handle.await {
                tracing::error!("failed to wait for SendAlerts job to terminate: {}", e);
            }
        }

        tracing::debug!("finished SendAlerts job");
    }
}

async fn send_alerts(repository: &Repository, notifier: &Notifier) {
    match repository.notification().send_alert_batch(notifier).await {
        Ok(delivered_alerts) => {
            for alert in delivered_alerts {
                tracing::debug!(
                    check_uuid = alert.check_uuid.to_string(),
                    alert_type = alert.notification_type.to_string(),
                    "alert delivered successfully",
                );
            }
        }
        Err(e) => tracing::error!("failed to send alert batch: {:?}", e),
    }
}
