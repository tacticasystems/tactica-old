use std::net::SocketAddr;

use clap::Parser;

mod shutdown;

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Serve the Tactica API
    Serve {
        /// The address to listen on
        #[clap(long, default_value = "0.0.0.0:8080")]
        listen_addr: SocketAddr,
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Command::Serve { listen_addr } => {
            let listener = tokio::net::TcpListener::bind(listen_addr)
                .await
                .expect("Failed to bind to address");

            let router = tactica_api::router();

            tactica_api::serve(listener, router)
                .with_graceful_shutdown(shutdown::shutdown_signal())
                .await
                .expect("Failed to serve API");
        }
    }
}
