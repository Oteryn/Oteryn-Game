//! Count unique defeats through the revision-sequenced Bestiary settlement.
use super::character_title_sources_fixtures::source_limits;
use super::fixture::{Harness, debug, fence, id};
use super::{TestResult, prepare_current_session};
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::fresh_admission::FreshAdmissionStore;

#[test]
fn deepling_achievements_grant_on_the_three_hundredth_distinct_defeat() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "deepling_achievements", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut fence = fence(1)?;
        let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(fence.game_session_id)
            .await?;
        fence.character_lease_generation = current.current_character_lease().generation();
        let state = h
            .root
            .read_character_progression(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("initialized progression")?;
        let races = crate::content::canonical_bestiary_rows(source_limits()).map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence.character_id).await;
        for (tag, race_key, achievement) in [
            (
                81,
                "oteryn:creature.deepling_guard",
                "oteryn:achievement/guardian_downfall",
            ),
            (
                82,
                "oteryn:creature.deepling_spellsinger",
                "oteryn:achievement/death_song",
            ),
            (
                83,
                "oteryn:creature.deepling_warrior",
                "oteryn:achievement/depth_dwellers",
            ),
        ] {
            let (race, _) = races
                .iter()
                .find(|(race, _)| race.key() == race_key)
                .ok_or("canonical Deepling Bestiary race")?;
            let request = |count: u32| -> TestResult<BestiaryKillRequest> {
                let mut occurrence = id(tag);
                occurrence[12..].copy_from_slice(&count.to_be_bytes());
                Ok(BestiaryKillRequest {
                    occurrence: BestiaryKillOccurrence::from_bytes(occurrence).map_err(debug)?,
                    race: race.clone(),
                    context: state.context.clone(),
                    policy_revision: state.policy_revision.clone(),
                    reward_revision: state.reward_revision.clone(),
                })
            };
            for count in 1..=299 {
                assert!(matches!(
                    slot.commit_bestiary(&h.root, &authority, &h.node, fence, request(count)?)
                        .await
                        .map_err(debug)?,
                    BestiaryKillOutcome::Committed(_)
                ));
            }
            assert!(matches!(
                slot.commit_bestiary(&h.root, &authority, &h.node, fence, request(299)?)
                    .await
                    .map_err(debug)?,
                BestiaryKillOutcome::AlreadyCommitted(_)
            ));
            assert_eq!(grant_event(&h, achievement).await?, None, "{achievement}");
            for count in [300, 300, 301] {
                slot.commit_bestiary(&h.root, &authority, &h.node, fence, request(count)?)
                    .await
                    .map_err(debug)?;
                assert_eq!(
                    grant_event(&h, achievement).await?.as_deref(),
                    Some(request(300)?.occurrence.as_bytes().as_slice()),
                    "{achievement} must retain the first threshold event"
                );
            }
        }
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn grant_event(h: &Harness, achievement: &str) -> TestResult<Option<Vec<u8>>> {
    let row = sqlx::query_scalar(
        "SELECT r.source_event_id FROM game_account_achievements f \
         JOIN game_account_achievement_grant_requests r USING (account_id, achievement_key) \
         WHERE f.achievement_key = $1 AND r.source_kind = 'oteryn:bestiary-kill'",
    )
    .bind(achievement)
    .fetch_optional(&h.pool)
    .await?;
    Ok(row)
}
