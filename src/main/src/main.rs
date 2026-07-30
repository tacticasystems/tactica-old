use clap::Parser;
use tokio::sync::oneshot;

use crate::{config::Config, shutdown::shutdown_signal, telemetry::init_telemetry};

mod config;
mod shutdown;
mod telemetry;

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Serve the Tactica API
    Serve
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Config::load().expect("Failed to load configuration");
    init_telemetry(config.telemetry).expect("Failed to initialize telemetry");

    match args.command {
        Command::Serve => {
            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
            let shutdown_timeout = config.api.graceful_shutdown_timeout.clone();

            let listener = tokio::net::TcpListener::bind(config.api.bind_addr.clone())
                .await
                .expect("Failed to bind to address");

            tracing::info!("Listening on {}", listener.local_addr().unwrap());

            let router = tactica_api::router(config.api);

            let server = tactica_api::serve(listener, router)
                .with_graceful_shutdown(async move {
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
