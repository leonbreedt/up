use std::{fs, net::SocketAddr, process};

use argh::FromArgs;
use camino::Utf8PathBuf;
use directories::ProjectDirs;
use tracing_subscriber::EnvFilter;

use crate::{api, database};

pub struct App {
    args: Args,
}

impl App {
    pub fn new() -> Self {
        Self {
            args: argh::from_env(),
        }
    }

    pub async fn run(&self) {
        if std::env::var_os("RUST_BACKTRACE").is_none() {
            std::env::set_var("RUST_BACKTRACE", "1")
        }

        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "up_server=debug,tower_http=debug,sqlx=debug")
        }

        if self.args.json {
            tracing_subscriber::fmt::fmt()
                .json()
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        } else {
            tracing_subscriber::fmt::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        }

        if let Err(e) = fs::create_dir_all(&self.args.data_dir) {
            tracing::error!(err = format!("{}", e), "failed to create data dir");
            process::exit(1);
        }

        let database = match database::connect(
            &self.args.database_url,
            1,
            self.args.database_max_connections,
        )
        .await
        {
            Ok(database) => database,
            Err(e) => {
                tracing::error!(err = format!("{}", e), "failed to connect to database");
                process::exit(1);
            }
        };

        if let Err(e) = database.migrate().await {
            tracing::error!(err = format!("{}", e), "failed to migrate database schema");
            process::exit(1);
        }

        let router = api::build(database);

        tracing::debug!(
            data_dir = self.args.data_dir.as_str(),
            ip = self.args.listen_address.ip().to_string().as_str(),
            port = self.args.listen_address.port(),
            url = format!(
                "http://{}:{}",
                self.args.listen_address.ip(),
                self.args.listen_address.port()
            ),
            "server started"
        );

        axum::Server::bind(&self.args.listen_address)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    }
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
    /// the database URL to connect to (default: sqlite://DATA_DIR/up.db, or DATABASE_URL environment variable)
    #[argh(option, default = "default_database_url()")]
    database_url: String,
    /// the maximum number of connections in the PostgreSQL connection pool (default: 20, or DATABASE_MAX_CONNECTIONS environment variable)
    #[argh(option, default = "default_database_max_connections()")]
    database_max_connections: u32,
    /// directory in which to store database file and any other temporary files (default: OS local app data dir)
    #[argh(option, default = "default_data_dir()")]
    data_dir: Utf8PathBuf,
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

fn default_data_dir() -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(
        ProjectDirs::from("io", "sector42", "up")
            .expect("failed to determine OS-specific app dir")
            .data_local_dir()
            .to_owned(),
    )
    .unwrap()
}

fn default_database_url() -> String {
    format!("sqlite://{}/up.db", default_data_dir().as_str())
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
