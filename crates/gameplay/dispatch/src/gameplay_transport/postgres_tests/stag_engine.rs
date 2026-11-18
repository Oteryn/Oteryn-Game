//! Cathedral and Court Warlock wire USE against the committed B3 placements.
use super::fixture::{Harness, debug};
use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
use super::quest_world_callbacks::{input_item, local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::ExactActorRef;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{court_warlock as court, stag_cathedral as cathedral, world_callbacks::{Offering, Position}};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
    decode_use_result, encode_item_use_intent, encode_use_intent,
};

pub(super) const CATHEDRAL: &str = "stag-engine-cathedral";
pub(super) const COURT: &str = "order-of-the-stag-court-warlock";

#[test]
fn cathedral_pick_requires_ground_starlight_and_shares_descent_until_reset_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::kill_reward_e2e::scenario_thread::run(CATHEDRAL, || {
        super::quest_world_callbacks::scenario(CATHEDRAL, cathedral::SOURCE, false)
    })
}

#[test]
fn court_warlock_lever_admits_stage_eleven_and_starts_first_wave_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::kill_reward_e2e::scenario_thread::run(COURT, || {
        super::quest_world_callbacks::scenario(COURT, court::LEVER, false)
    })
}

pub(super) fn start(name: &str) -> Position {
    if name == CATHEDRAL {
        Position { y: 32468, ..cathedral::SOURCE }
    } else {
        Position { x: 33842, y: 32146, floor: -3 }
    }
}

pub(super) fn world(name: &str, native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    let start = start(name);
    let start = (start.x, start.y);
    let tile = |position, ids| ExpectedTile { position, ids, counted_entry: None };
    let (crops, appearances) = if name == CATHEDRAL {
        (vec![
            NativeCrop {
                bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content/world/placements/region-z04-x126-y126.b3")),
                region: (4, 126, 126),
                bounds: (32401, 32465, 32403, 32468),
                start,
                expected: vec![tile((32402, 32467), vec![516]), tile((32402, 32465), vec![417, 17469, 9101])],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content/world/placements/region-z05-x126-y126.b3")),
                region: (5, 126, 126),
                bounds: (32401, 32466, 32404, 32471),
                start: (32402, 32467),
                expected: vec![tile((32402, 32467), vec![15785, 7114, 29333]), tile((32403, 32470), vec![15785, 7151])],
            },
        ], &[cathedral::CLOSED][..])
    } else {
        (vec![
            NativeCrop {
                bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content/world/placements/region-z03-x132-y125.b3")),
                region: (3, 132, 125),
                bounds: (33841, 32141, 33844, 32147),
                start,
                expected: vec![tile((33843, 32147), vec![499, 8913]), tile(start, vec![18013])],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content/world/placements/region-z02-x132-y125.b3")),
                region: (2, 132, 125),
                bounds: (33830, 32133, 33847, 32150),
                start: (33838, 32141),
                expected: vec![tile((33838, 32141), vec![499, 4573, 10561, 6379, 7112])],
            },
        ], &[][..])
    };
    native_scene_with_appearances(native, crops, &[], appearances)
}

fn target(owner: &ComposedFreshAdmission<'_, '_, '_>, pos: Position, ordinal: u8) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("bundle map")?;
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal).ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget { placement, expected_revision: map.facts.object_revision(key) })
}

fn pace(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("world USE pacing")?;
    Ok(())
}

async fn use_fixed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    pos: Position,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    pace(owner)?;
    let payload = encode_use_intent(&target(owner, pos, ordinal)?).map_err(debug)?;
    let reply = wire(owner, admitted, admitted.continuity.next_command_id, 2, payload).await?;
    decode_use_result(&reply).map_err(debug)
}

async fn use_pick(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<UseDisposition> {
    pace(owner)?;
    let pick = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", cathedral::PICK, "definition-r1"))
        .ok_or("pick definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(owner, cathedral::SOURCE, 0)?)),
        use_with_position: None,
    }).map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(
        owner, *admitted, admitted.continuity.next_command_id, &payload, None,
    ).await?;
    *admitted = next;
    Ok(result)
}

pub(super) async fn exercise_cathedral(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    actor: ExactActorRef,
) -> TestResult {
    let stand = |item: &str| Offering { position: cathedral::STAND, item: item.into(), count: 1 };
    input_item(owner, fence.item_fence(), &stand(cathedral::PICK), 60, true).await?;
    assert_eq!(use_fixed(owner, admitted, cathedral::SOURCE, 0).await?, UseDisposition::NothingToUse);
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Rejected);
    input_item(owner, fence.item_fence(), &stand(cathedral::STARLIGHT), 61, false).await?;
    owner.states.loop_ground.invalidate();
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Committed);
    let pos = TilePos { x: cathedral::SOURCE.x, y: cathedral::SOURCE.y, floor: cathedral::SOURCE.floor };
    let map = owner.bundle_map().ok_or("map")?;
    let first_due = map.facts.next_movement_decay_time().ok_or("opening deadline")?;
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Committed);
    let map = owner.bundle_map().ok_or("map")?;
    let due = map.facts.next_movement_decay_time().ok_or("renewed opening deadline")?;
    assert!(due > first_due);
    assert_eq!(map.facts.expire_movement_items(first_due), 0);
    assert_eq!(use_fixed(owner, admitted, cathedral::SOURCE, 0).await?, UseDisposition::Committed);
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), local(cathedral::DESTINATION));
        let warlocks = runtime.current_live_creature_census().map_err(debug)?.into_iter()
            .filter(|actor| runtime.matches_live_creature_identity(*actor, cathedral::WARLOCK.as_bytes())).count();
        assert_eq!(warlocks, 1);
    }
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.expire_movement_items(due), 2);
    assert_eq!(map.facts.quest_portal_destination(pos, due), None);
    let ground = map.overlay.base().tile(pos.x, pos.y, pos.floor).ok_or("cathedral source")?.ids()[0];
    assert_eq!(map.facts.base_entry(pos, 0, ground).ok_or("restored floor")?.appearance_id, cathedral::CLOSED);
    let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(cathedral::STARLIGHT).fetch_one(&h.pool).await?;
    assert_eq!(quantity, 1);
    Ok(())
}

pub(super) async fn exercise_court(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    actor: ExactActorRef,
) -> TestResult {
    let seed = |stage: i64| sqlx::query(
        "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
         VALUES(encode($1,'hex')::uuid,$2,$3,$4,2)
         ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(court::TRACK).bind(court::QUEST).bind(stage).execute(&h.pool);
    seed(10).await?;
    assert_eq!(use_fixed(owner, admitted, court::LEVER, 1).await?, UseDisposition::Rejected);
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(start(COURT)));
    seed(11).await?;
    assert_eq!(use_fixed(owner, admitted, court::LEVER, 1).await?, UseDisposition::Committed);
    let due = owner.states.spell_states.lock().await.encounter_phases.court_warlock_due_us().ok_or("first wave deadline")?;
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), local(court::CENTER));
        states.encounter_phases.tick_court_warlock(&mut runtime, due - 1);
        assert!(runtime.current_live_creature_census().map_err(debug)?.is_empty());
        states.encounter_phases.tick_court_warlock(&mut runtime, due);
        let creatures = runtime.current_live_creature_census().map_err(debug)?;
        assert_eq!(creatures.len(), court::RAUBRITTERS_PER_WAVE);
        assert!(creatures.iter().all(|actor| court::RAUBRITTERS.iter()
            .any(|kind| runtime.matches_live_creature_identity(*actor, kind.as_bytes()))));
    }
    let (stage, receipts): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
                (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2)",
    ).bind(court::TRACK).bind(court::KEY).fetch_one(&h.pool).await?;
    assert_eq!((stage, receipts), (11, 0));
    Ok(())
}
