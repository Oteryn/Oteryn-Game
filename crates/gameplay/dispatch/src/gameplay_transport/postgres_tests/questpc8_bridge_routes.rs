//! The two historical bridge routes share native scene and carried-tool owners.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn scenario(name: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-bridge-route", move || {
        quest_world_callbacks::scenario(name, trigger, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_bridge_route {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc8_life_ring_native_bridge_pick_cache_and_exact_rewards() -> TestResult {
    // Reuse the committed programme and bridge scenarios after shared-engine convergence.
    scenario(
        "authored-life-ring-entry",
        Position {
            x: 32437,
            y: 32238,
            floor: -10,
        },
    )?;
    scenario(
        "authored-life-ring-bridge",
        Position {
            x: 32414,
            y: 32231,
            floor: -10,
        },
    )
}

#[test]
fn questpc8_torch_native_academy_lever_rat_and_reward_once() -> TestResult {
    scenario(
        "authored-torch",
        Position {
            x: 32097,
            y: 32204,
            floor: -8,
        },
    )
}

use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
#[path = "questpc8_bridge_routes/world.rs"]
mod fixture_world;
async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<crate::foundation::MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position())
}
async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..64 {
        let at = position(owner, admitted).await?;
        let dir = if at.x < x {
            StepDirection::East
        } else if at.x > x {
            StepDirection::West
        } else if at.y < y {
            StepDirection::South
        } else if at.y > y {
            StepDirection::North
        } else {
            return Ok(());
        };
        step(owner, admitted, *command, dir).await?;
        *command += 1;
    }
    Err("route did not arrive".into())
}
async fn claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    at: TilePos,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    let target = object_target(owner, at, ordinal)?;
    let result = use_target(owner, admitted, *command, &target).await?;
    *command += 1;
    Ok(result)
}

pub(super) fn torch_world() -> TestResult<crate::map::boot::BundleWorld> {
    fixture_world::torch_world()
}

pub(super) async fn exercise_torch(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    owner.premium_coordinator = None;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let mut command = 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    let lever = TilePos {
        x: 32098,
        y: 32204,
        floor: -8,
    };
    assert_eq!(
        claim(owner, admitted, &mut command, lever, 1).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, &mut command, 32097, 32205).await?;
    walk(owner, admitted, &mut command, 32098, 32205).await?;
    walk(owner, admitted, &mut command, 32099, 32205).await?;
    let creature = owner
        .runtime
        .lock()
        .await
        .admit_pinned_test_creature(crate::foundation::MovementLocalPosition {
            x: 32100,
            y: 32205,
            floor: 8,
        })
        .map_err(debug)?;
    assert_eq!(
        claim(owner, admitted, &mut command, lever, 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        position(owner, admitted).await?,
        crate::foundation::MovementLocalPosition {
            x: 32102,
            y: 32205,
            floor: 8
        }
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(creature)
            .map_err(debug)?
            .position(),
        crate::foundation::MovementLocalPosition {
            x: 32103,
            y: 32205,
            floor: 8
        }
    );
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(creature)
        .map_err(debug)?;
    let moved:Vec<u8>=sqlx::query_scalar("SELECT g.spatial_position FROM game_item_ground_locations g JOIN game_map_item_mint_receipts m USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3031'").fetch_one(&h.pool).await?;
    let mut expected = Vec::new();
    expected.extend_from_slice(&32102_i32.to_be_bytes());
    expected.extend_from_slice(&32205_i32.to_be_bytes());
    expected.extend_from_slice(&(-8_i16).to_be_bytes());
    assert_eq!(
        moved, expected,
        "same durable gold instance moved off the closing bridge"
    );
    walk(owner, admitted, &mut command, 32103, 32204).await?;
    let opposite = TilePos {
        x: 32104,
        y: 32204,
        floor: -8,
    };
    assert_eq!(
        claim(owner, admitted, &mut command, opposite, 1).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, &mut command, 32103, 32205).await?;
    walk(owner, admitted, &mut command, 32097, 32205).await?;
    walk(owner, admitted, &mut command, 32097, 32188).await?;
    let cache = TilePos {
        x: 32097,
        y: 32187,
        floor: -8,
    };
    assert_eq!(
        claim(owner, admitted, &mut command, cache, 1).await?,
        UseDisposition::Rejected,
        "native Rat death required"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    super::grizzly_npc::questpc8_campaign::hunt(owner, admitted, "oteryn:creature.rat", 1, command)
        .await?;
    assert_eq!(
        claim(owner, admitted, &mut command, cache, 1).await?,
        UseDisposition::Committed
    );
    let quantity:i64=sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key='oteryn:item.tibia.i2920' AND i.lifecycle=1").bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(quantity, 1);
    assert_eq!(
        claim(owner, admitted, &mut command, cache, 1).await?,
        UseDisposition::Rejected
    );
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    assert_eq!(
        claim(owner, admitted, &mut command, cache, 1).await?,
        UseDisposition::Rejected
    );
    Ok(())
}
