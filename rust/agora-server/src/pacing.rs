//! A run's pacing gate and pacing control (SPEC-003-R11 to R14).

use agora_protocol::{IntervalMs, Pacing, PacingMode, SessionId};
use tokio::time::Instant;

/// Why a pacing request was refused.
pub enum PacingRejection {
    NotViewing,
    ControlHeld,
    NotController,
    NotPaused,
}

/// Whether a ready step may start now.
pub enum Gate {
    Open,
    /// The interval holds the step until this time.
    Until(Instant),
    /// Paused without a granted step.
    Closed,
}

/// A run's pacing mode, its controller, and what the gate needs to know about past steps.
pub struct Pacer {
    mode: PacingMode,
    /// The interval a run uses when its first viewer arrives.
    default_interval: IntervalMs,
    controller: Option<SessionId>,
    /// While paused, whether the controller has allowed one step.
    step_granted: bool,
    /// When the most recent step started.
    last_step: Option<Instant>,
}

impl Pacer {
    /// A pacer for a run without viewers, which is unlimited.
    pub fn new(default_interval: IntervalMs) -> Self {
        Self {
            mode: PacingMode::Unlimited,
            default_interval,
            controller: None,
            step_granted: false,
            last_step: None,
        }
    }

    pub fn gate(&self, now: Instant) -> Gate {
        match self.mode {
            PacingMode::Unlimited => Gate::Open,
            PacingMode::Paused if self.step_granted => Gate::Open,
            PacingMode::Paused => Gate::Closed,
            PacingMode::Interval { ms } => match self.last_step {
                Some(last) if last + ms.as_duration() > now => Gate::Until(last + ms.as_duration()),
                _ => Gate::Open,
            },
        }
    }

    /// Record that a step started, using up any granted step.
    pub fn step_started(&mut self, now: Instant) {
        self.last_step = Some(now);
        self.step_granted = false;
    }

    /// The run gained its first viewer: live viewing starts at the default interval.
    pub fn viewers_arrived(&mut self) {
        self.set_mode(PacingMode::Interval {
            ms: self.default_interval,
        });
        self.controller = None;
    }

    /// The run lost its last viewer: agents advance without limit.
    pub fn viewers_left(&mut self) {
        self.set_mode(PacingMode::Unlimited);
        self.controller = None;
    }

    /// `session`, a viewer, claims control. Returns whether the controller changed.
    pub fn claim(&mut self, session: &SessionId) -> Result<bool, PacingRejection> {
        match &self.controller {
            Some(controller) if controller == session => Ok(false),
            Some(_) => Err(PacingRejection::ControlHeld),
            None => {
                self.controller = Some(session.clone());
                Ok(true)
            }
        }
    }

    pub fn set(&mut self, session: &SessionId, mode: PacingMode) -> Result<(), PacingRejection> {
        self.check_controller(session)?;
        self.set_mode(mode);
        Ok(())
    }

    pub fn step_once(&mut self, session: &SessionId) -> Result<(), PacingRejection> {
        self.check_controller(session)?;
        if self.mode != PacingMode::Paused {
            return Err(PacingRejection::NotPaused);
        }
        self.step_granted = true;
        Ok(())
    }

    /// `session` stopped viewing. If it held control, control passes to `successor`, the
    /// remaining viewer with the oldest connection. Returns whether the controller changed.
    pub fn viewer_left(&mut self, session: &SessionId, successor: Option<SessionId>) -> bool {
        if self.controller.as_ref() != Some(session) {
            return false;
        }
        self.controller = successor;
        true
    }

    /// The pacing state as `session` sees it.
    pub fn state_for(&self, session: &SessionId) -> Pacing {
        Pacing {
            mode: self.mode,
            you_control: self.controller.as_ref() == Some(session),
        }
    }

    fn set_mode(&mut self, mode: PacingMode) {
        self.mode = mode;
        // A granted step belongs to the pause it was granted in.
        self.step_granted = false;
    }

    fn check_controller(&self, session: &SessionId) -> Result<(), PacingRejection> {
        if self.controller.as_ref() == Some(session) {
            Ok(())
        } else {
            Err(PacingRejection::NotController)
        }
    }
}
