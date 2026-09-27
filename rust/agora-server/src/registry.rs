//! The server's table of live runs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use agora_protocol::RunId;

use crate::run::RunHandle;

/// Live runs by ID, shared by every connection. A run removes itself when it is released.
#[derive(Clone, Default)]
pub struct Registry {
    runs: Arc<Mutex<HashMap<RunId, RunHandle>>>,
}

impl Registry {
    pub fn insert(&self, id: RunId, handle: RunHandle) {
        self.lock().insert(id, handle);
    }

    pub fn get(&self, id: &RunId) -> Option<RunHandle> {
        self.lock().get(id).cloned()
    }

    pub fn remove(&self, id: &RunId) {
        self.lock().remove(id);
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<RunId, RunHandle>> {
        // The map is only read or written in single statements, so a panic elsewhere cannot
        // leave it inconsistent.
        self.runs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
