//! Human and machine rendering for the CLI verbs. Human output
//! truncates ids and paths and stays plain (pipe-tolerant); machine
//! output is serde of the library's own types and never truncates.

use crate::cli::agent_name;
use crate::schema::Agent;

/// The width under which human-table values are never truncated.
pub const TRUNCATE_WIDTH: usize = 38;

/// Truncate a long value for human output, keeping its tail (the most
/// specific part of a path) and marking the cut.
pub fn truncate(value: &str, keep_full: bool) -> String {
    if keep_full || value.chars().count() <= TRUNCATE_WIDTH {
        value.to_string()
    } else {
        let tail: String = value
            .chars()
            .skip(value.chars().count() - (TRUNCATE_WIDTH - 1))
            .collect();
        format!("…{tail}")
    }
}

/// Left-pad a value into a fixed-width column.
pub fn column(value: &str, width: usize) -> String {
    let visible = value.chars().count();
    if visible >= width {
        value.to_string()
    } else {
        format!("{value}{}", " ".repeat(width - visible))
    }
}

/// The dash human output shows for values an agent never recorded.
pub const DASH: &str = "—";

/// The agent display name.
pub fn agent(agent: Agent) -> &'static str {
    agent_name(agent)
}

/// Right-align a number into a fixed-width column.
pub fn number(value: usize, width: usize) -> String {
    number_str(&value.to_string(), width)
}

/// Right-align text into a fixed-width column.
pub fn number_str(value: &str, width: usize) -> String {
    if value.chars().count() >= width {
        value.to_string()
    } else {
        format!("{}{value}", " ".repeat(width - value.chars().count()))
    }
}

/// Singularize "sessions" for count lines ("1 session").
pub fn count_line(count: usize) -> String {
    match count {
        1 => "1 session".to_string(),
        n => format!("{n} sessions"),
    }
}
