//! The actual entry and timeout commands keep the same live attack owner state
//! through both scope barriers. No copied actor or manually installed destination
//! attack entry can satisfy these assertions.
use super::{Harness, TestResult, debug, exchange_command, wire};
use crate::combat_attack::AttackState;
use crate::foundation::{CommandRef, ExactActorRef, GameSessionState, RuntimeScopeRefV1};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::activity_scope::ActivityScope;
use oteryn_protocol_oteryn::boss_encounter::BossActivityReply;
use oteryn_protocol_oteryn::attack::{ChaseMode, FightMode, FightModes};
use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode};
use oteryn_simulation_determinism::SemanticTimeMicros;

pub(super) struct AttackContinuity {
    state: AttackState<ExactActorRef, CommandRef>,
    modes: ExpertFightModes,
    observed_at: SemanticTimeMicros,
    deadline: SemanticTimeMicros,
}

pub(super) async fn prepare_attack(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<AttackContinuity> {
    let actor = admitted.runtime_actor.ok_or("source attack actor")?;
    let now = owner.owner_now();
    let runtime = owner.runtime.lock().await;
    let mut attack = owner.states.attack.lock().await;
    let modes = ExpertFightModes {
        modes: FightModes {
            fight_mode: FightMode::Defensive,
            chase: ChaseMode::Chase,
            secure: false,
        },
        expert: ExpertMode::RedFist,
    };
    assert!(attack.set_expert_modes(&runtime, actor, admitted.game_session_id, modes));
    // The existing hit owner supplies its checked-in in-fight duration. This
    // test never invents or extends a deadline during either scope transfer.
    attack.record_hit_taken(&runtime, actor, admitted.game_session_id, now);
    let state = attack.party_attack_state(actor, admitted.game_session_id)
        .ok_or("source native attack entry")?.clone();
    let deadline = state.in_fight_until().ok_or("source in-fight deadline")?;
    assert!(state.in_fight(now));
    Ok(AttackContinuity { state, modes, observed_at: now, deadline })
}

async fn assert_attack(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    expected: &AttackContinuity,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("transferred attack actor")?;
    let runtime = owner.runtime.lock().await;
    runtime.player_control_facts(actor, admitted.game_session_id).map_err(debug)?;
    let attack = owner.states.attack.lock().await;
    assert_eq!(
        attack.party_attack_state(actor, admitted.game_session_id),
        Some(&expected.state),
        "the native coordinator must install the retained actor-owned attack state",
    );
    assert_eq!(attack.expert_modes(actor, admitted.game_session_id), expected.modes);
    assert_eq!(
        attack.in_fight_until(actor, admitted.game_session_id, expected.observed_at),
        Some(expected.deadline),
    );
    assert_eq!(attack.in_fight_until(actor, admitted.game_session_id, expected.deadline), None);
    Ok(())
}

pub(super) async fn assert_entry_and_timeout_return(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss_key: &str,
    expected: AttackContinuity,
) -> TestResult {
    assert_attack(instance, *admitted, &expected).await?;
    let source = *admitted;
    let (deadline, now): (i64, i64) = sqlx::query_as(
        "SELECT deadline,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities",
    ).fetch_one(&h.pool).await?;
    // Case::Entry uses the existing isolated one-second room policy. Wait for
    // its real durable deadline; the command cannot manufacture a Timeout.
    tokio::time::sleep(std::time::Duration::from_millis(
        u64::try_from(deadline.saturating_sub(now).max(0))? + 5,
    )).await;
    let (_, reply, barrier) = exchange_command(
        origin, admitted, boss_key, true, wire::COMMAND_TYPE_BOSS_TIMEOUT,
    ).await?;
    let BossActivityReply::Accepted { handoff, deadline_unix_ms: None } = reply else {
        return Err(format!("real timeout return expected: {reply:?}").into());
    };
    let barrier = barrier.ok_or("return scope barrier")?;
    assert_eq!(barrier.handoff, handoff);
    assert_eq!(barrier.source_session, *source.game_session_id.as_bytes());
    assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
    assert_ne!(source.game_session_id, admitted.game_session_id);
    let RuntimeScopeRefV1::Channel { world_id, channel_id } =
        origin.runtime.lock().await.binding().runtime_scope()
    else {
        return Err("actual origin Channel".into());
    };
    assert_eq!(barrier.scope, ActivityScope::Channel {
        world: *world_id.as_bytes(), channel: *channel_id.as_bytes(),
    });
    assert_eq!(barrier.connection_generation, 1);
    assert_eq!(
        barrier.lease_generation,
        source.item_fence.ok_or("Instance lease")?.character_lease_generation + 1,
    );
    assert_attack(origin, *admitted, &expected).await?;
    assert!(instance.runtime.lock().await
        .player_control_facts(source.runtime_actor.ok_or("old Instance actor")?, source.game_session_id)
        .is_err());
    assert_eq!(
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(source.game_session_id).await?.0.session_state(),
        GameSessionState::Terminal,
    );
    eprintln!("BOSS_ACTIVITY_ATTACK_STATE_ENTRY_AND_RETURN POSTGRES_CASE_COMPLETED");
    Ok(())
}
