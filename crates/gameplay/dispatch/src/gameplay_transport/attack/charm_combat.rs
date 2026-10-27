//! Charm consumers at the existing attack, leech and source damage seams.
use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_gameplay_progression::charm_effects::{
    CharmCatalogueRead, CharmDamageKind, CharmEffectOutcome, CharmElement, CharmHookEvent,
    CharmOutcome, CharmResult,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn evaluate(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    catalogue: &dyn CharmCatalogueRead,
    actor: ExactActorRef,
    session: GameSessionId,
    race: &str,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    event: CharmHookEvent,
) -> Vec<CharmOutcome> {
    let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
        return Vec::new();
    };
    let Some(state) = states.charm_state(runtime, actor, session) else {
        return Vec::new();
    };
    run_charm_hook(
        catalogue,
        state,
        *lease.character_id().as_bytes(),
        race,
        root,
        occurrence,
        event,
    )
    .unwrap_or_default()
}

// The pure evaluator still identifies historically unavailable systems. Only these consumers
// discharge that marker locally; all other FailedClosed outcomes stay unapplied.
pub(super) fn effect(outcome: &CharmOutcome) -> Option<CharmEffectOutcome> {
    match outcome.result {
        CharmResult::Applied(effect)
        | CharmResult::FailedClosed {
            evaluated: effect, ..
        } => Some(effect),
        _ => None,
    }
}

pub(super) fn bonuses(
    offence: &mut oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence,
    outcomes: &[CharmOutcome],
) {
    for outcome in outcomes {
        let (slot, percent) = match effect(outcome) {
            Some(CharmEffectOutcome::CriticalHitChanceBonus { percent }) => {
                (&mut offence.critical_chance, percent)
            }
            Some(CharmEffectOutcome::CriticalExtraDamageBonus { percent }) => {
                (&mut offence.critical_damage, percent)
            }
            Some(CharmEffectOutcome::LifeLeechBonus { percent }) => {
                (&mut offence.life_leech, percent)
            }
            Some(CharmEffectOutcome::ManaLeechBonus { percent }) => {
                (&mut offence.mana_leech, percent)
            }
            _ => continue,
        };
        // Charms augment the corresponding equipment skill; they do not grant it.
        if *slot == 0 {
            continue;
        }
        *slot = slot.saturating_add(u16::try_from(percent.hundredths()).unwrap_or(u16::MAX));
    }
}

pub(super) fn damage_amount(
    runtime: &ChannelRuntimeV1,
    target: ExactActorRef,
    damage: CharmDamageKind,
    amount: u64,
    armor_draw: u64,
) -> Option<u32> {
    let snapshot = runtime.companion_snapshot(target).ok()?;
    let mut amount = u32::try_from(amount).unwrap_or(u32::MAX);
    if damage.reduced_by_armor {
        amount = oteryn_gameplay_combat_rules::resolution::player_armor_mitigated(
            AttackConstants::checked_in()?,
            amount,
            snapshot.state.policy.armor?,
            armor_draw,
        );
    }
    if !damage.ignores_resistances {
        let element = match damage.element {
            CharmElement::Physical => "physical",
            CharmElement::Fire => "fire",
            CharmElement::Earth => "earth",
            CharmElement::Energy => "energy",
            CharmElement::Ice => "ice",
            CharmElement::Holy => "holy",
            CharmElement::Death => "death",
        };
        amount = u32::try_from(
            oteryn_gameplay_combat_rules::resolution::creature_damage_response(
                &snapshot.state.policy,
                i64::from(amount),
                element,
            )?,
        )
        .ok()?;
    }
    Some(amount)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
    use crate::gameplay_transport::actor_spell::tests::runtime_with_player;

    #[test]
    fn charm_bonuses_augment_only_existing_equipment_skills() {
        use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
        use oteryn_gameplay_progression::charm_effects::{
            CharmHook, CharmMissingSystem, CharmPercent,
        };
        let percent = CharmPercent::from_hundredths(400).expect("percent");
        let outcomes: Vec<_> = [
            CharmEffectOutcome::CriticalHitChanceBonus { percent },
            CharmEffectOutcome::CriticalExtraDamageBonus { percent },
            CharmEffectOutcome::LifeLeechBonus { percent },
            CharmEffectOutcome::ManaLeechBonus { percent },
        ]
        .into_iter()
        .map(|evaluated| CharmOutcome {
            charm_key: "fixture:charm".into(),
            stage: 1,
            hook: CharmHook::AttackHit,
            result: CharmResult::FailedClosed {
                reason: CharmMissingSystem::CriticalHit,
                evaluated,
            },
        })
        .collect();
        let mut empty = EquippedOffence::default();
        bonuses(&mut empty, &outcomes);
        assert_eq!(empty, EquippedOffence::default());
        let mut equipped = EquippedOffence::from_sums(100, 5000, 100, 200);
        bonuses(&mut equipped, &outcomes);
        assert_eq!(
            (
                equipped.critical_chance,
                equipped.critical_damage,
                equipped.life_leech,
                equipped.mana_leech
            ),
            (500, 5400, 500, 600)
        );
    }

    #[test]
    fn charm_evaluation_error_suppresses_only_the_effects() {
        let (mut runtime, actor, session) = runtime_with_player(26);
        let character =
            crate::foundation::CharacterId::decode(session.as_bytes()).expect("character");
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(character, 1).expect("lease"),
            )
            .expect("bind");
        let mut states = ChannelSpellStates::default();
        let catalogue =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().expect("catalogue");
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let occurrence = DecisionOccurrenceId::from_bytes([2; 16]);
        assert!(
            evaluate(
                &states,
                &runtime,
                &catalogue,
                actor,
                session,
                "oteryn:creature.rat",
                &root,
                occurrence,
                CharmHookEvent::IncomingCreatureAttack
            )
            .is_empty()
        );
        let wrong_character =
            crate::domain::CharacterId::from_bytes(*runtime_with_player(27).2.as_bytes())
                .expect("other character");
        assert!(states.install_charm_state(
            &runtime,
            actor,
            session,
            CombatCharmState::new(wrong_character, CharacterCharmState::default())
        ));
        assert!(
            evaluate(
                &states,
                &runtime,
                &catalogue,
                actor,
                session,
                "oteryn:creature.rat",
                &root,
                occurrence,
                CharmHookEvent::IncomingCreatureAttack
            )
            .is_empty()
        );
        assert!(runtime.player_control_facts(actor, session).is_ok());
    }

    #[test]
    fn charm_snapshot_is_session_scoped_and_prunes_a_logged_out_generation() {
        let (mut runtime, actor, session) = runtime_with_player(24);
        let character =
            crate::domain::CharacterId::from_bytes(*session.as_bytes()).expect("character");
        let mut states = ChannelSpellStates::default();
        assert!(states.install_charm_state(
            &runtime,
            actor,
            session,
            CombatCharmState::new(character, CharacterCharmState::default())
        ));
        let other = runtime_with_player(25).2;
        assert!(states.charm_state(&runtime, actor, other).is_none());
        runtime
            .remove_terminal_session(session, actor)
            .expect("terminal session");
        assert!(states.charm_state(&runtime, actor, session).is_none());
        let reservation = runtime
            .reserve_fresh_session(other)
            .expect("reserve replacement");
        let replacement = runtime
            .commit_fresh_session(reservation)
            .expect("replacement");
        assert!(states.install_charm_state(
            &runtime,
            replacement,
            other,
            CombatCharmState::new(character, CharacterCharmState::default())
        ));
        assert_eq!(states.charm_states.len(), 1);
        assert!(states.charm_state(&runtime, replacement, session).is_none());
    }
}
