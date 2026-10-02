//! Steps for usage and cost metrics (spec 01_usage_tokens).

use crate::steps::{current_session, last_message, save_session, BotSpyWorld};
use botspy::{Message, ModelCost, RateLimitState, Role, SessionCost, Usage};
use cucumber::{then, when};

/// The model key the cost-record fixture records per-model cost under.
const COST_MODEL: &str = "claude-opus-5-5";

fn usage_from_table(step: &cucumber::gherkin::Step) -> Usage {
    let rows = &step.table.as_ref().expect("usage table").rows;
    let mut usage = Usage::default();
    for row in rows.iter().skip(1) {
        let field = row[0].as_str();
        let value = row[1].as_str();
        match field {
            "input_tokens" => usage.input_tokens = Some(value.parse().expect("u64")),
            "output_tokens" => usage.output_tokens = Some(value.parse().expect("u64")),
            "cache_read_tokens" => usage.cache_read_tokens = Some(value.parse().expect("u64")),
            "cache_write_tokens" => usage.cache_write_tokens = Some(value.parse().expect("u64")),
            "cached_input_tokens" => usage.cached_input_tokens = Some(value.parse().expect("u64")),
            "reasoning_tokens" => usage.reasoning_tokens = Some(value.parse().expect("u64")),
            "model" => usage.model = Some(value.to_string()),
            other => panic!("unknown usage field: {other}"),
        }
    }
    usage
}

/// Record a message carrying usage; it also opens turn "turn-1" containing
/// it, because usage scopes accumulate per turn and per thread.
fn record_usage_message(world: &mut BotSpyWorld, usage: Usage) {
    let mut session = current_session(world);
    let message = Message {
        role: Role::Assistant,
        turn_id: Some("turn-1".to_string()),
        usage: Some(usage),
        ..Message::default()
    };
    session.messages.push(message);
    session
        .turns
        .entry("turn-1".to_string())
        .or_insert_with(|| botspy::Turn {
            id: "turn-1".to_string(),
            ..botspy::Turn::default()
        });
    world.last_turn = Some("turn-1".to_string());
    save_session(world, session);
}

#[when(regex = r#"I record an assistant message with usage:"#)]
fn record_assistant_usage(world: &mut BotSpyWorld, step: &cucumber::gherkin::Step) {
    record_usage_message(world, usage_from_table(step));
}

#[then(regex = r#"the last message usage has ([a-z_]+) ([0-9]+)"#)]
fn last_message_usage_field(world: &mut BotSpyWorld, field: String, expected: u64) {
    let usage = last_message(world)
        .usage
        .expect("last message has no usage");
    let actual = match field.as_str() {
        "input_tokens" => usage.input_tokens,
        "output_tokens" => usage.output_tokens,
        "cache_read_tokens" => usage.cache_read_tokens,
        "cache_write_tokens" => usage.cache_write_tokens,
        "cached_input_tokens" => usage.cached_input_tokens,
        "reasoning_tokens" => usage.reasoning_tokens,
        other => panic!("unknown usage field: {other}"),
    };
    assert_eq!(actual, Some(expected), "usage field {field}");
}

#[then(regex = r#"the last message usage has model "([^"]+)""#)]
fn last_message_usage_model(world: &mut BotSpyWorld, model: String) {
    let usage = last_message(world)
        .usage
        .expect("last message has no usage");
    assert_eq!(usage.model.as_deref(), Some(model.as_str()));
}

#[when(regex = r#"the session carries a cost record:"#)]
fn record_cost(world: &mut BotSpyWorld, step: &cucumber::gherkin::Step) {
    let mut cost = SessionCost::default();
    for row in step.table.as_ref().expect("cost table").rows.iter().skip(1) {
        let field = row[0].as_str();
        let value = row[1].as_str();
        match field {
            "total_cost_usd" => cost.total_cost_usd = Some(value.parse().expect("f64")),
            "total_api_duration_ms" => {
                cost.total_api_duration_ms = Some(value.parse().expect("u64"))
            }
            "total_tool_duration_ms" => {
                cost.total_tool_duration_ms = Some(value.parse().expect("u64"))
            }
            "total_lines_added" => cost.total_lines_added = Some(value.parse().expect("u64")),
            "total_lines_removed" => cost.total_lines_removed = Some(value.parse().expect("u64")),
            other => panic!("unknown cost field: {other}"),
        }
    }
    cost.per_model.insert(
        COST_MODEL.to_string(),
        ModelCost {
            cost_usd: cost.total_cost_usd,
        },
    );
    let mut session = current_session(world);
    session.cost = Some(cost);
    save_session(world, session);
}

#[then(regex = r#"the session cost is ([0-9.]+) USD"#)]
fn session_cost_is(world: &mut BotSpyWorld, expected: f64) {
    let cost = current_session(world)
        .cost
        .expect("session has no cost record");
    assert_eq!(cost.total_cost_usd, Some(expected));
}

#[then(regex = r#"the session cost per model "([^"]+)" is present"#)]
fn session_cost_per_model(world: &mut BotSpyWorld, model: String) {
    let cost = current_session(world)
        .cost
        .expect("session has no cost record");
    assert!(
        cost.per_model.contains_key(&model),
        "no per-model cost for {model}"
    );
}

#[when(regex = r#"the session carries rate-limit state:"#)]
fn record_rate_limit(world: &mut BotSpyWorld, step: &cucumber::gherkin::Step) {
    let mut state = RateLimitState::default();
    for row in step
        .table
        .as_ref()
        .expect("rate-limit table")
        .rows
        .iter()
        .skip(1)
    {
        let field = row[0].as_str();
        let value = row[1].as_str();
        match field {
            "plan_type" => state.plan_type = Some(value.to_string()),
            "used_percent" => state.used_percent = Some(value.parse().expect("f64")),
            "window_minutes" => state.window_minutes = Some(value.parse().expect("u64")),
            "resets_at" => state.resets_at = Some(value.to_string()),
            other => panic!("unknown rate-limit field: {other}"),
        }
    }
    let mut session = current_session(world);
    session.rate_limit = Some(state);
    save_session(world, session);
}

#[then(regex = r#"the session rate limit is ([0-9.]+) percent used in a ([0-9]+) minute window"#)]
fn session_rate_limit(world: &mut BotSpyWorld, percent: f64, minutes: u64) {
    let state = current_session(world)
        .rate_limit
        .expect("session has no rate-limit state");
    assert_eq!(state.used_percent, Some(percent));
    assert_eq!(state.window_minutes, Some(minutes));
}

#[then(regex = r#"the session plan type is "([^"]+)""#)]
fn session_plan_type(world: &mut BotSpyWorld, plan: String) {
    let state = current_session(world)
        .rate_limit
        .expect("session has no rate-limit state");
    assert_eq!(state.plan_type.as_deref(), Some(plan.as_str()));
}

#[then(regex = r#"the turn usage accumulates the message usages of its turn"#)]
fn turn_usage_accumulates(world: &mut BotSpyWorld) {
    let session = current_session(world);
    let turn_id = world.last_turn.clone().expect("no turn was recorded");
    let expected: Usage = session
        .messages
        .iter()
        .filter(|m| m.turn_id.as_deref() == Some(turn_id.as_str()))
        .fold(Usage::default(), |mut acc, m| {
            if let Some(usage) = &m.usage {
                acc.add(usage);
            }
            acc
        });
    let actual = session
        .turn_usage(&turn_id)
        .unwrap_or_else(|| panic!("turn {turn_id} has no usage"));
    assert_eq!(actual, expected, "turn usage does not match message sum");
}

#[then(regex = r#"the thread usage accumulates the turn usages of the session"#)]
fn thread_usage_accumulates(world: &mut BotSpyWorld) {
    let session = current_session(world);
    let expected = session
        .turns
        .keys()
        .fold(Usage::default(), |mut acc, turn_id| {
            if let Some(usage) = session.turn_usage(turn_id) {
                acc.add(&usage);
            }
            acc
        });
    let actual = session
        .thread_usage()
        .expect("thread has no usage to accumulate");
    assert_eq!(actual, expected, "thread usage does not match turn sum");
}

#[then(regex = r#"the last message has no usage"#)]
fn last_message_no_usage(world: &mut BotSpyWorld) {
    assert!(
        last_message(world).usage.is_none(),
        "usage should be absent, not zero"
    );
}

#[then(regex = r#"the session has no usage"#)]
fn session_no_usage(world: &mut BotSpyWorld) {
    assert!(
        current_session(world).usage.is_none(),
        "session usage should be absent, not zero"
    );
}

#[then(regex = r#"the session has no cost"#)]
fn session_no_cost(world: &mut BotSpyWorld) {
    assert!(
        current_session(world).cost.is_none(),
        "session cost should be absent"
    );
}
