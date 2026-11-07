//! Zirella's source instruction sequence through the actual NPC command path.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use std::sync::Arc;

#[test]
fn questpc3_zirella_lessons_reach_ground_delivery_without_fabricating_it() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC3_ZIRELLA POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Zirella lessons panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let started = std::time::Instant::now();
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "questpc3_zirella_lessons", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("generation")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content_and_capacity(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            8,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let [zirella] =
            super::classic_quests::install(&owner, &admitted, ["oteryn:npc.zirella"]).await?;
        eprintln!(
            "QUESTPC3_ZIRELLA POSTGRES_ASSERTION_REACHED duration_ms={}",
            started.elapsed().as_millis()
        );
        super::classic_quests::talk(&mut owner, &admitted, zirella, 4100, "hi").await?;
        super::classic_quests::talk(&mut owner, &admitted, zirella, 4101, "quest").await?;
        for stage in 1..=6_u64 {
            let command = 4101 + stage;
            super::classic_quests::talk(&mut owner, &admitted, zirella, command, "yes").await?;
            for name in ["zirella_npc_greet_storage", "zirella_quest_log"] {
                let key = format!("oteryn:quest-progress/quest/u8_2/the_beginning_quest/{name}");
                assert_eq!(
                    super::classic_quests::progress(&owner, &admitted, &authority, &key).await?,
                    i64::try_from(stage)?
                );
            }
        }
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/beginning/zirella-%'").fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, 6,
            "instructions do not manufacture delivery or pay its reward"
        );
        let xp: i64 =
            sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(xp, 0);
        super::classic_quests::talk(&mut owner, &admitted, zirella, 4200, "bye").await?;
        super::classic_quests::talk(&mut owner, &admitted, zirella, 4201, "hi").await?;
        super::classic_quests::talk(&mut owner, &admitted, zirella, 4202, "explain").await?;
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/beginning/zirella-%'").fetch_one(&h.pool).await?;
        assert_eq!(after, receipts, "repeating instructions grants no progress");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    eprintln!(
        "QUESTPC3_ZIRELLA POSTGRES_CASE_COMPLETED duration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
