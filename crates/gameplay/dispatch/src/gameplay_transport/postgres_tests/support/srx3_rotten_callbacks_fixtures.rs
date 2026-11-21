use super::TestResult;

/// The normal owner batch must carry the same current Player DATA as a real
/// native cast; source health callbacks may never receive a fabricated vocation.
pub(super) async fn publish_native_fixture_player_source(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    actor: crate::foundation::ExactActorRef,
    command: crate::foundation::CommandRef,
    now: u64,
) -> TestResult {
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("actual source lane")?;
    let active = owner
        .active_generation
        .ok_or("actual source active generation")?;
    let native = active
        .native_gameplay()
        .ok_or("actual source native policy")?;
    let room = owner
        .effective_rooms()
        .gate_field_step_ingress()
        .ok_or("actual source map")?;
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let mut context = (&mut *runtime, &*states, &permit);
    owner
        .root
        .try_issue_semantic_pass()?
        .run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                use crate::durability::spell_item_transaction::{
                    assert_spell_item_authority_in_transaction, begin_spell_owner_transaction,
                };
                let (runtime, states, permit) = context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let authority = assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence,
                    command,
                    native.source_digest(),
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                crate::gameplay_transport::soul_war_callbacks::publish_command_source(
                    &mut tx, &authority, active, room, runtime, states, actor, now,
                )
                .await?;
                tx.rollback().await?;
                Ok(())
            })
        })
        .await?;
    Ok(())
}
