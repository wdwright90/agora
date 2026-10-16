//! Rust client library for Agora servers.
//!
//! [`Client::connect`] opens a connection and completes the version handshake. Creating or
//! joining a run turns it into a [`Session`] and its [`AgentEvents`]. The session's methods
//! send requests and wait for their responses, and the library numbers requests itself; the
//! session can be cloned so several tasks can make requests. Observations, and removals of the
//! session's agents, arrive in order through [`AgentEvents::next`], owned by whichever task
//! handles steps. The newest view and
//! pacing state, for viewers, are held in latest-value handles ([`Session::view`] and
//! [`Session::pacing`]), so a slow reader skips stale states instead of queueing them.
//!
//! Messages follow the client protocol (SPEC-002, `docs/contracts/client-protocol.md`). The
//! library's behavior is specified by SPEC-004 (`docs/components/client/specs/client-library.md`).

mod client;
mod connection;
mod error;

pub use client::{AgentEvents, Client, Session, SessionInfo};
pub use connection::{AgentEvent, ObservationBatch};
pub use error::ClientError;

/// Protocol types used in this library's API.
pub use agora_protocol as protocol;
