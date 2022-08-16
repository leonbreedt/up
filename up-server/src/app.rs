use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use argh::FromArgs;
use dotenv::dotenv;
use miette::{IntoDiagnostic, Result};
use tracing_subscriber::EnvFilter;

use crate::{api, database, integrations, jobs, notifier::Notifier, repository::Repository};

static JSON_OUTPUT: AtomicBool = AtomicBool::new(false);

pub struct App {
    args: Args,
}

impl App {
    pub fn new() -> Self {
        Self {
            args: argh::from_env(),
        }
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
                .init();
        } else {
            JSON_OUTPUT.store(false, Ordering::Relaxed);
            tracing_subscriber::fmt::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        }

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
        let mut enqueue_alerts_job = jobs::EnqueueAlerts::with_repository(repository.clone());
        let mut send_alerts_job =
            jobs::SendAlerts::with_repository(repository.clone(), notifier.clone());

        let router = api::build(repository, notifier);

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

        enqueue_alerts_job.spawn().await;
        send_alerts_job.spawn().await;

        let server = axum::Server::bind(&self.args.listen_address)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>());

        let graceful = server.with_graceful_shutdown(shutdown_signal(
            &mut enqueue_alerts_job,
            &mut send_alerts_job,
        ));
        graceful.await.into_diagnostic()?;

        tracing::debug!("server terminated");

        Ok(())
    }
}

async fn shutdown_signal(
    enqueue_alerts_job: &mut jobs::EnqueueAlerts,
    send_alerts_job: &mut jobs::SendAlerts,
) {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to handle Ctrl-C signal");
    tracing::info!("ctrl-c received");

    enqueue_alerts_job.stop().await;
    send_alerts_job.stop().await;
}

#[derive(FromArgs)]
/// The UP server.
struct Args {
    /// server address:port to listen on (default: 127.0.0.1:8080, PORT environment variable can override default port 8080)
    #[argh(
        option,
        default = "SocketAddr::from(([127, 0, 0, 1], default_listen_port()))"
    )]
    listen_address: SocketAddr,
    /// the database URL to connect to (default: postgres://127.0.0.1:5432/up, or DATABASE_URL environment variable)
    #[argh(option, default = "default_database_url()")]
    database_url: String,
    /// the maximum number of connections in the PostgreSQL connection pool (default: 20, or DATABASE_MAX_CONNECTIONS environment variable)
    #[argh(option, default = "default_database_max_connections()")]
    database_max_connections: u32,
    /// use JSON for log messages
    #[argh(switch)]
    json: bool,
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
