//! SHIELD-DEFENCE-1 (`Creature::blockHit`, `Player::getDefense`): a creature hit on a player
//! spends one of the player's blocks and meets the defence of what the player's hands hold.

use super::charm_combat;
use super::skill_training::{TrainingDue, train_hit};
use super::*;
use oteryn_gameplay_progression::charm_effects::{
    CharmCatalogueRead, CharmEffectOutcome, CharmHookEvent,
};

use crate::ability::creature_bite::{CreatureBiteVitals, CreatureHit};
use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::spell::mana_training::CompiledTrainingFormula;
use oteryn_gameplay_combat_rules::attack::formulas::DefenceSource;
use oteryn_gameplay_combat_rules::resolution::player_defence::player_defence_bounds;
use oteryn_gameplay_progression::hit_training::SkillFamily;

pub(in crate::gameplay_transport) struct IncomingCharm<'a> {
    pub source: ExactActorRef,
    pub catalogue: &'a dyn CharmCatalogueRead,
    pub root: GameplayDecisionRoot,
    pub occurrence: DecisionOccurrenceId,
    pub sequence: u64,
    pub race: &'a str,
    pub base_damage: u32,
}

/// Source-qualified secondary effects carried through the existing melee mitigation hooks.
/// The player owner revalidates this work stamp before the atomic HP/condition transition.
pub(in crate::gameplay_transport) struct IncomingMeleeConditions {
    pub source: ExactActorRef,
    pub definitions: Vec<crate::foundation::ConditionDefinition>,
    pub root: GameplayDecisionRoot,
    pub occurrence: DecisionOccurrenceId,
    pub occurrence_text: String,
    pub stamp: crate::foundation::RuntimeWorkStamp,
}

impl ChannelAttackStates {
    /// Spend one of `actor`'s blocks on a creature hit at `now`; the budget is created settled
    /// at the first hit, like a creature's (ATTACK-0 §5). `Some((fight mode, swung within the
    /// attack interval))` when the hit is blocked.
    pub(in crate::gameplay_transport) fn take_player_block(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(FightMode, bool)> {
        let constants = AttackConstants::checked_in()?;
        let entry = self.entry(runtime, actor, session)?;
        let budget = entry
            .defence_blocks
            .get_or_insert_with(|| BlockBudget::settled(&constants.block, now));
        if !budget.try_block(&constants.block, now) {
            return None;
        }
        Some((
            fight_mode(entry.modes.fight_mode),
            entry
                .state
                .swung_within_interval(now, constants.attack_interval_micros()),
        ))
    }
}

/// The vitals a monster bite reaches with the player's defence: the spell states' vitals and
/// armor, with the block budget and fight mode of the attack states. A block trains shielding
/// (SKILL-TRAIN-1) by `formula`; players whose tries reach a level go to `trained`.
pub(in crate::gameplay_transport) struct PlayerBiteVitals<'a> {
    pub(in crate::gameplay_transport) quest_sessions:
        Option<&'a super::monk_shrines::QuestSessions>,
    pub(in crate::gameplay_transport) charm: Option<IncomingCharm<'a>>,
    pub(in crate::gameplay_transport) conditions: Option<IncomingMeleeConditions>,
    pub(in crate::gameplay_transport) states: &'a mut ChannelSpellStates,
    pub(in crate::gameplay_transport) attack: &'a mut ChannelAttackStates,
    pub(in crate::gameplay_transport) formula: Option<&'a CompiledTrainingFormula>,
    pub(in crate::gameplay_transport) trained: &'a mut TrainingDue,
}

impl CreatureBiteVitals for PlayerBiteVitals<'_> {
    fn melee_damage_taken(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        damage: u32,
    ) -> u32 {
        let Some(sessions) = self.quest_sessions else {
            return damage;
        };
        let Some(state) = self.states.get(runtime, target, session) else {
            return damage;
        };
        super::monk_shrines::taken(
            sessions,
            session,
            state.character_facts().vocation,
            i64::from(damage),
        )
        .and_then(|n| n.try_into().ok())
        .unwrap_or(damage)
    }

    fn dodge_creature_attack(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        base_damage: u32,
    ) -> bool {
        let Some(charm) = &mut self.charm else {
            return false;
        };
        charm.base_damage = base_damage;
        charm_combat::evaluate(
            self.states,
            runtime,
            charm.catalogue,
            target,
            session,
            charm.race,
            &charm.root,
            charm.occurrence,
            CharmHookEvent::IncomingCreatureAttack,
        )
        .iter()
        .any(|o| {
            matches!(
                charm_combat::effect(o),
                Some(CharmEffectOutcome::DodgeAttack)
            )
        })
    }

    fn reflect_equipment_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        magnitude: u32,
        stream: &[u8],
        now: OwnerTime,
    ) -> bool {
        if self.states.is_dead(target)
            || self.states.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
            || runtime.player_control_facts(target, session).is_err()
            || !self
                .states
                .get(runtime, target, session)
                .is_some_and(|state| state.owned_conditions().accepts_time(now.get()))
        {
            return false;
        }

        if magnitude > 0 && self.charm.is_some() {
            return true;
        }
        self.states.reflect_equipment(
            runtime, source, target, session, element, magnitude, stream, now,
        )
    }

    fn apply_creature_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        magnitude: u32,
        now: OwnerTime,
    ) -> Option<CreatureHit> {
        if magnitude > 0
            && let Some(charm) = &self.charm
        {
            use crate::player_lethal::{
                PlayerHitKind, PlayerLethalVitals, SourceHitConditions, SourceHitEquipment,
            };
            let text = self
                .conditions
                .as_ref()
                .map(|conditions| conditions.occurrence_text.clone())
                .unwrap_or_else(|| {
                    format!(
                        "bite:{}:{}",
                        super::hex(&charm.source.placement_identity()),
                        charm.sequence
                    )
                });
            let base_speed = u16::try_from(
                self.states
                    .get(runtime, target, target_session)?
                    .owned_base_speed(),
            )
            .ok()?;
            let facts = crate::foundation::ApplicationFacts {
                now: now.get(),
                base_speed,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &charm.root,
                occurrence: charm.occurrence,
            };
            let conditions = self
                .conditions
                .as_ref()
                .map(|conditions| SourceHitConditions {
                    definitions: &conditions.definitions,
                    facts: &facts,
                    immunities: &[],
                    fence: None,
                    stamp: conditions.stamp,
                });
            let equipment_magnitude = magnitude;
            let magnitude = if conditions.is_some() {
                magnitude
            } else {
                let amount =
                    if self
                        .states
                        .elite_bounty(runtime, target, target_session, charm.race)
                    {
                        magnitude.saturating_mul(2)
                    } else {
                        magnitude
                    };
                let percent = self.states.prey_percent(
                    runtime,
                    target,
                    target_session,
                    charm.race,
                    2,
                    now.get(),
                );
                amount - (u64::from(amount) * u64::from(percent) / 100) as u32
            };
            let receipt = charm_combat::source_vitals::CharmSourceVitals {
                states: self.states,
                attack: self.attack,
                catalogue: charm.catalogue,
                root: charm.root.clone(),
            }
            .apply_source_incoming_hit(
                runtime,
                charm.source,
                target,
                target_session,
                magnitude,
                charm.base_damage,
                Some(charm.occurrence),
                PlayerHitKind::Physical,
                &text,
                now,
                conditions,
                Some(SourceHitEquipment {
                    native_bite: true,
                    magnitude: equipment_magnitude,
                    element: Some("physical"),
                    stream: b"bite",
                }),
            )?;
            return Some(CreatureHit {
                damage: crate::ability::creature_bite::CreatureDamage {
                    applied: receipt.applied,
                    health_after: receipt.health_after,
                },
                vitals_revision: receipt.vitals_revision,
                death: receipt.death.map(|death| death.occurrence_bytes()),
            });
        }
        let was_dead = self.states.is_dead(target);
        // Zero/fully blocked/dodged primary hits do not attach melee conditions.
        // The admitted source's nonzero primary and secondary state commit together.
        if magnitude > 0
            && let Some(source) = &self.conditions
        {
            let base_speed = u16::try_from(
                self.states
                    .get(runtime, target, target_session)?
                    .owned_base_speed(),
            )
            .ok()?;
            let facts = crate::foundation::ApplicationFacts {
                now: now.get(),
                base_speed,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &source.root,
                occurrence: source.occurrence,
            };
            let receipt =
                crate::player_lethal::PlayerLethalVitals::apply_composite_attack_damage_current(
                    self.states,
                    runtime,
                    target,
                    target_session,
                    magnitude,
                    &source.occurrence_text,
                    source.source,
                    &source.definitions,
                    &facts,
                    &[],
                    source.stamp,
                )?;
            if !was_dead && let (Some(death), Some(charm)) = (receipt.death, &self.charm) {
                self.attack.capture_death_charm(
                    runtime,
                    self.states,
                    charm.catalogue,
                    target,
                    target_session,
                    charm.source,
                    death.occurrence_bytes(),
                    &charm.root,
                );
            }
            return Some(CreatureHit {
                damage: crate::ability::creature_bite::CreatureDamage {
                    applied: receipt.applied,
                    health_after: receipt.health_after,
                },
                vitals_revision: receipt.vitals_revision,
                death: receipt.death.map(|death| death.occurrence_bytes()),
            });
        }
        let magnitude = if self.charm.as_ref().is_some_and(|c| {
            self.states
                .elite_bounty(runtime, target, target_session, c.race)
        }) {
            magnitude.saturating_mul(2)
        } else {
            magnitude
        };
        let percent = self.charm.as_ref().map_or(0, |c| {
            self.states
                .prey_percent(runtime, target, target_session, c.race, 2, now.get())
        });
        let magnitude = magnitude - (u64::from(magnitude) * u64::from(percent) / 100) as u32;
        let hit = self.states.apply_creature_damage_kind_from(
            runtime,
            target,
            target_session,
            magnitude,
            crate::player_lethal::PlayerHitKind::Physical,
            now,
            super::super::analyser::HitSource {
                source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(
                    self.charm
                        .as_ref()
                        .and_then(|charm| super::super::analyser::race(charm.race)),
                ),
                attacker: self.charm.as_ref().map(|charm| charm.source),
            },
            self.charm.as_ref().and_then(|charm| {
                crate::gameplay_transport::actor_spell::native_public_death_cause(
                    runtime,
                    charm.source,
                )
            }),
        )?;
        if !was_dead && let (Some(death), Some(charm)) = (hit.death, &self.charm) {
            self.attack.capture_death_charm(
                runtime,
                self.states,
                charm.catalogue,
                target,
                target_session,
                charm.source,
                death,
                &charm.root,
            );
        }
        Some(hit)
    }

    fn target_armor(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
    ) -> u32 {
        self.states.target_armor(runtime, target, target_session)
    }

    fn target_physical_protection(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
    ) -> Option<crate::content::ReferenceRationalPercent> {
        self.states
            .target_physical_protection(runtime, target, target_session)
    }

    fn target_defence(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        now: OwnerTime,
    ) -> Option<(u32, u32)> {
        let constants = AttackConstants::checked_in()?;
        let state = self.states.get(runtime, target, target_session)?;
        let hand = state.equipped_defence()?;
        let skill = match hand.skill_index {
            Some(index) => state
                .swing_skills(index, now.get())?
                .map_or(constants.starting_skill, |skills| skills.attack),
            None => 0,
        };
        let bonus = state.wheel_battle_instinct(runtime, target, now.get())?;
        let skill = skill.checked_add(match hand.skill_index {
            Some(1..=3) => bonus.melee,
            Some(5) => bonus.shielding,
            _ => 0,
        })?;
        let (mode, swung) = self.attack.take_player_block(
            runtime,
            target,
            target_session,
            SemanticTimeMicros::from_micros(now.get()),
        )?;
        // `Player::onBlockHit`: a block trains shielding only with a shield in hand.
        let shield = hand.source == DefenceSource::Shield;
        train_hit(
            self.states,
            runtime,
            self.formula,
            target,
            target_session,
            SkillFamily::Shielding,
            now.get(),
            |hits| hits.block_hit(shield),
            self.trained,
        );
        player_defence_bounds(constants, hand, skill, mode, swung)
    }
}
