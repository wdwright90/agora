//! The order in which a step runs its systems (ADR-003).
//!
//! A step is a fixed sequence of [`Stage`]s. Every system belongs to exactly one stage, and
//! each stage finishes before the next begins. Perception runs in its own schedule, after a
//! step and whenever observations are needed without one, such as at Start.
//!
//! CDD-001's stage table documents the same order, and a test keeps the two in agreement.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel, SingleThreadedExecutor};

/// The stages of a step. [`Stage::ALL`] sets the order they run in: to reorder stages, reorder
/// that list; to add one, add a variant, place it in the list, and add a row to CDD-001's stage
/// table.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Stage {
    /// Agents' actions, one agent at a time in the step's shuffled order.
    Actions,
    /// Effects of where agents ended up, such as eating.
    Interactions,
    /// Energy accounting: decay, exertion, and starvation.
    Metabolism,
    /// Removal of starved agents and queued removals.
    Removal,
    /// Ecology rules, such as respawning food.
    Ecology,
    /// Queued spawns.
    Membership,
    /// The state ID goes from N to N+1.
    Commit,
}

impl Stage {
    /// Every stage, in run order. This is the only place the order is set.
    pub(crate) const ALL: [Self; 7] = [
        Self::Actions,
        Self::Interactions,
        Self::Metabolism,
        Self::Removal,
        Self::Ecology,
        Self::Membership,
        Self::Commit,
    ];
}

/// Label of the schedule that executes a step.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Step;

/// Label of the schedule that generates observations for the current state.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Perceive;

/// An empty schedule with the simulation's settings: a single-threaded executor, and
/// ambiguity detection that makes two systems with conflicting access and no declared order
/// fail the build. Results therefore never depend on how systems are scheduled.
pub(crate) fn new_schedule(label: impl ScheduleLabel) -> Schedule {
    let mut schedule = Schedule::new(label);
    schedule.set_build_settings(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Error,
        ..ScheduleBuildSettings::default()
    });
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule
}

/// The step schedule with its stages chained in run order and no systems yet.
pub(crate) fn step_schedule() -> Schedule {
    let mut schedule = new_schedule(Step);
    for pair in Stage::ALL.windows(2) {
        schedule.configure_sets(pair[0].before(pair[1]));
    }
    schedule
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC-001-R21: the stage order in code matches CDD-001's stage table.
    #[test]
    fn r21_stage_order_matches_the_cdd() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/components/simulation/cdd.md"
        );
        let cdd = std::fs::read_to_string(path).expect("CDD-001 is readable");
        let table: Vec<String> = cdd
            .lines()
            .skip_while(|line| !line.starts_with("### Step stages"))
            .skip_while(|line| !line.starts_with('|'))
            .take_while(|line| line.starts_with('|'))
            .skip(2)
            .map(|row| {
                row.split('|')
                    .nth(1)
                    .expect("a table row has cells")
                    .trim()
                    .trim_matches('*')
                    .to_owned()
            })
            .collect();

        let mut expected: Vec<String> = Stage::ALL.iter().map(|s| format!("{s:?}")).collect();
        expected.push("Perception".to_owned());
        assert_eq!(table, expected);
    }

    /// SPEC-001-R21: the step schedule chains every stage, in the order of `Stage::ALL`.
    #[test]
    fn r21_step_schedule_runs_stages_in_order() {
        #[derive(Resource, Default)]
        struct Ran(Vec<Stage>);

        let mut world = World::new();
        world.init_resource::<Ran>();
        let mut schedule = step_schedule();
        for stage in Stage::ALL {
            schedule.add_systems((move |mut ran: ResMut<Ran>| ran.0.push(stage)).in_set(stage));
        }
        schedule.run(&mut world);
        assert_eq!(world.resource::<Ran>().0, Stage::ALL);
    }

    /// SPEC-001-R21: two systems in one stage that write the same data without a declared
    /// order fail the schedule build, and declaring the order fixes it.
    #[test]
    fn r21_unordered_conflicting_systems_are_rejected() {
        #[derive(Resource, Default)]
        struct Shared(u32);
        fn first(mut shared: ResMut<Shared>) {
            shared.0 += 1;
        }
        fn second(mut shared: ResMut<Shared>) {
            shared.0 *= 2;
        }

        let mut world = World::new();
        world.init_resource::<Shared>();
        let mut ambiguous = step_schedule();
        ambiguous.add_systems((first, second).in_set(Stage::Actions));
        assert!(ambiguous.initialize(&mut world).is_err());

        let mut ordered = step_schedule();
        ordered.add_systems((first, second).chain().in_set(Stage::Actions));
        assert!(ordered.initialize(&mut world).is_ok());

        // Stage order applies across stages with no systems in between.
        let mut staged = step_schedule();
        staged.add_systems((first.in_set(Stage::Actions), second.in_set(Stage::Commit)));
        assert!(staged.initialize(&mut world).is_ok());
    }
}
