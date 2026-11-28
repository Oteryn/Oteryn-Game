use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::reward_callbacks::offered_for;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    use super::classic_quests_fixtures::{install, talk};

    // Captured with quest_definition_hashes on d481467205 and origin/main.
    const SAVED_HASH: &str = "eec799aef21078c75e2e7ffaabab6e71f8c59f9310d376854f9bbe9ab57ddab5";
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let report = offered_for(catalogue)?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/hunting-tasks/trolls/report")
        .ok_or("troll report")?;
    let [phase, count] = report.storage.as_slice() else {
        return Err("troll report must own the phase and completed count".into());
    };

    super::qc_pinned_world::seed_saved_progress(
        harness,
        owner,
        admitted,
        &report.quest,
        SAVED_HASH,
        &phase.track,
        phase.minimum,
    )
    .await?;
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        harness,
        owner,
        admitted,
        &[(count.track.clone(), count.minimum)],
        120,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let saved = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let pin = saved
        .states()
        .get(&report.quest)
        .ok_or("saved hunt state")?;
    let stored_hash: String = pin
        .definition_hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(stored_hash, SAVED_HASH);
    assert_eq!(saved.tracks().get(&phase.track), Some(&phase.minimum));

    let [daniel] = install(owner, admitted, [report.npc.as_str()]).await?;
    talk(owner, admitted, daniel, 121, "hi").await?;
    talk(owner, admitted, daniel, 122, "mission task").await?;
    let after_mission = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        after_mission.tracks().get(&phase.track),
        Some(&phase.minimum)
    );

    let replies = talk(owner, admitted, daniel, 123, &report.keywords[0]).await?;
    assert!(
        report.reply.iter().all(|line| replies.contains(line)),
        "{replies:?}"
    );
    let after_report = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after_report.tracks().get(&phase.track), Some(&phase.set));
    Ok(())
}

pub(super) async fn dialogue(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    name: &str,
) -> TestResult {
    use super::classic_quests_fixtures::{install, progress, talk};
    use super::qengine_outfit_stories_fixtures::{
        seed_quest_prerequisites, select_outfit_capabilities,
    };
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog;

    let source = NpcRuleCatalog::runtime().map_err(debug)?;
    let rule = |key: &str| {
        source
            .programs()
            .iter()
            .flat_map(|program| &program.document.rules)
            .find(|rule| rule.key.strip_prefix("oteryn:quest-npc/hunting-tasks/") == Some(key))
            .ok_or("regional hunt rule")
    };
    let (ask_key, hunt) = match name {
        "regional-hunt-dialogue-daniel" => ("rotworms/ask", "rotworms"),
        "regional-hunt-dialogue-lugri" => ("necromancers/ask", "necromancers"),
        _ => ("pirates/ask", "pirates"),
    };
    let ask = rule(ask_key)?;
    let phase = format!("oteryn:quest-progress/hunting-tasks/{hunt}/phase");
    let [npc] = install(owner, admitted, [ask.npc.as_str()]).await?;
    let reward = if name.starts_with("regional-hunt-dialogue-raymond") {
        select_outfit_capabilities(owner, admitted)?;
        let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
        let reward = offered_for(catalogue)?
            .into_iter()
            .find(|callback| callback.key == "oteryn:quest-reward/l4-pirate-base")
            .ok_or("Raymond outfit reward")?;
        let values = reward
            .requires
            .iter()
            .map(|r| (r.track.clone(), r.minimum))
            .collect::<Vec<_>>();
        seed_quest_prerequisites(harness, owner, admitted, &values, 120).await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        Some(reward)
    } else {
        None
    };
    talk(owner, admitted, npc, 121, "hi").await?;
    if name != "regional-hunt-dialogue-raymond-mixed" {
        assert_eq!(talk(owner, admitted, npc, 122, "task").await?, ask.reply);
    }
    if name == "regional-hunt-dialogue-daniel" {
        // At level 30 declining rotworms cannot quote or start the level-40 hunt.
        talk(owner, admitted, npc, 123, "no").await?;
        assert_ne!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(admitted.game_session_id)?,
            3
        );
        assert_eq!(
            progress(
                owner,
                admitted,
                owner.character,
                "oteryn:quest-progress/hunting-tasks/cyclops/phase"
            )
            .await?,
            0
        );
        assert_eq!(talk(owner, admitted, npc, 124, "task").await?, ask.reply);
        assert_eq!(
            talk(owner, admitted, npc, 125, "yes cyclops").await?,
            rule("rotworms/accept")?.reply
        );
        assert_eq!(progress(owner, admitted, owner.character, &phase).await?, 1);
    } else if let Some(reward) = reward {
        let keyword = &reward.keywords[0];
        let message = if name.ends_with("-mixed") {
            format!("{keyword} task")
        } else {
            keyword.clone()
        };
        talk(owner, admitted, npc, 123, &message).await?;
        let outfit = reward.storage.first().ok_or("outfit track")?;
        assert_eq!(
            progress(owner, admitted, owner.character, &outfit.track).await?,
            outfit.set
        );
        if !name.ends_with("-mixed") {
            talk(owner, admitted, npc, 124, "yes").await?;
            assert_eq!(progress(owner, admitted, owner.character, &phase).await?, 0);
        }
    } else {
        talk(owner, admitted, npc, 123, "outfit").await?;
        assert_eq!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(admitted.game_session_id)?,
            1
        );
        talk(owner, admitted, npc, 124, "yes").await?;
        assert_eq!(progress(owner, admitted, owner.character, &phase).await?, 0);
        assert_eq!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(admitted.game_session_id)?,
            2
        );
    }
    Ok(())
}
