//! Tutorial greetings must commit through the real NPC dispatcher.
use super::classic_quests::{install, progress, talk};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

const SANTIAGO: &str = "oteryn:npc.santiago";
const LOG: &str = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_quest_log";

#[test]
fn quest_l4_beginning_santiago_greeting_commits_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUEST_L4_BEGINNING_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("quest-l4-beginning", || {
        super::quest_world_callbacks::scenario(
            "quest-l4-beginning",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUEST_L4_BEGINNING POSTGRES_CASE_COMPLETED duration_ms={} success={}",
        started.elapsed().as_millis(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [santiago] = install(owner, admitted, [SANTIAGO]).await?;
    talk(owner, admitted, santiago, 9300, "hi").await?;
    let value = progress(owner, admitted, authority, LOG).await?;
    eprintln!("QUEST_L4_BEGINNING POSTGRES_ASSERTION_REACHED greeting_value={value}");
    assert_eq!(value, 1, "source greeting starts Santiago's log durably");
    talk(owner, admitted, santiago, 9301, "bye").await?;
    talk(owner, admitted, santiago, 9302, "hi").await?;
    assert_eq!(progress(owner, admitted, authority, LOG).await?, 1);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key = 'oteryn:quest-reward/questpc3/beginning/greet'")
        .fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    Ok(())
}
