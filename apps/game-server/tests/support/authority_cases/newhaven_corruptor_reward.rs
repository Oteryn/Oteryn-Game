//! Real tutorial claim owner: unavailable reward retry, first Knight claim and restart replay.
use crate::bestiary_postgres_harness as fixture;
use crate::combat_death_reward_postgres_cases::{capture, death_fixture};
use crate::durability::character_tutorial::TutorialProgressOutcome;
use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};

#[test]
fn newhaven_corruptor_knight_claim_replays_after_reopen_and_rejects_stale_session()
-> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else { return Ok(()); };
    std::thread::Builder::new().name("newhaven-corruptor".into()).stack_size(64 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join().map_err(|_| "Newhaven Corruptor scenario thread failed")?
        .map_err(Into::into)
}

fn scenario(admin: String) -> fixture::TestResult {
    fixture::runtime()?.block_on(async {
        eprintln!("NEW HAVEN: real durable Corruptor reward claim/reopen/session fence");
        let h = fixture::Harness::create(admin, "newhaven_corruptor_knight", true).await?;
        let result = async {
            let seal = h.recovery.seal_current().map_err(fixture::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(fixture::debug)?;
            let current = fixture::fence(1)?;
            let progress = TutorialProgress { step: TutorialStep::Island, hunting: true, corruptor: true };
            h.root.character_tutorial_progress(&authority, &h.node, current, Some((0, progress))).await.map_err(fixture::debug)?;
            let mut creature = death_fixture()?;
            creature.strike_by("newhaven:corruptor", crate::foundation::CombatDeathFixture::HEALTH,
                crate::foundation::CharacterId::decode(current.character_id.as_bytes()).map_err(fixture::debug)?)
                .map_err(fixture::debug)?;
            creature.project_death().map_err(fixture::debug)?;
            let actor = creature.actor();
            let death = capture(&mut creature, actor)?.death;
            assert!(h.root.claim_tutorial_corruptor(&authority, &h.node, current, death, Vec::new()).await.map_err(fixture::debug)?.is_empty());
            assert_eq!(h.count("game_tutorial_corruptor_claims").await?, 0);
            assert!(matches!(h.root.character_tutorial_progress(&authority, &h.node, current, None).await.map_err(fixture::debug)?,
                TutorialProgressOutcome::Read(Some(stored)) if stored.version == 1 && stored.progress.corruptor));
            // Seed the source-supported Knight build in the existing fixture database.
            sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'knight',0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid)")
                .bind(current.character_id.as_bytes().as_slice()).bind(fixture::id(110).as_slice()).execute(&h.pool).await?;
            let reward = vec![3359, 3285, 3375];
            assert_eq!(h.root.claim_tutorial_corruptor(&authority, &h.node, current, death, reward.clone()).await.map_err(fixture::debug)?, reward);
            assert_eq!(h.count("game_tutorial_corruptor_claims").await?, 1);
            assert!(matches!(h.root.character_tutorial_progress(&authority, &h.node, current, None).await.map_err(fixture::debug)?,
                TutorialProgressOutcome::Read(Some(stored)) if stored.version == 2 && !stored.progress.corruptor));
            let reopened = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(reopened.maintain_ready_once().await?);
            let reopened_authority = reopened.open_character_authority(&seal).await.map_err(fixture::debug)?;
            assert_eq!(reopened.claim_tutorial_corruptor(&reopened_authority, &h.node, current, death, reward.clone()).await.map_err(fixture::debug)?, reward);
            let mut stale = current;
            stale.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(fixture::debug)?;
            assert!(reopened.claim_tutorial_corruptor(&reopened_authority, &h.node, stale, death, reward).await.is_err());
            assert_eq!(h.count("game_tutorial_corruptor_claims").await?, 1);
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
