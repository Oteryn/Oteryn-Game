//! Supplemental quest-owner transaction proof; dispatch tests exercise lethal capture.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::character_progression::ExperienceRewardOccurrence;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome,
    native_kills::{NativeDeathSource, NativeQuestDeath},
    source_deaths::SourceDeathContext,
};
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, source_deaths};
use std::sync::Arc;

#[test]
fn overlordcr_quest_owner_rejects_non_damage_and_replays_all_four_flags_postgres()
-> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    for element in ["earth", "energy", "fire", "ice"] {
                        let h = fixture::Harness::create(admin.clone(), "overlordcr_owner", true)
                            .await?;
                        let result = exercise(&h, element).await;
                        h.cleanup().await?;
                        result?;
                    }
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "OVERLORDCR_OWNER_PG_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "overlord quest owner worker panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness, element: &str) -> fixture::TestResult {
    let catalogue = Arc::new(
        load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    );
    let creature = format!("oteryn:creature.{element}_overlord");
    let rule = source_deaths::rules()?
        .iter()
        .find(|rule| rule.creature == creature)
        .ok_or("overlord rule")?;
    let track = "oteryn:quest-progress/quest/u8_2/elemental_spheres/boss_storage";
    assert_eq!(rule.effects.len(), 1);
    assert_eq!(rule.effects[0].track, track);
    assert_eq!(rule.effects[0].set, Some(1));
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::fence(1)?;
    let mut death = NativeQuestDeath {
        transition: rule.transition.clone(),
        creature,
        observed_revision: 1,
        definition_hash: None,
        source: NativeDeathSource::Source(SourceDeathContext {
            scope: fence.runtime_scope,
            scope_generation: fence.scope_ownership_generation.get(),
            character: *fence.character_id.as_bytes(),
            session: fence.game_session_id,
            lease_generation: fence.character_lease_generation,
            position: [100, 100, -7],
            dealt_damage: false,
            top_damage_character: None,
        }),
    };
    let cause = QuestCause::CreatureDeath(
        ExperienceRewardOccurrence::from_bytes(fixture::id(100)).map_err(fixture::debug)?,
    );
    let refused = h
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
    assert!(matches!(refused, QuestTransitionOutcome::Refused(_)));
    assert_eq!(h.root_revision().await?, "1");
    if let NativeDeathSource::Source(source) = &mut death.source {
        source.dealt_damage = true;
    }
    let committed = h
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
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    let value: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(value, 1);
    let replay = h
        .root
        .commit_character_native_quest_kill(
            &authority,
            &h.node,
            fixture::fence(2)?,
            cause,
            death,
            catalogue,
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
    assert_eq!(receipts, 1);
    Ok(())
}
