//! Continuous native portal, source key/cache collection, native deaths and completion.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.deeper_fibula_quest";

#[test]
fn questpc6_fibula_complete_native_path_level_claims_replay_and_paid_repurchase() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-fibula-completion")
}
#[test]
fn questpc6_fibula_completion_refuses_level_fifty_eight_kills_with_missing_claims() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-fibula-completion-missing-claims")
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let chests = super::quest_catalogue_chests::fibula_cases()?;
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z10-x125-y126.b3"
        )),
        oteryn_world_bundle::bundle::TILE_LIMITS,
        &mut oteryn_world_bundle::sector::Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    let palette: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements/index.json"
    )))?;
    let tile = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .find(|t| t.x == 32212 && t.y == 32433)
        .ok_or("source expertise portal")?;
    let portal = tile
        .items
        .iter()
        .find(|i| palette["palette"][i.palette as usize]["source_item_id"] == 1949)
        .ok_or("source forcefield")?;
    assert_eq!(portal.attrs.teleport, Some((32281, 32389, 10)));
    let mut cells = Vec::new();
    for goal in oteryn_gameplay_quests::quest::native_kill_goals::load()? {
        for p in &goal.origins {
            cells.push(Position {
                x: p.x,
                y: p.y,
                floor: -i8::try_from(p.floor)?,
            });
        }
    }
    cells.extend([
        Position {
            x: 32200,
            y: 32474,
            floor: -10,
        },
        Position {
            x: 32281,
            y: 32389,
            floor: -10,
        },
        Position {
            x: 32213,
            y: 32433,
            floor: -10,
        },
        Position {
            x: 32218,
            y: 32402,
            floor: -10,
        },
    ]);
    super::quest_catalogue_chests::catalogue_source_worlds(
        &chests,
        &[(
            Position {
                x: 32212,
                y: 32433,
                floor: -10,
            },
            1949,
            portal.attrs.clone(),
        )],
        &cells,
        true,
    )
}

async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position())
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    dir: StepDirection,
) -> TestResult<(u64, Vec<u8>)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let n = admitted.continuity.next_command_id;
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(dir);
    super::quest_world_callbacks::wire(owner, admitted, n, 1, payload.clone()).await?;
    Ok((n, payload))
}
async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..256 {
        let from = at(owner, admitted).await?;
        let dir = if from.x < x {
            StepDirection::East
        } else if from.x > x {
            StepDirection::West
        } else if from.y < y {
            StepDirection::South
        } else if from.y > y {
            StepDirection::North
        } else {
            return Ok(());
        };
        step(owner, admitted, dir).await?;
        assert_ne!(
            at(owner, admitted).await?,
            from,
            "the framed step must move on the qualified fixture path"
        );
    }
    Err("bounded source path exceeded".into())
}
async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    chest: &super::quest_catalogue_chests::CatalogueChest,
) -> TestResult {
    walk(
        owner,
        admitted,
        i32::from(chest.position.0) - 1,
        i32::from(chest.position.1),
    )
    .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let p = crate::map::overlay::TilePos {
        x: chest.position.0,
        y: chest.position.1,
        floor: chest.position.2,
    };
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(p, 1)
            .ok_or("source claim handle")?
            .to_be_bytes(),
    );
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed,
        "{}",
        chest.claim
    );
    Ok(())
}
async fn completed(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<bool> {
    let copy = owner
        .root
        .read_character_quest_state(
            owner.character,
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    Ok(copy.states().get(QUEST).is_some_and(|s| s.completed))
}
async fn receipts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-reward/fibula-complete'),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=18)").fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let started = std::time::Instant::now();
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/fibula-complete")
        .ok_or("Fibula terminal completion is not bound")?;
    let value = serde_json::to_value(&callback)?;
    assert_eq!(value["minimum_level"], 50);
    assert_eq!(value["completes"], true);
    assert_eq!(
        value["required_claims"]
            .as_array()
            .ok_or("required source claims")?
            .len(),
        6
    );
    super::questpc6_fibula_purchase::exercise(
        owner,
        admitted,
        h,
        "qengine-rewards-fibula-key-coins",
    )
    .await?;
    let npc = {
        let runtime = owner.runtime.lock().await;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| runtime.npc_key(n.actor) == Some("oteryn:npc.dermot"))
            .ok_or("Dermot")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        }
    };
    // The native source gate is covered separately; start on its cavern side and
    // execute the actual raw source portal, not a fixture quest teleport callback.
    walk(owner, admitted, 32213, 32433).await?;
    let (command, payload) = step(owner, admitted, StepDirection::West).await?;
    let destination = MovementLocalPosition {
        x: 32281,
        y: 32389,
        floor: 10,
    };
    assert_eq!(at(owner, admitted).await?, destination);
    super::quest_world_callbacks::wire(owner, admitted, command, 1, payload).await?;
    assert_eq!(
        at(owner, admitted).await?,
        destination,
        "native portal replay is inert"
    );
    let chests = super::quest_catalogue_chests::fibula_cases()?;
    let key = chests
        .iter()
        .find(|c| c.claim.ends_with("id3980"))
        .ok_or("source key3980")?;
    claim(owner, admitted, key).await?;
    let key_number:Option<i32>=sqlx::query_scalar("SELECT k.key_number FROM game_item_keys k JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i2972'").fetch_optional(&h.pool).await?;
    assert_eq!(
        key_number,
        Some(3980),
        "the western corpse earns the exact real key"
    );
    super::questpc6_native_goals::prepare(owner, admitted).await?;
    super::questpc6_native_goals::active_goals(owner, admitted).await?;
    if name.ends_with("missing-claims") {
        super::questpc6_fibula_route::level(owner, admitted, 50).await?;
        walk(owner, admitted, 32217, 32401).await?;
        let before = receipts(h).await?;
        assert_eq!(
            super::qengine_rewards::wire_grant(owner, admitted, npc, &callback)
                .await?
                .0,
            NpcIntentDisposition::Rejected
        );
        assert_eq!(
            receipts(h).await?,
            before,
            "eight kills/level50/key3980 cannot replace the five missing caches"
        );
        assert!(!completed(owner).await?);
    } else {
        for chest in &chests {
            if chest.claim != key.claim {
                claim(owner, admitted, chest).await?;
            }
        }
        walk(owner, admitted, 32217, 32401).await?;
        let before = receipts(h).await?;
        assert_eq!(
            super::qengine_rewards::wire_grant(owner, admitted, npc, &callback)
                .await?
                .0,
            NpcIntentDisposition::Rejected,
            "all claims and kills still require native level50"
        );
        assert_eq!(receipts(h).await?, before);
        assert!(!completed(owner).await?);
        super::questpc6_fibula_route::level(owner, admitted, 50).await?;
        let (result, _, command) =
            super::qengine_rewards::wire_grant(owner, admitted, npc, &callback).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert!(completed(owner).await?);
        let after = receipts(h).await?;
        assert_eq!(
            after,
            (1, before.1, before.2),
            "terminal confirmation creates no duplicate cache loot or fee"
        );
        let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: "yes".into(),
        })
        .map_err(debug)?;
        super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
        assert_eq!(
            receipts(h).await?,
            after,
            "terminal replay cannot complete twice"
        );
        let repeat = oteryn_gameplay_quests::quest::reward_callbacks::load()?
            .into_iter()
            .find(|c| c.key.ends_with("fibula-key-repeat"))
            .ok_or("repeat sale")?;
        assert_eq!(
            super::qengine_rewards::wire_grant(owner, admitted, npc, &repeat)
                .await?
                .0,
            NpcIntentDisposition::Ok
        );
        assert!(
            completed(owner).await?,
            "a later paid source key sale never reopens the quest"
        );
        assert_eq!(receipts(h).await?.2, after.2 + 1);
    }
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
