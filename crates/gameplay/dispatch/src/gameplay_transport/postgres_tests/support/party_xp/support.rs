//! Shared private fixture mechanics for the real PARTY-XP PostgreSQL dispatch cases.

use super::*;

pub(super) async fn join_two<A: std::future::Future, B: std::future::Future>(
    a: A,
    b: B,
) -> (A::Output, B::Output) {
    let mut a = std::pin::pin!(a);
    let mut b = std::pin::pin!(b);
    let mut x = None;
    let mut y = None;
    std::future::poll_fn(move |cx| {
        if x.is_none()
            && let std::task::Poll::Ready(v) = a.as_mut().poll(cx)
        {
            x = Some(v);
        }
        if y.is_none()
            && let std::task::Poll::Ready(v) = b.as_mut().poll(cx)
        {
            y = Some(v);
        }
        if x.is_some() && y.is_some() {
            std::task::Poll::Ready((x.take().expect("first"), y.take().expect("second")))
        } else {
            std::task::Poll::Pending
        }
    })
    .await
}

pub(in crate::gameplay_transport) async fn initialize_build(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    vocation: &str,
    seed: u8,
) -> TestResult<(
    crate::spell::cast::CharacterCastFacts,
    crate::durability::character_build::DurableBuildState,
)> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current fence")?
        .ok_or("fence")?;
    let binding = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            fence,
            crate::durability::character_progression::ProgressionInitializationRequest {
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    let before = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        "content-1",
    )
    .map_err(debug)?;
    let after = crate::durability::character_build::convert_vocation(&formula, &before, vocation)
        .map_err(debug)?;
    if before != after {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        slot.commit_build(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::character_build::BuildChangeRequest {
                occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(
                    seed,
                ))
                .map_err(debug)?,
                cause: if before.vocation() == "none" {
                    crate::durability::character_build::BuildCause::VocationChoice
                } else {
                    crate::durability::character_build::BuildCause::VocationTrial
                },
                before,
                after: after.clone(),
                pruned_stance: None,
            },
            &formula,
            None,
        )
        .await
        .map_err(debug)?;
    }
    let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::load_character_cast_facts(
            owner.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("official classed Character facts".into());
    };
    Ok((facts, after))
}

/// Predict both block-budget outcomes of the first unarmed swing for a fixture command.
/// The test chooses an input with the required damage and still dispatches the real swing.
pub(in super::super) fn fixture_fist_damage(
    runtime: &ChannelRuntimeV1,
    (actor, session): (
        crate::foundation::ExactActorRef,
        crate::foundation::GameSessionId,
    ),
    state: &crate::spell::cast::PlayerSpellState,
    target: crate::foundation::ExactActorRef,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> TestResult<impl Fn(u64) -> Option<(i64, i64)> + use<>> {
    use oteryn_gameplay_combat_rules::{
        attack::{AttackConstants, formulas::player_monk_formula, player_fist_formula},
        resolution::{
            creature_melee_mitigation, creature_physical_resistance, swing_draw, swing_mitigated,
        },
    };
    use oteryn_simulation_determinism::GameplayDecisionRoot;

    let constants = AttackConstants::checked_in().ok_or("attack constants")?;
    let facts = state.character_facts();
    let skills = state
        .swing_skills(0, now.get())
        .flatten()
        .ok_or("fixture fist skills")?;
    let formula = if matches!(
        facts.vocation,
        crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
    ) {
        player_monk_formula(constants)
    } else {
        player_fist_formula(constants)
    };
    let (minimum, maximum) = formula
        .bounds(&crate::spell::formula::FormulaInputs {
            level: facts.level,
            magic_level: facts.magic_level,
            base_power: None,
            attack_skill: skills.attack,
            attack_value: constants.fist.attack,
            attack_factor: AttackConstants::PLAYER_ATTACK_FACTOR,
            shielding_skill: skills.shielding,
            shield_defense: None,
        })
        .map_err(debug)?;
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
    let defense = runtime
        .effective_creature_defense(&snapshot)
        .map_err(debug)?;
    let policy = snapshot.state.policy;
    let armor = policy.armor.ok_or("fixture creature armor")?;
    Ok(move |command| {
        let attacker = actor.placement_identity();
        let lineage = (*session.as_bytes(), command);
        let raw = swing_draw(
            &root,
            attacker,
            lineage,
            0,
            b"damage-draw",
            "attack.damage",
            minimum,
            maximum,
        )?;
        let damage = |defense| {
            let defended =
                swing_mitigated(&root, constants, attacker, lineage, 0, raw, defense, 0)?;
            let resisted = creature_physical_resistance(&policy, defended)?;
            let armored = swing_mitigated(
                &root, constants, attacker, lineage, 0, resisted, None, armor,
            )?;
            creature_melee_mitigation(&policy, armored)
        };
        Some((damage(Some((defense / 2, defense)))?, damage(None)?))
    })
}
