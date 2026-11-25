//! PARITY-EQUIP-MODIFIERS-1: the worn equipment's critical hit on a drawn swing and its life and
//! mana leech on a committed creature hit.

use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::{EquippedOffence, leech_amount};

// Crystal 201189f8 src/utils/utils_definitions.hpp:141,195.
pub(super) const CRITICAL_EFFECT: u16 = 173;
const FATAL_EFFECT: u16 = 230;

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
/// before the target's defence, armor or the PvP halving. The caller supplies the attack's
/// shared tagged critical draw; target-specific damage draws remain independent. EQMOD2 then
/// draws the tier/amplified fatal hit and adds its rounded 60% bonus. `None` when a required draw cannot be made (fail closed).
pub(super) fn critical_swing(
    offence: &EquippedOffence,
    root: &GameplayDecisionRoot,
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    damage: i64,
    critical_roll: u64,
) -> Option<(i64, Option<u16>)> {
    if damage <= 0 || (!offence.rolls_critical() && offence.weapon_tier == 0) {
        return Some((damage, None));
    }
    let damage = u32::try_from(damage).ok()?;
    let mut effect =
        (critical_roll <= u64::from(offence.player_critical().0)).then_some(CRITICAL_EFFECT);
    let damage = if offence.rolls_critical() {
        offence.critical(damage, critical_roll)
    } else {
        damage
    };
    let damage = if offence.weapon_tier > 0 {
        let roll = u64::try_from(swing_draw(
            root,
            attacker,
            lineage,
            ordinal,
            b"fatal-hit",
            "attack.fatal-hit",
            0,
            10_000,
        )?)
        .ok()?;
        // Crystal 201189f8 src/game/game.cpp:8103-8106 gives fatal precedence.
        // The existing rule adds a rounded point to a one-point hit on any
        // successful roll. Use that unsaturated probe to identify the outcome.
        if offence.fatal(1, roll) > 1 {
            effect = Some(FATAL_EFFECT);
        }
        offence.fatal(damage, roll)
    } else {
        damage
    };
    Some((i64::from(damage), effect))
}

/// Only an applied owner receipt permits leech. The admitted attack supplies
/// its pre-Prey, pre-overkill damage; HP loss is not the leech basis. Both gains
/// use the existing compare-committed vitals write and cannot replay.
pub(super) fn leech_hit(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    offence: &EquippedOffence,
    damage: u32,
    result: &crate::foundation::OwnerDamageResult,
) {
    if !result.applied || (offence.life_leech == 0 && offence.mana_leech == 0) {
        return;
    }
    let Some(next) = states.get(runtime, actor, session).and_then(|state| {
        state.after_leech(
            leech_amount(damage, offence.life_leech),
            leech_amount(damage, offence.mana_leech),
        )
    }) else {
        return;
    };
    let gained = states.get(runtime, actor, session).map_or(0, |before| {
        next.vitals().health.saturating_sub(before.vitals().health)
    });
    if states.commit(runtime, actor, session, next) {
        states.analyser.impact(actor, session, gained, None);
    }
}

/// Reuse target admission for each occupied flank tile; no generic radius or chain.
pub(super) fn cleave_targets(
    entry: &ActorAttack,
    selected: &drain::SelectedSwing,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    now: SemanticTimeMicros,
) -> Vec<ExactActorRef> {
    if selected.weapon.ranged.is_some()
        || !matches!(selected.weapon.weapon, SwingWeapon::Melee { .. })
    {
        return Vec::new();
    }
    let Ok(from) = runtime.read_actor_position(entry.actor) else {
        return Vec::new();
    };
    let Ok(to) = runtime.read_actor_position(selected.target) else {
        return Vec::new();
    };
    let (from, to) = (from.position(), to.position());
    let Some(tiles) = oteryn_gameplay_combat_rules::resolution::equipped_offence::cleave_tiles(
        (from.x, from.y),
        (to.x, to.y),
    ) else {
        return Vec::new();
    };
    let visible = runtime.visible_entities();
    let mut targets = Vec::new();
    for tile in tiles {
        for row in visible.creatures.iter().chain(if entry.modes.secure {
            [].iter()
        } else {
            visible.players.iter()
        }) {
            if row.actor == entry.actor || row.actor == selected.target {
                continue;
            }
            let Ok(position) = runtime.read_actor_position(row.actor) else {
                continue;
            };
            let at = position.position();
            if at.floor != to.floor || (at.x, at.y) != tile {
                continue;
            }
            let Some(facts) =
                target_facts(runtime, room, entry.actor, entry.session, row.actor, now)
            else {
                continue;
            };
            if facts.present_and_visible
                && facts.attackable_kind
                && facts.alive
                && facts.same_floor
                && facts.distance <= 1
                && !facts.attacker_in_protection_zone
                && !facts.target_in_protection_zone
                && !facts.attacker_reentry_protected
                && !facts.target_reentry_protected
            {
                targets.push(row.actor);
            }
        }
    }
    targets
}

#[cfg(test)]
mod global_leech_tests {
    use super::*;
    use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player};
    use crate::spell::cast::PlayerSpellState;

    #[test]
    fn global_auto_leech_uses_calculated_damage_and_rejects_unapplied_receipt() -> Result<(), String>
    {
        let (runtime, actor, session) = runtime_with_player(64);
        let mut states = ChannelSpellStates::default();
        let mut player = PlayerSpellState::new(FACTS, 0, 0).ok_or("character invalid")?;
        player.restore_saved_vitals(20, 20);
        states.actors.push((actor, session, player));
        let offence = EquippedOffence::from_sums(0, 0, 1600, 1600);
        let mut receipt = crate::foundation::OwnerDamageResult {
            applied: true,
            health_before: 20,
            health_after: 0,
        };
        leech_hit(
            &mut states,
            &runtime,
            actor,
            session,
            &offence,
            100,
            &receipt,
        );
        let after = states
            .get(&runtime, actor, session)
            .ok_or("current caster missing")?
            .clone();
        assert_eq!((after.vitals().health, after.vitals().mana), (36, 36));
        receipt.applied = false;
        leech_hit(
            &mut states,
            &runtime,
            actor,
            session,
            &offence,
            100,
            &receipt,
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&after));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, CommandId, GameSessionId, WorldId};

    #[test]
    fn a_fatal_roll_replaces_the_critical_cue_for_the_same_hit()
    -> Result<(), Box<dyn std::error::Error>> {
        let id = |suffix| {
            let mut bytes = [0; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes[15] = suffix;
            bytes
        };
        let actor =
            ExactActorRef::transport_fixture(WorldId::decode(&id(1))?, ChannelId::decode(&id(2))?);
        let command = CommandRef::new(GameSessionId::decode(&id(3))?, CommandId::new(1)?);
        let root = (0_u8..=u8::MAX)
            .map(|seed| GameplayDecisionRoot::from_bytes([seed; 32]))
            .find(|root| {
                swing_draw(
                    root,
                    actor,
                    command,
                    0,
                    b"fatal-hit",
                    "attack.fatal-hit",
                    0,
                    10_000,
                )
                .is_some_and(|roll| roll < 905)
            })
            .ok_or("no fatal draw")?;
        let offence = EquippedOffence {
            weapon_tier: 10,
            ..EquippedOffence::default()
        };

        assert_eq!(
            critical_swing(&offence, &root, actor, command, 0, 100, 1),
            Some((176, Some(FATAL_EFFECT)))
        );
        assert_eq!(
            critical_swing(
                &EquippedOffence::default(),
                &root,
                actor,
                command,
                0,
                100,
                1
            ),
            Some((110, Some(CRITICAL_EFFECT)))
        );
        assert_eq!(
            critical_swing(&offence, &root, actor, command, 0, 0, 1),
            Some((0, None))
        );
        Ok(())
    }
}
