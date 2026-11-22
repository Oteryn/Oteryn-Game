use super::classic_quests_fixtures::{install, talk};
use super::fixture::Harness;
use super::quest_world_callbacks_fixtures::{input_item, npc_scenario};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::npc_command::reward_callbacks::dice;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use std::sync::Arc;

const ORIGIN: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const TRACK: &str = "oteryn:quest-progress/kissing-a-pig/zebron-wins";
const NPC: &str = "oteryn:npc.zebron";
const SOURCE: &str = "https://tibia.fandom.com/wiki/Kissing_a_Pig_Quest/Spoiler";

#[test]
fn zebron_dice_losses_charge_and_reset_receipts_then_three_wins_earn_the_lesson() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("zebron-dice-receipts", || {
        npc_scenario(ORIGIN, |owner, admitted, harness| {
            Box::pin(exercise(owner, admitted, harness))
        })
    })
}

fn round(name: &str, minimum: i64, maximum: i64, next: i64) -> serde_json::Value {
    let win = name != "loss";
    let quest = "oteryn:quest.authored.kissing_a_pig_quest";
    serde_json::json!({
        "key": format!("oteryn:quest-reward/kissing-a-pig/zebron-{name}"),
        "quest": quest, "npc": NPC, "keywords": ["yes"], "topic": 6301,
        "next_topic": if next == 3 { 0 } else { 6301 },
        "storage": [{"track": TRACK, "minimum": minimum, "maximum": maximum, "set": next}],
        "take": [], "fee": (!win).then_some(serde_json::json!({"gold": 25, "include_bank": false})),
        "dice_outcome": if win { "win" } else { "loss" },
        "reply": [if win { "You win with |PLAYER_ROLL| over |NPC_ROLL|." } else { "You lose with |PLAYER_ROLL| against |NPC_ROLL|." }],
        "sources": [SOURCE]
    })
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let recipe_sha256: String = <sha2::Sha256 as sha2::Digest>::digest(SOURCE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let bindings = Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema": 1, "quests": [{
            "quest": "oteryn:quest.authored.kissing_a_pig_quest",
            "recipe_sha256": recipe_sha256,
            "sources": [SOURCE],
            "tracks": [{"key": TRACK, "initial": 0, "minimum": 0, "maximum": 3}],
            "stages": [{"key": "zebron-dice", "completes": false, "trigger": {
                "kind": "npc_dialogue", "callback": round("win-one", 0, 0, 1),
                "alternatives": [round("win-two", 1, 1, 2), round("win-three", 2, 2, 3), round("loss", 0, 2, 0)]
            }}]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("base catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    let [npc] = install(owner, admitted, [NPC]).await?;
    owner.states.npc_conversations.fixture_reward_offers(&serde_json::json!({
        "schema": 1, "offers": [{
            "npc": NPC, "keywords": ["worthy"], "topic": 0, "next_topic": 6301, "storage": [],
            "reply": ["Your bet is 25 gold a game, beat me 3 times in a row and I will tell you all my little secrets."]
        }]
    }).to_string())?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: ORIGIN,
            item: "oteryn:item.tibia.i3031".into(),
            count: 25,
        },
        77,
        true,
    )
    .await?;
    talk(owner, admitted, npc, 6299, "hi").await?;
    talk(owner, admitted, npc, 6300, "worthy").await?;

    let mut id = 6301;
    for (desired, streak) in [
        (std::cmp::Ordering::Greater, 1),
        (std::cmp::Ordering::Equal, 1),
        (std::cmp::Ordering::Less, 0),
        (std::cmp::Ordering::Greater, 1),
        (std::cmp::Ordering::Greater, 2),
        (std::cmp::Ordering::Greater, 3),
    ] {
        let (command, (player, opponent)) = (id..id + 512)
            .find_map(|command| {
                let roll = dice::roll(session, command)?;
                (roll.0.cmp(&roll.1) == desired).then_some((command, roll))
            })
            .ok_or("source roll")?;
        id = command + 1;
        if desired == std::cmp::Ordering::Equal {
            let result =
                super::qengine_rewards_fixtures::talk(owner, admitted, npc, command, "yes").await?;
            assert_eq!(
                result,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
            );
        } else {
            let lines = talk(owner, admitted, npc, command, "yes").await?;
            assert_eq!(
                lines,
                [if player > opponent {
                    format!("You win with {player} over {opponent}.")
                } else {
                    format!("You lose with {player} against {opponent}.")
                }]
            );
            if desired == std::cmp::Ordering::Less {
                assert_eq!(talk(owner, admitted, npc, command, "yes").await?, lines);
            }
        }
        let value: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(TRACK)
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(value, streak);
    }
    let fees: (i64, i64) = sqlx::query_as(
        "SELECT count(*), coalesce(sum(fee_gold_units),0)::bigint FROM game_quest_reward_fees",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(fees, (1, 25));
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/kissing-a-pig/zebron-%'")
        .fetch_one(&harness.pool).await?;
    assert_eq!(receipts, 5);
    assert_eq!(
        super::questpc7_outfit_items_fixtures::quantity(harness, 3031).await?,
        0
    );
    Ok(())
}
