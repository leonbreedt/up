use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use argh::FromArgs;
use dotenv::dotenv;
use miette::{Diagnostic, IntoDiagnostic, Result};
use thiserror::Error;
use tracing_subscriber::EnvFilter;
use up_core::jwt::{self, DEFAULT_AUDIENCE, DEFAULT_ISSUER};
use up_core::JWKS_ENV;

use crate::{api, database, integrations, jobs, notifier::Notifier, repository::Repository};

static JSON_OUTPUT: AtomicBool = AtomicBool::new(false);

pub struct App {
    args: Args,
}

#[derive(Error, Diagnostic, Debug)]
pub enum AppError {
    #[error("required environment variable {name} is not set, required for {purpose}")]
    #[diagnostic(code(up::error::env))]
    MissingEnvironmentVariable { name: String, purpose: String },
    #[error("configuration error: {0}")]
    #[diagnostic(code(up::error::configuration))]
    ConfigurationError(#[from] up_core::Error),
}

impl App {
    pub fn new() -> Self {
        Self::with_args(argh::from_env())
    }

    pub fn with_args(args: Args) -> Self {
        Self { args }
    }

    pub fn json_output() -> bool {
        JSON_OUTPUT.load(Ordering::Relaxed)
    }

    pub async fn run(&self) -> Result<()> {
        dotenv().ok();

        miette::set_panic_hook();

        if std::env::var_os("RUST_BACKTRACE").is_none() {
            std::env::set_var("RUST_BACKTRACE", "1")
        }

        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "up_server=debug,tower_http=debug,sqlx=debug")
        }

        if self.args.json {
            JSON_OUTPUT.store(true, Ordering::Relaxed);
            tracing_subscriber::fmt::fmt()
                .json()
                .with_env_filter(EnvFilter::from_default_env())
                .try_init()
                .ok();
        } else {
            JSON_OUTPUT.store(false, Ordering::Relaxed);
            tracing_subscriber::fmt::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .try_init()
                .ok();
        }

        let jwks = env_or_error(JWKS_ENV, "JWT verification")?;

        let jwt_verifier = Arc::new(
            jwt::Verifier::new_from_jwks(&jwks, Some(DEFAULT_ISSUER), Some(DEFAULT_AUDIENCE))
                .map_err(AppError::ConfigurationError)?,
        );

        let key_ids = jwt_verifier.key_ids();
        tracing::debug!(
            "using {} key(s) to verify JWTs: {}",
            key_ids.len(),
            key_ids.join(", ")
        );

        let database = database::connect(
            &self.args.database_url,
            1,
            self.args.database_max_connections,
        )
        .await?;

        database.migrate().await?;

        let repository = Repository::new(database.clone());
        let postmark_client = integrations::postmark::PostmarkClient::new()?;
        let notifier = Notifier::new(repository.clone(), postmark_client);

        let mut enqueue_alerts_job: Option<jobs::EnqueueAlerts> = None;
        let mut send_alerts_job: Option<jobs::SendAlerts> = None;

        if !self.args.disable_background_jobs {
            enqueue_alerts_job = Some(jobs::EnqueueAlerts::with_repository(repository.clone()));
            send_alerts_job = Some(jobs::SendAlerts::with_repository(
                repository.clone(),
                notifier.clone(),
            ));
        } else {
            tracing::debug!("background jobs disabled, alerts will not be sent");
        }

        let router = api::build(repository, notifier, jwt_verifier);

        tracing::debug!(
            ip = self.args.listen_address.ip().to_string().as_str(),
            port = self.args.listen_address.port(),
            url = format!(
                "http://{}:{}",
                self.args.listen_address.ip(),
                self.args.listen_address.port()
            ),
            "server started"
        );

        if !self.args.disable_background_jobs {
            enqueue_alerts_job.as_mut().unwrap().spawn().await;
            send_alerts_job.as_mut().unwrap().spawn().await;
        }

        let server = axum::Server::bind(&self.args.listen_address)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>());

        let graceful = server.with_graceful_shutdown(shutdown_signal(
            enqueue_alerts_job.as_mut(),
            send_alerts_job.as_mut(),
        ));
        graceful.await.into_diagnostic()?;

        tracing::debug!("server terminated");

        Ok(())
    }
}

async fn shutdown_signal(
    enqueue_alerts_job: Option<&mut jobs::EnqueueAlerts>,
    send_alerts_job: Option<&mut jobs::SendAlerts>,
) {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to handle Ctrl-C signal");
    tracing::info!("ctrl-c received");

    if let Some(enqueue_alerts_job) = enqueue_alerts_job {
        enqueue_alerts_job.stop().await;
    }
    if let Some(send_alerts_job) = send_alerts_job {
        send_alerts_job.stop().await;
    }
}

#[derive(FromArgs)]
/// The UP server.
pub struct Args {
    /// server address:port to listen on (default: 0.0.0.0:8080, PORT environment variable can override default port 8080)
    #[argh(
        option,
        default = "SocketAddr::from(([0, 0, 0, 0], default_listen_port()))"
    )]
    pub listen_address: SocketAddr,
    /// the database URL to connect to (default: postgres://127.0.0.1:5432/up, or DATABASE_URL environment variable)
    #[argh(option, default = "default_database_url()")]
    pub database_url: String,
    /// the maximum number of connections in the PostgreSQL connection pool (default: 20, or DATABASE_MAX_CONNECTIONS environment variable)
    #[argh(option, default = "default_database_max_connections()")]
    pub database_max_connections: u32,
    /// use JSON for log messages
    #[argh(switch)]
    pub json: bool,
    /// disable background jobs
    #[argh(switch)]
    pub disable_background_jobs: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            listen_address: SocketAddr::from(([127, 0, 0, 1], default_listen_port())),
            database_url: default_database_url(),
            database_max_connections: default_database_max_connections(),
            json: false,
            disable_background_jobs: false,
        }
    }
}

const DEFAULT_LISTEN_PORT: u16 = 8080;

fn default_listen_port() -> u16 {
    if let Ok(port_str) = std::env::var("PORT") {
        if let Ok(port) = port_str.parse() {
            tracing::debug!("using port from PORT environment variable");
            port
        } else {
            DEFAULT_LISTEN_PORT
        }
    } else {
        DEFAULT_LISTEN_PORT
    }
}

const DEFAULT_DATABASE_URL: &str = "postgres://127.0.0.1:5432/up";

fn default_database_url() -> String {
    if let Ok(value) = std::env::var("DATABASE_URL") {
        value
    } else {
        DEFAULT_DATABASE_URL.to_string()
    }
}

const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 4;

fn default_database_max_connections() -> u32 {
    if let Ok(value) = std::env::var("DATABASE_MAX_CONNECTIONS") {
        value
            .parse()
            .ok()
            .unwrap_or(DEFAULT_DATABASE_MAX_CONNECTIONS)
    } else {
        DEFAULT_DATABASE_MAX_CONNECTIONS
    }
}

fn env_or_error(name: &str, purpose: &str) -> Result<String, AppError> {
    if let Ok(value) = std::env::var(name) {
        Ok(value)
    } else {
        Err(AppError::MissingEnvironmentVariable {
            name: name.to_string(),
            purpose: purpose.to_string(),
        })
    }
}
