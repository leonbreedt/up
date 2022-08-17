use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, TypedHeader},
    headers::UserAgent,
    response::IntoResponse,
    Extension,
};

use crate::{api::v1::ApiError, mask, repository::Repository};

pub async fn ping(
    Path(key): Path<String>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    repository: Extension<Repository>,
) -> Result<impl IntoResponse, ApiError> {
    match repository.check().ping(key.as_str()).await {
        Ok(Some(uuid)) => {
            tracing::debug!(
                remote_ip = remote_addr.ip().to_string().as_str(),
                remote_port = remote_addr.port(),
                user_agent = user_agent.as_str(),
                check_uuid = uuid.to_string(),
                key = mask::ping_key(key.as_str()),
                "ping received"
            );
        }
        Ok(None) => {
            tracing::trace!(key = key, "ignoring ping received, unknown key")
        }
        Err(e) => {
            tracing::error!(err = format!("{:?}", e), "failed to process ping")
        }
    }

    // Don't give callers a signal whether a ping exists or not.
    Ok("OK")
}
