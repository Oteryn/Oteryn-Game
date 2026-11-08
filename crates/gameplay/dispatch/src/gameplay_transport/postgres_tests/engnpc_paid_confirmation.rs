//! A live paid quote owns its confirmation even when the NPC also has quest rewards.
use super::{ComposedFreshAdmission, Dependencies, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[test]
fn engnpc_live_paid_quote_owns_yes_with_reward_callback() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::npc_paid_travel::scenario_with_reward_callback()
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "paid confirmation PG thread failed")??;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "ENGNPC_PG_EXECUTED paid_confirmation duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn assert_quote_scope(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    npc: EntityRef,
) -> TestResult {
    let session = dependencies.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "quote fence")?
        .ok_or("quote session")?
        .item_fence();
    let command = UseCommand {
        game_session_id: session,
        command_id: 3,
        item_fence: Some(fence),
    };
    let intent = NpcTalkIntent {
        npc_actor: npc,
        text: "  YES  ".into(),
    };
    let context = {
        let runtime = dependencies.runtime.lock().await;
        owner
            .states
            .npc_conversations
            .current_npc(
                &owner.states.chat,
                &runtime,
                dependencies.actor,
                session,
                &intent,
            )
            .ok_or("quote context")?
    };
    assert!(owner.has_pending_paid_confirmation(command, &intent, &context));
    let mut wrong = intent.clone();
    wrong.npc_actor.generation = wrong.npc_actor.generation.wrapping_add(1);
    assert!(!owner.has_pending_paid_confirmation(command, &wrong, &context));
    wrong = intent.clone();
    wrong.text = "no".into();
    assert!(!owner.has_pending_paid_confirmation(command, &wrong, &context));
    let mut wrong_context = context.clone();
    wrong_context.0 = wrong_context.0.wrapping_add(1);
    assert!(!owner.has_pending_paid_confirmation(command, &intent, &wrong_context));
    wrong_context = context.clone();
    wrong_context.1 = wrong_context.1.wrapping_add(1);
    assert!(!owner.has_pending_paid_confirmation(command, &intent, &wrong_context));
    let mut wrong_command = command;
    wrong_command.item_fence = None;
    assert!(!owner.has_pending_paid_confirmation(wrong_command, &intent, &context));
    let mut wrong_fence = fence;
    wrong_fence.character_lease_generation += 1;
    wrong_command.item_fence = Some(wrong_fence);
    assert!(!owner.has_pending_paid_confirmation(wrong_command, &intent, &context));
    let origin = owner.clock_origin;
    owner.clock_origin = origin
        .checked_sub(std::time::Duration::from_secs(60))
        .ok_or("expired clock")?;
    assert!(!owner.has_pending_paid_confirmation(command, &intent, &context));
    owner.clock_origin = origin
        .checked_add(std::time::Duration::from_secs(60))
        .ok_or("future clock")?;
    assert!(!owner.has_pending_paid_confirmation(command, &intent, &context));
    owner.clock_origin = origin;
    assert!(owner.has_pending_paid_confirmation(command, &intent, &context));
    Ok(())
}
