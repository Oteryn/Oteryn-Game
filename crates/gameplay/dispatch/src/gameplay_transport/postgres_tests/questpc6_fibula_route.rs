//! Source Fibula level gate using native XP and framed USE, without seeded action attributes.
use super::fixture::{debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

#[test]
fn questpc6_fibula_source_expertise_level_gate_refusal_and_replay() -> TestResult {
    super::m06_keys::source_key_door(32212, 32435, -10, 1050)
}

pub(super) async fn level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    level: u32,
) -> TestResult {
    let session = admitted.game_session_id;
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(session)
        .ok_or("task-level native policy")?;
    let target = binding
        .policy
        .thresholds
        .iter()
        .find(|row| row.level == level)
        .ok_or("task-level native threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "task-level fence")?
        .ok_or("task-level fence")?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("task-level progression")?;
    let amount = target
        .checked_sub(before.total_experience.get())
        .ok_or("task-level XP delta")?;
    if amount > 0 {
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(u8::try_from(level)?))
                        .map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(amount),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(|error| format!("Fibula gate XP {level}: {error:?}"))?;
        assert!(
            matches!(outcome, ExperienceCommitOutcome::Committed(_)),
            "{outcome:?}"
        );
    }
    let after = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("task-level updated progression")?;
    assert_eq!(
        after.level, level,
        "the native XP owner establishes task eligibility"
    );
    // Rebuild the admission projection from the Character owner's coherent facts.
    // Fresh source Characters have no vocation; the classed level-up hook does not
    // install their facts. No level or vocation is invented in this fixture.
    let facts = match crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
        owner.root,
        owner.character,
        fence.character_id,
    )
    .await
    {
        crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        }
        | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
            facts,
            ..
        } => facts,
        other => return Err(format!("native admission facts unavailable: {other:?}").into()),
    };
    assert_eq!(facts.level, level);
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        *states = Default::default();
        let installed = states
            .initialize(
                &runtime,
                admitted.runtime_actor.ok_or("actor")?,
                session,
                facts,
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("native facts projection")?;
        assert_eq!(
            installed.character_facts().level,
            level,
            "the live predicate must use the committed native XP level"
        );
    }
    Ok(())
}
