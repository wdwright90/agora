//! The `agora-server` executable.

use std::net::SocketAddr;
use std::time::Duration;

use agora_server::{ServerConfig, serve};
use clap::Parser;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Host Agora simulation runs for WebSocket clients.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value = "127.0.0.1:7878")]
    listen: SocketAddr,
    /// Seconds a session survives without a connection before it expires.
    #[arg(long, default_value_t = ServerConfig::DEFAULT_SESSION_EXPIRY.as_secs())]
    session_expiry_secs: u64,
    /// Seconds a run survives with no sessions before it is released.
    #[arg(long, default_value_t = ServerConfig::DEFAULT_RUN_RELEASE.as_secs())]
    run_release_secs: u64,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();
    let config = ServerConfig {
        session_expiry: Duration::from_secs(args.session_expiry_secs),
        run_release: Duration::from_secs(args.run_release_secs),
    };
    let listener = TcpListener::bind(args.listen).await?;
    info!(address = %listener.local_addr()?, ?config, "listening");
    tokio::select! {
        () = serve(listener, config) => {}
        result = tokio::signal::ctrl_c() => {
            result?;
            info!("shutting down");
        }
    }
    Ok(())
}
