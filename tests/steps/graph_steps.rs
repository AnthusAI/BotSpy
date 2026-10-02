//! Steps for the session graph (spec 03_session_graph).

use crate::steps::{
    adapter_owning, current_session, find_session, parse_agent, save_session, save_session_by_id,
    BotSpyWorld,
};
use botspy::{BattleLink, ForkPoint, Peer, Session, SubagentInfo};
use cucumber::{given, then, when};

fn session_in(world: &BotSpyWorld, id: &str) -> Session {
    find_session(world, id).unwrap_or_else(|| panic!("session {id} not found"))
}

fn new_subagent_session(
    world: &mut BotSpyWorld,
    id: &str,
    kind: &str,
    parent_id: &str,
    nesting_depth: Option<u64>,
) {
    let parent = session_in(world, parent_id);
    let session = Session {
        id: id.to_string(),
        agent: parent.agent,
        project_id: parent.project_id.clone(),
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        parent_id: Some(parent_id.to_string()),
        root_id: parent.root_id.clone().or(Some(parent_id.to_string())),
        subagent: Some(SubagentInfo {
            kind: kind.to_string(),
            nesting_depth,
        }),
        ..Session::default()
    };
    let adapter = adapter_owning(world, parent_id).expect("parent adapter");
    adapter.add_session(session);
    world.current_session = Some(id.to_string());
}

#[given(
    regex = r#"a fixture sub-agent session "([^"]+)" of kind "([^"]+)" with parent "([^"]+)"$"#
)]
fn subagent_session(world: &mut BotSpyWorld, id: String, kind: String, parent: String) {
    new_subagent_session(world, &id, &kind, &parent, None);
}

#[given(
    regex = r#"a fixture sub-agent session "([^"]+)" of kind "([^"]+)" with parent "([^"]+)" at nesting depth ([0-9]+)$"#
)]
fn subagent_session_depth(
    world: &mut BotSpyWorld,
    id: String,
    kind: String,
    parent: String,
    depth: u64,
) {
    new_subagent_session(world, &id, &kind, &parent, Some(depth));
}

#[given(
    regex = r#"a fixture session "([^"]+)" from agent "([^"]+)" in project "([^"]+)" in battle "([^"]+)"$"#
)]
fn battle_session(
    world: &mut BotSpyWorld,
    id: String,
    agent: String,
    project: String,
    battle_id: String,
) {
    let adapter = crate::steps::adapter_for(world, &agent);
    adapter.add_session(Session {
        id: id.clone(),
        agent: parse_agent(&agent),
        project_id: project,
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        battle: Some(BattleLink {
            battle_id,
            winning_conversation_id: None,
        }),
        ..Session::default()
    });
    world.current_session = Some(id);
}

#[when(regex = r#"I fork session "([^"]+)" at ordinal ([0-9]+) into "([^"]+)""#)]
fn fork_session(world: &mut BotSpyWorld, source: String, ordinal: u64, fork_id: String) {
    let parent = session_in(world, &source);
    let session = Session {
        id: fork_id.clone(),
        agent: parent.agent,
        project_id: parent.project_id.clone(),
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        parent_id: Some(source.clone()),
        root_id: parent.root_id.clone().or(Some(source)),
        fork: Some(ForkPoint { ordinal }),
        ..Session::default()
    };
    let adapter = adapter_owning(world, &session.parent_id.clone().expect("fork parent"))
        .expect("parent adapter");
    adapter.add_session(session);
    world.current_session = Some(fork_id);
}

#[when(regex = r#"I spawn session "([^"]+)" from session "([^"]+)""#)]
fn spawn_session(world: &mut BotSpyWorld, child: String, parent_id: String) {
    let parent = session_in(world, &parent_id);
    let session = Session {
        id: child.clone(),
        agent: parent.agent,
        project_id: parent.project_id.clone(),
        started_at: "2026-10-01T09:00:00Z".to_string(),
        last_activity_at: "2026-10-01T09:00:00Z".to_string(),
        parent_id: Some(parent_id.clone()),
        root_id: parent.root_id.clone().or(Some(parent_id.clone())),
        ..Session::default()
    };
    let adapter = adapter_owning(world, &parent_id).expect("parent adapter");
    adapter.add_session(session);
    world.current_session = Some(child);
}

#[when(regex = r#"the battle "([^"]+)" is won by "([^"]+)""#)]
fn battle_won(world: &mut BotSpyWorld, battle_id: String, winner: String) {
    let mut session = session_in(world, &winner);
    let battle = session
        .battle
        .as_mut()
        .unwrap_or_else(|| panic!("session {} is not in a battle", session.id));
    assert_eq!(battle.battle_id, battle_id, "winner is in another battle");
    battle.winning_conversation_id = Some(winner);
    save_session_by_id(world, session);
}

#[when(regex = r#"I record a peer cloud agent "([^"]+)" with status "([^"]+)""#)]
fn record_peer(world: &mut BotSpyWorld, peer_id: String, status: String) {
    let mut session = current_session(world);
    session.peers.push(Peer {
        peer_id,
        status: Some(status),
        has_local_transcript: false,
    });
    save_session(world, session);
}

#[then(regex = r#"session "([^"]+)" has parent_id "([^"]+)""#)]
fn has_parent_id(world: &mut BotSpyWorld, id: String, parent: String) {
    let session = session_in(world, &id);
    assert_eq!(session.parent_id.as_deref(), Some(parent.as_str()));
}

#[then(regex = r#"session "([^"]+)" has root_id "([^"]+)""#)]
fn has_root_id(world: &mut BotSpyWorld, id: String, root: String) {
    let session = session_in(world, &id);
    assert_eq!(session.root_id.as_deref(), Some(root.as_str()));
}

#[then(regex = r#"session "([^"]+)" records sub-agent kind "([^"]+)""#)]
fn records_subagent_kind(world: &mut BotSpyWorld, id: String, kind: String) {
    let session = session_in(world, &id);
    let subagent = session
        .subagent
        .unwrap_or_else(|| panic!("session {id} is not a sub-agent"));
    assert_eq!(subagent.kind, kind);
}

#[then(regex = r#"session "([^"]+)" records nesting depth ([0-9]+)"#)]
fn records_nesting_depth(world: &mut BotSpyWorld, id: String, depth: u64) {
    let session = session_in(world, &id);
    let subagent = session
        .subagent
        .unwrap_or_else(|| panic!("session {id} is not a sub-agent"));
    assert_eq!(subagent.nesting_depth, Some(depth));
}

#[then(regex = r#"session "([^"]+)" records fork ordinal ([0-9]+)"#)]
fn records_fork_ordinal(world: &mut BotSpyWorld, id: String, ordinal: u64) {
    let session = session_in(world, &id);
    let fork = session
        .fork
        .unwrap_or_else(|| panic!("session {id} is not a fork"));
    assert_eq!(fork.ordinal, ordinal);
}

#[then(regex = r#"session "([^"]+)" records battle "([^"]+)" and winning conversation "([^"]+)""#)]
fn records_battle_win(world: &mut BotSpyWorld, id: String, battle_id: String, winner: String) {
    let battle = session_in(world, &id)
        .battle
        .unwrap_or_else(|| panic!("session {id} is not in a battle"));
    assert_eq!(battle.battle_id, battle_id);
    assert_eq!(
        battle.winning_conversation_id.as_deref(),
        Some(winner.as_str())
    );
}

#[then(regex = r#"session "([^"]+)" records battle "([^"]+)" without a win"#)]
fn records_battle_no_win(world: &mut BotSpyWorld, id: String, battle_id: String) {
    let battle = session_in(world, &id)
        .battle
        .unwrap_or_else(|| panic!("session {id} is not in a battle"));
    assert_eq!(battle.battle_id, battle_id);
    assert!(battle.winning_conversation_id.is_none());
}

#[then(regex = r#"session "([^"]+)" records peer id "([^"]+)""#)]
fn records_peer(world: &mut BotSpyWorld, id: String, peer_id: String) {
    let session = session_in(world, &id);
    assert!(
        session.peers.iter().any(|p| p.peer_id == peer_id),
        "session {id} records no peer {peer_id}"
    );
}

#[then(regex = r#"the peer session has no local transcript"#)]
fn peer_has_no_transcript(world: &mut BotSpyWorld) {
    let session = current_session(world);
    assert!(
        !session.peers.is_empty(),
        "session {} records no peers",
        session.id
    );
    assert!(
        session.peers.iter().all(|p| !p.has_local_transcript),
        "peers should have no local transcript"
    );
}
