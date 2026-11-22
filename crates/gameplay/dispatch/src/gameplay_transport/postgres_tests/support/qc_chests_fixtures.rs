//! Lane qc: personal quest chests through the production world-callback owner.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CharacterId, CharacterLease};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

/// Scenario names `qc-<quest>-chest` select the callback with that key suffix.
pub(super) fn matches(name: &str) -> bool {
    name.starts_with("qc-") && name.ends_with("-chest")
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
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reward = callback.effects.item_reward.as_ref().ok_or("reward")?;
    let held = async || -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances \
             WHERE lifecycle=1 AND definition_production_key=$1",
        )
        .bind(&reward.item)
        .fetch_one(&h.pool)
        .await?)
    };
    let completed = async || -> TestResult<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE \
             character_id=encode($1,'hex')::uuid AND quest_key=$2 \
             AND completed_character_revision IS NOT NULL)",
        )
        .bind(id(41).as_slice())
        .bind(&callback.quest)
        .fetch_one(&h.pool)
        .await?)
    };
    assert_eq!(held().await?, 0);
    assert!(!completed().await?);
    let command = admitted.continuity.next_command_id;
    let first =
        super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&first).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(held().await?, i64::from(reward.count));
    assert_eq!(completed().await?, callback.completes);
    let audits = async || -> TestResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
                .fetch_one(&h.pool)
                .await?,
        )
    };
    let before = audits().await?;
    // The same command id replays the committed result without a second mint.
    let replay =
        super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    // The chest is personal: the shared object is untouched and a new use is refused.
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0
    );
    let next = admitted.continuity.next_command_id;
    let second = super::quest_world_callbacks_fixtures::wire(owner, admitted, next, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&second).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_eq!(audits().await?, before);
    assert_eq!(held().await?, i64::from(reward.count));
    Ok(())
}
