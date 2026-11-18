//! Crystal 201189f dreamers_challenge_quest/actions_tic_tac_teleport.lua:1-62.
use super::fixture::debug;
use super::quest_world_callbacks::{scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::{BoardToken, Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

const LEVER: Position = at(32834, 32285);
const PORTAL: Position = at(32836, 32288);
const PILLARS: [Position; 4] = [
    at(32835, 32285),
    at(32836, 32285),
    at(32837, 32285),
    at(32838, 32285),
];
pub(super) const TOKENS: [(Position, u16); 9] = [
    (at(32845, 32264), 3548),
    (at(32843, 32266), 3548),
    (at(32843, 32268), 3548),
    (at(32845, 32268), 3548),
    (at(32844, 32267), 3548),
    (at(32840, 32269), 3548),
    (at(32841, 32269), 3547),
    (at(32840, 32268), 3547),
    (at(32842, 32267), 3547),
];
pub(super) const READY: &str = "dreamers-board-ready";
pub(super) const MISSING: &str = "dreamers-board-missing";

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

fn tile(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}

pub(super) fn binding() -> TestResult<Callback> {
    let tokens: Vec<_> = TOKENS
        .iter()
        .map(|&(position, appearance)| BoardToken {
            position,
            appearance,
        })
        .collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/dreamers-challenge-tic-tac-pillar-switch",
        "quest":"oteryn:quest.dreamer_s_challenge_quest","trigger":"use",
        "selector":{"position": LEVER,"native":{"position":LEVER,"ordinal":1,"appearance":2772}},
        "lever":[2772,2773],"reversible":true,"return_on_failure":false,
        "passages":[],"storage":[],"offerings":[],
        "effects":{"board_portal":{"tokens":tokens,"pillars":PILLARS,"pillar":2153,
            "portal":PORTAL,"sealed":6298,"opened":1949,"open_on":2772}},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/actions_tic_tac_teleport.lua:1-62"]
    }))?)
}

pub(super) fn world(
    callback: &Callback,
    solved: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    let objects = PILLARS
        .into_iter()
        .map(|p| (p, 2153))
        .chain(TOKENS.into_iter().take(if solved { 9 } else { 8 }))
        .chain([(PORTAL, 6298)])
        .collect::<Vec<_>>();
    world_cells(
        callback,
        LEVER,
        2772,
        1,
        &objects,
        &[1949],
        None,
        &[TOKENS[8].0],
    )
}

#[test]
fn dreamers_tic_tac_tokens_open_the_shared_portal_on_the_matching_lever_face() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(READY, || scenario(READY, LEVER, false))
}

#[test]
fn dreamers_tic_tac_lever_toggles_the_pillars_with_an_incomplete_board() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(MISSING, || scenario(MISSING, LEVER, false))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    solved: bool,
) -> TestResult {
    let key = placement_key(tile(LEVER), 1).ok_or("lever placement")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    for revision in 0..=2 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let payload = encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)?;
        let result = wire(owner, admitted, revision + 1, 2, payload).await?;
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        let map = owner.bundle_map().ok_or("map")?;
        for pillar in PILLARS {
            let key = placement_key(tile(pillar), 1).ok_or("pillar placement")?;
            assert_eq!(map.facts.base_hidden(key), revision % 2 == 0);
        }
        let portal = tile(PORTAL);
        let id = map
            .overlay
            .base()
            .tile(portal.x, portal.y, portal.floor)
            .ok_or("portal tile")?
            .ids()[1];
        assert_eq!(
            map.facts
                .base_entry(portal, 1, id)
                .ok_or("portal item")?
                .appearance_id,
            if solved && revision == 1 { 1949 } else { 6298 }
        );
    }
    Ok(())
}
