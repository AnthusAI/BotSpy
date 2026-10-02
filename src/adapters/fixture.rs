//! In-memory fixture adapter: drives the behavior specifications and serves
//! as the reference implementation of the adapter contract.

use crate::adapter::Adapter;
use crate::schema::Agent;
use crate::session::{Session, SessionMap, SessionSummary};
use std::fmt;
use std::sync::Mutex;

pub struct FixtureAdapter {
    agent: Agent,
    sessions: Mutex<SessionMap>,
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
        }
    }

    pub fn add_session(&self, session: Session) {
        self.sessions
            .lock()
            .expect("fixture adapter lock poisoned")
            .insert(session.id.clone(), session);
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
        self.sessions
            .lock()
            .expect("fixture adapter lock poisoned")
            .get(id)
            .cloned()
    }
}
