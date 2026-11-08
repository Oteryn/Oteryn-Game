//! M30 durable-owner PG seam. Native roster/projection is covered in entities/dispatch.
pub use oteryn_gameplay_admission_evidence as admission_evidence;
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
pub use oteryn_gameplay_domain::domain;
pub use oteryn_gameplay_durability::{character_bootstrap_intent, durability};
pub use oteryn_gameplay_entities as foundation;
pub use oteryn_gameplay_native_admission_source as native_admission_source;

// This canonical shared harness supplies more writers than this focused case needs.
#[allow(
    dead_code,
    reason = "canonical PG harness; this target uses the native quest owner"
)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod fixture;

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
        "../../quests/src/quest/authored_bindings/fixtures/kill.json"
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
