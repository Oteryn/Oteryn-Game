//! Player setup and fresh durable admission shared by the W11 settlement cases.
use super::super::rotten_blood_progression_fixtures::{place_npc, talk};
use super::super::soul_war_fixtures::current;
use super::super::{ComposedFreshAdmission, Dependencies, Harness, TestResult, debug};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{CharacterId, CharacterLease};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome, LogoutReply};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;

pub(super) async fn prepare(
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let fence = current(owner, admitted.game_session_id).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
    }
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence.item_fence());
    owner.admit_progression_session(admitted).await;
    deps.initialize(owner).await;
    owner.admit_quest_session(admitted).await;
    Ok(())
}

pub(super) async fn converse(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    npc: EntityRef,
    first_command: u64,
    lines: &[&str],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("W11 player actor")?;
    for (command, text) in (first_command..).zip(lines.iter().copied()) {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("W11 NPC command clock")?;
        talk(owner, actor, admitted.game_session_id, npc, command, text).await?;
    }
    Ok(())
}

pub(super) async fn npc(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    key: &str,
) -> TestResult<EntityRef> {
    let actor = admitted.runtime_actor.ok_or("W11 player actor")?;
    place_npc(owner, actor, admitted.game_session_id, key).await
}

pub(super) async fn relog(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult<CurrentCharacterGameplayFence> {
    let old_session = admitted.game_session_id;
    let before = current(owner, old_session).await?;
    admitted.item_fence = Some(before.item_fence());
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    assert!(
        owner
            .current_quest_fence_durable(old_session)
            .await?
            .is_none()
    );
    let (session, _) = super::super::full_world_support::relogin_request(h).await?;
    assert_ne!(session, old_session);
    let after = current(owner, session).await?;
    assert_eq!(after.character_id, before.character_id);
    assert!(after.character_lease_generation > before.character_lease_generation);
    Ok(after)
}
