//! PARITY-EQUIP-MODIFIERS-1: the worn equipment's critical hit on a drawn swing and its life and
//! mana leech on a committed creature hit.

use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::{EquippedOffence, leech_amount};

/// The attacker's installed offence and the current swing's world event bonus.
pub(super) fn attacker_offence(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> EquippedOffence {
    use oteryn_gameplay_foundation::scheduled_events;

    let Some(state) = states.get(runtime, actor, session) else {
        return EquippedOffence::default();
    };
    let mut offence = state.equipped_offence();
    offence.exaltation_overload = scheduled_events::current_rates().exaltation_overload
        && runtime.read_actor_position(actor).is_ok_and(|snapshot| {
            let position = snapshot.position();
            scheduled_events::exaltation_overload_allowed_at(position.x, position.y)
        });
    offence
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
) -> Option<i64> {
    if damage <= 0 {
        return Some(damage);
    }
    let damage = u32::try_from(damage).ok()?;
    let damage = if offence.rolls_critical() {
        offence.critical(damage, critical_roll)
    } else {
        damage
    };
    let damage = if offence.weapon_tier > 0 || offence.exaltation_overload {
        let roll = swing_draw(
            root,
            attacker,
            lineage,
            ordinal,
            b"fatal-hit",
            "attack.fatal-hit",
            0,
            10_000,
        )?;
        offence.fatal(damage, u64::try_from(roll).ok()?)
    } else {
        damage
    };
    Some(i64::from(damage))
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
mod exaltation_overload_tests {
    use super::*;
    use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player};
    use crate::spell::cast::PlayerSpellState;
    use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};

    #[test]
    fn exaltation_overload_rolls_onslaught_without_equipment_only_on_the_mainland()
    -> Result<(), String> {
        // The deterministic schedule clock uses 2030-01-01 00:00:00 UTC.
        let schedule = Schedule::parse(
            r#"{"events":[{"name":"Exaltation Overload","startdate":"01/01/2030","enddate":"01/02/2030"}]}"#,
        )
        .map_err(|error| format!("{error:?}"))?;
        let swing = |(x, y), at| -> Result<i64, String> {
            let _clock = test_clock::set(schedule.clone(), at);
            let (mut runtime, actor, session) = runtime_with_player(65);
            runtime
                .initialize_source_pinned_lab_player_position(
                    actor,
                    session,
                    MovementLocalPosition { x, y, floor: -7 },
                )
                .map_err(|error| format!("{error:?}"))?;
            let mut states = ChannelSpellStates::default();
            let player = PlayerSpellState::new(FACTS, 0, 0).ok_or("character invalid")?;
            states.actors.push((actor, session, player));
            let offence = attacker_offence(&states, &runtime, actor, session);
            let lineage =
                CommandRef::new(session, CommandId::new(1).map_err(|e| format!("{e:?}"))?);
            let root = GameplayDecisionRoot::from_bytes([1; 32]);

            // Find an occurrence whose fatal draw succeeds only with the event.
            let ordinal = (0..u16::MAX)
                .find(|ordinal| {
                    swing_draw(
                        &root,
                        actor,
                        lineage,
                        *ordinal,
                        b"fatal-hit",
                        "attack.fatal-hit",
                        0,
                        10_000,
                    )
                    .is_some_and(|roll| roll < 105)
                })
                .ok_or("no event-eligible draw")?;
            critical_swing(&offence, &root, actor, lineage, ordinal, 100, 10_000)
                .ok_or_else(|| "missing swing draw".into())
        };

        let start = 1_893_456_000;
        assert_eq!(swing((32369, 32241), start)?, 160);
        assert_eq!(swing((32369, 32241), start - 1)?, 100);
        assert_eq!(swing((32097, 32219), start)?, 100); // Rookgaard
        assert_eq!(swing((32062, 31895), start)?, 100); // Dawnport
        Ok(())
    }
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
