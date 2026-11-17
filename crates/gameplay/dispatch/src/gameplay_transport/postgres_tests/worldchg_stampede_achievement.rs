//! Global Stampede kill credit through the dispatch owner's revision sequencer.
//! TibiaWiki Stampede Mini World Change/Spoiler oldid=887155: five kills;
//! no personal entry, premium entitlement, or Guide Tiko dialogue is required.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

const RACE: &str = "oteryn:creature.terrified_elephant";
const ACHIEVEMENT: &str = "oteryn:achievement/trail_of_the_ape_god";

#[test]
fn worldchg_stampede_five_kills_grant_trail_once_without_personal_programme() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::qengine_rewards::scenario("qengine-rewards-stampede-achievement")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    // The bestiary kill settles only for a session bound to its Character progression.
    owner.admit_progression_session(admitted).await;
    assert!(owner.premium_coordinator.is_none());
    let production = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    for key in [
        "oteryn:quest-world/m01c-stampede-entry",
        "oteryn:quest-transition/native-kills/m01c-stampede-elephant",
        "oteryn:quest-reward/m01c-stampede-collector",
    ] {
        assert!(
            production.catalogue().transition(key).is_none(),
            "retired {key}"
        );
    }
    // Temporary diagnostic: which reward row and Bestiary race key the settle would use.
    {
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native policies")?;
        match native.reward_table().row(RACE) {
            Ok(row) => eprintln!(
                "[DBG-QGB2] row ok race_key={:?} threshold={:?} xp={:?}",
                row.race.as_ref().map(|race| race.key().to_owned()),
                row.race.as_ref().map(|race| race.final_kill_threshold()),
                row.xp_amount
            ),
            Err(reason) => eprintln!("[DBG-QGB2] row refused reason={reason:?}"),
        }
        eprintln!(
            "[DBG-QGB2] progression_binding={}",
            owner.progression_binding(admitted.game_session_id).is_some()
        );
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT race_key,kill_count FROM game_character_bestiary_progress WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
        eprintln!("[DBG-QGB2] progress rows before={rows:?}");
    }
    let mut first_grant = None;
    for count in 1..=6_u8 {
        // Real native damage -> duplicate lethal projections -> two reward drains.
        // The helper reclaims the creature only after both drains finish.
        super::qengine_kill_stories::death(owner, admitted, RACE, 98_000 + u64::from(count))
            .await?;
        {
            let rows: Vec<(String, i64)> = sqlx::query_as(
                "SELECT race_key,kill_count FROM game_character_bestiary_progress")
                .fetch_all(&h.pool).await?;
            eprintln!("[DBG-QGB2] after death {count}: all progress rows={rows:?}");
        }
        let progress = owner
            .root
            .read_bestiary_progress(
                owner.character,
                crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                RACE,
            )
            .await
            .map_err(debug)?
            .ok_or("kill progress")?;
        assert_eq!(progress.kill_count, u32::from(count));
        let grants: Vec<(String, Vec<u8>, i64)> = sqlx::query_as(
            "SELECT source_kind,source_event_id,earned_at FROM game_account_achievement_grant_requests WHERE character_id=encode($1,'hex')::uuid AND achievement_key=$2")
            .bind(id(41).as_slice()).bind(ACHIEVEMENT).fetch_all(&h.pool).await?;
        let facts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE account_id=encode($1,'hex')::uuid AND achievement_key=$2")
            .bind(id(40).as_slice()).bind(ACHIEVEMENT).fetch_one(&h.pool).await?;
        assert_eq!(grants.len(), usize::from(count >= 5));
        assert_eq!(facts, i64::from(count >= 5));
        if count == 5 {
            let grant = grants.into_iter().next().ok_or("fifth kill grant")?;
            assert_eq!(grant.0, "oteryn:bestiary-kill");
            first_grant = Some(grant);
        } else if count == 6 {
            assert_eq!(
                grants.first(),
                first_grant.as_ref(),
                "sixth kill preserves original grant"
            );
        }
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_bestiary_kill_receipts WHERE character_id=encode($1,'hex')::uuid AND race_key=$2")
            .bind(id(41).as_slice()).bind(RACE).fetch_one(&h.pool).await?;
        assert_eq!(
            receipts,
            i64::from(count),
            "duplicate projections and drains credit each death once"
        );
    }
    let personal: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.stampede_mini_world_change'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(personal, 0);
    Ok(())
}
