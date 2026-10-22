//! Genuine death receipts reuse the last-position owner's policy and PG fixture.
use super::*;
use crate::durability::character_progression::ProgressionInitializationRequest;
use crate::durability::character_tutorial::TutorialProgressOutcome;
use crate::durability::npc_travel::{PendingArrival, TravelOccurrence};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_content::content::newhaven_travel::NewhavenRoute;
use oteryn_gameplay_quests::tutorial::{ISLET_TEMPLE, TutorialProgress, TutorialStep};

#[test]
fn tutorial_death_returns_citizen_to_islet_and_cancels_a_pending_departure() -> TestResult {
    let admin = configured_admin().ok_or("tutorial death requires routed PostgreSQL")?;
    runtime()?.block_on(async {
        let h = Harness::create(admin, "tutorial_death", false).await?;
        let result = async {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut request = death(92)?;
            h.root.initialize_character_progression(&authority, &h.node, fence(1)?, ProgressionInitializationRequest {
                context: request.context.clone(), policy_revision: request.policy_revision.clone(),
                reward_revision: request.reward_revision.clone(), policy: request.policy.clone(),
            }).await.map_err(debug)?;
            h.root.initialize_familiar_group(&authority, &h.node, fence(1)?).await.map_err(debug)?;
            let progress = TutorialProgress { step: TutorialStep::Island, hunting: true, corruptor: true };
            h.root.character_tutorial_progress(&authority, &h.node, fence(1)?, Some((0, progress))).await.map_err(debug)?;
            let route = NewhavenRoute::Seagull;
            let (x, y, floor) = route.destination();
            let arrival = PendingArrival { occurrence: TravelOccurrence::from_bytes(id(93)).map_err(debug)?, world_id: fence(1)?.runtime_scope.world_id(), destination: LastPositionCell { x, y, floor } };
            h.root.commit_newhaven_travel(&authority, &h.node, fence(1)?, CommandRef::new(fence(1)?.game_session_id, CommandId::new(1).map_err(debug)?), arrival, route).await.map_err(debug)?;
            request.respawn_position = [ISLET_TEMPLE.0.to_be_bytes().as_slice(), ISLET_TEMPLE.1.to_be_bytes().as_slice(), ISLET_TEMPLE.2.to_be_bytes().as_slice()].concat();
            assert!(matches!(h.root.commit_character_death(&authority, &h.node, fence(1)?, request.clone()).await.map_err(debug)?, CharacterDeathOutcome::Committed(_)));
            assert_eq!(h.count("game_character_pending_arrivals").await?, 0);
            assert_eq!(h.count("game_character_pending_respawns").await?, 1);
            assert!(matches!(h.root.character_tutorial_progress(&authority, &h.node, fence(2)?, None).await.map_err(debug)?, TutorialProgressOutcome::Read(Some(stored)) if stored.version == 2 && stored.progress.step == TutorialStep::Islet && stored.progress.hunting && stored.progress.corruptor));
            assert!(matches!(h.root.commit_character_death(&authority, &h.node, fence(1)?, request).await.map_err(debug)?, CharacterDeathOutcome::AlreadyCommitted(_)));
            assert_eq!(h.root_revision().await?, "2");
            let mut placed = FinalLastPosition { record: record(ISLET_TEMPLE.0), write_sequence: 50 };
            placed.record.cell = LastPositionCell { x: ISLET_TEMPLE.0, y: ISLET_TEMPLE.1, floor: ISLET_TEMPLE.2 };
            assert!(h.root.consume_pending_respawn_placed(&authority, &h.node, fence(2)?, None, placed).await.map_err(debug)?.is_some());
            assert_eq!(h.count("game_character_pending_respawns").await?, 0);
            assert_eq!(h.root.read_character_last_position(&authority, &h.node, fence(2)?).await.map_err(debug)?.ok_or("missing respawn position")?.record.cell, placed.record.cell);
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
