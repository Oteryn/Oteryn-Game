use super::ChannelSpellStates;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::{
    ApplicationFacts, ChannelRuntimeV1, ConditionDefinition, ConditionValues, ExactActorRef,
    owner_timer::SemanticTimeMicros,
};
use crate::player_lethal::{PlayerHitKind, PlayerLethalVitals, SourceAppearanceTarget};
use oteryn_gameplay_encounters::bosses::phase_controller::duke_krule::{DukeEffect, ElementalForm};
use oteryn_protocol_oteryn::analyser::AnalyserDamageSource;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

pub(super) fn apply(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    source: ExactActorRef,
    effect: DukeEffect,
    now: u64,
) {
    match effect {
        DukeEffect::Transform { player, finish_us } => {
            let look_type = match player.form {
                ElementalForm::Fire => 49,
                ElementalForm::Water => 286,
            };
            let Some(appearance) = native
                .spell_appearances()
                .and_then(|index| index.for_look_type(look_type))
            else {
                return;
            };
            let Ok(duration_ms) = u32::try_from(finish_us.saturating_sub(now) / 1_000) else {
                return;
            };
            let Some(definition) = ConditionDefinition::new(
                "grave-danger:duke-krule:elemental-form",
                1,
                ConditionValues::Outfit {
                    look_type,
                    duration_ms,
                },
            )
            .and_then(|definition| definition.with_appearance(appearance.selection())) else {
                return;
            };
            let Some(current) = states.get(runtime, player.actor, player.session) else {
                return;
            };
            let Ok(base_speed) = u16::try_from(current.owned_base_speed()) else {
                return;
            };
            let Ok(stamp) = runtime.issue_owner_work() else {
                return;
            };
            let root = GameplayDecisionRoot::from_bytes(native.source_digest());
            let target = SourceAppearanceTarget {
                actor: player.actor,
                session: Some(player.session),
                immunities: Vec::new(),
                facts: ApplicationFacts {
                    now,
                    base_speed,
                    mana_shield_capacity: 0,
                    target_is_player: true,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes(player.actor.placement_identity()),
                },
            };
            let _ = states.apply_source_appearance_batch_current(
                runtime,
                stamp,
                source,
                &[definition],
                &[target],
            );
        }
        DukeEffect::Damage {
            attacker,
            target,
            amount,
            starts,
            pulse,
        } => {
            let kind = match attacker.form {
                ElementalForm::Fire => PlayerHitKind::Fire,
                ElementalForm::Water => PlayerHitKind::Ice,
            };
            let token = u128::from_le_bytes(source.placement_identity());
            let caster = u128::from_le_bytes(attacker.actor.placement_identity());
            let occurrence = format!("duke-krule:{token:x}:{caster:x}:{starts}:{pulse}");
            let _ = states.apply_typed_attack_damage_from(
                runtime,
                target.actor,
                target.session,
                amount,
                kind,
                &occurrence,
                SemanticTimeMicros::from_micros(now),
                AnalyserDamageSource::None,
            );
        }
    }
}
