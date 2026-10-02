//! Cucumber runner: executes the Gherkin behavior specifications in
//! features/. The specs are the backbone; this runner makes them executable.
//! Scenarios tagged @wip are specified but not implemented yet, so they are
//! excluded here and stay excluded from CI until their specs pass.

mod steps;

use cucumber::World;
use steps::BotSpyWorld;

/// Keeps a scenario when neither it, its rule, nor its feature is tagged
/// `@wip` (specs still being written or not implemented yet).
fn not_wip(
    feature: &cucumber::gherkin::Feature,
    rule: Option<&cucumber::gherkin::Rule>,
    scenario: &cucumber::gherkin::Scenario,
) -> bool {
    let mut tags = feature
        .tags
        .iter()
        .chain(rule.into_iter().flat_map(|r| r.tags.iter()))
        .chain(scenario.tags.iter());
    !tags.any(|tag| tag == "wip")
}

#[tokio::main]
async fn main() {
    BotSpyWorld::cucumber()
        .filter_run_and_exit("features", not_wip)
        .await;
}
