//! Agora server: hosts simulation runs and connects clients to them.
//!
//! Each connection speaks the client protocol (SPEC-002, `docs/contracts/client-protocol.md`).
//! Each run is a task that owns its simulation and its sessions. Session and run lifetimes are
//! specified by SPEC-003 (`docs/components/server/specs/sessions-and-runs.md`) under the server
//! CDD.

mod catalog;
mod config;
mod connection;
mod registry;
mod requests;
mod run;
mod wire;

use std::time::Duration;

use tokio::net::TcpListener;
use tracing::{Instrument, info_span, warn};

pub use catalog::EMPTY_GRID_10X10;
pub use config::ServerConfig;
pub use requests::RETAINED_RESULTS;

use crate::registry::Registry;

/// Accept connections on `listener` and serve them. Runs until the task is dropped.
pub async fn serve(listener: TcpListener, config: ServerConfig) {
    let registry = Registry::default();
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                tokio::spawn(
                    connection::serve(stream, registry.clone(), config)
                        .instrument(info_span!("connection", %peer)),
                );
            }
            Err(e) => {
                // Usually a transient resource limit. Pause so a persistent error doesn't spin.
                warn!(error = %e, "failed to accept a connection");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}
