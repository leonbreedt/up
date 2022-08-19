use std::sync::Arc;

use axum::{
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use up_core::{auth::Role, jwt};
use uuid::Uuid;

use crate::{
    api::{HEALTH_URI, PING_URI},
    mask, repository,
    repository::dto::{User, UserRole},
};

const SKIP_AUTH_URIS: &[&str] = &[PING_URI, HEALTH_URI];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    #[serde(skip_serializing)]
    pub user_id: i64,
    #[serde(skip_serializing)]
    pub user_uuid: Uuid,
    #[serde(skip_serializing)]
    pub account_uuids: Vec<Uuid>,
    #[serde(skip_serializing)]
    pub project_uuids: Vec<Uuid>,
    pub email: String,
    pub roles: Vec<Role>,
}

impl Identity {
    pub fn is_administrator(&self) -> bool {
        self.roles.contains(&Role::Administrator)
    }

    pub fn is_assigned_to_project(&self, uuid: &Uuid) -> bool {
        self.project_uuids.contains(uuid)
    }

    pub fn is_assigned_to_account(&self, uuid: &Uuid) -> bool {
        self.account_uuids.contains(uuid)
    }
}

impl From<&UserRole> for Role {
    fn from(role: &UserRole) -> Self {
        match role {
            UserRole::Administrator => Role::Administrator,
            UserRole::Member => Role::Member,
            UserRole::Viewer => Role::Viewer,
        }
    }
}

impl From<User> for Identity {
    fn from(u: User) -> Self {
        Self {
            user_id: u.id,
            user_uuid: u.uuid,
            account_uuids: u.account_uuids,
            project_uuids: u.project_uuids,
            email: u.email,
            roles: u.roles.iter().map(|r| r.into()).collect(),
        }
    }
}

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
    let jwt_verifier = req.extensions().get::<Arc<jwt::Verifier>>().unwrap();

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
                let identity: Identity = user.into();
                tracing::trace!(
                    user_uuid = identity.user_uuid.to_string(),
                    email = mask::email(&identity.email),
                    account_uuids = format!("{:?}", identity.account_uuids),
                    project_uuids = format!("{:?}", identity.project_uuids),
                    roles = format!("{:?}", identity.roles),
                    "user authorized"
                );
                req.extensions_mut().insert(identity);
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
