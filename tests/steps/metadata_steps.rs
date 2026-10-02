//! Steps for optional session metadata (spec 11_session_metadata).

use crate::steps::{current_session, save_session, BotSpyWorld};
use botspy::{SessionMetadata, SessionSummary};
use cucumber::{given, then, when};

fn set_metadata(world: &mut BotSpyWorld, f: impl FnOnce(&mut SessionMetadata)) {
    let mut session = current_session(world);
    f(&mut session.metadata);
    save_session(world, session);
}

#[when(regex = r#"the session records title "([^"]+)", git branch "([^"]+)" and pr_url "([^"]+)""#)]
fn record_title_branch_pr(world: &mut BotSpyWorld, title: String, branch: String, pr: String) {
    set_metadata(world, |m| {
        m.title = Some(title);
        m.git_branch = Some(branch);
        m.pr_url = Some(pr);
    });
}

#[when(
    regex = r#"the session records git branch "([^"]+)", git commit "([^"]+)" and origin "([^"]+)""#
)]
fn record_git_coordinates(world: &mut BotSpyWorld, branch: String, commit: String, origin: String) {
    set_metadata(world, |m| {
        m.git_branch = Some(branch);
        m.git_commit = Some(commit);
        m.git_origin_url = Some(origin);
    });
}

#[when(regex = r#"the session records cwd "([^"]+)""#)]
fn record_cwd(world: &mut BotSpyWorld, cwd: String) {
    set_metadata(world, |m| m.cwd = Some(cwd));
}

#[when(regex = r#"the session is archived and pinned"#)]
fn record_archived_pinned(world: &mut BotSpyWorld) {
    set_metadata(world, |m| {
        m.archived = Some(true);
        m.pinned = Some(true);
        m.status = Some("archived".to_string());
    });
}

#[when(regex = r#"the session records app version "([^"]+)" and model "([^"]+)""#)]
fn record_app_version_model(world: &mut BotSpyWorld, version: String, model: String) {
    set_metadata(world, |m| {
        m.app_version = Some(version);
        m.models = vec![model];
    });
}

#[given(
    regex = r#"a fixture session "([^"]+)" from agent "([^"]+)" in project "([^"]+)" that started on "([^"]+)""#
)]
fn session_with_model(
    world: &mut BotSpyWorld,
    id: String,
    agent: String,
    project: String,
    model: String,
) {
    let adapter = crate::steps::adapter_for(world, &agent);
    adapter.add_session(botspy::Session {
        id: id.clone(),
        agent: crate::steps::parse_agent(&agent),
        project_id: project,
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        metadata: SessionMetadata {
            models: vec![model],
            ..SessionMetadata::default()
        },
        ..botspy::Session::default()
    });
    world.current_session = Some(id);
}

#[when(regex = r#"the session changes its model mid-flight to "([^"]+)""#)]
fn change_model_mid_flight(world: &mut BotSpyWorld, model: String) {
    set_metadata(world, |m| {
        if !m.models.last().is_some_and(|last| *last == model) {
            m.models.push(model);
        }
    });
}

/// The summary of the current fixture session, as listing would produce it.
fn summary(world: &mut BotSpyWorld) -> SessionSummary {
    SessionSummary::from(&current_session(world))
}

#[then(regex = r#"the session summary includes ([a-z_ ]+?) "(.*)"$"#)]
fn summary_includes_string(world: &mut BotSpyWorld, field: String, expected: String) {
    let metadata = summary(world).metadata;
    let field = field.trim().to_string();
    let actual: Option<String> = match field.as_str() {
        "title" => metadata.title,
        "git branch" => metadata.git_branch,
        "git_branch" => metadata.git_branch,
        "git commit" => metadata.git_commit,
        "git_commit" => metadata.git_commit,
        "origin" | "git_origin_url" => metadata.git_origin_url,
        "cwd" => metadata.cwd,
        "status" => metadata.status,
        "app version" | "app_version" => metadata.app_version,
        "model" => metadata.models.last().cloned(),
        "models" => (!metadata.models.is_empty()).then(|| metadata.models.join(", ")),
        "pr_url" => metadata.pr_url,
        other => panic!("unknown summary metadata field: {other}"),
    };
    assert_eq!(actual, Some(expected), "summary metadata field {field}");
}

#[then(regex = r#"the session summary includes ([a-z_]+) true$"#)]
fn summary_includes_true(world: &mut BotSpyWorld, field: String) {
    let metadata = summary(world).metadata;
    let actual = match field.as_str() {
        "archived" => metadata.archived,
        "pinned" => metadata.pinned,
        other => panic!("unknown boolean summary metadata field: {other}"),
    };
    assert_eq!(actual, Some(true), "summary metadata field {field}");
}

#[then(regex = r#"the session summary has no title$"#)]
fn summary_has_no_title(world: &mut BotSpyWorld) {
    assert!(
        summary(world).metadata.title.is_none(),
        "title should be absent, not empty"
    );
}
