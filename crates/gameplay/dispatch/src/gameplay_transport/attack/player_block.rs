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
    pub catalogue: &'a dyn CharmCatalogueRead,
    pub root: GameplayDecisionRoot,
    pub occurrence: DecisionOccurrenceId,
    pub sequence: u64,
    pub race: &'a str,
    pub base_damage: u32,
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
    pub(in crate::gameplay_transport) charm: Option<IncomingCharm<'a>>,
    pub(in crate::gameplay_transport) states: &'a mut ChannelSpellStates,
    pub(in crate::gameplay_transport) attack: &'a mut ChannelAttackStates,
    pub(in crate::gameplay_transport) formula: Option<&'a CompiledTrainingFormula>,
    pub(in crate::gameplay_transport) trained: &'a mut TrainingDue,
}

impl CreatureBiteVitals for PlayerBiteVitals<'_> {
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
        if magnitude > 0
            && let Some(charm) = &self.charm
        {
            let outcomes = charm_combat::evaluate(
                self.states,
                runtime,
                charm.catalogue,
                target,
                session,
                charm.race,
                &charm.root,
                charm.occurrence,
                CharmHookEvent::IncomingCreatureHit {
                    base_damage: u64::from(charm.base_damage),
                },
            );
            for outcome in &outcomes {
                let Some(CharmEffectOutcome::ReflectDamage { damage, amount }) =
                    charm_combat::effect(outcome)
                else {
                    continue;
                };
                let Ok(draw) = oteryn_simulation_determinism::deterministic_decision_u64(
                    &charm.root,
                    charm.occurrence,
                    "charm.parry-armor",
                    0,
                ) else {
                    continue;
                };
                let Some(amount) =
                    charm_combat::damage_amount(runtime, source, damage, amount, draw)
                        .filter(|n| *n > 0)
                else {
                    continue;
                };
                let Ok(lease) = runtime.bound_attacker_lease(target, session) else {
                    continue;
                };
                if let Ok(result) = runtime.commit_source_damage(
                    source,
                    target,
                    session,
                    b"charm-parry-bite",
                    charm.sequence,
                    amount,
                    now.get() / 1_000,
                ) && result.applied
                    && result.health_after == 0
                {
                    self.states.equipment_reflection_kills.push((
                        source,
                        crate::combat::CapturedRewardPrincipal {
                            character: lease.character_id(),
                            lease_generation: lease.generation(),
                            actor: target,
                            session,
                        },
                        now.get(),
                    ));
                    // Parry can kill the source. The admitted incoming hit still lands;
                    // equipment reflection has no living source to damage after that kill.
                    return true;
                }
            }
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
        self.states
            .apply_creature_damage(runtime, target, target_session, magnitude, now)
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
