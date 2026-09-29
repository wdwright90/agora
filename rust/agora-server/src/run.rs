//! One run's task. It owns the run's simulation, its sessions, and their timers, and handles
//! commands from connections one at a time, which serializes all calls into the simulation.
//! It advances the simulation once every agent has an accepted action and the pacing gate
//! allows it. It pushes each session its agents' observations, and each viewer the newest view
//! and any pacing change.

use std::collections::HashMap;
use std::future;

use agora_protocol::{
    AgentObservation, AgentView, CatalogEntryId, CloseReason, IntervalMs, Observation, Pacing,
    PacingMode, RunId, RunPhase, ServerMessage, SessionId, StateId, View,
};
use agora_sim::{
    AgentId, Move, Observations, Placement, SimConfig, Simulation, SpawnError, Status, Submission,
    SubmitError,
};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{Instant, sleep_until};
use tracing::{Instrument, debug, info, info_span};

use crate::catalog::CatalogEntry;
use crate::config::ServerConfig;
use crate::pacing::{Gate, Pacer, PacingRejection};
use crate::registry::Registry;

/// Capacity of a run's command queue. Each connection waits for its reply before sending
/// another command, so the queue holds at most one command per connection.
const COMMAND_QUEUE: usize = 64;

/// Where a run sends a session's pushed messages. The session's connection writes them after
/// the response it is sending, if any.
pub type Outbox = mpsc::UnboundedSender<ServerMessage>;

/// Where a run publishes views to a viewer's connection. It holds only the newest view, so a
/// slow viewer skips states instead of queueing them.
pub type ViewSlot = watch::Sender<Option<View>>;

/// A connection's handle to a run's task.
#[derive(Clone)]
pub struct RunHandle {
    commands: mpsc::Sender<Command>,
}

/// The run's task has ended because the run was released.
#[derive(Debug)]
pub struct RunGone;

/// A session established by creating or joining a run.
pub struct SessionInfo {
    pub session_id: SessionId,
    pub catalog_entry: CatalogEntryId,
    pub state_id: StateId,
    pub phase: RunPhase,
}

/// Why the run refused a Start.
pub enum StartRejection {
    NotCreator,
    AlreadyStarted,
    NotEligible,
}

/// Why the run refused a whole submission.
pub enum SubmitRejection {
    NotStarted,
    WrongState {
        expected: agora_sim::StateId,
        supplied: agora_sim::StateId,
    },
}

/// Why the run refused one action entry.
pub enum EntryRejection {
    NotOwned,
    Sim(SubmitError),
}

/// The outcome of each action entry, in submission order.
pub type EntryResults = Vec<(AgentId, Result<(), EntryRejection>)>;

enum Command {
    Join {
        outbox: Outbox,
        reply: oneshot::Sender<SessionInfo>,
    },
    Spawn {
        session: SessionId,
        placement: Placement,
        reply: oneshot::Sender<Result<AgentId, SpawnError>>,
    },
    Start {
        session: SessionId,
        reply: oneshot::Sender<Result<(), StartRejection>>,
    },
    Submit {
        session: SessionId,
        state: agora_sim::StateId,
        actions: Vec<(AgentId, Move)>,
        reply: oneshot::Sender<Result<EntryResults, SubmitRejection>>,
    },
    Watch {
        session: SessionId,
        connection: u64,
        slot: ViewSlot,
        reply: oneshot::Sender<(View, Pacing)>,
    },
    ClaimPacing {
        session: SessionId,
        reply: oneshot::Sender<Result<(), PacingRejection>>,
    },
    SetPacing {
        session: SessionId,
        mode: PacingMode,
        reply: oneshot::Sender<Result<(), PacingRejection>>,
    },
    StepOnce {
        session: SessionId,
        reply: oneshot::Sender<Result<(), PacingRejection>>,
    },
    Leave {
        session: SessionId,
        reply: oneshot::Sender<()>,
    },
    Close {
        session: SessionId,
        reply: oneshot::Sender<Result<(), NotCreator>>,
    },
    Disconnect {
        session: SessionId,
    },
}

/// A Close from a session without creator authority.
pub struct NotCreator;

/// Whether the run's task continues after a command or deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Continue,
    /// The run is released now.
    Release,
}

impl RunHandle {
    /// Establish a new non-creator session whose pushed messages go to `outbox`.
    pub async fn join(&self, outbox: Outbox) -> Result<SessionInfo, RunGone> {
        self.call(|reply| Command::Join { outbox, reply }).await
    }

    /// Spawn an agent owned by `session`.
    pub async fn spawn(
        &self,
        session: SessionId,
        placement: Placement,
    ) -> Result<Result<AgentId, SpawnError>, RunGone> {
        self.call(|reply| Command::Spawn {
            session,
            placement,
            reply,
        })
        .await
    }

    /// Start the run on behalf of `session`.
    pub async fn start(&self, session: SessionId) -> Result<Result<(), StartRejection>, RunGone> {
        self.call(|reply| Command::Start { session, reply }).await
    }

    /// Submit actions for agents owned by `session`, all targeting `state`.
    pub async fn submit(
        &self,
        session: SessionId,
        state: agora_sim::StateId,
        actions: Vec<(AgentId, Move)>,
    ) -> Result<Result<EntryResults, SubmitRejection>, RunGone> {
        self.call(|reply| Command::Submit {
            session,
            state,
            actions,
            reply,
        })
        .await
    }

    /// Make `session` a viewer that receives views through `slot`. `connection` orders viewers
    /// by connection age for pacing handover. Returns the current view and pacing state.
    pub async fn watch(
        &self,
        session: SessionId,
        connection: u64,
        slot: ViewSlot,
    ) -> Result<(View, Pacing), RunGone> {
        self.call(|reply| Command::Watch {
            session,
            connection,
            slot,
            reply,
        })
        .await
    }

    /// Claim pacing control for `session`.
    pub async fn claim_pacing(
        &self,
        session: SessionId,
    ) -> Result<Result<(), PacingRejection>, RunGone> {
        self.call(|reply| Command::ClaimPacing { session, reply })
            .await
    }

    /// Set the pacing mode on behalf of `session`.
    pub async fn set_pacing(
        &self,
        session: SessionId,
        mode: PacingMode,
    ) -> Result<Result<(), PacingRejection>, RunGone> {
        self.call(|reply| Command::SetPacing {
            session,
            mode,
            reply,
        })
        .await
    }

    /// Allow one step while paused, on behalf of `session`.
    pub async fn step_once(
        &self,
        session: SessionId,
    ) -> Result<Result<(), PacingRejection>, RunGone> {
        self.call(|reply| Command::StepOnce { session, reply })
            .await
    }

    /// End `session`, removing its agents.
    pub async fn leave(&self, session: SessionId) -> Result<(), RunGone> {
        self.call(|reply| Command::Leave { session, reply }).await
    }

    /// Close the run for every session, on behalf of `session`.
    pub async fn close(&self, session: SessionId) -> Result<Result<(), NotCreator>, RunGone> {
        self.call(|reply| Command::Close { session, reply }).await
    }

    /// Report that `session` lost its connection.
    pub async fn disconnect(&self, session: SessionId) {
        // A released run has no sessions left to update.
        let _ = self.commands.send(Command::Disconnect { session }).await;
    }

    async fn call<T>(
        &self,
        command: impl FnOnce(oneshot::Sender<T>) -> Command,
    ) -> Result<T, RunGone> {
        let (reply, response) = oneshot::channel();
        self.commands
            .send(command(reply))
            .await
            .map_err(|_| RunGone)?;
        // Commands still queued when a run is released are dropped with their reply senders.
        response.await.map_err(|_| RunGone)
    }
}

/// Create a run from a catalog entry, register it, and start its task. Returns the new run's
/// ID, its handle, and the creator's session, whose pushed messages go to `outbox`.
pub fn create(
    catalog_entry: CatalogEntryId,
    entry: CatalogEntry,
    registry: &Registry,
    config: ServerConfig,
    outbox: Outbox,
) -> (RunId, RunHandle, SessionInfo) {
    let id = RunId::new(random_id()).expect("random IDs are non-empty");
    let seed = rand::random();
    let sim = Simulation::new(SimConfig {
        width: entry.width,
        height: entry.height,
        seed,
    })
    .expect("catalog entries have valid grid dimensions");
    // The run outlives the connection that created it, so its span has no parent.
    let span = info_span!(parent: None, "run", %id);
    info!(parent: &span, %catalog_entry, seed, "run created");

    let mut run = Run {
        id: id.clone(),
        catalog_entry,
        sim,
        sessions: HashMap::new(),
        release_at: None,
        pacer: Pacer::new(default_interval(&config)),
        advance_at: None,
        registry: registry.clone(),
        config,
    };
    let creator = span.in_scope(|| run.add_session(true, outbox));
    let (commands, receiver) = mpsc::channel(COMMAND_QUEUE);
    let handle = RunHandle { commands };
    registry.insert(id.clone(), handle.clone());
    tokio::spawn(run.run(receiver).instrument(span));
    (id, handle, creator)
}

struct Run {
    id: RunId,
    catalog_entry: CatalogEntryId,
    sim: Simulation,
    sessions: HashMap<SessionId, Session>,
    /// When the run is released. Set only while the run has no sessions.
    release_at: Option<Instant>,
    pacer: Pacer,
    /// When a ready step may start. Set only while the pacing interval holds it back.
    advance_at: Option<Instant>,
    registry: Registry,
    config: ServerConfig,
}

struct Session {
    creator: bool,
    /// Agents this session owns, in spawn order, which is ascending ID order.
    agents: Vec<AgentId>,
    /// Where to push this session's messages. `None` while it has no connection.
    outbox: Option<Outbox>,
    /// While the session is a connected viewer, where to publish views, and its connection's
    /// acceptance order.
    viewer: Option<Viewer>,
    /// When the session expires. Set only while it has no connection.
    expires_at: Option<Instant>,
}

struct Viewer {
    slot: ViewSlot,
    connection: u64,
}

impl Run {
    async fn run(mut self, mut commands: mpsc::Receiver<Command>) {
        loop {
            let deadline = self.next_deadline();
            tokio::select! {
                // Prefer commands, so a join that arrived before a deadline is handled first.
                biased;
                command = commands.recv() => match command {
                    Some(command) => if self.handle(command) == Flow::Release {
                        break;
                    },
                    // The registry keeps a sender until the run is released.
                    None => return,
                },
                () = sleep_until_some(deadline) => {
                    if self.on_deadline(Instant::now()) == Flow::Release {
                        break;
                    }
                }
            }
        }
        // Dropping the run drops its sessions' outboxes and view slots.
        self.registry.remove(&self.id);
        info!("run released");
    }

    fn handle(&mut self, command: Command) -> Flow {
        match command {
            Command::Join { outbox, reply } => {
                let session = self.add_session(false, outbox);
                if self.release_at.take().is_some() {
                    info!("release timer cancelled");
                }
                let _ = reply.send(session);
            }
            Command::Spawn {
                session,
                placement,
                reply,
            } => {
                let result = self.sim.spawn(placement);
                if let Ok(agent) = result {
                    self.session_mut(&session).agents.push(agent);
                    info!(%session, agent = agent.0, "agent spawned");
                    self.publish_view();
                }
                let _ = reply.send(result);
            }
            Command::Start { session, reply } => {
                let _ = reply.send(self.start(&session));
            }
            Command::Submit {
                session,
                state,
                actions,
                reply,
            } => {
                let _ = reply.send(self.submit(&session, state, actions));
                self.advance_if_ready(Instant::now());
            }
            Command::Watch {
                session,
                connection,
                slot,
                reply,
            } => {
                let first_viewer = !self.has_viewers();
                let viewer = Viewer { slot, connection };
                if self.session_mut(&session).viewer.replace(viewer).is_none() {
                    info!(%session, "session is watching");
                }
                if first_viewer {
                    self.pacer.viewers_arrived();
                    info!("first viewer arrived; pacing reset to the default interval");
                }
                let _ = reply.send((self.view(), self.pacer.state_for(&session)));
            }
            Command::ClaimPacing { session, reply } => {
                let result = if self.session_mut(&session).viewer.is_none() {
                    Err(PacingRejection::NotViewing)
                } else {
                    self.pacer.claim(&session)
                };
                let changed = matches!(result, Ok(true));
                let _ = reply.send(result.map(|_| ()));
                if changed {
                    info!(%session, "pacing control claimed");
                    self.push_pacing();
                }
            }
            Command::SetPacing {
                session,
                mode,
                reply,
            } => {
                let result = self.pacer.set(&session, mode);
                let changed = result.is_ok();
                let _ = reply.send(result);
                if changed {
                    info!(?mode, "pacing set");
                    self.push_pacing();
                    self.advance_if_ready(Instant::now());
                }
            }
            Command::StepOnce { session, reply } => {
                let _ = reply.send(self.pacer.step_once(&session));
                self.advance_if_ready(Instant::now());
            }
            Command::Disconnect { session: id } => {
                let expires_at = Instant::now() + self.config.session_expiry;
                let session = self.session_mut(&id);
                session.outbox = None;
                let was_viewer = session.viewer.take().is_some();
                session.expires_at = Some(expires_at);
                info!(session = %id, "session disconnected; expiry timer started");
                if was_viewer {
                    self.viewer_left(&id);
                }
            }
            Command::Leave { session, reply } => {
                let flow = self.leave(&session);
                let _ = reply.send(());
                return flow;
            }
            Command::Close { session, reply } => {
                if !self.session_mut(&session).creator {
                    let _ = reply.send(Err(NotCreator));
                    return Flow::Continue;
                }
                self.close(CloseReason::ClosedByCreator, Some(&session));
                let _ = reply.send(Ok(()));
                return Flow::Release;
            }
        }
        Flow::Continue
    }

    /// End a session at its own request: remove its agents, hand over pacing control if it
    /// held it, and let a step waiting only for its agents execute. A run left with no
    /// sessions is released at once, and a setup run whose creator leaves is closed, because
    /// it could never start.
    fn leave(&mut self, id: &SessionId) -> Flow {
        let session = self
            .sessions
            .remove(id)
            .expect("a connected session has not expired");
        for &agent in &session.agents {
            self.sim
                .remove(agent)
                .expect("a session's agents are in the world");
        }
        info!(session = %id, agents = session.agents.len(), "session left");
        if session.creator && self.phase() == RunPhase::Setup {
            self.close(CloseReason::CreatorLeft, None);
            return Flow::Release;
        }
        if self.sessions.is_empty() {
            return Flow::Release;
        }
        if !session.agents.is_empty() {
            self.publish_view();
        }
        if session.viewer.is_some() {
            // Also advances a step the agents' removal made ready.
            self.viewer_left(id);
        } else {
            self.advance_if_ready(Instant::now());
        }
        Flow::Continue
    }

    /// Tell every connected session except `initiator` that the run is closed. The caller
    /// releases the run.
    fn close(&self, reason: CloseReason, initiator: Option<&SessionId>) {
        info!(?reason, "run closed");
        for (id, session) in &self.sessions {
            if Some(id) == initiator {
                continue;
            }
            if let Some(outbox) = &session.outbox {
                let _ = outbox.send(ServerMessage::RunClosed { reason });
            }
        }
    }

    fn start(&mut self, session: &SessionId) -> Result<(), StartRejection> {
        if !self.session_mut(session).creator {
            return Err(StartRejection::NotCreator);
        }
        if self.phase() != RunPhase::Setup {
            return Err(StartRejection::AlreadyStarted);
        }
        // A run needs agents or connected viewers to start.
        if self.sim.agent_count() == 0 && !self.has_viewers() {
            return Err(StartRejection::NotEligible);
        }
        let observations = self
            .sim
            .start()
            .map_err(|_| StartRejection::AlreadyStarted)?;
        info!("run started");
        self.deliver(&observations);
        self.publish_view();
        Ok(())
    }

    fn submit(
        &mut self,
        session: &SessionId,
        state: agora_sim::StateId,
        actions: Vec<(AgentId, Move)>,
    ) -> Result<EntryResults, SubmitRejection> {
        let readiness = self.sim.readiness();
        if readiness.status == Status::Setup {
            return Err(SubmitRejection::NotStarted);
        }
        if state != readiness.state {
            return Err(SubmitRejection::WrongState {
                expected: readiness.state,
                supplied: state,
            });
        }
        let owned = self.session_mut(session).agents.clone();
        Ok(actions
            .into_iter()
            .map(|(agent, action)| {
                let result = if owned.contains(&agent) {
                    self.sim
                        .submit(Submission {
                            agent,
                            target_state: state,
                            action,
                        })
                        .map_err(EntryRejection::Sim)
                } else {
                    Err(EntryRejection::NotOwned)
                };
                (agent, result)
            })
            .collect())
    }

    /// Execute a step if every agent has an accepted action and the pacing gate is open. When
    /// the interval holds a ready step back, `advance_at` schedules it.
    fn advance_if_ready(&mut self, now: Instant) {
        self.advance_at = None;
        if !self.sim.readiness().is_ready() {
            return;
        }
        match self.pacer.gate(now) {
            Gate::Open => {}
            Gate::Until(at) => {
                self.advance_at = Some(at);
                return;
            }
            Gate::Closed => return,
        }
        self.pacer.step_started(now);
        let observations = self.sim.advance().expect("the run is ready to advance");
        debug!(state = observations.state.0, "step executed");
        self.deliver(&observations);
        self.publish_view();
    }

    /// A viewer stopped viewing: hand pacing control to the remaining viewer with the oldest
    /// connection, or return to unlimited pacing when no viewers remain.
    fn viewer_left(&mut self, session: &SessionId) {
        let successor = self
            .sessions
            .iter()
            .filter_map(|(id, s)| s.viewer.as_ref().map(|v| (v.connection, id)))
            .min()
            .map(|(_, id)| id.clone());
        if successor.is_none() {
            self.pacer.viewers_left();
            info!("last viewer left; pacing is unlimited");
        } else if self.pacer.viewer_left(session, successor.clone()) {
            info!(controller = ?successor, "pacing control handed over");
            self.push_pacing();
        }
        self.advance_if_ready(Instant::now());
    }

    /// Push each connected viewer the pacing state as it sees it.
    fn push_pacing(&self) {
        for (id, session) in &self.sessions {
            if let (Some(_), Some(outbox)) = (&session.viewer, &session.outbox) {
                let _ = outbox.send(ServerMessage::PacingUpdate {
                    pacing: self.pacer.state_for(id),
                });
            }
        }
    }

    fn has_viewers(&self) -> bool {
        self.sessions
            .values()
            .any(|session| session.viewer.is_some())
    }

    /// Publish the current view to every connected viewer, replacing any view it has not sent.
    fn publish_view(&self) {
        if !self.has_viewers() {
            return;
        }
        let view = self.view();
        for viewer in self.sessions.values().filter_map(|s| s.viewer.as_ref()) {
            viewer.slot.send_replace(Some(view.clone()));
        }
    }

    fn view(&self) -> View {
        let view = self.sim.view();
        View {
            state_id: StateId::new(view.state.0).expect("state IDs stay below 2^53"),
            phase: self.phase(),
            width: view.width,
            height: view.height,
            agents: view
                .agents
                .iter()
                .map(|agent| AgentView {
                    agent_id: agora_protocol::AgentId::new(agent.id.0)
                        .expect("agent IDs stay below 2^53"),
                    x: agent.position.x,
                    y: agent.position.y,
                })
                .collect(),
        }
    }

    /// Push each connected session the observations of the agents it owns. Sessions without a
    /// connection or without agents receive nothing. A connection sends the response it is
    /// working on before these.
    fn deliver(&self, observations: &Observations) {
        let state_id = StateId::new(observations.state.0).expect("state IDs stay below 2^53");
        for session in self.sessions.values() {
            let Some(outbox) = &session.outbox else {
                continue;
            };
            if session.agents.is_empty() {
                continue;
            }
            let observations = session
                .agents
                .iter()
                .filter(|agent| observations.by_agent.contains_key(agent))
                .map(|agent| AgentObservation {
                    agent_id: agora_protocol::AgentId::new(agent.0)
                        .expect("agent IDs stay below 2^53"),
                    // Simulation observations are empty for the MVP.
                    observation: Observation {},
                })
                .collect();
            // A connection that has just closed drops its receiver; its disconnect follows.
            let _ = outbox.send(ServerMessage::Observations {
                state_id,
                observations,
            });
        }
    }

    fn add_session(&mut self, creator: bool, outbox: Outbox) -> SessionInfo {
        let session_id = SessionId::new(random_id()).expect("random IDs are non-empty");
        self.sessions.insert(
            session_id.clone(),
            Session {
                creator,
                agents: Vec::new(),
                outbox: Some(outbox),
                viewer: None,
                expires_at: None,
            },
        );
        info!(session = %session_id, creator, "session established");
        let state = self.sim.readiness().state;
        SessionInfo {
            session_id,
            catalog_entry: self.catalog_entry.clone(),
            state_id: StateId::new(state.0).expect("state IDs stay below 2^53"),
            phase: self.phase(),
        }
    }

    /// Handle deadlines as of `now`: expire sessions, removing their agents; start a step the
    /// pacing interval held back; and start the release timer. Returns whether the run should be
    /// released.
    fn on_deadline(&mut self, now: Instant) -> Flow {
        let sim = &mut self.sim;
        let mut removed = false;
        let mut creator_expired = false;
        self.sessions.retain(|id, session| {
            let expired = session.expires_at.is_some_and(|at| at <= now);
            if expired {
                creator_expired |= session.creator;
                for &agent in &session.agents {
                    sim.remove(agent)
                        .expect("a session's agents are in the world");
                    removed = true;
                }
                info!(
                    session = %id,
                    creator = session.creator,
                    agents = session.agents.len(),
                    "session expired; its agents were removed"
                );
            }
            !expired
        });
        // A setup run whose creator has expired can never start.
        if creator_expired && self.phase() == RunPhase::Setup {
            self.close(CloseReason::CreatorExpired, None);
            return Flow::Release;
        }
        if removed {
            self.publish_view();
        }
        // The removed agents may have been the only ones without an action, or the step
        // interval may have passed.
        self.advance_if_ready(now);
        if self.sessions.is_empty() && self.release_at.is_none() {
            self.release_at = Some(now + self.config.run_release);
            info!("run has no sessions; release timer started");
        }
        if self.release_at.is_some_and(|at| at <= now) {
            Flow::Release
        } else {
            Flow::Continue
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.sessions
            .values()
            .filter_map(|session| session.expires_at)
            .chain(self.release_at)
            .chain(self.advance_at)
            .min()
    }

    fn session_mut(&mut self, id: &SessionId) -> &mut Session {
        // Only sessions without a connection expire, and commands come from connected sessions.
        self.sessions
            .get_mut(id)
            .expect("a connected session has not expired")
    }

    fn phase(&self) -> RunPhase {
        match self.sim.readiness().status {
            Status::Setup => RunPhase::Setup,
            Status::Collecting { .. } | Status::StartedEmpty => RunPhase::Started,
        }
    }
}

/// The configured step interval, within the range the protocol allows.
fn default_interval(config: &ServerConfig) -> IntervalMs {
    let ms = u64::try_from(config.step_interval.as_millis()).unwrap_or(u64::MAX);
    IntervalMs::new(ms.clamp(IntervalMs::MIN, IntervalMs::MAX)).expect("clamped into range")
}

async fn sleep_until_some(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => future::pending().await,
    }
}

/// A random 128-bit identifier in hexadecimal. Run and session IDs are opaque to clients.
fn random_id() -> String {
    format!("{:032x}", rand::random::<u128>())
}
