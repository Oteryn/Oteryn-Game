//! Empty Crystal missions cannot be advertised or admitted as personal quests.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestRefusal, QuestStateCatalogue, QuestTransition, authored_bindings, reward_callbacks,
};

#[test]
fn qm55_donor_unavailable_missions_have_no_offer_or_durable_personal_progress() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm55_unavailable", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED donor_unavailable duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 unavailable worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(f::debug)?;
    let offers = reward_callbacks::offered_for(catalogue.catalogue())?;
    let retained = reward_callbacks::load_for(catalogue.catalogue())?;
    assert!(
        retained.iter().any(
            |callback| callback.quest == "oteryn:quest.authored.bank_robbery_mini_world_change"
        ),
        "unavailable offers must retain historical callback evidence"
    );
    assert!(offers.iter().all(authored_bindings::personal_event_offered));
    assert!(
        !offers.iter().any(
            |callback| callback.quest == "oteryn:quest.authored.bank_robbery_mini_world_change"
        )
    );
    assert!(!offers.iter().any(|callback| {
        oteryn_gameplay_quests::quest::shared_world_progress::accepted_task(&callback.key).is_some()
    }));
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    for quest in [
        "bank_robbery_mini_world_change",
        "chakoya_iceberg_mini_world_change",
        "devovorga_s_essence_mini_world_change",
        "down_the_drain_mini_world_change",
        "fire_from_the_earth_mini_world_change",
        "hive_born_world_change",
        "hive_outpost_mini_world_change",
        "jungle_camp_mini_world_change",
        "kingsday_mini_world_change",
        "lumberjack_mini_world_change",
    ] {
        let quest = format!("oteryn:quest.authored.{quest}");
        let authored_bindings::PersonalEventProfile::Unavailable { source, line } =
            authored_bindings::personal_event_profile(&quest).ok_or("missing donor profile")?
        else {
            return Err("expected typed unavailable profile".into());
        };
        assert!(source.contains("201189f871d23597b82caf88fd611454a2ddefb8"));
        assert!(line > 0);
        // The real durable caller must reject even a catalogue declaring a
        // completion; absence in the NPC offer adapter alone is insufficient.
        let key = format!(
            "oteryn:quest-transition/qm55/unavailable/{}",
            quest.trim_start_matches("oteryn:quest.authored.")
        );
        let c = std::sync::Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![],
                vec![QuestTransition {
                    key: key.clone(),
                    quest,
                    effects: vec![],
                    completes: true,
                    experience: None,
                    world_change: None,
                }],
            )
            .map_err(f::debug)?,
        );
        let result = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(1)?,
                QuestTransitionRequest {
                    transition_key: key,
                    cause: QuestCause::Command(CommandRef::new(
                        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
                        CommandId::new(1).map_err(f::debug)?,
                    )),
                },
                c,
            )
            .await?;
        assert!(
            matches!(
                result,
                QuestTransitionOutcome::Refused(QuestRefusal::NotSupported)
            ),
            "{result:?}"
        );
        assert_eq!(h.root_revision().await?, "1");
    }
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 0);
    Ok(())
}
