use super::TestResult;
use super::fixture::debug;
use crate::foundation::{CommandId, CommandRef, ExactActorRef, OwnerDamageCommand};

pub(super) async fn kill(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    target: ExactActorRef,
    creature: &str,
    command: u64,
) -> TestResult {
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let lease = runtime
        .bound_attacker_lease(actor, admitted.game_session_id)
        .map_err(debug)?;
    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            target,
            actor,
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(command).map_err(debug)?,
            ),
            0,
            OwnerDamageCommand {
                target: creature.as_bytes(),
                occurrence: b"engenc2",
                binding: b"engenc2-phase-death",
                damage: health,
            },
        )
        .map_err(debug)?;
    for _ in 0..2 {
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: Some(&states),
                activity: &[],
                queue: &queue,
                rewards: owner.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: admitted.game_session_id,
                actor,
            },
            now,
            Some(observed),
        );
    }
    Ok(())
}
