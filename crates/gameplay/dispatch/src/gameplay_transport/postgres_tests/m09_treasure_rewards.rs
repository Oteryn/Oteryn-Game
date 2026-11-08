//! M09: two independent source-room caches, never one invented multi-item chest.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CharacterId, CharacterLease};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn m09_treasure_telescope_once_and_replay() -> TestResult {
    run("m09-treasure-telescope")
}

#[test]
fn m09_treasure_three_ingots_once_and_replay() -> TestResult {
    run("m09-treasure-ingots")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with(name))
        .ok_or("treasure callback")?;
    let trigger = callback
        .selector
        .position
        .ok_or("treasure chest position")?;
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, trigger, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
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
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("chest key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement: placement.clone(),
        expected_revision: 0,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let first =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&first).map_err(debug)?,
        UseDisposition::Committed
    );
    let reward = callback.effects.item_reward.as_ref().ok_or("reward")?;
    let received: Vec<(String, i64)> = sqlx::query_as("SELECT definition_production_key,sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i9616','oteryn:item.tibia.i9058') GROUP BY definition_production_key ORDER BY definition_production_key")
        .fetch_all(&h.pool).await?;
    assert_eq!(
        received,
        vec![(reward.item.clone(), i64::from(reward.count))]
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
        .fetch_one(&h.pool)
        .await?;
    let replay =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    let next_command = admitted.continuity.next_command_id;
    let claimed_payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 1,
        })
        .map_err(debug)?;
    let second =
        super::quest_world_callbacks::wire(owner, admitted, next_command, 2, claimed_payload)
            .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&second).map_err(debug)?,
        UseDisposition::Rejected
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, before);
    let claims: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind(&callback.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(claims, 1);
    Ok(())
}
