//! Reuse the existing combat-death fixture; no constructed substitute death identity.
use super::*;
use crate::bestiary_postgres_harness as fixture;
use crate::durability::character_tutorial::TutorialProgressOutcome;
use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};

#[test]
fn tutorial_corruptor_claim_clears_once_replays_and_rejects_stale_authority() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async {
        let h = fixture::Harness::create(admin, "tutorial_corruptor", true).await?;
        let result = async {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let current = fixture::fence(1)?;
            let progress = TutorialProgress { step: TutorialStep::Island, hunting: true, corruptor: true };
            h.root.character_tutorial_progress(&authority, &h.node, current, Some((0, progress))).await.map_err(debug)?;
            let mut creature = death_fixture()?;
            creature.strike_by("tutorial:corruptor", CombatDeathFixture::HEALTH, crate::foundation::CharacterId::decode(current.character_id.as_bytes()).map_err(debug)?).map_err(debug)?;
            creature.project_death().map_err(debug)?;
            let actor = creature.actor();
            let death = capture(&mut creature, actor)?.death;
            let mut stale = current;
            stale.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
            assert!(h.root.claim_tutorial_corruptor(&authority, &h.node, stale, death, Vec::new()).await.is_err());
            // This Character has no vocation. It cannot select another vocation's Items.
            assert!(matches!(h.root.claim_tutorial_corruptor(&authority, &h.node, current, death, vec![3359,3285,3375]).await, Err(CharacterProgressionError::InvalidInput)));
            assert_eq!(h.count("game_tutorial_corruptor_claims").await?, 0);
            assert!(h.root.claim_tutorial_corruptor(&authority, &h.node, current, death, Vec::new()).await.map_err(debug)?.is_empty());
            assert!(h.root.claim_tutorial_corruptor(&authority, &h.node, current, death, Vec::new()).await.map_err(debug)?.is_empty());
            assert_eq!(h.count("game_tutorial_corruptor_claims").await?, 1);
            assert!(matches!(h.root.character_tutorial_progress(&authority, &h.node, current, None).await.map_err(debug)?, TutorialProgressOutcome::Read(Some(stored)) if stored.version == 2 && !stored.progress.corruptor && stored.progress.hunting));
            assert!(h.root.claim_tutorial_corruptor(&authority, &h.node, stale, death, Vec::new()).await.is_err());
            assert_eq!(h.root_revision().await?, "1");
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
