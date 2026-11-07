//! Source offer, exact confirmation, insufficient held money, and one atomic paid fur reward.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, TestResult, prepare_current_session};
use super::barriers::{progress, quantity};
use super::maps::{append_fixture_input, say};
use super::rakes::seed;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use std::collections::BTreeMap;
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const ANIMALS: &str = oteryn_gameplay_quests::quest::threatened_dreams::ANIMALS;
const NOTES: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01/poacher_notes";
const CALLBACK: &str = "oteryn:quest-reward/questpc3/threatened-irmana/fur";
const SEED: &str = "oteryn:quest-transition/fixture/threatened-irmana-prior-story";
const READY: &str = "oteryn:quest-transition/fixture/threatened-irmana-notes";
#[test]
fn threatened_irmana_exact_paid_fur_offer_is_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_IRMANA POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){
 Ok(result)=>{eprintln!("THREATENED_IRMANA POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},
 Err(panic)=>{eprintln!("THREATENED_IRMANA POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}
 }} )?.join().map_err(|_|"irmana scenario panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "threatened_irmana", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, pin) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
                "content-1",
            )?;
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
        let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let mut tracks = BTreeMap::new();
        tracks.insert(
            ANIMALS.to_owned(),
            source
                .catalogue()
                .track(ANIMALS)
                .ok_or("animals track")?
                .clone(),
        );
        tracks.insert(
            NOTES.to_owned(),
            source
                .catalogue()
                .track(NOTES)
                .cloned()
                .unwrap_or(QuestTrack {
                    key: NOTES.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 1,
                }),
        );
        let mut transitions = Vec::new();
        if let Some(t) = source.catalogue().transition(CALLBACK) {
            transitions.push(t.clone());
        }
        for (key, notes) in [(SEED, 0), (READY, 1)] {
            transitions.push(QuestTransition {
world_change: None,
                key: key.into(),
                quest: QUEST.into(),
                effects: [(ANIMALS, 7), (NOTES, notes)]
                    .into_iter()
                    .map(|(track, value)| QuestEffect {
                        track: track.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(value),
                    })
                    .collect(),
                completes: false,
                experience: None,
            });
        }
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
                .map_err(debug)?,
        ));
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        // Only prior money and mission are fixtures. No output, payment or successful receipt is seeded.
        super::super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i3035", 9),
                ("oteryn:item.tibia.i3031", 99),
            ],
        )
        .await?;
        seed(&owner, &admitted, SEED, 33000).await?;
        let [irmana] =
            super::super::classic_quests::install(&owner, &admitted, ["oteryn:npc.irmana"]).await?;
        say(&mut owner, &admitted, irmana, 33001, "hi").await?;
        say(&mut owner, &admitted, irmana, 33002, "fur").await?;
        say(&mut owner, &admitted, irmana, 33003, "yes").await?;
        assert_eq!(
            quantity(&h, "oteryn:item.tibia.i25238").await?,
            0,
            "notes prerequisite cannot be fabricated by dialogue"
        );
        assert_eq!(coin_worth(&h).await?, 999);
        seed(&owner, &admitted, READY, 33004).await?;
        say(&mut owner, &admitted, irmana, 33005, "hi").await?;
        say(&mut owner, &admitted, irmana, 33006, "fur").await?;
        say(&mut owner, &admitted, irmana, 33007, "yes").await?;
        assert_eq!(
            quantity(&h, "oteryn:item.tibia.i25238").await?,
            0,
            "999 held gold cannot pay the exact1000 fee"
        );
        assert_eq!(coin_worth(&h).await?, 999);
        assert_eq!(progress(&owner, &admitted, &authority, ANIMALS).await?, 7);
        append_fixture_input(&h, "oteryn:item.tibia.i3031", 90, 7).await?;
        say(&mut owner, &admitted, irmana, 33008, "hi").await?;
        say(&mut owner, &admitted, irmana, 33009, "fur").await?;
        say(&mut owner, &admitted, irmana, 33010, "no").await?;
        assert_eq!(
            coin_worth(&h).await?,
            1000,
            "nonmatching reply cannot charge"
        );
        assert_eq!(quantity(&h, "oteryn:item.tibia.i25238").await?, 0);
        say(&mut owner, &admitted, irmana, 33011, "hi").await?;
        say(&mut owner, &admitted, irmana, 33012, "fur").await?;
        say(&mut owner, &admitted, irmana, 33013, "yes please").await?;
        assert_eq!(
            coin_worth(&h).await?,
            1000,
            "only exact confirmation can charge"
        );
        say(&mut owner, &admitted, irmana, 33014, "yes").await?;
        let fur = quantity(&h, "oteryn:item.tibia.i25238").await?;
        let money = coin_worth(&h).await?;
        let stage = progress(&owner, &admitted, &authority, ANIMALS).await?;
        let fees: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=18")
                .fetch_one(&h.pool)
                .await?;
        say(&mut owner, &admitted, irmana, 33014, "yes").await?;
        say(&mut owner, &admitted, irmana, 33015, "yes").await?;
        assert_eq!(
            quantity(&h, "oteryn:item.tibia.i25238").await?,
            fur,
            "exact replay/fresh repeat cannot duplicate the fur"
        );
        assert_eq!(coin_worth(&h).await?, money);
        eprintln!(
            "THREATENED_IRMANA POSTGRES_EVIDENCE fur={fur} held_gold={money} stage={stage} fees={fees}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            (fur, money, stage, fees),
            (1, 0, 8, 1),
            "one exact1000 fee and fur must share the authoritative quest transaction"
        );
        Ok(())
    })
}
async fn coin_worth(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity*CASE definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances WHERE lifecycle=1").fetch_one(&h.pool).await?)
}
