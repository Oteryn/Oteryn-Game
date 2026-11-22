//! PARITY-EQUIP-MODIFIERS-1: the worn equipment's critical hit on a drawn swing and its life and
//! mana leech on a committed creature hit.

use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::{EquippedOffence, leech_amount};

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
pub(in crate::gameplay_transport) fn critical_swing(
    offence: &EquippedOffence,
    root: &GameplayDecisionRoot,
    attacker: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
    damage: i64,
    critical_roll: u64,
) -> Option<i64> {
    let bonus = oteryn_gameplay_foundation::scheduled_events::current_rates()
        .exaltation_overload
        .map_or(0, |event| event.onslaught_permyriad());
    let rolls_fatal = offence.weapon_tier > 0 || bonus > 0;
    if damage <= 0 || (!offence.rolls_critical() && !rolls_fatal) {
        return Some(damage);
    }
    let damage = u32::try_from(damage).ok()?;
    let damage = if offence.rolls_critical() {
        offence.critical(damage, critical_roll)
    } else {
        damage
    };
    let damage = if rolls_fatal {
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
        offence.fatal_with_bonus(damage, u64::try_from(roll).ok()?, bonus)
    } else {
        damage
    };
    Some(i64::from(damage))
}

/// Crystal game.cpp:7226-7234 checks one Ruse draw before ordinary mitigation.
pub(in crate::gameplay_transport) fn ruse(
    state: &crate::spell::cast::PlayerSpellState,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> bool {
    let Some(event) =
        oteryn_gameplay_foundation::scheduled_events::current_rates().exaltation_overload
    else {
        return false;
    };
    let bonus = u64::from(event.ruse_permyriad());
    if bonus == 0 {
        return false;
    }
    let threshold = state.equipped_ruse_chance() + bonus;
    oteryn_simulation_determinism::deterministic_decision_u64(root, occurrence, "attack.ruse", 0)
        .is_ok_and(|draw| draw % 10_001 < threshold)
}

impl ChannelAttackStates {
    /// Crystal player.cpp:10489-10517 runs Transcendence on the player's combat
    /// cadence. The server keeps one deterministic draw per two-second slot.
    pub(in crate::gameplay_transport) fn trigger_forge_avatar(
        &self,
        state: &mut crate::spell::cast::PlayerSpellState,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
        digest: &[u8; 32],
    ) {
        use crate::spell::Vocation;
        use crate::spell::combat_batch::{AvatarState, OwnerCombatChange};
        use oteryn_gameplay_domain::domain::condition::ConflictKey;
        use sha2::{Digest, Sha256};

        const CADENCE_MICROS: u64 = 2_000_000;
        // Crystal config.lua.dist:230 and player_wheel.cpp:3295-3330.
        const DURATION_MS: u64 = 7_000;

        let Some(event) =
            oteryn_gameplay_foundation::scheduled_events::current_rates().exaltation_overload
        else {
            return;
        };
        let recent_attack = self
            .party_attack_state(actor, session)
            .is_some_and(|attack| attack.swung_within_interval(now, CADENCE_MICROS))
            || state.forge_spell_attack_within(now.get(), CADENCE_MICROS);
        if (now.get() % CADENCE_MICROS) >= 1_000_000
            || !recent_attack
            || state
                .owned_conditions()
                .active_at(ConflictKey::Attributes, now.get())
                .is_some_and(|condition| condition.definition().key() == "spell.avatar.attributes")
        {
            return;
        }
        let threshold =
            state.equipped_transcendence_chance() + u64::from(event.transcendence_permyriad());
        let hash = Sha256::new()
            .chain_update(b"oteryn:forge-avatar:v1")
            .chain_update(session.as_bytes())
            .chain_update(actor.placement_identity())
            .chain_update((now.get() / CADENCE_MICROS).to_be_bytes())
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&hash[..16]);
        let root = GameplayDecisionRoot::from_bytes(*digest);
        if !oteryn_simulation_determinism::deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "attack.transcendence",
            0,
        )
        .is_ok_and(|draw| draw % 10_001 < threshold)
        {
            return;
        }
        // The vocation avatar appearances are the same as the qualified native spells.
        let outfit_look_type = match state.character_facts().vocation {
            Vocation::Knight | Vocation::EliteKnight => 1593,
            Vocation::Paladin | Vocation::RoyalPaladin => 1594,
            Vocation::Sorcerer | Vocation::MasterSorcerer => 1595,
            Vocation::Druid | Vocation::ElderDruid => 1596,
            Vocation::Monk | Vocation::ExaltedMonk => 1823,
            Vocation::None => return,
        };
        let now_ms = now.get().div_ceil(1_000);
        let _ = crate::spell::actor_conditions::apply_combat_change(
            state,
            &OwnerCombatChange::Avatar(AvatarState {
                expires_ms: now_ms.saturating_add(DURATION_MS),
                outfit_look_type,
                incoming_reduction_percent: 15,
                critical_chance_percent: 100,
                critical_extra_percentage_points: 15,
            }),
            now_ms,
        );
    }
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
