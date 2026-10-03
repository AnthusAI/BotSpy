//! In-memory fixture adapter: drives the behavior specifications and serves
//! as the reference implementation of the adapter contract.

use crate::adapter::Adapter;
use crate::schema::Agent;
use crate::session::{Session, SessionMap, SessionSummary};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct FixtureAdapter {
    agent: Agent,
    sessions: Mutex<SessionMap>,
    /// Test-infra observability only, in the SkipCounter spirit: how many
    /// times `open` was called. The specs use it to prove that a refresh
    /// pass does not re-open unchanged sessions. Deliberate src/adapters
    /// touch for this initiative (BOTSPY-c7795b).
    opens: AtomicUsize,
    /// Session ids discoverable but unopenable (test-infra only).
    blocked: Mutex<BTreeSet<String>>,
}

impl fmt::Debug for FixtureAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self
            .sessions
            .lock()
            .map(|sessions| sessions.len())
            .unwrap_or_default();
        f.debug_struct("FixtureAdapter")
            .field("agent", &self.agent)
            .field("sessions", &count)
            .finish()
    }
}

impl FixtureAdapter {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent,
            sessions: Mutex::new(SessionMap::new()),
            opens: AtomicUsize::new(0),
            blocked: Mutex::new(BTreeSet::new()),
        }
    }

    pub fn add_session(&self, session: Session) {
        self.sessions
            .lock()
            .expect("fixture adapter lock poisoned")
            .insert(session.id.clone(), session);
    }

    /// Drop a session from the fixture (specs use it to prove a session
    /// gone from its source is pruned from the store).
    pub fn remove_session(&self, id: &str) -> bool {
        self.sessions
            .lock()
            .expect("fixture adapter lock poisoned")
            .remove(id)
            .is_some()
    }

    /// Make a session discoverable but unopenable (specs use it to prove
    /// a session the adapter reports but cannot open is counted as an
    /// error, never silently dropped). Test-infra observability only, in
    /// the SkipCounter spirit.
    pub fn block_open(&self, id: &str) {
        self.blocked
            .lock()
            .expect("fixture adapter lock poisoned")
            .insert(id.to_string());
    }

    /// How many times `open` has been called on this adapter.
    pub fn open_count(&self) -> usize {
        self.opens.load(Ordering::SeqCst)
    }
}

impl Adapter for FixtureAdapter {
    fn agent(&self) -> Agent {
        self.agent
    }

    fn discover(&self) -> Vec<SessionSummary> {
        self.sessions
            .lock()
            .expect("fixture adapter lock poisoned")
            .values()
            .map(SessionSummary::from)
            .collect()
    }

    fn open(&self, id: &str) -> Option<Session> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        if self
            .blocked
            .lock()
            .expect("fixture adapter lock poisoned")
            .contains(id)
        {
            return None;
        }
        self.sessions
            .lock()
            .expect("fixture adapter lock poisoned")
            .get(id)
            .cloned()
    }
}
