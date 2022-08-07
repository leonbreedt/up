use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Extension;

use crate::{
    api::v1::ApiError,
    mask,
    repository::{Repository, RepositoryError},
};

pub async fn ping(
    Path(key): Path<String>,
    repository: Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    match repository.check().ping_check(key.as_str()).await {
        Ok(found) => {
            if found {
                tracing::debug!(key = mask::ping_key(key.as_str()), "ping received");
            } else {
                tracing::trace!(key = key, "ignoring ping received, unknown key")
            }
        }
        Err(RepositoryError::NotFoundPingKey { key: _key }) => {
            tracing::trace!("ignoring ping key not found")
        }
        Err(e) => {
            tracing::error!(err = format!("{:?}", e), "failed to process ping")
        }
    }

    // Don't give callers a signal whether a ping exists or not.
    Ok("OK")
}
