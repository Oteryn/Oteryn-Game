//! Native objective guardians and exact lethal credit, without quest progress fixtures.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result, encode_step_intent};

const CREATURE: &str = "oteryn:creature.corym_charlatan";
const COUNTER: &str = "oteryn:quest-progress/questpc7/hidden/kills";
const GUARDIAN: MovementLocalPosition = MovementLocalPosition { x:33080, y:32015, floor:13 };

#[test]
fn questpc7_native_objectives_spawn_once_and_credit_exact_lethal_occurrences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let start=std::time::Instant::now();
    let result=super::kill_reward_e2e::scenario_thread::run("questpc7-native-deaths",|| {
        super::quest_world_callbacks::scenario("questpc7-hidden-deaths",Position {x:33079,y:32015,floor:-13},false)
    });
    eprintln!("QUESTPC7_NATIVE_DEATHS POSTGRES_CASE_COMPLETED duration={:?} success={}",start.elapsed(),result.is_ok());
    result
}

async fn counter(owner:&ComposedFreshAdmission<'_, '_, '_>, admitted:&AdmittedSession)->TestResult<i64>{
    let f=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
    let state=owner.root.read_character_quest_state(owner.character,f.character_id).await.map_err(debug)?;
    Ok(state.tracks().get(COUNTER).copied().unwrap_or(-1))
}

async fn guardian(owner:&ComposedFreshAdmission<'_, '_, '_>)->TestResult<Option<ExactActorRef>> {
    let runtime=owner.runtime.lock().await;
    let found:Vec<_>=runtime.positioned_melee_census().map_err(debug)?.into_iter()
        .filter(|(_,p,session)|session.is_none() && p.position()==GUARDIAN).map(|(actor,_,_)|actor).collect();
    assert!(found.len()<=1,"refresh cannot duplicate a guardian slot");
    Ok(found.first().copied())
}

async fn spawn(owner:&ComposedFreshAdmission<'_, '_, '_>, at:MovementLocalPosition)->TestResult<ExactActorRef>{
    let mut runtime=owner.runtime.lock().await;
    let policy=runtime.companion_policy(CREATURE).map_err(debug)?;
    let actor=runtime.admit_encounter_actor(CREATURE,policy.maximum_health,at).map_err(debug)?;
    runtime.install_creature_policy(actor,CREATURE).map_err(debug)?;
    Ok(actor)
}

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>, admitted:&mut AdmittedSession,h:&Harness)->TestResult{
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_NATIVE_DEATHS POSTGRES_STARTED version={version}");
    let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
    native.install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
    let actor=admitted.runtime_actor.ok_or("player")?;
    let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
    owner.runtime.lock().await.bind_attacker_lease(actor,admitted.game_session_id,
        crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,fence.character_lease_generation).map_err(debug)?).map_err(debug)?;
    assert_eq!(counter(owner,admitted).await?,-1,"no subject progress seeds");
    assert_eq!(guardian(owner).await?,None);
    let command=admitted.continuity.next_command_id;
    let reply=super::quest_world_callbacks::wire(owner,admitted,command,COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,encode_step_intent(StepDirection::East)).await?;
    assert_eq!(decode_step_result(&reply).map_err(debug)?,StepDisposition::Moved);
    assert_eq!(counter(owner,admitted).await?,0);
    owner.refresh_quest_session(admitted.game_session_id,false).await;
    let first=guardian(owner).await?.expect("an active objective must admit its native guardian");
    owner.refresh_quest_session(admitted.game_session_id,false).await;
    assert_eq!(guardian(owner).await?,Some(first));
    // The right species outside the selected room cannot satisfy the objective.
    let outside=spawn(owner,MovementLocalPosition{x:33083,..GUARDIAN}).await?;
    lethal(owner,admitted,outside,5000).await?;
    assert_eq!(counter(owner,admitted).await?,0);
    for count in 1..=5 {
        let target=guardian(owner).await?.ok_or("guardian respawn")?;
        lethal(owner,admitted,target,5000+count).await?;
        assert_eq!(counter(owner,admitted).await?,i64::try_from(count)?);
        owner.refresh_quest_session(admitted.game_session_id,false).await;
        assert_eq!(guardian(owner).await?,None,"dead guardian is retired after settlement");
        owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(61)).ok_or("clock")?;
        owner.refresh_quest_session(admitted.game_session_id,false).await;
        if count==5 {assert_eq!(guardian(owner).await?,None,"quota completion cannot respawn another guardian");}
    }
    let extra=spawn(owner,GUARDIAN).await?;
    lethal(owner,admitted,extra,5010).await?;
    assert_eq!(counter(owner,admitted).await?,5,"bounded quota cannot over-credit");
    Ok(())
}

async fn lethal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
    command: u64,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert!(
        owner
            .refresh_source_party_presence(actor, admitted.game_session_id)
            .await
    );
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(debug)?;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_swing_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                ),
                1,
                crate::foundation::OwnerDamageCommand {
                    target: CREATURE.as_bytes(),
                    occurrence: admitted.game_session_id.as_bytes(),
                    binding: b"questpc7-native-objectives",
                    damage: health,
                },
            )
            .map_err(debug)?;
        // Duplicate lethal notification must retain just one quest credit.
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
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
    }
    assert!(queue.has_session(admitted.game_session_id));
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    Ok(())
}
