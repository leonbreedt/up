use axum::headers::HeaderMap;
use axum::http::HeaderValue;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{Connection, PgConnection};
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener},
    time::Duration,
};
use thiserror::Error;
use tracing::Level;
use up_core::auth::Role;
use up_core::jwt::{DEFAULT_AUDIENCE, DEFAULT_ISSUER};
use up_core::{jwt, SERVER_CERTIFICATE_ENV};
use url::Url;
use uuid::Uuid;

use up_server::{
    app::{App, Args},
    database::Database,
};

const BASE_DATABASE_URL: &str = "postgres://127.0.0.1:5432";
const SUBJECT_ADMINISTRATOR: &str = "6J60GGVAJE8TYTV8J9AS0JEN1B";
const SUBJECT_MEMBER: &str = "0Y0Q9K8Z3H9VJBVCX9671M84W6";
const SUBJECT_VIEWER: &str = "55201Y1AT28PCARNW21BY1K839";
const SUBJECT_NO_ACCOUNT_VIEWER: &str = "50AMCDE3BA97WSBMJ85C51D8GC";

pub mod api;

pub struct TestApp {
    database_name: String,
    database: Database,
    url: Url,
    jwt_generator: jwt::Generator,
}

#[derive(Error, Debug)]
pub enum TestError {
    #[error("failed to connect to test server: {0}")]
    ConnectError(#[source] reqwest::Error),
    #[error("failed to check test server health")]
    HealthCheckError,
    #[error("failed to parse URL: {0}")]
    UrlError(#[from] url::ParseError),
    #[error("failed to execute request: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("missing SERVER_CERTIFICATE environment variable, needed to issue test JWT")]
    MissingServerCertificateEnvError,
    #[error("failed to generate JWT to use for tests")]
    JWTGenerationError(#[source] up_core::Error),
    #[error("failed to serialize/deserialize JSON: {0}")]
    JSONSerializationError(#[from] serde_json::Error),
}

pub enum TestUser {
    Anonymous,
    Administrator,
    Viewer,
    Member,
    NoAccountViewer,
}

impl TestApp {
    pub async fn start_and_connect(user: TestUser) -> (Self, TestClient) {
        let app = Self::start().await;
        let client = app.connect(user).await.unwrap();
        (app, client)
    }

    pub async fn start() -> Self {
        dotenv::dotenv().ok();

        let database_name = format!("it_{}", Uuid::new_v4().to_string());
        let mut conn = PgConnection::connect(BASE_DATABASE_URL)
            .await
            .expect("failed to connect to database");
        sqlx::query(&format!("CREATE DATABASE \"{}\"", database_name))
            .execute(&mut conn)
            .await
            .expect("failed to create test database");
        conn.close()
            .await
            .expect("failed to close temporary connection");

        tracing::trace!("created test database {}", database_name);

        let database_url = format!("{}/{}", BASE_DATABASE_URL, database_name);
        let database = Database::new(&database_url, 1, 1)
            .await
            .expect("failed to connect to test database");
        database
            .migrate()
            .await
            .expect("failed to migrate test database");
        let port = next_available_port();
        let listen_address = SocketAddr::from(([127, 0, 0, 1], port));

        let app = App::with_args(Args {
            listen_address,
            database_url,
            disable_background_jobs: true,
            ..Args::default()
        });

        let server_certificate_pem = std::env::var(SERVER_CERTIFICATE_ENV)
            .expect("missing SERVER_CERTIFICATE environment variable, needed to issue test JWT")
            .as_bytes()
            .to_vec();

        let jwt_generator =
            jwt::Generator::new_from_pem(&server_certificate_pem, DEFAULT_ISSUER, DEFAULT_AUDIENCE)
                .expect("failed to create JWT generator");

        let _ = tokio::spawn(async move { app.run().await });

        let url =
            Url::parse(&format!("http://127.0.0.1:{}", port)).expect("failed to generate URL");

        Self {
            database_name,
            database,
            url,
            jwt_generator,
        }
    }

    pub async fn connect(&self, user: TestUser) -> Result<TestClient, TestError> {
        let mut remaining_tries = 50;
        let client = reqwest::Client::new();

        while remaining_tries > 0 {
            let result = client
                .request(reqwest::Method::GET, self.url.join("/health").unwrap())
                .send()
                .await;
            match result {
                Ok(res) => {
                    if res.text().await.unwrap().trim() == "UP" {
                        break;
                    } else {
                        return Err(TestError::HealthCheckError);
                    }
                }
                Err(e) => {
                    if let Some(source) = e.source() {
                        if let Some(hyper_error) = source.downcast_ref::<hyper::Error>() {
                            if hyper_error.is_connect() {
                                std::thread::sleep(Duration::from_millis(20));
                                remaining_tries -= 1;
                                continue;
                            }
                        }
                    }
                    return Err(TestError::ConnectError(e));
                }
            }
        }

        let auth = match user {
            TestUser::Anonymous => None,
            TestUser::Administrator => Some((SUBJECT_ADMINISTRATOR, vec![Role::Administrator])),
            TestUser::Member => Some((SUBJECT_MEMBER, vec![Role::Member])),
            TestUser::Viewer => Some((SUBJECT_VIEWER, vec![Role::Viewer])),
            TestUser::NoAccountViewer => Some((SUBJECT_NO_ACCOUNT_VIEWER, vec![Role::Viewer])),
        };

        Ok(TestClient(
            client,
            self.url.clone(),
            match auth {
                None => None,
                Some((subject, roles)) => Some(
                    self.jwt_generator
                        .generate(subject, 1, Some(roles))
                        .map_err(TestError::JWTGenerationError)?,
                ),
            },
        ))
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
}

pub struct TestClient(reqwest::Client, Url, Option<String>);

pub type TestResult<T> = Result<T, TestError>;

impl TestClient {
    pub async fn get_string(&self, path: &str) -> TestResult<String> {
        Ok(self
            .0
            .request(reqwest::Method::GET, self.1.join(path)?)
            .headers(self.headers())
            .send()
            .await?
            .text()
            .await?)
    }

    pub async fn get<RS: DeserializeOwned>(&self, path: &str) -> TestResult<RS> {
        self.execute_json_request_response(reqwest::Method::GET, path, None::<()>)
            .await
    }

    pub async fn post<RQ: Serialize, RS: DeserializeOwned>(
        &self,
        path: &str,
        body: RQ,
    ) -> TestResult<RS> {
        self.execute_json_request_response(reqwest::Method::POST, path, Some(body))
            .await
    }

    async fn execute_json_request_response<RQ: Serialize, RS: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<RQ>,
    ) -> Result<RS, TestError> {
        let mut req = self.0.request(method, self.1.join(path)?);
        req = req.headers(self.headers());
        if let Some(body) = body {
            if tracing::event_enabled!(Level::DEBUG) {
                tracing::debug!(
                    body = serde_json::to_string(&body).unwrap(),
                    "sending request"
                );
            }
            req = req.json(&body);
        }
        let response = self.0.execute(req.build()?).await?;
        response
            .error_for_status_ref()
            .map_err(TestError::RequestError)?;
        if tracing::event_enabled!(Level::DEBUG) {
            let bytes = response.bytes().await?;
            let json: serde_json::Value = serde_json::from_slice(&bytes)?;
            tracing::debug!(
                body = serde_json::to_string(&json).unwrap(),
                "received response"
            );
            Ok(serde_json::from_value(json)?)
        } else {
            Ok(response.json().await?)
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = &self.2 {
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
            );
        }
        headers
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            futures::executor::block_on(async {
                self.database.close().await;
                if let Ok(mut conn) = PgConnection::connect(BASE_DATABASE_URL).await {
                    if let Err(e) = sqlx::query(&format!(
                        "DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)",
                        self.database_name
                    ))
                    .execute(&mut conn)
                    .await
                    {
                        tracing::error!(
                            "failed to drop test database {}: {}",
                            self.database_name,
                            e
                        )
                    }
                    conn.close()
                        .await
                        .expect("failed to close temporary connection");
                }
            })
        });

        tracing::trace!("test database {} dropped", self.database_name);
    }
}

fn next_available_port() -> u16 {
    for _ in 0..10 {
        if let Some(port) = bind_os_available_port() {
            return port;
        }
    }

    panic!("no port available")
}

fn bind_os_available_port() -> Option<u16> {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .ok()
}
