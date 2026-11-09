//! M30 durable-owner cases mounted in the routed authority lane.
use crate::{durability, foundation};
use crate::bestiary_postgres_harness as fixture;

// Execute the production queue directly without compiling dispatch's unrelated lib-test suite.
#[path = "../../../../../crates/gameplay/dispatch/src/gameplay_transport/quest_hits/queue.rs"]
mod dispatcher_hit_queue;

use durability::character_progression::ExperienceRewardOccurrence;
use durability::quest_state::encounter_deaths::EncounterDeathContext;
use durability::quest_state::native_kills::{NativeDeathSource, NativeQuestDeath};
use durability::quest_state::{QuestCause, QuestTransitionOutcome};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks,
};
use serde_json::json;
use std::sync::Arc;

const ENTRY: &str = "oteryn:quest-callback/engenc-pg-entry";
const COUNT: &str = "oteryn:quest-progress/authored-kills/count";
const PHASE: &str = "oteryn:quest-progress/authored-kills/phase";
const KILL: &str = "oteryn:quest-transition/native-kills/authored-engine-rat";
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";

fn catalogue(maximum: i64, advance: Option<i64>) -> fixture::TestResult<Arc<QuestStateCatalogue>> {
    let mut value: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../crates/gameplay/quests/src/quest/authored_bindings/fixtures/kill.json"
    ))?;
    let quest = &mut value["quests"][0];
    quest["tracks"][0]["initial"] = json!(4);
    quest["stages"].as_array_mut().ok_or("stages")?.remove(0);
    quest["stages"][0]["completes"] = json!(false);
    let counter = &mut quest["stages"][0]["trigger"]["counter"];
    counter["maximum"] = json!(maximum);
    counter["encounter_source"] = json!(ENTRY);
    if let Some(value) = advance {
        counter["advance_gate_to"] = json!(value);
    } else {
        counter
            .as_object_mut()
            .ok_or("counter")?
            .remove("advance_gate_to");
    }
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-callback/behemoth")
        .ok_or("template")?;
    callback.key = ENTRY.into();
    callback.quest = QUEST.into();
    callback.passages.clear();
    callback.storage.clear();
    callback.selector.position = Some(world_callbacks::Position {
        x: 100,
        y: 99,
        floor: -7,
    });
    callback.selector.unique = None;
    callback.selector.action = None;
    callback.selector.native = None;
    callback.effects.encounter = Some(serde_json::from_value(json!({"minimum_level":1,
        "players":[{"from":{"x":99,"y":99,"floor":-7},"to":{"x":100,"y":100,"floor":-7}}],
        "monsters":[{"position":{"x":101,"y":100,"floor":-7},"creature":"oteryn:creature.rat"}],
        "room_min":{"x":100,"y":100,"floor":-7},"room_max":{"x":101,"y":100,"floor":-7},"timing":null}))?);
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    let template = encounter.monsters[0].clone();
    let count = u16::try_from(maximum)?;
    encounter.monsters = (0..count)
        .map(|offset| {
            let mut monster = template.clone();
            monster.position.x += offset;
            monster
        })
        .collect();
    encounter.room_max.x = 100 + count;
    quest["stages"].as_array_mut().ok_or("stages")?.push(
        json!({"key":"entry","completes":false,"trigger":{"kind":"map_use","callback":callback}}),
    );
    let bindings = Bindings::parse(&serde_json::to_vec(&value)?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("base")?;
    Ok(Arc::new(bindings.attach(&base)?))
}

#[test]
fn engenc_admitted_death_is_fenced_atomic_and_replay_safe_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engenc_admitted_death", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!("ENGENC_M30_PG_EXECUTED duration={:?}", started.elapsed());
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC PG worker panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let catalogue = catalogue(1, None)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::fence(1)?;
    let context = EncounterDeathContext {
        source: ENTRY.into(),
        scope: fence.runtime_scope,
        scope_generation: fence.scope_ownership_generation.get(),
        character: *fence.character_id.as_bytes(),
        session: fence.game_session_id,
        lease_generation: fence.character_lease_generation,
        participant: [7; 16],
        defender: [8; 16],
    };
    let death = NativeQuestDeath {
        transition: KILL.into(),
        creature: "oteryn:creature.rat".into(),
        observed_revision: 1,
        definition_hash: catalogue.definition_hash(QUEST),
        source: NativeDeathSource::Encounter(context),
    };
    let cause = QuestCause::CreatureDeath(
        ExperienceRewardOccurrence::from_bytes(fixture::id(100)).map_err(fixture::debug)?,
    );
    let mut stale = death.clone();
    if let NativeDeathSource::Encounter(context) = &mut stale.source {
        context.lease_generation += 1;
    }
    let refused = h
        .root
        .commit_character_native_quest_kill(
            &authority,
            &h.node,
            fence,
            cause,
            stale,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    if !matches!(refused, QuestTransitionOutcome::Refused(_)) {
        return Err("stale native recipient admitted quest progress".into());
    }
    assert_eq!(h.root_revision().await?, "1");
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        receipts, 0,
        "stale recipient publishes no receipt or progress"
    );
    let outcome = h
        .root
        .commit_character_native_quest_kill(
            &authority,
            &h.node,
            fence,
            cause,
            death.clone(),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    let count: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(COUNT)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(count, 1);
    let phase_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(PHASE)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        phase_rows, 0,
        "a death must not rewrite the admission epoch"
    );
    let replay = h
        .root
        .commit_character_native_quest_kill(
            &authority,
            &h.node,
            fixture::fence(2)?,
            cause,
            death,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 1, "replay cannot duplicate defender credit");
    Ok(())
}

#[test]
fn engenc_source_threshold_keeps_admission_epoch_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h =
                        fixture::Harness::create(admin, "engenc_source_threshold", true).await?;
                    let result = threshold_exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_THRESHOLD_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC threshold PG worker panicked")?
        .map_err(Into::into)
}
async fn threshold_exercise(h: &fixture::Harness) -> fixture::TestResult {
    let catalogue = catalogue(2, Some(5))?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    for ordinal in 0_u8..2 {
        let fence = fixture::fence(u64::from(ordinal) + 1)?;
        let death = NativeQuestDeath {
            transition: KILL.into(),
            creature: "oteryn:creature.rat".into(),
            observed_revision: 1,
            definition_hash: catalogue.definition_hash(QUEST),
            source: NativeDeathSource::Encounter(EncounterDeathContext {
                source: ENTRY.into(),
                scope: fence.runtime_scope,
                scope_generation: fence.scope_ownership_generation.get(),
                character: *fence.character_id.as_bytes(),
                session: fence.game_session_id,
                lease_generation: fence.character_lease_generation,
                participant: [7; 16],
                defender: [8 + ordinal; 16],
            }),
        };
        let cause = QuestCause::CreatureDeath(
            ExperienceRewardOccurrence::from_bytes(fixture::id(110 + ordinal))
                .map_err(fixture::debug)?,
        );
        let outcome = h
            .root
            .commit_character_native_quest_kill(
                &authority,
                &h.node,
                fence,
                cause,
                death,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(fixture::debug)?;
        if !matches!(outcome, QuestTransitionOutcome::Committed(_)) {
            return Err("admitted defender was rejected after an earlier nonterminal kill".into());
        }
        let count: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(COUNT)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, i64::from(ordinal) + 1);
        if ordinal == 0 {
            let phases: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1",
            )
            .bind(PHASE)
            .fetch_one(&h.pool)
            .await?;
            if phases != 0 {
                return Err("nonterminal kill rewrote the admission epoch".into());
            }
        }
    }
    let phase: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(PHASE)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(phase, 5);
    assert_eq!(h.root_revision().await?, "3");
    Ok(())
}

#[test]
fn engenc_hit_objective_rejects_unproven_use_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engenc_hit_unproven", true).await?;
                    let result = exercise_unproven_hit(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_HIT_ADMISSION_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC hit PG worker panicked")?
        .map_err(Into::into)
}

fn hit_catalogue() -> fixture::TestResult<Arc<QuestStateCatalogue>> {
    let mut value: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../crates/gameplay/quests/src/quest/authored_bindings/fixtures/kill.json"
    ))?;
    let quest = &mut value["quests"][0];
    quest["tracks"][0]["initial"] = json!(4);
    quest["stages"].as_array_mut().ok_or("stages")?.remove(0);
    quest["stages"][0]["completes"] = json!(false);
    quest["stages"][0]["trigger"] = json!({"kind":"hit","objective":{
        "key":"oteryn:quest-transition/native-hits/training",
        "creature":"oteryn:creature.rat","source":{"kind":"weapon","item":"oteryn:item.spear"},
        "track":COUNT,"gate":PHASE,"gate_value":4,"advance_gate_to":5
    }});
    let bindings = Bindings::parse(&serde_json::to_vec(&value)?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("base")?;
    Ok(Arc::new(bindings.attach(&base)?))
}

async fn exercise_unproven_hit(h: &fixture::Harness) -> fixture::TestResult {
    let catalogue = hit_catalogue()?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::fence(1)?;
    let command = foundation::CommandRef::new(
        fence.game_session_id,
        foundation::CommandId::new(1).map_err(fixture::debug)?,
    );
    let outcome = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fence,
            durability::quest_state::QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/native-hits/training".into(),
                cause: QuestCause::Use(command),
            },
            catalogue,
        )
        .await;
    assert!(
        outcome.is_err(),
        "USE without committed native damage must never mint hit credit: {outcome:?}"
    );
    assert_eq!(h.root_revision().await?, "1");
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 0);
    let tracks: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(tracks, 0);
    Ok(())
}

#[test]
fn engenc_native_hit_is_atomic_fenced_and_replay_safe_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engenc_native_hit", true).await?;
                    let result = exercise_native_hit(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_NATIVE_HIT_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC native hit PG worker panicked")?
        .map_err(Into::into)
}

type NativeHitRuntime = (
    foundation::ChannelRuntimeV1,
    foundation::ExactActorRef,
    foundation::ExactActorRef,
    foundation::CommandRef,
);

fn native_hit_runtime(lease_generation: u64) -> fixture::TestResult<NativeHitRuntime> {
    let fence = fixture::fence(1)?;
    let foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err("fixture channel".into());
    };
    let mut runtime = foundation::ChannelRuntimeV1::from_committed_assignment(
        world_id,
        channel_id,
        foundation::NodeId::decode(&fixture::id(3)).map_err(fixture::debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        foundation::ChannelContentPin::test(world_id),
    )
    .map_err(fixture::debug)?;
    let reservation = runtime
        .reserve_fresh_session(fence.game_session_id)
        .map_err(fixture::debug)?;
    let player = runtime
        .commit_fresh_session(reservation)
        .map_err(fixture::debug)?;
    runtime
        .bind_attacker_lease(
            player,
            fence.game_session_id,
            foundation::CharacterLease::new(
                foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(fixture::debug)?,
                lease_generation,
            )
            .map_err(fixture::debug)?,
        )
        .map_err(fixture::debug)?;
    let target = runtime
        .admit_monster_lab_creature(
            foundation::MovementLocalPosition {
                x: 2,
                y: 1,
                floor: 7,
            },
            "engenc:training",
            100,
        )
        .map_err(fixture::debug)?;
    let command = foundation::CommandRef::new(
        fence.game_session_id,
        foundation::CommandId::new(1).map_err(fixture::debug)?,
    );
    Ok((runtime, player, target, command))
}

fn native_hit_witness(
    lease_generation: u64,
) -> fixture::TestResult<foundation::runtime_actor_carrier::CommittedNativeHit> {
    let (mut runtime, player, target, command) = native_hit_runtime(lease_generation)?;
    let mut owner = runtime.borrow_exact_actor_commit();
    let result = owner
        .commit_damage_for_bound_attacker(
            target,
            player,
            command,
            0,
            foundation::OwnerDamageCommand {
                target: b"engenc:training",
                occurrence: &[],
                binding: b"engenc:training-hit",
                damage: 5,
            },
        )
        .map_err(fixture::debug)?;
    assert!(result.applied);
    owner
        .committed_hit_for_bound_attacker(target, player, command, 0)
        .map_err(fixture::debug)?
        .ok_or_else(|| "native hit witness absent".into())
}

async fn exercise_native_hit(h: &fixture::Harness) -> fixture::TestResult {
    use durability::quest_state::native_hits::NativeQuestHit;
    let catalogue = hit_catalogue()?;
    let hash = catalogue.definition_hash(QUEST).ok_or("hash")?;
    let hit = |witness, item: &str| {
        NativeQuestHit::from_committed(
            "oteryn:quest-transition/native-hits/training".into(),
            "oteryn:creature.rat".into(),
            item.into(),
            None,
            1,
            hash,
            witness,
        )
    };
    let witness = native_hit_witness(1)?;
    let valid = hit(witness, "oteryn:item.spear");
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::fence(1)?;
    assert!(
        h.root
            .commit_character_native_quest_hit(
                &authority,
                &h.node,
                fence,
                hit(witness, "oteryn:item.sword"),
                Arc::clone(&catalogue)
            )
            .await
            .is_err()
    );
    let refused = h
        .root
        .commit_character_native_quest_hit(
            &authority,
            &h.node,
            fence,
            hit(native_hit_witness(2)?, "oteryn:item.spear"),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(refused, QuestTransitionOutcome::Refused(_)));
    let future = NativeQuestHit::from_committed(
        "oteryn:quest-transition/native-hits/training".into(),
        "oteryn:creature.rat".into(),
        "oteryn:item.spear".into(),
        None,
        2,
        hash,
        witness,
    );
    let refused = h
        .root
        .commit_character_native_quest_hit(
            &authority,
            &h.node,
            fence,
            future,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(refused, QuestTransitionOutcome::Refused(_)));
    assert_eq!(h.root_revision().await?, "1");
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(before, 0);
    let committed = h
        .root
        .commit_character_native_quest_hit(
            &authority,
            &h.node,
            fence,
            valid.clone(),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    assert_eq!(h.root_revision().await?, "2");
    let replay = h
        .root
        .commit_character_native_quest_hit(
            &authority,
            &h.node,
            fence,
            valid,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='native_hit' AND cause_ordinal=0").fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    let count: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(COUNT)
            .fetch_one(&h.pool)
            .await?;
    let phase: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(PHASE)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!((count, phase), (1, 5));
    Ok(())
}

#[test]
fn engenc_hit_revision_slot_uses_current_cursor_and_replays_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h =
                        fixture::Harness::create(admin, "engenc_hit_revision_slot", true).await?;
                    let result = exercise_hit_revision_slot(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_HIT_REVISION_SLOT_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC revision slot PG worker panicked")?
        .map_err(Into::into)
}

async fn exercise_hit_revision_slot(h: &fixture::Harness) -> fixture::TestResult {
    use durability::character_revision_sequencer::CharacterRevisionSequencer;
    use durability::quest_state::native_hits::NativeQuestHit;
    let catalogue = hit_catalogue()?;
    let hit = NativeQuestHit::from_committed(
        "oteryn:quest-transition/native-hits/training".into(),
        "oteryn:creature.rat".into(),
        "oteryn:item.spear".into(),
        None,
        1,
        catalogue.definition_hash(QUEST).ok_or("hash")?,
        native_hit_witness(1)?,
    );
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let mismatched = fixture::fence(99)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(mismatched.character_id).await;
    let committed = slot
        .commit_native_quest_hit(
            &h.root,
            &authority,
            &h.node,
            mismatched,
            &hit,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    assert_eq!(
        slot.cursor(&h.root, &authority)
            .await
            .map_err(fixture::debug)?
            .get(),
        2
    );
    let replay = slot
        .commit_native_quest_hit(
            &h.root,
            &authority,
            &h.node,
            fixture::fence(1)?,
            &hit,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        slot.cursor(&h.root, &authority)
            .await
            .map_err(fixture::debug)?
            .get(),
        2
    );
    assert_eq!(h.root_revision().await?, "2");
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='native_hit'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    Ok(())
}

#[test]
fn engenc_prepared_hit_captures_before_settlement_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engenc_prepared_hit", true).await?;
                    let result = exercise_prepared_hit(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_PREPARED_HIT_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC prepared hit PG worker panicked")?
        .map_err(Into::into)
}

async fn exercise_prepared_hit(h: &fixture::Harness) -> fixture::TestResult {
    use durability::character_revision_sequencer::CharacterRevisionSequencer;
    use durability::quest_state::native_hit_capture::PreparedQuestHit;
    let catalogue = hit_catalogue()?;
    let fence = fixture::fence(1)?;
    let (mut runtime, player, target, command) = native_hit_runtime(1)?;
    let other = runtime
        .admit_monster_lab_creature(
            foundation::MovementLocalPosition {
                x: 3,
                y: 1,
                floor: 7,
            },
            "engenc:other-training",
            100,
        )
        .map_err(fixture::debug)?;
    let lease = runtime
        .bound_attacker_lease(player, fence.game_session_id)
        .map_err(fixture::debug)?;
    let transition = "oteryn:quest-transition/native-hits/training";
    let mut plan = PreparedQuestHit::prepare(
        &catalogue,
        transition,
        &Default::default(),
        1,
        player,
        target,
        lease,
        command,
        0,
    )
    .ok_or("prepared hit")?;
    assert!(plan.pending().is_none());
    let inactive = std::collections::BTreeMap::from([(PHASE.into(), 3)]);
    assert!(
        PreparedQuestHit::prepare(
            &catalogue, transition, &inactive, 1, player, target, lease, command, 0,
        )
        .is_none()
    );
    let other_command = foundation::CommandRef::new(
        command.game_session_id(),
        foundation::CommandId::new(2).map_err(fixture::debug)?,
    );
    let mut command_plan = PreparedQuestHit::prepare(
        &catalogue,
        transition,
        &Default::default(),
        1,
        player,
        target,
        lease,
        other_command,
        0,
    )
    .ok_or("command plan")?;
    let mut ordinal_plan = PreparedQuestHit::prepare(
        &catalogue,
        transition,
        &Default::default(),
        1,
        player,
        target,
        lease,
        command,
        1,
    )
    .ok_or("ordinal plan")?;
    assert_eq!(runtime.live_creature_health(target), Some(100));
    let stale = native_hit_witness(2)?;
    assert!(!plan.capture(stale, "oteryn:creature.rat", "oteryn:item.spear", None));
    {
        let mut owner = runtime.borrow_exact_actor_commit();
        owner
            .commit_damage_for_bound_attacker(
                other,
                player,
                command,
                0,
                foundation::OwnerDamageCommand {
                    target: b"engenc:other-training",
                    occurrence: &[],
                    binding: b"engenc:other-target-hit",
                    damage: 5,
                },
            )
            .map_err(fixture::debug)?;
        let other_witness = owner
            .committed_hit_for_bound_attacker(other, player, command, 0)
            .map_err(fixture::debug)?
            .ok_or("other target hit")?;
        assert!(!plan.capture(
            other_witness,
            "oteryn:creature.rat",
            "oteryn:item.spear",
            None
        ));

        owner
            .commit_damage_for_bound_attacker(
                target,
                player,
                command,
                0,
                foundation::OwnerDamageCommand {
                    target: b"engenc:training",
                    occurrence: &[],
                    binding: b"engenc:prepared-hit",
                    damage: 5,
                },
            )
            .map_err(fixture::debug)?;
        let witness = owner
            .committed_hit_for_bound_attacker(target, player, command, 0)
            .map_err(fixture::debug)?
            .ok_or("committed hit")?;
        assert!(!plan.capture(witness, "oteryn:creature.rat", "oteryn:item.sword", None));
        assert!(!plan.capture(witness, "oteryn:creature.dragon", "oteryn:item.spear", None));
        assert!(
            !command_plan.capture(witness, "oteryn:creature.rat", "oteryn:item.spear", None),
            "another command's witness cannot fill this plan"
        );
        assert!(
            !ordinal_plan.capture(witness, "oteryn:creature.rat", "oteryn:item.spear", None),
            "another sub-ordinal's witness cannot fill this plan"
        );
        assert!(command_plan.pending().is_none());
        assert!(ordinal_plan.pending().is_none());
        assert!(plan.capture(witness, "oteryn:creature.rat", "oteryn:item.spear", None));
        let original = plan.pending().ok_or("pending")?.occurrence();
        assert!(!plan.capture(witness, "oteryn:creature.rat", "oteryn:item.spear", None));
        assert_eq!(plan.pending().ok_or("retained")?.occurrence(), original);
    }
    assert_eq!(runtime.live_creature_health(target), Some(95));
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(fence.character_id).await;
    let committed = slot
        .commit_native_quest_hit(
            &h.root,
            &authority,
            &h.node,
            fence,
            plan.pending().ok_or("pending")?,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    assert_eq!(h.root_revision().await?, "2");
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='native_hit'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    Ok(())
}

#[test]
fn engenc_dispatch_hit_queue_retains_unknown_outcome_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h =
                        fixture::Harness::create(admin, "engenc_dispatch_hit_queue", true).await?;
                    let result = exercise_dispatch_hit_queue(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_DISPATCH_HIT_QUEUE_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC dispatch hit queue PG worker panicked")?
        .map_err(Into::into)
}

async fn exercise_dispatch_hit_queue(h: &fixture::Harness) -> fixture::TestResult {
    use dispatcher_hit_queue::{HitQueue, Snapshot};
    use durability::character_revision_sequencer::CharacterRevisionSequencer;
    let catalogue = hit_catalogue()?;
    let fence = fixture::fence(1)?;
    let (mut runtime, actor, target, command) = native_hit_runtime(1)?;
    let lease = runtime
        .bound_attacker_lease(actor, fence.game_session_id)
        .map_err(fixture::debug)?;
    let mut queue = HitQueue::default();
    assert!(queue.install(
        &runtime,
        fence.game_session_id,
        Snapshot::new(
            Arc::clone(&catalogue),
            actor,
            fence.game_session_id,
            lease,
            1,
            &Default::default()
        )
    ));
    {
        let abandoned = queue
            .prepare(
                &runtime,
                actor,
                target,
                command,
                0,
                "oteryn:creature.rat",
                "oteryn:item.spear",
                None,
            )
            .map_err(fixture::debug)?;
        drop(abandoned);
    }
    assert!(!queue.has_pending());
    {
        let mut capture = queue
            .prepare(
                &runtime,
                actor,
                target,
                command,
                0,
                "oteryn:creature.rat",
                "oteryn:item.spear",
                None,
            )
            .map_err(fixture::debug)?;
        let mut native = runtime.borrow_exact_actor_commit();
        native
            .commit_damage_for_bound_attacker(
                target,
                actor,
                command,
                0,
                foundation::OwnerDamageCommand {
                    target: b"engenc:training",
                    occurrence: &[],
                    binding: b"engenc:queued-hit",
                    damage: 5,
                },
            )
            .map_err(fixture::debug)?;
        let witness = native
            .committed_hit_for_bound_attacker(target, actor, command, 0)
            .map_err(fixture::debug)?
            .ok_or("native hit")?;
        assert_eq!(
            capture.capture(witness, "oteryn:creature.rat", "oteryn:item.spear", None),
            1
        );
    }
    let first = queue.pending();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].actor, actor);
    assert_eq!(first[0].session, fence.game_session_id);
    assert_eq!(first[0].lease, lease);
    assert_eq!(
        first[0].transition,
        "oteryn:quest-transition/native-hits/training"
    );
    let occurrence = first[0].hit.occurrence();
    drop(first); // Cancellation before acknowledgement leaves the original source retained.
    assert_eq!(queue.pending()[0].hit.occurrence(), occurrence);
    assert!(queue.install(
        &runtime,
        fence.game_session_id,
        Snapshot::new(
            Arc::clone(&catalogue),
            actor,
            fence.game_session_id,
            lease,
            1,
            &Default::default()
        )
    ));
    assert_eq!(queue.pending()[0].hit.occurrence(), occurrence);

    {
        let later = foundation::CommandRef::new(
            command.game_session_id(),
            foundation::CommandId::new(2).map_err(fixture::debug)?,
        );
        let mut capture = queue
            .prepare(
                &runtime,
                actor,
                target,
                later,
                0,
                "oteryn:creature.rat",
                "oteryn:item.spear",
                None,
            )
            .map_err(fixture::debug)?;
        let mut native = runtime.borrow_exact_actor_commit();
        native
            .commit_damage_for_bound_attacker(
                target,
                actor,
                later,
                0,
                foundation::OwnerDamageCommand {
                    target: b"engenc:training",
                    occurrence: &[],
                    binding: b"engenc:later-hit",
                    damage: 5,
                },
            )
            .map_err(fixture::debug)?;
        let witness = native
            .committed_hit_for_bound_attacker(target, actor, later, 0)
            .map_err(fixture::debug)?
            .ok_or("later hit")?;
        assert_eq!(
            capture.capture(witness, "oteryn:creature.rat", "oteryn:item.spear", None),
            0
        );
    }
    assert_eq!(queue.pending()[0].hit.occurrence(), occurrence);
    let pending = queue.pending().remove(0);
    let mut stale_ack = pending.clone();
    stale_ack.hit = durability::quest_state::native_hits::NativeQuestHit::from_committed(
        pending.transition.clone(),
        "oteryn:creature.rat".into(),
        "oteryn:item.spear".into(),
        None,
        1,
        catalogue.definition_hash(QUEST).ok_or("hash")?,
        native_hit_witness(2)?,
    );
    assert!(!queue.acknowledge(&stale_ack));
    assert_eq!(queue.pending()[0].hit.occurrence(), occurrence);

    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    assert!(pending.current_for(&catalogue));
    let mut rebound = catalogue.authored_bindings().ok_or("bindings")?.clone();
    rebound.quests[0]
        .sources
        .push("changed source qualification witness".into());
    let base = QuestStateCatalogue::empty("content-1").ok_or("base")?;
    let changed = Arc::new(rebound.attach(&base)?);
    assert!(!pending.current_for(&changed));
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(fence.character_id).await;
    assert!(
        slot.commit_native_quest_hit(&h.root, &authority, &h.node, fence, &pending.hit, changed)
            .await
            .is_err()
    );
    assert_eq!(h.root_revision().await?, "1");

    let committed = slot
        .commit_native_quest_hit(
            &h.root,
            &authority,
            &h.node,
            fence,
            &pending.hit,
            Arc::clone(&pending.catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    // Simulate losing the acknowledgement after COMMIT, followed by a coherent refresh.
    assert!(queue.install(&runtime, fence.game_session_id, None));
    assert!(
        queue.has_pending(),
        "refresh must retain a committed-but-unacknowledged hit"
    );
    let retry = queue.pending().remove(0);
    assert_eq!(retry.hit.occurrence(), occurrence);
    let replay = slot
        .commit_native_quest_hit(
            &h.root,
            &authority,
            &h.node,
            fixture::fence(2)?,
            &retry.hit,
            Arc::clone(&retry.catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert!(queue.acknowledge(&retry));
    assert!(!queue.has_pending());
    assert!(!queue.acknowledge(&retry));
    assert_eq!(h.root_revision().await?, "2");
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='native_hit'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    Ok(())
}

#[test]
fn engenc_direct_rune_hit_requires_exact_spell_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engenc_direct_rune_hit", true).await?;
                    let result = exercise_direct_rune_hit(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_DIRECT_RUNE_HIT_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC direct rune PG worker panicked")?
        .map_err(Into::into)
}

async fn exercise_direct_rune_hit(h: &fixture::Harness) -> fixture::TestResult {
    use dispatcher_hit_queue::{HitQueue, Snapshot, direct_damage_target};
    use durability::character_revision_sequencer::CharacterRevisionSequencer;
    use foundation::runtime_actor_spell_types::{OwnerCombatChange, OwnerCombatEffect};
    use oteryn_gameplay_quests::quest::authored_bindings::{Trigger, hits::HitSource};
    let original = hit_catalogue()?;
    let mut bindings = original.authored_bindings().ok_or("bindings")?.clone();
    let Trigger::Hit { objective } = &mut bindings.quests[0].stages[0].trigger else {
        return Err("hit objective".into());
    };
    objective.source = HitSource::Rune {
        item: "oteryn:item.training_rune".into(),
        spell: "candidate:spell/rune/training".into(),
    };
    let base = QuestStateCatalogue::empty("content-1").ok_or("base")?;
    let catalogue = Arc::new(bindings.attach(&base)?);
    let fence = fixture::fence(1)?;
    let (mut runtime, actor, target, command) = native_hit_runtime(1)?;
    let lease = runtime
        .bound_attacker_lease(actor, fence.game_session_id)
        .map_err(fixture::debug)?;
    let effect = OwnerCombatEffect {
        target,
        sub_ordinal: 0,
        change: OwnerCombatChange::Damage {
            target_atom: "engenc:training".into(),
            magnitude: 5,
        },
    };
    assert_eq!(
        direct_damage_target(std::slice::from_ref(&effect)),
        Some((target, 0))
    );
    let other = runtime
        .admit_monster_lab_creature(
            foundation::MovementLocalPosition {
                x: 3,
                y: 1,
                floor: 7,
            },
            "engenc:other-rune-target",
            100,
        )
        .map_err(fixture::debug)?;
    let mut area = effect.clone();
    area.target = other;
    assert_eq!(direct_damage_target(&[effect.clone(), area]), None);
    let heal = OwnerCombatEffect {
        target,
        sub_ordinal: 0,
        change: OwnerCombatChange::Heal {
            target_atom: "engenc:training".into(),
            magnitude: 5,
        },
    };
    assert_eq!(direct_damage_target(&[heal]), None);
    let mut queue = HitQueue::default();
    assert!(queue.install(
        &runtime,
        fence.game_session_id,
        Snapshot::new(
            Arc::clone(&catalogue),
            actor,
            fence.game_session_id,
            lease,
            1,
            &Default::default()
        )
    ));
    {
        let mut capture = queue
            .prepare(
                &runtime,
                actor,
                target,
                command,
                0,
                "oteryn:creature.rat",
                "oteryn:item.training_rune",
                Some("candidate:spell/rune/training"),
            )
            .map_err(fixture::debug)?;
        let mut native = runtime.borrow_exact_actor_commit();
        native
            .commit_damage_for_bound_attacker(
                target,
                actor,
                command,
                0,
                foundation::OwnerDamageCommand {
                    target: b"engenc:training",
                    occurrence: &[],
                    binding: b"engenc:rune-hit",
                    damage: 5,
                },
            )
            .map_err(fixture::debug)?;
        let witness = native
            .committed_hit_for_bound_attacker(target, actor, command, 0)
            .map_err(fixture::debug)?
            .ok_or("rune damage")?;
        assert_eq!(
            capture.capture(
                witness,
                "oteryn:creature.rat",
                "oteryn:item.training_rune",
                None
            ),
            0
        );
        assert_eq!(
            capture.capture(
                witness,
                "oteryn:creature.rat",
                "oteryn:item.training_rune",
                Some("candidate:spell/rune/other")
            ),
            0
        );
        assert_eq!(
            capture.capture(
                witness,
                "oteryn:creature.rat",
                "oteryn:item.training_rune",
                Some("candidate:spell/rune/training")
            ),
            1
        );
    }
    let pending = queue.pending().remove(0);
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(fence.character_id).await;
    let committed = slot
        .commit_native_quest_hit(
            &h.root,
            &authority,
            &h.node,
            fence,
            &pending.hit,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    assert!(queue.acknowledge(&pending));
    assert!(!queue.has_pending());
    assert_eq!(h.root_revision().await?, "2");
    let phase: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(PHASE)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(phase, 5);
    Ok(())
}

#[test]
fn engenc_child_of_destiny_training_hits_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engenc_child_training", true).await?;
                    let result = exercise_child_training(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGENC_CHILD_TRAINING_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGENC child training worker panicked")?
        .map_err(Into::into)
}

async fn exercise_child_training(h: &fixture::Harness) -> fixture::TestResult {
    use durability::character_revision_sequencer::CharacterRevisionSequencer;
    use durability::quest_state::native_hits::NativeQuestHit;
    const CHILD: &str = "oteryn:quest.authored.child_of_destiny_quest";
    const CHILD_PHASE: &str = "oteryn:quest-progress/engenc/child/phase";
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/quests/authored-bindings/engenc-child-of-destiny.json"),
    )?;
    let mut bindings = Bindings::parse(&bytes)?;
    bindings.verify_embedded_recipe_evidence()?;
    // Focused fixture begins at the training phase; production binding starts at zero.
    bindings.quests[0]
        .tracks
        .iter_mut()
        .find(|track| track.key == CHILD_PHASE)
        .ok_or("child phase")?
        .initial = 2;
    let catalogue =
        Arc::new(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("base")?)?);
    let hash = catalogue.definition_hash(CHILD).ok_or("child definition")?;
    let fence = fixture::fence(1)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(fence.character_id).await;
    let (mut runtime, player, target, _) = native_hit_runtime(1)?;
    for (index, item, spell) in [
        (0_u64, "oteryn:item.tibia.i17110", None),
        (
            1_u64,
            "oteryn:item.tibia.i17512",
            Some("candidate:spell/rune/lightest_magic_missile_rune"),
        ),
    ] {
        let command = foundation::CommandRef::new(
            fence.game_session_id,
            foundation::CommandId::new(index + 1).map_err(fixture::debug)?,
        );
        let witness = {
            let mut owner = runtime.borrow_exact_actor_commit();
            owner
                .commit_damage_for_bound_attacker(
                    target,
                    player,
                    command,
                    0,
                    foundation::OwnerDamageCommand {
                        target: b"engenc:training",
                        occurrence: &[],
                        binding: b"engenc:child-training",
                        damage: 5,
                    },
                )
                .map_err(fixture::debug)?;
            owner
                .committed_hit_for_bound_attacker(target, player, command, 0)
                .map_err(fixture::debug)?
                .ok_or("successful native hit")?
        };
        let wrong_source = NativeQuestHit::from_committed(
            format!(
                "oteryn:quest-transition/native-hits/engenc-child-s{}",
                index + 3
            ),
            "oteryn:creature.party_skeleton".into(),
            "oteryn:item.tibia.i2382".into(),
            spell.map(str::to_owned),
            index + 1,
            hash,
            witness,
        );
        assert!(
            slot.commit_native_quest_hit(
                &h.root,
                &authority,
                &h.node,
                fence,
                &wrong_source,
                Arc::clone(&catalogue)
            )
            .await
            .is_err()
        );
        let hit = NativeQuestHit::from_committed(
            format!(
                "oteryn:quest-transition/native-hits/engenc-child-s{}",
                index + 3
            ),
            "oteryn:creature.party_skeleton".into(),
            item.into(),
            spell.map(str::to_owned),
            index + 1,
            hash,
            witness,
        );
        let committed = slot
            .commit_native_quest_hit(
                &h.root,
                &authority,
                &h.node,
                fence,
                &hit,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(fixture::debug)?;
        assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
        let replay = slot
            .commit_native_quest_hit(
                &h.root,
                &authority,
                &h.node,
                fence,
                &hit,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(fixture::debug)?;
        assert!(matches!(
            replay,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        let phase: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(CHILD_PHASE)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(phase, i64::try_from(index)? + 3);
    }
    assert_eq!(h.root_revision().await?, "3");
    let counts: Vec<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key IN ('oteryn:quest-progress/engenc/child/spear','oteryn:quest-progress/engenc/child/rune') ORDER BY track_key")
        .bind(CHILD).fetch_all(&h.pool).await?;
    assert_eq!(counts, [1, 1]);
    Ok(())
}
