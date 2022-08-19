use axum::{
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use up_core::jwt;

use crate::{
    api::{HEALTH_URI, PING_URI},
    mask, repository,
};

const SKIP_AUTH_URIS: &[&str] = &[PING_URI, HEALTH_URI];

pub async fn auth_middleware<B>(
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    for prefix in SKIP_AUTH_URIS {
        if req.uri().path().starts_with(prefix) {
            tracing::trace!(path = req.uri().path(), "skipping authorization");
            return Ok(next.run(req).await);
        }
    }

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let repository = req.extensions().get::<repository::Repository>().unwrap();
    let jwt_verifier = req
        .extensions()
        .get::<Arc<jwt::Verifier>>()
        .unwrap()
        .clone();

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        tracing::trace!("missing Authorization header");
        return Err(StatusCode::UNAUTHORIZED);
    };

    if !auth_header.starts_with("Bearer ") {
        tracing::trace!("unsupported Authorization type");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let claims = match jwt_verifier.verify(&auth_header[7..]) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::trace!("failed to verify user JWT: {:?}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    if let Some(subject) = claims.subject {
        match repository.auth().find_user_by_subject(&subject).await {
            Ok(Some(user)) => {
                tracing::trace!(
                    user_uuid = user.uuid.to_string(),
                    email = mask::email(&user.email),
                    "user authorized"
                );
                req.extensions_mut().insert(user);
                return Ok(next.run(req).await);
            }
            Ok(None) => {
                tracing::trace!(subject = subject, "user not found in repository");
                Err(StatusCode::UNAUTHORIZED)
            }
            Err(e) => {
                tracing::trace!("failed to authorize user: {:?}", e);
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    } else {
        tracing::trace!("JWT has no subject claim");
        Err(StatusCode::UNAUTHORIZED)
    }
}
