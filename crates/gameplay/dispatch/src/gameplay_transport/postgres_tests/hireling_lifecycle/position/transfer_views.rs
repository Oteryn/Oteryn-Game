//! The real exit must carry existing values, not reconstruct fresh clocks.
use super::*;
use crate::foundation::GameSessionId;

pub(super) struct Views {
    source: GameSessionId,
    instants: [u64;3],
    stamina: u16,
    future_stamina: [Option<u16>;3],
    prey: Option<(Vec<u8>,[bool;3],u32,bool)>,
    charm: Option<String>,
    pvp: Option<String>,
}

pub(super) async fn before(owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, actor: ExactActorRef) -> TestResult<Views> {
    let source = admitted.game_session_id;
    // Publish the source observation through its existing fenced owner. An
    // absent durable PvP row produces an empty live CombatLock, including when
    // the wire refreshes it before the exit. Capture that exact owned state.
    assert!(owner.refresh_pvp_locks(actor, source).await,
        "current native House PvP observation before handoff");
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let mut attack = owner.states.attack.lock().await;
    let unused = GameSessionId::decode(&super::super::id(198)).map_err(debug)?;
    let prepared = states.prepare_house_transfer_views(&runtime, &attack, actor, source, unused)
        .ok_or("current source data is ready for native transfer")?;
    assert!(states.commit_house_transfer_views(&runtime, &mut attack, prepared).is_none(),
        "preparing a view token grants no successor session before the native commit");
    let now = owner.owner_now().get();
    let instants = [now, now+60_000_000, now+120_000_000];
    Ok(Views {
        source, instants,
        stamina: states.live_stamina(actor, source).ok_or("source live stamina")?,
        future_stamina: instants.map(|at| states.stamina_after_experience(actor, source, at)),
        prey: states.house_prey_transfer_observation(actor, source),
        charm: states.charm_states.iter().find(|(a,s,_)| *a==actor && *s==source)
            .map(|(_,_,state)| format!("{state:?}")),
        pvp: states.pvp_locks.iter().find(|(a,s,_)| *a==actor && *s==source)
            .map(|(_,_,state)| format!("{state:?}")),
    })
}

pub(super) async fn after(owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, actor: ExactActorRef, before: Views) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let attack = owner.states.attack.lock().await;
    let destination = admitted.game_session_id;
    assert_ne!(destination, before.source);
    assert_eq!(states.live_stamina(actor, before.source), None);
    assert_eq!(states.live_stamina(actor, destination), Some(before.stamina));
    assert_eq!(before.instants.map(|at| states.stamina_after_experience(actor, destination, at)),
        before.future_stamina, "the same HuntingClock determines future native XP debits");
    assert_eq!(states.house_prey_transfer_observation(actor, before.source), None);
    assert_eq!(states.house_prey_transfer_observation(actor, destination), before.prey);
    assert!(!states.charm_states.iter().any(|(a,s,_)| *a==actor && *s==before.source));
    assert_eq!(states.charm_states.iter().find(|(a,s,_)| *a==actor && *s==destination)
        .map(|(_,_,state)| format!("{state:?}")), before.charm);
    assert!(!states.pvp_locks.iter().any(|(a,s,_)| *a==actor && *s==before.source));
    assert_eq!(states.pvp_locks.iter().find(|(a,s,_)| *a==actor && *s==destination)
        .map(|(_,_,state)| format!("{state:?}")), before.pvp);
    let unused = GameSessionId::decode(&super::super::id(198)).map_err(debug)?;
    assert!(states.prepare_house_transfer_views(&runtime, &attack, actor, before.source, unused).is_none(),
        "a retired source cannot move values into another command namespace");
    Ok(())
}
