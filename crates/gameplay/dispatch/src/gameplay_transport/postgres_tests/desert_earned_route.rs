//! Compose the source level gate, party rite and physical earned rewards.
use super::TestResult;
use crate::map::{overlay::TilePos, view::MapFacts};

#[test]
fn desert_dungeon_native_gate_rite_earned_rewards_and_exit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::desert_party_rite::scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Desert earned route panicked")??;
    println!(
        "DESERT_EARNED_ROUTE POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

pub(super) fn assert_source_bindings(world: &crate::map::boot::BundleWorld) -> TestResult {
    for (x, y, appearance, unique, action) in [
        (32673, 32100, 5102, None, Some(1020)),
        (32668, 32069, 2472, Some(6157), None),
        (32675, 32069, 2472, Some(6158), None),
    ] {
        let key = crate::map::view::placement_key(TilePos { x, y, floor: -8 }, 1)
            .ok_or("source gate/chest placement")?;
        assert!(
            world
                .facts()
                .quest_callback_matches(key, unique, action, Some(appearance)),
            "source gate/chest binding missing at {x},{y},z8"
        );
    }
    Ok(())
}

use super::{
    AdmittedSession, ComposedFreshAdmission,
    fixture::{Harness, debug},
};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::{
    world_object::{UseDisposition, WorldObjectTarget},
    world_spatial::{StepDirection, StepDisposition},
};

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    x: u16,
    y: u16,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("source map")?;
    let key =
        crate::map::view::placement_key(TilePos { x, y, floor: -8 }, 1).ok_or("source target")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<MovementLocalPosition> {
    owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map(|p| p.position())
        .map_err(debug)
}

pub(super) async fn enter(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let gate = target(owner, 32673, 32100)?;
    assert_eq!(
        at(owner, admitted).await?,
        MovementLocalPosition {
            x: 32673,
            y: 32101,
            floor: 8
        }
    );
    assert_eq!(
        use_target(owner, admitted, &gate).await?,
        UseDisposition::Committed
    );
    let gate_cell = MovementLocalPosition {
        x: 32673,
        y: 32100,
        floor: 8,
    };
    let entered = at(owner, admitted).await? == gate_cell;
    let walked = walk_to(
        owner,
        admitted,
        Position {
            x: 32673,
            y: 32087,
            floor: -8,
        },
    )
    .await?;
    assert!(
        entered || walked.contains(&gate_cell),
        "the earned route must cross the actual level20 gate"
    );
    assert!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .doors()
            .blocks(TilePos {
                x: 32673,
                y: 32100,
                floor: -8
            }),
        "the source level door closes after the entrant leaves it"
    );
    Ok(())
}

/// Search only the committed source cells through the production collision checker,
/// then execute every edge through real protocol STEP. No placement is committed by the search.
pub(super) async fn walk_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    goal: Position,
) -> TestResult<Vec<MovementLocalPosition>> {
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let end = super::quest_world_callbacks::local(goal);
    let path = {
        let map = owner.bundle_map().ok_or("source map")?;
        let runtime = owner.runtime.lock().await;
        let expected = runtime.read_actor_position(actor).map_err(debug)?;
        let start = expected.position();
        assert_eq!(
            start.floor, end.floor,
            "flat walk must not invent a floor transfer"
        );
        let mut queue = VecDeque::from([(start.x, start.y)]);
        let mut seen = BTreeSet::from([(start.x, start.y)]);
        let mut previous = BTreeMap::new();
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let mut directions = vec![
            (0, -1, StepDirection::North),
            (1, 0, StepDirection::East),
            (0, 1, StepDirection::South),
            (-1, 0, StepDirection::West),
        ];
        if admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1)
        {
            directions.extend([
                (1, -1, StepDirection::NorthEast),
                (1, 1, StepDirection::SouthEast),
                (-1, 1, StepDirection::SouthWest),
                (-1, -1, StepDirection::NorthWest),
            ]);
        }
        while let Some((x, y)) = queue.pop_front() {
            if (x, y) == (end.x, end.y) {
                break;
            }
            for (dx, dy, direction) in directions.iter().copied() {
                let next = (x + dx, y + dy);
                if !seen.insert(next) {
                    continue;
                }
                let position = MovementLocalPosition {
                    x: next.0,
                    y: next.1,
                    floor: start.floor,
                };
                if crate::map::floor::prepare_quest_callback_teleport(
                    &runtime,
                    &owner.movement_cells,
                    actor,
                    admitted.game_session_id,
                    expected,
                    position,
                    false,
                    &closed,
                )
                .is_err()
                {
                    continue;
                }
                previous.insert(next, ((x, y), direction));
                queue.push_back(next);
            }
        }
        let mut cursor = (end.x, end.y);
        let mut result = Vec::new();
        while cursor != (start.x, start.y) {
            let (parent, direction) = previous
                .get(&cursor)
                .ok_or_else(|| format!("no source walk from {start:?} to {end:?}"))?;
            result.push((
                *direction,
                MovementLocalPosition {
                    x: cursor.0,
                    y: cursor.1,
                    floor: start.floor,
                },
            ));
            cursor = *parent;
        }
        result.reverse();
        result
    };
    let mut walked = Vec::new();
    for (direction, expected) in path {
        step(owner, admitted, direction).await?;
        let actual = at(owner, admitted).await?;
        assert_eq!(
            actual, expected,
            "source STEP follows the actual planned cell"
        );
        walked.push(actual);
    }
    Ok(walked)
}

pub(super) async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let before = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
    )
    .await?;
    let result = oteryn_protocol_oteryn::world_spatial::decode_step_result_profile(
        &reply,
        admitted.continuity.selected_capabilities.contains(13),
        admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1),
    )
    .map_err(debug)?;
    assert_eq!(
        result.disposition,
        StepDisposition::Moved,
        "literal source STEP must move"
    );
    if direction.is_diagonal() {
        assert!(
            result.step_wait_ms > 0,
            "the negotiated diagonal carries its native pacing receipt"
        );
        assert_eq!(
            result.step_wait_ms % 150,
            0,
            "diagonal cost is three rounded 50ms source beats"
        );
    }
    let after = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        after.revision(),
        before.revision() + 1,
        "one diagonal is one atomic native step, never two partial cardinal commits"
    );
    Ok(())
}

async fn use_id(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("current session")?
            .item_fence(),
    );
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn use_target(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    let command = admitted.continuity.next_command_id;
    use_id(owner, admitted, command, target).await
}

pub(super) async fn earn(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    for (x, claim) in [
        (32668, "oteryn:reward-claim.kv.desert1"),
        (32675, "oteryn:reward-claim.kv.desert2"),
    ] {
        walk_to(
            owner,
            admitted,
            Position {
                x,
                y: 32070,
                floor: -8,
            },
        )
        .await?;
        let chest = target(owner, x, 32069)?;
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            use_target(owner, admitted, &chest).await?,
            UseDisposition::Committed,
            "earned {claim}"
        );
        assert_eq!(
            use_id(owner, admitted, command, &chest).await?,
            UseDisposition::Committed,
            "exact original chest command replays"
        );
        assert_eq!(
            use_target(owner, admitted, &chest).await?,
            UseDisposition::NothingToUse,
            "a fresh command cannot award {claim} again"
        );
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1",
        )
        .bind(claim)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(count, 1);
    }
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert_eq!(
        backpack.entries.len(),
        2,
        "both earned rewards have native Character custody"
    );
    let coins = &backpack
        .entries
        .iter()
        .find(|e| e.item.definition.production_key == "oteryn:item.tibia.i3035")
        .ok_or("platinum")?
        .item;
    assert_eq!(coins.quantity, 100);
    let bag = &backpack
        .entries
        .iter()
        .find(|e| e.item.definition.production_key == "oteryn:item.tibia.i2857")
        .ok_or("earned bag")?
        .item;
    assert_eq!(bag.quantity, 1);
    let children:Vec<(String,String,i64)>=sqlx::query_as("SELECT i.item_instance_id::text,i.definition_production_key,i.quantity FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal")
        .bind(bag.item_instance_id.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        children
            .iter()
            .map(|(_, key, count)| (key.clone(), *count))
            .collect::<Vec<_>>(),
        [3084, 3098, 3046, 3077]
            .map(|id| (format!("oteryn:item.tibia.i{id}"), 1))
            .to_vec()
    );
    use crate::content::ReferenceItemField::Known;
    let policy = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_policy("oteryn:item.tibia.i3084", "definition-r1"))
        .ok_or("full native charge definition")?;
    assert!(
        matches!(&policy.record().semantics.charges,Known(charges) if charges.count==Known(250))
    );
    let partial: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=$1::uuid",
    )
    .bind(&children[0].0)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        partial, 0,
        "absent native timed state means exactly the full250 definition charges"
    );
    let atomic:(i64,i64)=sqlx::query_as("SELECT count(*),count(DISTINCT created_xact_id::text) FROM game_item_audit_outbox WHERE item_instance_id=encode($1,'hex')::uuid OR item_instance_id IN(SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
        .bind(bag.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic,
        (5, 1),
        "one native container MINT owns the bag and every child"
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        receipts, 2,
        "one native MINT per independently once-only source reward"
    );
    walk_to(
        owner,
        admitted,
        Position {
            x: 32671,
            y: 32061,
            floor: -8,
        },
    )
    .await?;
    step(owner, admitted, StepDirection::North).await?;
    assert_eq!(
        at(owner, admitted).await?,
        MovementLocalPosition {
            x: 32671,
            y: 32059,
            floor: 7
        },
        "source stairs1947 floorchange north returns from the reward room through native movement"
    );
    println!(
        "DESERT_EARNED_ROUTE NATIVE_GATE_RITE_REWARDS_EXIT four_real_characters two_once_claims six_minted_instances full250_charges"
    );
    Ok(())
}
