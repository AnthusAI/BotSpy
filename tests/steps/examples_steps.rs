//! Steps for the example consumers (spec 04_examples/consumers.feature).

use crate::steps::BotSpyWorld;
use botspy::{KnownPart, Message, Part, Role, Session};
use cucumber::{given, then, when};

// The meter scoring module is the example's own source, compiled into this
// test crate via #[path]: the scenarios below therefore exercise the real
// example logic, and the compile itself proves it only touches the public
// botspy API.
#[path = "../../examples/meter/scoring.rs"]
mod meter_scoring;

pub use meter_scoring::MeterReport;

fn text_session(id: &str, texts: &[&str]) -> Session {
    Session {
        id: id.to_string(),
        agent: botspy::Agent::ClaudeCode,
        project_id: "examples".to_string(),
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        messages: texts
            .iter()
            .map(|text| Message {
                role: Role::User,
                parts: vec![Part::Known(KnownPart::Text {
                    text: (*text).to_string(),
                    extra: None,
                })],
                ..Message::default()
            })
            .collect(),
        ..Session::default()
    }
}

#[given(regex = r#"^a session with messages "([^"]+)", "([^"]+)", and "([^"]+)"$"#)]
fn session_with_three_messages(
    world: &mut BotSpyWorld,
    first: String,
    second: String,
    third: String,
) {
    world.example_sessions = vec![text_session("example-1", &[&first, &second, &third])];
}

#[given(regex = r#"^sessions "([^"]+)" with ([0-9]+) thanks and "([^"]+)" with ([0-9]+) F-bombs$"#)]
fn sessions_with_counts(
    world: &mut BotSpyWorld,
    first: String,
    thanks: usize,
    second: String,
    f_bombs: usize,
) {
    let thanks_texts: Vec<String> = (0..thanks)
        .map(|_| "thanks, that helped".to_string())
        .collect();
    let bomb_texts: Vec<String> = (0..f_bombs)
        .map(|_| "what the f*** is this".to_string())
        .collect();
    let thanks_refs: Vec<&str> = thanks_texts.iter().map(String::as_str).collect();
    let bomb_refs: Vec<&str> = bomb_texts.iter().map(String::as_str).collect();
    world.example_sessions = vec![
        text_session(&first, &thanks_refs),
        text_session(&second, &bomb_refs),
    ];
}

#[given(regex = r#"^the unified history with ([0-9]+) fixture sessions$"#)]
fn unified_history_with_sessions(world: &mut BotSpyWorld, count: usize) {
    world.example_sessions = (0..count)
        .map(|index| {
            let texts = ["thanks, that unblocked me", "the corpus sweep is done"];
            text_session(&format!("example-{index}"), &texts)
        })
        .collect();
}

#[when(regex = r#"^the meter scores the session$"#)]
fn meter_scores_session(world: &mut BotSpyWorld) {
    let session = world.example_sessions.first().expect("no session to score");
    let (thanks, f_bombs) = meter_scoring::score_session(session);
    world.meter_report = Some(MeterReport {
        thanks,
        f_bombs,
        sessions: 1,
    });
}

#[when(regex = r#"^the meter scores the unified history$"#)]
fn meter_scores_history(world: &mut BotSpyWorld) {
    world.meter_report = Some(meter_scoring::score_history(&world.example_sessions));
}

#[when(regex = r#"^an example consumer is built against the published crate$"#)]
fn example_consumer_built(world: &mut BotSpyWorld) {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = manifest.join("examples/meter/main.rs");
    let scoring = manifest.join("examples/meter/scoring.rs");
    assert!(main.is_file(), "the meter example binary is missing");
    assert!(scoring.is_file(), "the meter scoring module is missing");
    world.example_sources = Some(
        std::fs::read_to_string(&main).expect("read meter main")
            + "\n"
            + &std::fs::read_to_string(&scoring).expect("read meter scoring"),
    );
}

#[then(regex = r#"^it reports ([0-9]+) thanks and ([0-9]+) F-bombs?$"#)]
fn meter_reports_counts(world: &mut BotSpyWorld, thanks: u64, f_bombs: u64) {
    let report = world.meter_report.as_ref().expect("no meter report");
    assert_eq!(
        report.thanks, thanks,
        "the meter counted the wrong number of thanks"
    );
    assert_eq!(
        report.f_bombs, f_bombs,
        "the meter counted the wrong number of F-bombs"
    );
}

#[then(regex = r#"^it reports ([0-9]+) thanks and ([0-9]+) F-bombs? across ([0-9]+) sessions$"#)]
fn meter_reports_totals(world: &mut BotSpyWorld, thanks: u64, f_bombs: u64, sessions: u64) {
    meter_reports_counts(world, thanks, f_bombs);
    let report = world.meter_report.as_ref().expect("no meter report");
    assert_eq!(
        report.sessions, sessions,
        "the meter aggregated the wrong number of sessions"
    );
}

#[then(regex = r#"^the ratio favors thanks$"#)]
fn ratio_favors_thanks(world: &mut BotSpyWorld) {
    let report = world.meter_report.as_ref().expect("no meter report");
    assert!(
        meter_scoring::favors_thanks(report),
        "the ratio did not favor thanks: {report:?}"
    );
}

/// Import roots an example consumer may touch: the crate's public surface —
/// public modules plus crate-root type re-exports (`botspy::Session` and
/// friends, which always start with an uppercase letter).
fn is_public_root(import: &str) -> bool {
    let Some(rest) = import.strip_prefix("botspy::") else {
        return false;
    };
    rest.starts_with('{')
        || PUBLIC_MODULES.iter().any(|module| rest.starts_with(module))
        || rest
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_uppercase())
}

const PUBLIC_MODULES: &[&str] = &[
    "adapters", "adapter", "session", "schema", "importer", "snapshot",
];

const DEPENDENCY_FREE_ROOTS: &[&str] = &[
    "std::",
    "botspy",
    "crate::",
    "super::",
    "self::",
    "scoring::",
];

#[then(regex = r#"^it uses only the public API surface$"#)]
fn uses_only_public_api(world: &mut BotSpyWorld) {
    // The meter_scoring module above is compiled from the example's own
    // source, so the build itself is the proof; this check keeps botspy
    // import paths on the documented public roots as well.
    let sources = world.example_sources.as_ref().expect("no example sources");
    let imports = sources
        .lines()
        .map(str::trim_start)
        .filter_map(|line| line.strip_prefix("use ").map(str::trim_start));
    for import in imports.clone() {
        if import.starts_with("botspy::") {
            assert!(
                is_public_root(import),
                "example import {import:?} is not on the public API surface"
            );
        }
    }
    for import in imports {
        assert!(
            DEPENDENCY_FREE_ROOTS
                .iter()
                .any(|root| import.starts_with(root)),
            "example import {import:?} reaches outside botspy and the standard library"
        );
    }
}

#[then(regex = r#"^it adds no dependency to the core library$"#)]
fn adds_no_dependency(world: &mut BotSpyWorld) {
    let sources = world.example_sources.as_ref().expect("no example sources");
    for line in sources
        .lines()
        .map(str::trim_start)
        .filter_map(|line| line.strip_prefix("use ").map(str::trim_start))
    {
        assert!(
            DEPENDENCY_FREE_ROOTS
                .iter()
                .any(|root| line.starts_with(root)),
            "example import {line:?} reaches outside botspy and the standard library"
        );
    }
    let cargo_toml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    let dependencies = cargo_toml
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split('[').next())
        .unwrap_or_default();
    for extra in [
        "vader",
        "goemotions",
        "transformers",
        "candle",
        "tract",
        "ort",
    ] {
        assert!(
            !dependencies.contains(extra),
            "the core library picked up the example-only dependency {extra:?}"
        );
    }
}
