use sqlx::{Connection, PgConnection};
use std::net::{
    Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpListener, ToSocketAddrs,
    UdpSocket,
};
use up_server::app;
use up_server::app::{App, Args};
use uuid::Uuid;

use up_server::database::Database;

const BASE_DATABASE_URL: &str = "postgres://127.0.0.1:5432";

pub mod api;

pub struct TestApp {
    database_name: String,
    database: Database,
}

impl TestApp {
    pub async fn new() -> Self {
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
        let listen_address = SocketAddr::from(([127, 0, 0, 1], next_available_port()));

        let app = App::with_args(Args {
            listen_address,
            database_url,
            ..Args::default()
        });

        let _ = tokio::spawn(async move { app.run().await });

        Self {
            database_name,
            database,
        }
    }

    pub async fn start() {}
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
