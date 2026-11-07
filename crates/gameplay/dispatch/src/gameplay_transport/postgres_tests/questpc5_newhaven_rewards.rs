//! Shared level-gated item/XP callbacks; these seeded component preconditions do not qualify the route.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use std::collections::BTreeMap;
use std::sync::Arc;

#[test]
fn questpc5_newhaven_reward_level_item_xp_and_completion_are_atomic() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-newhaven-xp")
}

pub(super) async fn snapshot(h: &Harness) -> TestResult<(i64, i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i52732'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3572'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i51977'),(SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.newhaven_quest' AND completed_character_revision IS NOT NULL)")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

async fn seed_ready(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    guardian: &RewardCallback,
) -> TestResult {
    let original = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let callbacks: Vec<_> = reward_callbacks::load()?
        .into_iter()
        .filter(|c| c.quest == guardian.quest)
        .collect();
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    for callback in callbacks {
        for s in callback.storage {
            tracks.insert(
                s.track.clone(),
                original.track(&s.track).cloned().ok_or("track")?,
            );
        }
        transitions.push(
            original
                .transition(&callback.key)
                .cloned()
                .ok_or("transition")?,
        );
    }
    let setup = "oteryn:quest-transition/fixture-newhaven-reward-ready";
    transitions.push(QuestTransition {
        key: setup.into(),
        quest: guardian.quest.clone(),
        effects: guardian
            .storage
            .iter()
            .map(|s| QuestEffect {
                track: s.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(s.minimum),
            })
            .collect(),
        completes: false,
        experience: None,
    });
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            original.content_revision(),
            tracks.into_values().collect(),
            transitions,
        )
        .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: setup.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9000).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let guardian = reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/newhaven-guardian")
        .ok_or("guardian callback")?;
    seed_ready(owner, admitted, &guardian).await?;
    let [npc] = super::classic_quests::install(owner, admitted, [guardian.npc.as_str()]).await?;
    let say = super::qengine_rewards::talk;
    let mut command = admitted.continuity.next_command_id;
    assert_eq!(
        say(owner, admitted, npc, command, "hi").await?,
        NpcIntentDisposition::Ok
    );
    command += 1;
    let before = snapshot(h).await?;
    let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        level, 1,
        "real initial character tests the minimum level gate"
    );
    assert_eq!(
        say(owner, admitted, npc, command, "report").await?,
        NpcIntentDisposition::Rejected
    );
    command += 1;
    assert_eq!(
        snapshot(h).await?,
        before,
        "level refusal has no item, XP or completion"
    );
    // Raise the level through the existing durable progression owner, not SQL.
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression binding")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    h.root
        .commit_character_experience(
            authority,
            &h.node,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(100),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        say(owner, admitted, npc, command, "report").await?,
        NpcIntentDisposition::Ok
    );
    let replay = command;
    command += 1;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(snapshot(h).await?, (before.0 + 900, 1, 0, 0, 0));
    let after = snapshot(h).await?;
    let _ = say(owner, admitted, npc, replay, "report").await?;
    assert_eq!(
        snapshot(h).await?,
        after,
        "same command never doubles item or XP"
    );
    let _ = say(owner, admitted, npc, command, "report").await?;
    command += 1;
    assert_eq!(
        snapshot(h).await?,
        after,
        "fresh repeat never re-earns the milestone"
    );
    assert_eq!(
        say(owner, admitted, npc, command, "book").await?,
        NpcIntentDisposition::Rejected
    );
    command += 1;
    assert_eq!(
        snapshot(h).await?,
        after,
        "missing Book cannot grant scarf, XP or completion"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i51977".into(),
            count: 1,
        },
        250,
        true,
    )
    .await?;
    assert_eq!(snapshot(h).await?, (before.0 + 900, 1, 0, 1, 0));
    assert_eq!(
        say(owner, admitted, npc, command, "book").await?,
        NpcIntentDisposition::Ok
    );
    command += 1;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(snapshot(h).await?, (before.0 + 1700, 1, 1, 0, 1));
    let final_state = snapshot(h).await?;
    let _ = say(owner, admitted, npc, command, "book").await?;
    assert_eq!(
        snapshot(h).await?,
        final_state,
        "completed quest never repeats value"
    );
    Ok(())
}
