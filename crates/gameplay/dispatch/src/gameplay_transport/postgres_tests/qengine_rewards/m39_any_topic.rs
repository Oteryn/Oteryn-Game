//! M39 exercises source offer routing against a real admitted PostgreSQL session.
use super::*;

#[test]
fn m39_any_topic_resumes_an_existing_native_npc_conversation() -> TestResult {
    scenario("qengine-rewards-m39-any-topic")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] =
        super::super::classic_quests::install(owner, admitted, ["oteryn:npc.muriel"]).await?;
    owner.states.npc_conversations.fixture_reward_offers(
        &serde_json::json!({
            "schema": 1,
            "offers": [
                {
                    "npc": "oteryn:npc.muriel", "keywords": ["m39seed"],
                    "topic": 0, "next_topic": 99, "storage": [],
                    "reply": ["The existing conversation is open."]
                },
                {
                    "npc": "oteryn:npc.muriel", "keywords": ["m39resume"],
                    "topic": 0, "any_topic": true, "next_topic": 100,
                    "storage": [], "reply": ["The offer resumed from topic 99."]
                }
            ]
        })
        .to_string(),
    )?;
    let before = counts(h).await?;
    for (words, expected_topic) in [("hi", 0), ("m39seed", 99), ("m39resume", 100)] {
        let command = admitted.continuity.next_command_id;
        admitted.continuity.next_command_id = command.checked_add(1).ok_or("command overflow")?;
        assert_eq!(
            talk(owner, admitted, npc, command, words).await?,
            NpcIntentDisposition::Ok,
            "native NPC command {words}"
        );
        assert_eq!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(admitted.game_session_id)?,
            expected_topic,
            "the production dialogue must retain and then resume the existing topic"
        );
    }
    assert_eq!(
        counts(h).await?,
        before,
        "an offer grants no rewards or progress"
    );
    // Two stages share a keyword. Only the current stage owns its inventory
    // refusal; the later stage must not make that reply ambiguous.
    let mut current = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key.ends_with("/l2-bone-return"))
        .ok_or("Bone Pit callback")?;
    current.keywords = vec!["m39bounded".into()];
    current.topic = 100;
    current.minimum_level = None;
    current.storage[0].minimum = -1;
    current.storage[0].maximum = 0;
    current.inventory[0].minimum = 2;
    current.inventory[0].maximum = 3;
    current.failure_reply = vec!["Bring two or three samples.".into()];
    let mut later = current.clone();
    later.storage[0].minimum = 2;
    later.storage[0].maximum = 2;
    later.failure_reply = vec!["This is the later stage.".into()];
    owner
        .states
        .npc_conversations
        .fixture_reward_callbacks(vec![current, later])?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: admitted.continuity.next_command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "m39bounded".into(),
            },
        )
        .await;
    assert!(!outcome.unknown);
    assert_eq!(outcome.disposition, NpcIntentDisposition::Rejected);
    assert_eq!(
        outcome
            .conversation
            .ok_or("inventory refusal dialogue")?
            .lines,
        vec!["Bring two or three samples."]
    );
    assert_eq!(
        owner
            .states
            .npc_conversations
            .fixture_quest_topic(admitted.game_session_id)?,
        100
    );
    assert_eq!(
        counts(h).await?,
        before,
        "refusal writes no rewards or progress"
    );
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M39_ANY_TOPIC_NATIVE_PG server_version={version}");
    Ok(())
}
