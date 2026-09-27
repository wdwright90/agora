//! Pacing modes and the pacing state reported to viewers.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ids::OutOfRange;

/// A step interval in milliseconds, from 1 ms to one hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct IntervalMs(u64);

impl IntervalMs {
    pub const MIN: u64 = 1;
    pub const MAX: u64 = 60 * 60 * 1000;

    /// Returns `None` outside `MIN..=MAX`.
    pub const fn new(ms: u64) -> Option<Self> {
        if ms >= Self::MIN && ms <= Self::MAX {
            Some(Self(ms))
        } else {
            None
        }
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn as_duration(self) -> Duration {
        Duration::from_millis(self.0)
    }
}

impl TryFrom<u64> for IntervalMs {
    type Error = OutOfRange;

    fn try_from(ms: u64) -> Result<Self, Self::Error> {
        Self::new(ms).ok_or(OutOfRange {
            value: ms,
            min: Self::MIN,
            max: Self::MAX,
        })
    }
}

impl From<IntervalMs> for u64 {
    fn from(interval: IntervalMs) -> u64 {
        interval.0
    }
}

/// How a run's steps are paced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PacingMode {
    /// No step starts until the controller resumes or grants a single step.
    Paused,
    /// A step starts once every agent is ready, and no sooner than `ms` after the previous one.
    Interval { ms: IntervalMs },
    /// A step starts as soon as every agent is ready.
    Unlimited,
}

/// A run's pacing state, as one viewer sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pacing {
    pub mode: PacingMode,
    /// Whether this viewer's session holds pacing control.
    pub you_control: bool,
}
