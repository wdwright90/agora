//! Which controls the viewer offers, given what it knows about the run (SPEC-005-R05).

use agora_client::protocol::{Pacing, PacingMode, RunPhase, View};

/// The controls to enable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Controls {
    /// Start the run: the creator, in setup, with at least one agent. A run started with no
    /// agents could never step.
    pub start: bool,
    /// Ask for pacing control: offered while this viewer does not hold it. The server
    /// rejects the claim if another viewer does.
    pub claim: bool,
    /// Pause: the controller, while not paused.
    pub pause: bool,
    /// Resume at the chosen interval, and Step: the controller, while paused.
    pub resume: bool,
    pub step: bool,
    /// Set the interval or unlimited pacing: the controller.
    pub pacing: bool,
    /// Close the run for everyone: the creator, while watching.
    pub close: bool,
}

pub fn controls(creator: bool, view: Option<&View>, pacing: Option<&Pacing>) -> Controls {
    let start = creator
        && view.is_some_and(|view| view.phase == RunPhase::Setup && !view.agents.is_empty());
    let close = creator && view.is_some();
    let Some(pacing) = pacing else {
        return Controls {
            start,
            close,
            ..Controls::default()
        };
    };
    let controller = pacing.you_control;
    let paused = pacing.mode == PacingMode::Paused;
    Controls {
        start,
        claim: !controller,
        pause: controller && !paused,
        resume: controller && paused,
        step: controller && paused,
        pacing: controller,
        close,
    }
}

#[cfg(test)]
mod tests {
    use agora_client::protocol::{AgentId, AgentView, IntervalMs, KindId, StateId};

    use super::*;

    fn view(phase: RunPhase, agents: u64) -> View {
        View {
            state_id: StateId::new(0).unwrap(),
            phase,
            width: 10,
            height: 10,
            agents: (1..=agents)
                .map(|n| AgentView {
                    agent_id: AgentId::new(n).unwrap(),
                    kind: KindId::new("agent").unwrap(),
                    x: 0,
                    y: n as u32,
                })
                .collect(),
        }
    }

    fn pacing(mode: PacingMode, you_control: bool) -> Pacing {
        Pacing { mode, you_control }
    }

    fn interval() -> PacingMode {
        PacingMode::Interval {
            ms: IntervalMs::new(500).unwrap(),
        }
    }

    #[test]
    fn r05_start_needs_the_creator_in_setup_with_an_agent() {
        let setup = view(RunPhase::Setup, 1);
        assert!(controls(true, Some(&setup), None).start);
        assert!(!controls(false, Some(&setup), None).start);
        assert!(!controls(true, Some(&view(RunPhase::Setup, 0)), None).start);
        assert!(!controls(true, Some(&view(RunPhase::Started, 1)), None).start);
        assert!(!controls(true, None, None).start);
    }

    #[test]
    fn r05_closing_needs_the_creator() {
        let setup = view(RunPhase::Setup, 0);
        assert!(controls(true, Some(&setup), None).close);
        assert!(!controls(false, Some(&setup), None).close);
        assert!(!controls(true, None, None).close);
    }

    #[test]
    fn r05_pacing_controls_need_control() {
        let watching = controls(false, None, Some(&pacing(interval(), false)));
        assert_eq!(
            watching,
            Controls {
                claim: true,
                ..Controls::default()
            }
        );
        let running = controls(false, None, Some(&pacing(interval(), true)));
        assert!(running.pause && running.pacing && !running.claim);
        assert!(!running.resume && !running.step);
        let paused = controls(false, None, Some(&pacing(PacingMode::Paused, true)));
        assert!(paused.resume && paused.step && paused.pacing && !paused.pause);
    }
}
