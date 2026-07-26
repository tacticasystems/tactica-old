use clap::Parser;

use crate::{config::Config, telemetry::init_telemetry};

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
async fn main() {
    let args = Args::parse();
    let config = Config::load().expect("Failed to load configuration");
    init_telemetry(config.telemetry).expect("Failed to initialize telemetry");

    match args.command {
        Command::Serve => {
            let listener = tokio::net::TcpListener::bind(config.server.listen_addr)
                .await
                .expect("Failed to bind to address");

            tracing::info!("Listening on {}", listener.local_addr().unwrap());

            let router = tactica_api::router();

            tactica_api::serve(listener, router)
                .with_graceful_shutdown(shutdown::shutdown_signal())
                .await
                .expect("Failed to serve API");
        }
    }
}
