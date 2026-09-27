//! Server configuration.

use std::time::Duration;

/// Timeouts that govern session and run lifetimes (SPEC-003-R06, R07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
    /// How long a session survives without a connection before it expires.
    pub session_expiry: Duration,
    /// How long a run survives with no sessions before it is released.
    pub run_release: Duration,
}

impl ServerConfig {
    /// Provisional default for [`ServerConfig::session_expiry`].
    pub const DEFAULT_SESSION_EXPIRY: Duration = Duration::from_secs(2 * 60);
    /// Provisional default for [`ServerConfig::run_release`].
    pub const DEFAULT_RUN_RELEASE: Duration = Duration::from_secs(5 * 60);
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            session_expiry: Self::DEFAULT_SESSION_EXPIRY,
            run_release: Self::DEFAULT_RUN_RELEASE,
        }
    }
}
