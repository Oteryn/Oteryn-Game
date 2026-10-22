//! Native committed base under a qualified temporary Outfit, then owner clear and expiry.
use super::*;

pub(super) async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    let member = owner
        .active_generation
        .ok_or("generation")?
        .native_gameplay()
        .ok_or("native")?
        .spell_appearances()
        .ok_or("appearances")?
        .for_look_type(1593)
        .ok_or("qualified temporary outfit")?;
    let runtime = owner.runtime.lock().await;
    let mut states = owner.spell_states.lock().await;
    let state = states
        .get_mut(&runtime, actor, session)
        .ok_or("player state")?;
    crate::spell::actor_conditions::apply_appearance(
        state,
        member.selection(),
        member.look_type(),
        600_000,
        owner.owner_now().get() / 1000,
    )
    .map_err(debug)?;
    state.advance_batch_revision().map_err(debug)?;
    Ok(())
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    entity: appearance::EntityRef,
) -> TestResult {
    let transformed = owner
        .appearance_observation(actor, session, vec![entity])
        .await
        .ok_or("transformed")?;
    assert!(matches!(
        transformed.bindings[0].look,
        appearance::Look::Outfit {
            look_type: 1593,
            colours: [0, 0, 0, 0],
            ..
        }
    ));
    assert_eq!(transformed.bindings[0].selection_revision, Some(2));
    clear(owner, actor, session).await?;
    let cleared = owner
        .appearance_observation(actor, session, vec![entity])
        .await
        .ok_or("cleared")?;
    assert!(matches!(
        cleared.bindings[0].look,
        appearance::Look::Outfit {
            colours: [1, 2, 3, 4],
            ..
        }
    ));
    install(owner, actor, session).await?;
    // Advance the authoritative owner clock; there is no client timer or selection rewrite.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(601))
        .ok_or("owner time")?;
    let expired = owner
        .appearance_observation(actor, session, vec![entity])
        .await
        .ok_or("expired")?;
    assert_eq!(expired.bindings[0].look, cleared.bindings[0].look);
    assert_eq!(expired.bindings[0].selection_revision, Some(2));
    Ok(())
}

pub(super) async fn clear(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("player state")?;
        let before = state.owned_conditions().clone();
        let mut after = before.clone();
        assert!(
            after.remove_type(oteryn_gameplay_domain::domain::condition::ConditionType::Outfit)
        );
        state
            .apply_batch_conditions(&before, &after)
            .map_err(debug)?;
        state.advance_batch_revision().map_err(debug)?;
    }
    Ok(())
}
