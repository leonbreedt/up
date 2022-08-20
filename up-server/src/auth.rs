use std::collections::HashMap;
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
    mask,
    repository::{
        self,
        dto::{User, UserRole},
        RepositoryError,
    },
    shortid::ShortId,
};

const SKIP_AUTH_URIS: &[&str] = &[PING_URI, HEALTH_URI];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    #[serde(skip_serializing)]
    pub user_id: i64,
    #[serde(skip_serializing)]
    pub user_uuid: Uuid,
    #[serde(skip_serializing)]
    pub account_ids: HashMap<Uuid, i64>,
    #[serde(skip_serializing)]
    pub project_ids: HashMap<Uuid, i64>,
    pub email: String,
    #[serde(skip_serializing)]
    pub roles: HashMap<i64, Vec<Role>>,
}

const ENTITY_ACCOUNT: &str = "account";
const ENTITY_PROJECT: &str = "project";

impl Identity {
    pub fn is_administrator_in_account(&self, uuid: &Uuid) -> bool {
        self.has_role_in_account(uuid, Role::Administrator)
    }

    pub fn is_administrator_in_account_with_id(&self, id: i64) -> bool {
        self.has_role_in_account_with_id(id, Role::Administrator)
    }

    pub fn has_role_in_account(&self, uuid: &Uuid, role: Role) -> bool {
        self.account_ids
            .get(uuid)
            .map(|id| self.has_role_in_account_with_id(*id, role))
            .unwrap_or(false)
    }

    pub fn has_role_in_account_with_id(&self, id: i64, role: Role) -> bool {
        self.roles
            .get(&id)
            .map(|r| r.contains(&role))
            .unwrap_or(false)
    }

    pub fn is_assigned_to_account(&self, uuid: &Uuid) -> bool {
        self.account_ids.contains_key(uuid)
    }

    pub fn is_assigned_to_project(&self, uuid: &Uuid) -> bool {
        self.project_ids.contains_key(uuid)
    }

    pub fn get_account_id(&self, account_uuid: &Uuid) -> Result<i64, RepositoryError> {
        self.account_ids
            .get(account_uuid)
            .map(|id| *id)
            .ok_or(RepositoryError::NotFound {
                entity_type: ENTITY_ACCOUNT.to_string(),
                id: ShortId::from_uuid(account_uuid).to_string(),
            })
    }

    pub fn get_project_id(&self, project_uuid: &Uuid) -> Result<i64, RepositoryError> {
        self.project_ids
            .get(project_uuid)
            .map(|id| *id)
            .ok_or(RepositoryError::NotFound {
                entity_type: ENTITY_PROJECT.to_string(),
                id: ShortId::from_uuid(project_uuid).to_string(),
            })
    }

    pub fn project_ids(&self) -> Vec<i64> {
        self.project_ids.values().map(|v| *v).collect()
    }

    pub fn account_ids(&self) -> Vec<i64> {
        self.account_ids.values().map(|v| *v).collect()
    }

    pub fn ensure_assigned_to_account(&self, uuid: &Uuid) -> Result<(), RepositoryError> {
        if !self.is_assigned_to_account(uuid) {
            tracing::trace!(
                user_uuid = self.user_uuid.to_string(),
                account_uuid = uuid.to_string(),
                "user not assigned to account, rejecting API call"
            );
            return Err(RepositoryError::NotFound {
                entity_type: ENTITY_ACCOUNT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            });
        }
        Ok(())
    }

    pub fn ensure_assigned_to_project(&self, uuid: &Uuid) -> Result<(), RepositoryError> {
        if !self.is_assigned_to_project(uuid) {
            tracing::trace!(
                user_uuid = self.user_uuid.to_string(),
                project_uuid = uuid.to_string(),
                "user not assigned to project, rejecting API call"
            );
            return Err(RepositoryError::NotFound {
                entity_type: ENTITY_PROJECT.to_string(),
                id: ShortId::from_uuid(uuid).to_string(),
            });
        }
        Ok(())
    }
}

impl From<UserRole> for Role {
    fn from(role: UserRole) -> Self {
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
            account_ids: to_uuid_and_id_map(u.account_ids),
            project_ids: to_uuid_and_id_map(u.project_ids),
            email: u.email,
            roles: to_role_and_id_map(u.roles),
        }
    }
}

fn to_role_and_id_map(items: Vec<String>) -> HashMap<i64, Vec<Role>> {
    let mut map = HashMap::new();
    for item in items {
        let parsed: Vec<_> = item.split("|").collect();
        let account_id: i64 = parsed[1].parse().unwrap();
        let user_role: UserRole = parsed[0].parse().unwrap();
        let role: Role = user_role.into();
        let roles = map.entry(account_id).or_insert_with(Vec::new);
        if !roles.contains(&role) {
            roles.push(role);
        }
    }
    map
}

fn to_uuid_and_id_map(items: Vec<String>) -> HashMap<Uuid, i64> {
    HashMap::from_iter(
        items
            .iter()
            .map(|v| {
                let parsed: Vec<_> = v.split("|").collect();
                (parsed[0].parse().unwrap(), parsed[1].parse().unwrap())
            })
            .collect::<Vec<_>>(),
    )
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
                    account_uuids =
                        format!("{:?}", identity.account_ids.keys().collect::<Vec<_>>()),
                    project_uuids =
                        format!("{:?}", identity.project_ids.keys().collect::<Vec<_>>()),
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
