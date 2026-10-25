//! SKILL-TRAIN-1 (SKILLS-0 §2, §3.4): a player's melee swing at a creature trains the swing's
//! skill (`WeaponMelee::getSkillType`, `Weapon::useFist`) and a creature hit the player blocks
//! trains shielding (`Player::onBlockHit`), by the `hit_training` rules. Tries that reach a level
//! are saved in the same owner pass once its locks are released, never left to the cadence.

use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::spell::mana_training::CompiledTrainingFormula;
use oteryn_gameplay_combat_rules::resolution::{MeleeSkill, SwingWeapon};
use oteryn_gameplay_progression::hit_training::{HitBlock, HitTrainingCounters, SkillFamily};

/// The players whose banked tries reach a level in this owner pass.
pub(in crate::gameplay_transport) type TrainingDue = Vec<(ExactActorRef, GameSessionId)>;

/// The owned skill a swing with `weapon` trains: fists train fist fighting.
pub(in crate::gameplay_transport) const fn swing_family(weapon: SwingWeapon) -> SkillFamily {
    match weapon {
        SwingWeapon::Fists
        | SwingWeapon::Melee {
            skill: MeleeSkill::Fist,
            ..
        } => SkillFamily::Fist,
        SwingWeapon::Melee {
            skill: MeleeSkill::Club,
            ..
        } => SkillFamily::Club,
        SwingWeapon::Melee {
            skill: MeleeSkill::Sword,
            ..
        } => SkillFamily::Sword,
        SwingWeapon::Melee {
            skill: MeleeSkill::Axe,
            ..
        } => SkillFamily::Axe,
    }
}

/// How the creature's defence and armor met a swing (`Creature::blockHit`): a hit they leave
/// nothing of is blocked, any damage left is unblocked. Defence and armor blocks train alike.
pub(in crate::gameplay_transport) const fn swing_block(magnitude: i64) -> HitBlock {
    if magnitude <= 0 {
        HitBlock::Armor
    } else {
        HitBlock::Unblocked
    }
}

/// Train `family` of `actor` by `rule` at `now` (micros). A player without owned training, a
/// missing formula or a stale clock trains nothing; tries that reach a level push the player to
/// `due`.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) fn train_hit(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    formula: Option<&CompiledTrainingFormula>,
    actor: ExactActorRef,
    session: GameSessionId,
    family: SkillFamily,
    now: u64,
    rule: impl FnOnce(&mut HitTrainingCounters) -> u64,
    due: &mut TrainingDue,
) {
    let Some(formula) = formula else {
        return;
    };
    let Some(state) = states.get_mut(runtime, actor, session) else {
        return;
    };
    let Ok(trained) = state.train_skill(family, now, formula, rule) else {
        return;
    };
    if trained.checkpoint_due && !due.contains(&(actor, session)) {
        due.push((actor, session));
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Save the training of every player in `due` (SKILLS-0 §3.4: a reached level commits at
    /// once). Runs after the owner pass released its locks; the save takes its own.
    pub(in crate::gameplay_transport) async fn save_trained_levels(&self, due: TrainingDue) {
        for (actor, session) in due {
            let _ = self.save_spell_training(actor, session, false).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn melee(skill: MeleeSkill) -> SwingWeapon {
        SwingWeapon::Melee { attack: 7, skill }
    }

    #[test]
    fn a_swing_trains_its_weapon_skill_and_fists_train_fist_fighting() {
        assert_eq!(swing_family(SwingWeapon::Fists), SkillFamily::Fist);
        assert_eq!(swing_family(melee(MeleeSkill::Fist)), SkillFamily::Fist);
        assert_eq!(swing_family(melee(MeleeSkill::Club)), SkillFamily::Club);
        assert_eq!(swing_family(melee(MeleeSkill::Sword)), SkillFamily::Sword);
        assert_eq!(swing_family(melee(MeleeSkill::Axe)), SkillFamily::Axe);
    }

    #[test]
    fn a_hit_the_creature_mitigates_to_nothing_is_blocked() {
        assert_eq!(swing_block(0), HitBlock::Armor);
        assert_eq!(swing_block(-3), HitBlock::Armor);
        assert_eq!(swing_block(1), HitBlock::Unblocked);
    }

    #[test]
    fn a_blocked_swing_trains_only_within_the_streak_an_unblocked_hit_opens() {
        let mut hits = HitTrainingCounters::default();
        assert_eq!(hits.melee_hit(swing_block(0)), 0, "no streak, no tries");
        assert_eq!(hits.melee_hit(swing_block(5)), 1, "an unblocked hit trains");
        assert_eq!(
            hits.melee_hit(swing_block(0)),
            1,
            "the streak trains a blocked hit"
        );
    }

    #[test]
    fn a_block_trains_shielding_only_with_a_shield_in_the_open_streak() {
        let mut hits = HitTrainingCounters::default();
        assert_eq!(hits.block_hit(true), 0, "no streak, no tries");
        hits.melee_hit(swing_block(5));
        assert_eq!(hits.block_hit(false), 0, "a weapon block trains nothing");
        assert_eq!(hits.block_hit(true), 1);
    }
}
