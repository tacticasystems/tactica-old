use std::sync::Arc;

use async_trait::async_trait;
use clap::Parser;
use tactica_api::AppState;
use tactica_module_identity::{models::account::EmailAddress, ports::VerificationNotifier};
use tokio::sync::oneshot;

use crate::{config::Config, shutdown::shutdown_signal, telemetry::init_telemetry};

mod config;
mod shutdown;
mod telemetry;

struct DevelopmentVerificationNotifier;

#[async_trait]
impl VerificationNotifier for DevelopmentVerificationNotifier {
    async fn send_verification_code(
        &self,
        email: &EmailAddress,
        code: &str,
    ) -> Result<(), anyhow::Error> {
        tracing::info!(email = %email, verification_code = code, "development email verification code issued");
        Ok(())
    }
}

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Serve the Tactica API
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Config::load().expect("Failed to load configuration");
    init_telemetry(config.telemetry).expect("Failed to initialize telemetry");

    match args.command {
        Command::Serve => {
            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
            let shutdown_timeout = config.api.graceful_shutdown_timeout;

            let listener = tokio::net::TcpListener::bind(config.api.bind_addr)
                .await
                .expect("Failed to bind to address");

            tracing::info!("Listening on {}", listener.local_addr().unwrap());

            let db = tactica_db::Postgres::new(config.database)
                .await
                .expect("Failed to initialize database");

            db.migrate()
                .await
                .expect("Failed to run database migrations");

            let identity_service = tactica_module_identity::service::Service::new(
                db.clone(),
                DevelopmentVerificationNotifier,
                config.auth,
            );

            let router = tactica_api::router(config.api, AppState::new(Arc::new(identity_service)));

            let server = tactica_api::serve(listener, router).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });

            let mut server_task = tokio::spawn(async move { server.await });

            tokio::select! {
                result = &mut server_task => {
                    result??;
                    tracing::info!("graceful shutdown completed")
                }

                () = shutdown_signal() => {
                    tracing::info!("shutdown signal received");

                    let _ = shutdown_tx.send(());

                    match tokio::time::timeout(shutdown_timeout, &mut server_task).await {
                        Ok(result) => {
                            result??;
                            tracing::info!("graceful shutdown completed");
                        },

                        Err(_) => {
                            tracing::warn!(
                                timeout = ?shutdown_timeout,
                                "graceful shutdown timed out; terminating server task"
                            );

                            server_task.abort();

                            // Wait for cancellation to complete.
                            let _ = server_task.await;
                        }
                    }
                }
            }

            Ok(())
        }
    }
}
