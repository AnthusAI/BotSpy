//! The adapter contract: every source adapter implements this.
//!
//! Discovery finds sessions under the adapter's roots; opening a session
//! streams its records without loading whole transcripts into memory (that
//! streaming layer arrives with the per-agent adapters). Adapters are
//! read-only: they never write to the sources they tap.

use crate::schema::Agent;
use crate::session::{Session, SessionSummary};

pub trait Adapter: Send + Sync {
    /// The agent this adapter taps.
    fn agent(&self) -> Agent;

    /// Discover sessions available from this source.
    fn discover(&self) -> Vec<SessionSummary>;

    /// Open one session by id; `None` when this source does not know it.
    fn open(&self, id: &str) -> Option<Session>;
}
