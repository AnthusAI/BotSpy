//! Cucumber runner: executes the Gherkin behavior specifications in
//! features/. The specs are the backbone; this runner makes them executable.

mod steps;

use cucumber::World;
use steps::BotSpyWorld;

#[tokio::main]
async fn main() {
    BotSpyWorld::cucumber().run_and_exit("features").await;
}
