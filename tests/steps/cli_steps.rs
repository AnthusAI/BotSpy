//! Steps for the CLI specifications (features/cli/).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use botspy::cli;
use cucumber::{given, then, when};
use serde_json::Value;

use crate::BotSpyWorld;

static SEQ: AtomicUsize = AtomicUsize::new(0);

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn claude_projects_root() -> PathBuf {
    corpus_root().join("claude_code/projects")
}

fn cursor_store_path() -> PathBuf {
    corpus_root().join("cursor/store/state.vscdb")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("botspy-cli-spec").join(format!(
        "{}-{}-{tag}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn symlink_entry(link_path: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link_path).unwrap_or_else(|err| {
        panic!(
            "symlink {} -> {}: {err}",
            link_path.display(),
            target.display()
        )
    });
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

/// A deterministic Claude Code session with known ids, timestamps, and
/// text, so scenarios can assert on its content.
fn write_solo_session(home: &Path) {
    let dir = home.join(".claude/projects/extra-demo");
    std::fs::create_dir_all(&dir).expect("create extra project dir");
    std::fs::write(
        dir.join("solo-abc123.jsonl"),
        concat!(
            r#"{"type":"user","uuid":"u1","timestamp":"2026-09-15T12:00:00Z","message":{"role":"user","content":"hello from the fixture"}}"#,
            "\n",
            r#"{"type":"user","uuid":"u2","timestamp":"2026-09-15T12:05:00Z","message":{"role":"user","content":"second message"}}"#,
            "\n",
        ),
    )
    .expect("write solo transcript");
}

/// Lay out the fixture home the way the sources' real homes look,
/// pointing the per-agent locations at the synthetic corpus. Only the
/// listed sources get locations; the rest are missing.
fn build_home(home: &Path, sources: &[&str]) {
    for source in sources {
        match *source {
            "claude_code" => {
                let projects = home.join(".claude/projects");
                std::fs::create_dir_all(&projects).expect("create projects dir");
                let corpus = claude_projects_root();
                for entry in std::fs::read_dir(&corpus).expect("corpus claude projects") {
                    let project = entry.expect("corpus entry").path();
                    symlink_entry(&projects.join(file_name(&project)), &project);
                }
                write_solo_session(home);
            }
            "cursor" => {
                std::fs::create_dir_all(home.join(".cursor")).expect("create .cursor");
                symlink_entry(
                    &home.join(".cursor/state.vscdb"),
                    &cursor_store_path(),
                );
            }
            "codex" => symlink_entry(&home.join(".codex"), &corpus_root().join("codex")),
            "grok_bot" => {
                std::fs::create_dir_all(home.join(".grok")).expect("create .grok");
                symlink_entry(
                    &home.join(".grok/sand-client-persistence"),
                    &corpus_root().join("grok_bot/sand-client-persistence"),
                );
            }
            "antigravity" => {
                std::fs::create_dir_all(home.join(".gemini")).expect("create .gemini");
                symlink_entry(
                    &home.join(".gemini/antigravity"),
                    &corpus_root().join("antigravity"),
                );
            }
            other => panic!("unknown fixture source {other}"),
        }
    }
}

#[given(expr = "a fixture home built from the synthetic corpus")]
fn fixture_home(world: &mut BotSpyWorld) {
    let home = scratch("home");
    build_home(
        &home,
        &["claude_code", "cursor", "codex", "grok_bot", "antigravity"],
    );
    world.cli_home = Some(home);
}

#[given(regex = r#"^a fixture home with only the sources "(.*)"$"#)]
fn fixture_home_subset(world: &mut BotSpyWorld, list: String) {
    let sources: Vec<&str> = list.split(',').map(str::trim).collect();
    let home = scratch("home-subset");
    build_home(&home, &sources);
    world.cli_home = Some(home);
}

fn run_cli(world: &mut BotSpyWorld, args: &str, with_home: bool) {
    let mut argv: Vec<String> = vec!["botspy".to_string()];
    argv.extend(expand(args).split_whitespace().map(str::to_string));
    if with_home {
        argv.push("--home".to_string());
        argv.push(
            world
                .cli_home
                .as_ref()
                .expect("fixture home")
                .display()
                .to_string(),
        );
    }
    world.cli_run = Some(cli::run_from(argv));
}

/// Replace the spec placeholders with concrete fixture paths. A fresh
/// `{out_dir}` per step is fine: the snapshot path is asserted from the
/// output itself, not from this placeholder.
fn expand(text: &str) -> String {
    text.replace("{out_dir}", &scratch("out").to_string_lossy())
        .replace("{claude_projects_root}", &claude_projects_root().to_string_lossy())
        .replace("{cursor_store_path}", &cursor_store_path().to_string_lossy())
}

#[when(regex = r#"^I run "botspy (.*?)" against the fixture home$"#)]
fn run_against_home(world: &mut BotSpyWorld, args: String) {
    run_cli(world, &args, true);
}

#[when(regex = r#"^I run "botspy (.*?)" against the fixture corpus$"#)]
fn run_against_corpus(world: &mut BotSpyWorld, args: String) {
    run_cli(world, &args, false);
}

#[when(regex = r#"^I run "botspy (.*?)" with BOTSPY_HOME at the fixture home$"#)]
fn run_with_botspy_home(world: &mut BotSpyWorld, args: String) {
    let home = world.cli_home.clone().expect("fixture home");
    std::env::set_var("BOTSPY_HOME", &home);
    run_cli(world, &args, false);
    std::env::remove_var("BOTSPY_HOME");
}

#[when(regex = r#"^I run "botspy ([^"]*)"$"#)]
fn run_plain(world: &mut BotSpyWorld, args: String) {
    run_cli(world, &args, false);
}

fn run(world: &BotSpyWorld) -> &cli::RunOutcome {
    world.cli_run.as_ref().expect("no CLI run recorded")
}

fn json(world: &BotSpyWorld) -> Value {
    serde_json::from_str(&run(world).stdout).expect("stdout is valid JSON")
}

fn expand_captured(text: &str) -> String {
    expand(text)
}

fn assert_all_in(label: &str, haystack: &str, text: &str) {
    let needle = expand_captured(text);
    let parts: Vec<String> = needle
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    assert!(
        !parts.is_empty(),
        "no quoted expectations in {label}: {needle}"
    );
    for part in parts {
        assert!(
            haystack.contains(&part),
            "{label} does not contain {part:?}; actual: {haystack:?}"
        );
    }
}

fn assert_none_in(label: &str, haystack: &str, text: &str) {
    for part in text.split('"').skip(1).step_by(2) {
        assert!(
            !haystack.contains(part),
            "{label} must not contain {part:?}; actual: {haystack:?}"
        );
    }
}

fn full_claude_projects_path(world: &BotSpyWorld) -> String {
    world
        .cli_home
        .as_ref()
        .expect("fixture home")
        .join(".claude/projects")
        .display()
        .to_string()
}

#[then(regex = r#"^the exit code is ([0-9]+)$"#)]
fn exit_code(world: &mut BotSpyWorld, code: String) {
    let actual = run(world).code;
    assert_eq!(
        actual,
        code.parse::<i32>().expect("numeric exit code"),
        "unexpected exit code; stdout: {:?}, stderr: {:?}",
        run(world).stdout,
        run(world).stderr
    );
}

#[then(regex = r#"^stdout contains (.+)$"#)]
fn stdout_contains(world: &mut BotSpyWorld, text: String) {
    assert_all_in("stdout", &run(world).stdout, &text);
}

#[then(regex = r#"^stdout does not contain (.+)$"#)]
fn stdout_not_contains(world: &mut BotSpyWorld, text: String) {
    assert_none_in("stdout", &run(world).stdout, &text);
}

#[then(regex = r#"^stderr contains (.+)$"#)]
fn stderr_contains(world: &mut BotSpyWorld, text: String) {
    assert_all_in("stderr", &run(world).stderr, &text);
}

#[then(regex = r#"^stderr mentions (.+)$"#)]
fn stderr_mentions(world: &mut BotSpyWorld, text: String) {
    assert_all_in("stderr", &run(world).stderr, &text);
}

#[then(regex = r#"^stdout reports "([^"]+)"$"#)]
fn stdout_reports(world: &mut BotSpyWorld, expected: String) {
    let stdout = &run(world).stdout;
    assert!(
        stdout.contains(expected.as_str()),
        "stdout does not report {expected:?}; actual: {stdout:?}"
    );
}

#[then(regex = r#"^stdout parses as a JSON array of ([0-9]+) entries$"#)]
fn stdout_json_array(world: &mut BotSpyWorld, count: String) {
    let value = json(world);
    let count = count.parse::<usize>().expect("numeric count");
    assert!(
        value.is_array(),
        "stdout is not a JSON array: {value:?}"
    );
    let entries = value.as_array().expect("array");
    assert_eq!(
        entries.len(),
        count,
        "unexpected JSON array length; entries: {entries:?}"
    );
}

#[then(regex = r#"^stdout parses as a JSON object$"#)]
fn stdout_json_object(world: &mut BotSpyWorld) {
    let value = json(world);
    assert!(value.is_object(), "stdout is not a JSON object: {value:?}");
}

#[then(regex = r#"^stdout parses as a JSON object with "([^"]+)" "(.*)"$"#)]
fn stdout_json_object_with(world: &mut BotSpyWorld, key: String, expected: String) {
    let value = json(world);
    let object = value.as_object().expect("object");
    let actual = object
        .get(&key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing or non-string {key}: {value:?}"));
    assert_eq!(actual, expand_captured(&expected), "unexpected {key}");
}

#[then(regex = r#"^stdout parses as NDJSON with ([0-9]+) lines$"#)]
fn stdout_ndjson(world: &mut BotSpyWorld, count: String) {
    let stdout = &run(world).stdout;
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.trim().is_empty()).collect();
    let count = count.parse::<usize>().expect("numeric count");
    assert_eq!(
        lines.len(),
        count,
        "unexpected NDJSON line count; stdout: {stdout:?}"
    );
    for line in lines {
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|err| panic!("NDJSON line is not JSON ({err}): {line:?}"));
        assert!(value.is_object(), "NDJSON line is not an object: {line:?}");
    }
}

fn json_entries(world: &mut BotSpyWorld) -> Vec<Value> {
    let value = json(world);
    value
        .as_array()
        .unwrap_or_else(|| panic!("stdout is not a JSON array: {value:?}"))
        .clone()
}

fn entry_named<'a>(entries: &'a [Value], name: &str) -> &'a Value {
    entries
        .iter()
        .find(|entry| entry.get("name").and_then(Value::as_str) == Some(name))
        .unwrap_or_else(|| panic!("no JSON entry named {name}: {entries:?}"))
}

#[then(regex = r#"^the JSON entry "([^"]+)" has "([^"]+)" "(.*)"$"#)]
fn json_entry_string(world: &mut BotSpyWorld, name: String, key: String, expected: String) {
    let entries = json_entries(world);
    let entry = entry_named(&entries, &name);
    let actual = entry
        .get(&key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing or non-string {key} in {name}: {entry:?}"));
    assert_eq!(actual, expand_captured(&expected), "unexpected {key} of {name}");
}

#[then(regex = r#"^the JSON entry "([^"]+)" has "([^"]+)" (-?[0-9]+)$"#)]
fn json_entry_number(world: &mut BotSpyWorld, name: String, key: String, expected: String) {
    let entries = json_entries(world);
    let entry = entry_named(&entries, &name);
    let expected: i64 = expected.parse().expect("numeric expectation");
    let actual = entry
        .get(&key)
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("missing or non-numeric {key} in {name}: {entry:?}"));
    assert_eq!(actual, expected, "unexpected {key} of {name}");
}

#[then(regex = r#"^every JSON entry has a non-empty "([^"]+)"$"#)]
fn every_entry_non_empty(world: &mut BotSpyWorld, key: String) {
    for entry in json_entries(world) {
        let value = entry
            .get(&key)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("missing or non-string {key}: {entry:?}"));
        assert!(!value.is_empty(), "empty {key}: {entry:?}");
    }
}

#[then(regex = r#"^every JSON entry has "([^"]+)" "([^"]+)"$"#)]
fn every_entry_equals(world: &mut BotSpyWorld, key: String, expected: String) {
    for entry in json_entries(world) {
        let actual = entry
            .get(&key)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("missing or non-string {key}: {entry:?}"));
        assert_eq!(actual, expected, "unexpected {key}: {entry:?}");
    }
}

#[then(regex = r#"^every JSON entry has "([^"]+)", "([^"]+)" and "([^"]+)"$"#)]
fn every_entry_has_keys(world: &mut BotSpyWorld, first: String, second: String, third: String) {
    for entry in json_entries(world) {
        for key in [&first, &second, &third] {
            assert!(
                entry.get(key).is_some(),
                "missing {key}: {entry:?}"
            );
        }
    }
}

#[then(regex = r#"^every JSON entry has an "([^"]+)" list$"#)]
fn every_entry_has_list(world: &mut BotSpyWorld, key: String) {
    for entry in json_entries(world) {
        assert!(
            entry.get(&key).is_some_and(Value::is_array),
            "missing {key} list: {entry:?}"
        );
    }
}

#[then(expr = "no JSON entry has any issues")]
fn no_entry_has_issues(world: &mut BotSpyWorld) {
    for entry in json_entries(world) {
        let issues = entry
            .get("issues")
            .and_then(Value::as_array)
            .expect("issues list");
        assert!(
            issues.is_empty(),
            "unexpected issues: {issues:?} in {entry:?}"
        );
    }
}

#[then(regex = r#"^the JSON "([^"]+)" list has ([0-9]+) entries$"#)]
fn json_list_length(world: &mut BotSpyWorld, key: String, count: String) {
    let value = json(world);
    let list = value
        .get(&key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("missing {key} list: {value:?}"));
    assert_eq!(list.len(), count.parse::<usize>().expect("numeric count"));
}

#[then(regex = r#"^the JSON "([^"]+)" is (true|false)$"#)]
fn json_bool(world: &mut BotSpyWorld, key: String, expected: String) {
    let value = json(world);
    let actual = value
        .get(&key)
        .and_then(Value::as_bool)
        .unwrap_or_else(|| panic!("missing or non-bool {key}: {value:?}"));
    assert_eq!(actual, expected == "true", "unexpected {key}");
}

#[then(regex = r#"^the JSON "([^"]+)" is more than ([0-9]+)$"#)]
fn json_number_above(world: &mut BotSpyWorld, key: String, bound: String) {
    let value = json(world);
    let actual = value
        .get(&key)
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("missing or non-numeric {key}: {value:?}"));
    assert!(
        actual > bound.parse::<i64>().expect("numeric bound"),
        "{key} is not more than {bound}: {actual}"
    );
}

fn line_after(world: &BotSpyWorld, label: &str) -> String {
    let prefix = format!("{label} ");
    run(world)
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("no line after {label:?} in {:?}", run(world).stdout))
        .trim()
        .to_string()
}

#[then(regex = r#"^the path after "([^"]+)" exists$"#)]
fn path_after_exists(world: &mut BotSpyWorld, label: String) {
    let path = line_after(world, &label);
    assert!(
        Path::new(&path).exists(),
        "path after {label:?} does not exist: {path}"
    );
}

#[then(regex = r#"^the number after "([^"]+)" is more than ([0-9]+)$"#)]
fn number_after_above(world: &mut BotSpyWorld, label: String, bound: String) {
    let text = line_after(world, &label);
    let actual: i64 = text
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no number after {label:?}: {text:?}"));
    assert!(
        actual > bound.parse::<i64>().expect("numeric bound"),
        "number after {label:?} is not more than {bound}: {actual}"
    );
}

#[then(expr = "stdout contains the full claude projects path")]
fn stdout_has_full_claude_path(world: &mut BotSpyWorld) {
    let full = full_claude_projects_path(world);
    assert!(
        run(world).stdout.contains(&full),
        "stdout does not contain {full:?}; actual: {:?}",
        run(world).stdout
    );
}

#[then(expr = "stdout does not contain the full claude projects path")]
fn stdout_lacks_full_claude_path(world: &mut BotSpyWorld) {
    let full = full_claude_projects_path(world);
    assert!(
        !run(world).stdout.contains(&full),
        "stdout unexpectedly contains {full:?}"
    );
}