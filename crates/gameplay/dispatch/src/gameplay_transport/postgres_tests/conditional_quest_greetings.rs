//! Conditional donor greetings cannot infer an initial quest stage from a failed copy load.
//! Crystal 201189f: a_sleeping_dragon.lua:49-55; jack_springer.lua:61-70.
//! Exercise the actual NPC command route, including its ordinary greeting fallback.
use super::classic_quests::{install, talk};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

const DRAGON: &str = "oteryn:npc.a_sleeping_dragon";
const JACK: &str = "oteryn:npc.jack_springer";

#[test]
fn conditional_quest_greetings_missing_copy_refuse_in_real_npc_path() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CONDITIONAL_QUEST_GREETINGS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("conditional-quest-greetings", || {
        super::quest_world_callbacks::scenario(
            "conditional-quest-greetings",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    _authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [dragon, jack] = install(owner, admitted, [DRAGON, JACK]).await?;
    // Missing source entries, an unplaced NPC, or a broken talk service are fixture
    // failures, not the behavioral RED that this regression intends to observe.
    let dragon_source =
        oteryn_gameplay_npc::quest_greetings::get(DRAGON).ok_or("source Dragon greeting")?;
    assert_ne!(
        dragon_source.greet.text,
        dragon_source
            .variants
            .get("dream")
            .ok_or("source dream greeting")?
            .text,
    );
    let jack_initial = oteryn_gameplay_quests::quest::grave_danger::greeting(0)
        .ok_or("source Jack initial greeting")?;
    let jack_started = oteryn_gameplay_quests::quest::grave_danger::greeting(1)
        .ok_or("source Jack started greeting")?;
    assert_ne!(jack_initial, jack_started);
    for (offset, (key, npc)) in [(DRAGON, dragon), (JACK, jack)].into_iter().enumerate() {
        let command_id = 2600 + u64::try_from(offset)? * 2;
        let source =
            oteryn_gameplay_npc::quest_greetings::get(key).ok_or("source default greeting")?;
        let lines = talk(owner, admitted, npc, command_id, "hi").await?;
        assert_eq!(
            lines,
            vec![oteryn_gameplay_npc::quest_greetings::render_player_name(
                &source.greet.text,
                "Quest witness",
            )],
            "the admitted fixture has the source default before its copy is lost",
        );
        talk(owner, admitted, npc, command_id + 1, "bye").await?;
    }
    let receipts_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    let saved_copy = {
        let mut sessions = owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest mutex")?;
        let session = sessions
            .get_mut(&admitted.game_session_id)
            .ok_or("quest session")?;
        session
            .copy
            .take()
            .ok_or("the fixture admitted a real quest copy")?
    };
    let mut observed = Vec::new();
    for (offset, (key, npc)) in [(DRAGON, dragon), (JACK, jack)].into_iter().enumerate() {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let outcome = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 2610 + u64::try_from(offset)?,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "hi".into(),
                },
            )
            .await;
        observed.push((
            key,
            outcome.disposition,
            outcome.unknown,
            outcome.inventory_changed,
            outcome.conversation.map(|conversation| conversation.lines),
        ));
    }
    assert!(
        owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest mutex")?
            .get(&admitted.game_session_id)
            .ok_or("quest session")?
            .copy
            .is_none()
    );
    {
        let mut sessions = owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest mutex")?;
        sessions
            .get_mut(&admitted.game_session_id)
            .ok_or("quest session")?
            .copy = Some(saved_copy);
    }
    // An ordinary greeting may have opened server-side focus before the private
    // adapter discovered the unavailable copy. Refusal must unwind that unseen
    // opening; the next real hi after recovery must not be an empty repeated hi.
    let recovered = talk(owner, admitted, jack, 2620, "hi").await?;
    let receipts_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        receipts_after, receipts_before,
        "a failed read earns no quest mutation"
    );
    assert!(
        observed
            .iter()
            .all(|(_, disposition, unknown, inventory_changed, lines)| {
                *disposition == NpcIntentDisposition::Rejected
                    && !*unknown
                    && !*inventory_changed
                    && lines.is_none()
            }),
        "conditional greeting must refuse an unavailable quest copy without ordinary fallback: {observed:?}",
    );
    assert_eq!(
        recovered,
        jack_initial
            .iter()
            .map(|line| {
                oteryn_gameplay_npc::quest_greetings::render_player_name(line, "Quest witness")
            })
            .collect::<Vec<_>>(),
        "a refused conditional greeting must not leave unseen conversation focus after recovery",
    );
    Ok(())
}
