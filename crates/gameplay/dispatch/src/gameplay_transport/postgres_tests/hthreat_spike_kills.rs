//! Crystal 201189f8 creaturescripts_lower_spike_kill.lua:8–22 through lethal settlement.
use super::TestResult;
use super::full_world_support::world_scenario;
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition, authored_bindings::Bindings,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const QUEST: &str = "oteryn:quest.spike_task";
const PHASE: &str = "oteryn:quest-progress/spike-task/lower-hunt-assignment";
const COUNT: &str = "oteryn:quest-progress/spike-task/lower-hunt-kills";
const CREDIT: &str = "oteryn:quest-transition/native-kills/spike-task-lower-drillworm";
const DRILLWORM: &str = "oteryn:creature.drillworm";
const INSIDE: MovementLocalPosition = MovementLocalPosition {
    x: 32120,
    y: 32470,
    floor: 13,
};

fn bindings() -> TestResult<Bindings> {
    let shard: Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/definitions/quests-00200-00299.json"
    ))?;
    let recipe = shard["records"]
        .as_array()
        .ok_or("Quest records")?
        .iter()
        .map(|record| &record["definition"])
        .find(|definition| definition["identity"]["key"] == QUEST)
        .ok_or("canonical Spike definition")?;
    let digest = Sha256::digest(serde_json::to_vec(
        &recipe["oteryn_recipe"]["payload"]["recipe"],
    )?)
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect::<String>();
    let document = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,
        "admission":{"minimum_level":80,"premium":false,"personal_assignment":"spike_tasks"},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/spike_tasks/creaturescripts_lower_spike_kill.lua:8"],
        "tracks":[
            {"key":PHASE,"initial":0,"minimum":0,"maximum":1},
            {"key":COUNT,"initial":0,"minimum":0,"maximum":7}
        ],
        "stages":[{"key":"lower-drillworm","completes":false,"trigger":{"kind":"kill","counter":{
            "key":CREDIT,"creature":DRILLWORM,"track":COUNT,"gate":PHASE,
            "gate_value":1,"maximum":7,
            "death_area":{"minimum":[32120,32470,13],"maximum":[32345,32710,15]}
        }}}]
    }]});
    Ok(Bindings::parse(&serde_json::to_vec(&document)?)?)
}

async fn kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    killer: ExactActorRef,
    session: GameSessionId,
    position: MovementLocalPosition,
    ordinal: u64,
) -> TestResult {
    use super::fixture::debug;
    use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
    owner.refresh_quest_session(session, false).await;
    assert!(owner.refresh_source_party_presence(killer, session).await);
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let maximum = runtime
            .companion_policy(DRILLWORM)
            .map_err(debug)?
            .maximum_health;
        let target = runtime
            .admit_encounter_actor(DRILLWORM, maximum, position)
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, DRILLWORM)
            .map_err(debug)?;
        let lease = runtime
            .bound_attacker_lease(killer, session)
            .map_err(debug)?;
        let damage = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                killer,
                CommandRef::new(session, CommandId::new(ordinal).map_err(debug)?),
                0,
                OwnerDamageCommand {
                    target: DRILLWORM.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"spike-drillworm",
                    damage,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            record_creature_kill(
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
                    session,
                    actor: killer,
                },
                now,
                Some(observed),
            );
        }
        target
    };
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(target)
        .map_err(debug)?;
    Ok(())
}

#[test]
fn hthreat_spike_kills_require_personal_assignment_and_lower_spike_deaths() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run("hthreat-spike-kills", scenario)
}

fn scenario() -> TestResult {
    world_scenario!(
        "hthreat_spike_kills",
        |h,
         _request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            let bindings = bindings()?;
            bindings.verify_embedded_recipe_evidence()?;
            let mut missing = bindings.clone();
            missing.quests[0]
                .admission
                .as_mut()
                .ok_or("actor admission")?
                .personal_assignment = None;
            assert!(missing.verify_embedded_recipe_evidence().is_err());
            let base = QuestStateCatalogue::empty("content-1").ok_or("catalogue")?;
            owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
            // Provision the actor's level before calling the normal death owner.
            let mut setup = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *setup)
                .await?;
            sqlx::query("UPDATE game_character_progression_state SET level=80,total_experience=7915800 WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *setup).await?;
            setup.commit().await?;
            let count = async || -> TestResult<i64> {
                Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)")
                    .bind(COUNT).fetch_one(&h.pool).await?)
            };

            kill(&owner, actor, session, INSIDE, 100).await?;
            assert_eq!(count().await?, 0, "no credit before accepting the hunt");
            // Provision the result of personally accepting the assignment. Only lethal
            // work below may create the kill counter or credit receipt.
            let mut setup = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *setup)
                .await?;
            let seeded = sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(PHASE).bind(QUEST)
                .execute(&mut *setup).await?;
            assert_eq!(seeded.rows_affected(), 1);
            setup.commit().await?;
            kill(
                &owner,
                actor,
                session,
                MovementLocalPosition {
                    x: INSIDE.x - 1,
                    ..INSIDE
                },
                102,
            )
            .await?;
            assert_eq!(
                count().await?,
                0,
                "one tile outside the source bounds earns no credit"
            );
            for earned in 1..=8 {
                kill(&owner, actor, session, INSIDE, 102 + earned).await?;
                assert_eq!(count().await?, i64::try_from(earned.min(7))?);
            }
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1 AND NOT completes")
                .bind(CREDIT).fetch_one(&h.pool).await?;
            assert_eq!(receipts, 7, "one durable credit per qualified death");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}
