//! TibiaWiki Targuna Quest/Spoiler (curid 110238, oldid 1208547).
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[test]
fn sterling_opens_wiki_turn_in_and_awards_experience_once() -> TestResult {
    super::m16_personal_map_marker::m16_sterling_commits_personal_marker_and_delivers_delta_and_reconnect_snapshot()
}

pub(super) async fn open_wiki_turn_in(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
) -> TestResult<bool> {
    // The subsequent framed conversation still exercises the donor's mission keyword.
    let mut accepted = true;
    for word in ["hi", "quest"] {
        let command = admitted.continuity.next_command_id;
        admitted.continuity.next_command_id = command.checked_add(1).ok_or("command overflow")?;
        accepted &= super::qengine_rewards::talk(owner, admitted, npc, command, word).await?
            == oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok;
    }
    Ok(accepted
        && owner
            .states
            .npc_conversations
            .fixture_quest_topic(admitted.game_session_id)?
            == 2)
}

pub(super) async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
        .fetch_one(&h.pool)
        .await?)
}

#[test]
fn hidden_lizard_temple_forcefield_returns_to_targuna() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    use oteryn_gameplay_quests::quest::world_callbacks::Position;
    super::kill_reward_e2e::scenario_thread::run("qcx73-temple-return", || {
        super::quest_world_callbacks::stateless_scenario(
            "oteryn:quest-callback/targuna-temple-return",
            Position {
                x: 31941,
                y: 31928,
                floor: -9,
            },
            Position {
                x: 31935,
                y: 31861,
                floor: -7,
            },
        )
    })
}
