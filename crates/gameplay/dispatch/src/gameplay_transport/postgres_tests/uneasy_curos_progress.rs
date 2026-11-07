//! Source NPC report and briefing consume native death credit, not a test flag write.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
const NPC: &str = "oteryn:npc.curos";
const PREFIX: &str = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/";
#[test]
fn uneasy_curos_progress_reports_native_renegade_death() -> TestResult {
    scenario("report")
}
#[test]
fn uneasy_curos_progress_briefs_after_native_death_report() -> TestResult {
    scenario("brief")
}
#[test]
fn uneasy_curos_progress_refuses_report_without_death() -> TestResult {
    scenario("no-kill")
}
fn scenario(case: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(case, move || {
        super::quest_world_callbacks::scenario(
            &format!("uneasy-curos-progress-{case}"),
            Position {
                x: 33175,
                y: 31393,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "UNEASY_CUROS_PROGRESS POSTGRES_CASE_{} case={case} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
pub(super) async fn kill_renegade(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command: u64,
) -> TestResult {
    let creature = "oteryn:creature.renegade_orc";
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    owner.refresh_quest_session(session, false).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let origin = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let policy = runtime.companion_policy(creature).map_err(debug)?;
        let target = runtime
            .admit_encounter_actor(
                creature,
                policy.maximum_health,
                crate::foundation::MovementLocalPosition {
                    x: origin.x + 1,
                    ..origin
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, creature)
            .map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: creature.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"uneasy-native-death",
                    damage: policy.maximum_health,
                },
            )
            .map_err(debug)?;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
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
                session,
                actor,
            },
            now,
            Some(observed),
        );
        target
    };
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    assert!(!owner.runtime.lock().await.corpse_projection_pinned(target));
    Ok(())
}
async fn value(h: &Harness, admitted: &AdmittedSession, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(admitted.item_fence.ok_or("fence")?.character_id.as_bytes().as_slice()).bind(format!("{PREFIX}{track}")).fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    assert!(version.starts_with("PostgreSQL"));
    eprintln!("UNEASY_CUROS_PROGRESS actual_postgresql=true {version}");
    super::uneasy_curos::source_placement()?;
    super::uneasy_curos::prerequisite(owner, admitted, false).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    super::classic_quests::talk(owner, admitted, npc, 9400, "hi").await?;
    super::classic_quests::talk(owner, admitted, npc, 9401, "mission").await?;
    assert_eq!(value(h, admitted, "questline").await?, 1);
    assert_eq!(value(h, admitted, "quest_door").await?, 0);
    if !name.ends_with("no-kill") {
        kill_renegade(owner, admitted, 9402).await?;
        assert_eq!(value(h, admitted, "quest_door").await?, 1);
    }
    let reply = super::classic_quests::talk(owner, admitted, npc, 9403, "mission").await?;
    if name.ends_with("no-kill") {
        assert_eq!(value(h, admitted, "questline").await?, 1);
        return Ok(());
    }
    assert_eq!(
        value(h, admitted, "questline").await?,
        2,
        "Curos must accept actual native death credit"
    );
    assert!(
        reply
            .iter()
            .any(|line| line.contains("death of their leader")),
        "source report reply: {reply:?}"
    );
    if name.ends_with("brief") {
        let briefing = super::classic_quests::talk(owner, admitted, npc, 9404, "mission").await?;
        assert_eq!(value(h, admitted, "questline").await?, 3);
        assert!(
            briefing.iter().any(|line| line.contains("scrying device")),
            "source crystal briefing: {briefing:?}"
        );
    }
    Ok(())
}
