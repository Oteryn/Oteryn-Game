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
