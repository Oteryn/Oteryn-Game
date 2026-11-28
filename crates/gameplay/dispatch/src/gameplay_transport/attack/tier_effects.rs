//! Global Exaltation Forge proc chances from currently equipped items.

use super::cyclopedia::amplification_ppm;
use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_simulation_determinism::deterministic_decision_u64;
use sha2::Digest;

/// TibiaWiki Ruse rates, in hundredths of a percent, indexed by armor tier.
const RUSE: [u64; 11] = [0, 50, 103, 162, 228, 300, 378, 462, 552, 648, 751];

fn tier_proc(
    rates: &[u64; 11],
    tier: u8,
    boots: u8,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    purpose: &str,
) -> bool {
    let base = rates.get(usize::from(tier)).copied().unwrap_or(0);
    if base == 0 {
        return false;
    }
    let multiplier = 1_000_000 + amplification_ppm(boots) as u64;
    let threshold = u128::from(base * multiplier) << 64;
    deterministic_decision_u64(root, occurrence, purpose, 0)
        .is_ok_and(|draw| u128::from(draw) * 10_000_000_000 < threshold)
}

pub(super) fn ruse(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    target: ExactActorRef,
    session: GameSessionId,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> bool {
    states.get(runtime, target, session).is_some_and(|state| {
        let equipment = state.equipped_offence();
        tier_proc(
            &RUSE,
            equipment.armor_tier,
            equipment.boots_tier,
            root,
            occurrence,
            "tier_ruse",
        )
    })
}

/// TibiaWiki Momentum rates, in hundredths of a percent, indexed by helmet tier.
const MOMENTUM: [u64; 11] = [0, 200, 405, 620, 845, 1080, 1325, 1580, 1845, 2120, 2405];
const TIER_CHECK_MICROS: u64 = 2_000_000;

fn periodic_occurrence(
    actor: ExactActorRef,
    session: GameSessionId,
    window: u64,
) -> DecisionOccurrenceId {
    let digest = sha2::Sha256::new()
        .chain_update(b"oteryn:forge-periodic:v1")
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(window.to_be_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    DecisionOccurrenceId::from_bytes(id)
}

impl ChannelAttackStates {
    /// Called from the periodic spell owner under its current runtime and spell guards.
    pub(in crate::gameplay_transport) fn tick_momentum(
        &mut self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        (actor, session): (ExactActorRef, GameSessionId),
        now: SemanticTimeMicros,
        book: &crate::spell::SpellBook,
        outside_protection: bool,
    ) {
        use crate::gameplay_transport::combat_lock_fact::{CombatLockFact, PvpLockSource};
        if !outside_protection
            || states.has_pending_spell_commit(actor, session)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return;
        }
        let lock = CombatLockFact::read(states, self, actor, session, now);
        if lock.pve_in_fight_until.is_none() && lock.pvp != PvpLockSource::LiveBlocked {
            return;
        }
        let Some(state) = states.get_mut(runtime, actor, session) else {
            return;
        };
        let equipment = state.equipped_offence();
        if equipment.helmet_tier == 0 || state.health == 0 {
            return;
        }
        let Some(entry) = self.entry(runtime, actor, session) else {
            return;
        };
        let window = now.get() / TIER_CHECK_MICROS;
        if entry
            .last_momentum_check
            .is_some_and(|previous| now.get().saturating_sub(previous.get()) < TIER_CHECK_MICROS)
        {
            return;
        }
        entry.last_momentum_check = Some(now);
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        if !tier_proc(
            &MOMENTUM,
            equipment.helmet_tier,
            equipment.boots_tier,
            &root,
            periodic_occurrence(actor, session, window),
            "tier_momentum",
        ) {
            return;
        }

        let before = state.cooldowns.clone();
        let shorten = |deadline: &mut SemanticTimeMicros| {
            if *deadline > now {
                *deadline = SemanticTimeMicros::from_micros(
                    deadline
                        .get()
                        .saturating_sub(TIER_CHECK_MICROS)
                        .max(now.get()),
                );
            }
        };
        for deadline in state.cooldowns.spells.values_mut() {
            shorten(deadline);
        }
        for (key, deadline) in &mut state.cooldowns.groups {
            // S9 loads primary first and secondary second. Preserve runtime groups,
            // and protect a group used as primary if a catalogue assigns both roles.
            if book
                .spells
                .iter()
                .any(|spell| spell.groups.get(1).is_some_and(|group| group.key == *key))
                && !book
                    .spells
                    .iter()
                    .any(|spell| spell.groups.first().is_some_and(|group| group.key == *key))
            {
                shorten(deadline);
            }
        }
        let after = state.cooldowns.clone();
        states.record_cooldown_changes(runtime, actor, session, &before, &after, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player};
    use crate::spell::CooldownGroup;
    use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;

    #[test]
    fn global_momentum_shortens_only_eligible_cooldowns_once_per_combat_window()
    -> Result<(), String> {
        let (runtime, actor, session) = runtime_with_player(0x81);
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let window = (1..20_000)
            .find(|window| {
                deterministic_decision_u64(
                    &root,
                    periodic_occurrence(actor, session, *window),
                    "tier_momentum",
                    0,
                )
                .is_ok_and(|draw| {
                    let roll = u128::from(draw) * 10_000;
                    (400_u128 << 64..405_u128 << 64).contains(&roll)
                })
            })
            .ok_or("test draw between 4.00% and 4.05%")?;
        let now = SemanticTimeMicros::from_micros(window * TIER_CHECK_MICROS + 1_500_000);
        let mut book = crate::spell::cast::v1_spell_book()?;
        let spell = book.spells.first_mut().ok_or("fixture spell")?;
        spell.groups = vec![
            CooldownGroup {
                key: "attack".into(),
                cooldown_micros: 5_000_000,
            },
            CooldownGroup {
                key: "special".into(),
                cooldown_micros: 5_000_000,
            },
        ];

        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now)
            .ok_or("initialize player")?;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("current player")?;
        state.install_equipped_offence(EquippedOffence {
            helmet_tier: 2,
            ..EquippedOffence::default()
        });
        state
            .cooldowns
            .rearm(spell, now)
            .map_err(|error| format!("{error:?}"))?;
        let original = now
            .checked_add(5_000_000)
            .map_err(|error| format!("{error:?}"))?;
        let short = now
            .checked_add(500_000)
            .map_err(|error| format!("{error:?}"))?;
        let spell_key = spell.key.clone();
        state.cooldowns.spells.insert(spell_key.clone(), original);
        state.cooldowns.spells.insert("short".into(), short);
        state
            .cooldowns
            .groups
            .insert("runtime:stairhop".into(), original);
        let before = state.cooldowns.clone();

        let mut attack = ChannelAttackStates::default();
        attack.tick_momentum(&runtime, &mut states, (actor, session), now, &book, true);
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("player")?
                .cooldowns,
            before
        );
        attack.record_hit_taken(&runtime, actor, session, now);
        attack.tick_momentum(&runtime, &mut states, (actor, session), now, &book, false);
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("player")?
                .cooldowns,
            before
        );
        attack.tick_momentum(&runtime, &mut states, (actor, session), now, &book, true);
        let reduced = states
            .get(&runtime, actor, session)
            .ok_or("player")?
            .cooldowns
            .clone();
        let shortened = now
            .checked_add(3_000_000)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(reduced.spell_ready_at(&spell_key), Some(shortened));
        assert_eq!(reduced.spell_ready_at("short"), Some(now));
        assert_eq!(reduced.group_ready_at("special"), Some(shortened));
        assert_eq!(reduced.group_ready_at("attack"), Some(original));
        assert_eq!(reduced.group_ready_at("runtime:stairhop"), Some(original));
        let too_early = now
            .checked_add(1_000_000)
            .map_err(|error| format!("{error:?}"))?;
        attack.tick_momentum(
            &runtime,
            &mut states,
            (actor, session),
            too_early,
            &book,
            true,
        );
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("player")?
                .cooldowns,
            reduced
        );
        assert_eq!(
            attack
                .entries
                .first()
                .and_then(|entry| entry.last_momentum_check),
            Some(now)
        );
        Ok(())
    }
}
