//! SHIELD-DEFENCE-1 (`Creature::blockHit`, `Player::getDefense`): a creature hit on a player
//! spends one of the player's blocks and meets the defence of what the player's hands hold.

use super::skill_training::{TrainingDue, train_hit};
use super::*;
use crate::ability::creature_bite::{CreatureBiteVitals, CreatureHit};
use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::spell::mana_training::CompiledTrainingFormula;
use oteryn_gameplay_combat_rules::attack::formulas::DefenceSource;
use oteryn_gameplay_combat_rules::resolution::player_defence::player_defence_bounds;
use oteryn_gameplay_progression::hit_training::SkillFamily;

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
    pub(in crate::gameplay_transport) states: &'a mut ChannelSpellStates,
    pub(in crate::gameplay_transport) attack: &'a mut ChannelAttackStates,
    pub(in crate::gameplay_transport) formula: Option<&'a CompiledTrainingFormula>,
    pub(in crate::gameplay_transport) trained: &'a mut TrainingDue,
}

impl CreatureBiteVitals for PlayerBiteVitals<'_> {
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
