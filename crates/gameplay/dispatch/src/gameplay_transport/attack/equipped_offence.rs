//! PARITY-EQUIP-MODIFIERS-1: the worn equipment's critical hit on a drawn swing and its life and
//! mana leech on a committed creature hit.

use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::{
    CRITICAL_ROLL_MAX, CRITICAL_ROLL_MIN, EquippedOffence, leech_amount,
};

/// The attacker's installed offence; zero for an actor without spell state.
pub(super) fn attacker_offence(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> EquippedOffence {
    states
        .get(runtime, actor, session)
        .map(crate::spell::cast::PlayerSpellState::equipped_offence)
        .unwrap_or_default()
}

/// `Combat::applyExtensions` (`combat.cpp:2724-2742`): a drawn swing meets the critical roll
/// before the target's defence, armor or the PvP halving. The roll is its own tagged draw, so it
/// shifts no other draw of the swing. `None` when the draw cannot be made (fail closed).
pub(super) fn critical_swing(
    offence: EquippedOffence,
    root: &GameplayDecisionRoot,
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    damage: i64,
) -> Option<i64> {
    if damage <= 0 || !offence.rolls_critical() {
        return Some(damage);
    }
    let roll = swing_draw(
        root,
        attacker,
        lineage,
        ordinal,
        b"critical-hit",
        "attack.critical-hit",
        i64::try_from(CRITICAL_ROLL_MIN).ok()?,
        i64::try_from(CRITICAL_ROLL_MAX).ok()?,
    )?;
    let damage = u32::try_from(damage).ok()?;
    Some(i64::from(
        offence.critical(damage, u64::try_from(roll).ok()?),
    ))
}

/// `Game::applyLifeLeech` and `Game::applyManaLeech` (`game.cpp:8760-8764`, `8998-9056`): the
/// health the hit really took heals the attacker and restores its mana, in one compare-committed
/// vitals write. A leech that restores nothing writes nothing.
pub(super) fn leech_hit(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    offence: EquippedOffence,
    result: &crate::foundation::OwnerDamageResult,
) {
    if !result.applied || (offence.life_leech == 0 && offence.mana_leech == 0) {
        return;
    }
    let real = result
        .health_before
        .saturating_sub(result.health_after.max(0))
        .max(0);
    let Ok(real) = u32::try_from(real) else {
        return;
    };
    let Some(next) = states.get(runtime, actor, session).and_then(|state| {
        state.after_leech(
            leech_amount(real, offence.life_leech),
            leech_amount(real, offence.mana_leech),
        )
    }) else {
        return;
    };
    states.commit(runtime, actor, session, next);
}
