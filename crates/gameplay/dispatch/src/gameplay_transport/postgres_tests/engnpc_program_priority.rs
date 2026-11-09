//! A source programme may handle a command before an ineligible reward fallback.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use oteryn_gameplay_quests::quest::{
    reward_callbacks::RewardCallback,
    world_callbacks::{Position, Storage},
};
use oteryn_protocol_oteryn::{npc_service::NpcTalkIntent, world_spatial_entities::EntityRef};

#[test]
fn engnpc_eligible_npc_program_precedes_ineligible_reward_fallback() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("engnpc-program-priority".into())
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(
                "catalogue-npc-priority",
                Position {
                    x: 33294,
                    y: 31677,
                    floor: -15,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "programme priority thread failed")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED engnpc_eligible_npc_program_precedes_ineligible_reward_fallback duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    h: &Harness,
) -> TestResult {
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let mut callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.carried_item.is_none())
        .ok_or("fixture callback")?;
    callback.npc = "oteryn:npc.kesar_the_younger_day".into();
    callback.topic = 0;
    callback.state_ops.clear();
    callback.participants.clear();
    callback.requires.clear();
    callback.storage = vec![Storage {
        track: "oteryn:quest-progress/engnpc/unavailable".into(),
        minimum: 1,
        maximum: 1,
        set: 1,
        preserve: false,
        increment: false,
        at_least: false,
    }];
    callback.keywords = vec!["fixture fallback".into()];
    assert!(
        selected(owner, admitted, npc, &callback, "fixture fallback").await?,
        "an unmatched native programme must retain the reward fallback"
    );
    callback.keywords = vec!["help".into()];
    assert!(
        !selected(owner, admitted, npc, &callback, "help").await?,
        "ineligible reward fallback stole an eligible NPC programme command"
    );
    callback.topic = 1;
    assert!(
        !selected(owner, admitted, npc, &callback, "help").await?,
        "reward fallback must preserve its own topic guard"
    );
    callback.topic = 0;
    callback.storage.clear();
    assert!(
        selected(owner, admitted, npc, &callback, "help").await?,
        "eligible reward callbacks must retain precedence"
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(before, after, "advisory selection must not create receipts");
    Ok(())
}

async fn selected(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    callback: &RewardCallback,
    message: &str,
) -> TestResult<bool> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "priority fence")?
        .ok_or("priority session")?
        .item_fence();
    let runtime = owner.runtime.lock().await;
    Ok(
        crate::gameplay_transport::npc_command::fixture_prepare_reward_callbacks(
            owner,
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 2490,
                item_fence: Some(fence),
            },
            &NpcTalkIntent {
                npc_actor: npc,
                text: message.into(),
            },
            &runtime,
            std::slice::from_ref(callback),
        ),
    )
}
