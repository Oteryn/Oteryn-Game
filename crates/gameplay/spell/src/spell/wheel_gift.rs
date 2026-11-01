//! Gift of Life in the existing vitals successor. Crystal 201189f
//! player_wheel.cpp:3166-3187,3952-4002 and game.cpp:7741-7747.
//! The pinned onThink decreases this timer before its combat/PZ guard (3367-3375):
//! elapsed online time counts; loading a saved remainder never counts offline time.

use super::cast::PlayerSpellState;
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::{collections::BTreeMap, sync::Arc};

pub const MAX_GIFT_COOLDOWN_MICROS: u64 = 30 * 60 * 60 * 1_000_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GiftOfLifeState {
    loaded: bool,
    remaining_micros: u64,
    anchored_at: u64,
    generation: u64,
    saved_generation: u64,
    /// Admitted instant-spell identities and their optional secondary group.
    /// Shared across staged vitals copies; no client cooldown metadata is accepted.
    instant_secondary_groups: Arc<BTreeMap<String, Option<String>>>,
}

impl GiftOfLifeState {
    fn remaining(&self, now: u64) -> Option<u64> {
        self.loaded.then_some(())?;
        let elapsed = now.checked_sub(self.anchored_at)?;
        Some(self.remaining_micros.saturating_sub(elapsed))
    }
}

impl PlayerSpellState {
    pub fn wheel_gift_loaded(&self) -> bool {
        self.wheel_gift.loaded
    }

    /// A current admission reads this once. A later stats refresh cannot reset a live timer.
    pub fn restore_wheel_gift(&mut self, remaining_micros: u64, now: u64) -> bool {
        if self.wheel_gift.loaded || remaining_micros > MAX_GIFT_COOLDOWN_MICROS {
            return false;
        }
        self.wheel_gift = GiftOfLifeState {
            loaded: true,
            remaining_micros,
            anchored_at: now,
            generation: 0,
            saved_generation: 0,
            instant_secondary_groups: Arc::default(),
        };
        true
    }

    /// Install the current admitted catalogue with the durable timer's initial load.
    /// S9 orders groups primary first, then the optional secondary group.
    pub fn install_wheel_gift_spellbook(&mut self, book: &super::SpellBook) {
        self.wheel_gift.instant_secondary_groups = Arc::new(
            book.spells
                .iter()
                .filter(|spell| matches!(&spell.carrier, super::Carrier::Instant { .. }))
                .map(|spell| {
                    (
                        spell.key.clone(),
                        spell.groups.get(1).map(|group| group.key.clone()),
                    )
                })
                .collect(),
        );
    }

    /// A trigger is checkpointed on the next owner pass; actor end saves elapsed time as well.
    pub fn wheel_gift_save(&self, now: u64, force: bool) -> Option<(u64, u64)> {
        let gift = &self.wheel_gift;
        if !force && gift.generation == gift.saved_generation {
            return None;
        }
        Some((gift.generation, gift.remaining(now)?))
    }

    /// An acknowledgement from an older snapshot cannot settle a newer activation.
    pub fn wheel_gift_saved(&mut self, generation: u64) {
        if generation == self.wheel_gift.generation {
            self.wheel_gift.saved_generation = generation;
        }
    }

    /// Stage healing, mana and cooldown changes before the same hit commits its one revision.
    /// Agony callers do not invoke this hook. A missing durable load fails the perk closed.
    pub(super) fn apply_wheel_gift(
        &mut self,
        damage: u32,
        now: u64,
    ) -> Result<(), SpellCastDisposition> {
        let stage = self
            .wheel_effects_at(now)
            .and_then(|effects| effects.revelation_stages.get("gift_of_life").copied())
            .unwrap_or(0);
        let (percent, hours) = match stage {
            1 => (20_u32, 30_u64),
            2 => (25, 20),
            3 => (30, 10),
            _ => return Ok(()),
        };
        if self.health == 0
            || self.facts.max_health == 0
            || damage < self.health
            || self.wheel_gift.remaining(now) != Some(0)
            || (u64::from(damage - self.health) * 100 / u64::from(self.facts.max_health))
                > u64::from(percent)
        {
            return Ok(());
        }
        let generation = self
            .wheel_gift
            .generation
            .checked_add(1)
            .ok_or(SpellCastDisposition::Rejected)?;
        self.health = self
            .health
            .saturating_add((u64::from(self.facts.max_health) * u64::from(percent) / 100) as u32)
            .min(self.facts.max_health);
        self.mana = self
            .mana
            .saturating_add((u64::from(self.facts.max_mana) * u64::from(percent) / 100) as u32)
            .min(self.facts.max_mana);
        // Crystal player_wheel.cpp:3422-3451 visits each active instant-spell condition.
        // Its secondary group is shortened once per spell, including shared groups;
        // primary groups and unrelated movement exhaustion are untouched.
        for (key, deadline) in &mut self.cooldowns.spells {
            if deadline.get() <= now {
                continue;
            }
            let Some(secondary) = self.wheel_gift.instant_secondary_groups.get(key) else {
                continue;
            };
            if let Some(group_deadline) = secondary
                .as_ref()
                .and_then(|group| self.cooldowns.groups.get_mut(group))
                && group_deadline.get() > now
            {
                *group_deadline = SemanticTimeMicros::from_micros(
                    group_deadline.get().saturating_sub(60_000_000).max(now),
                );
            }
            *deadline =
                SemanticTimeMicros::from_micros(deadline.get().saturating_sub(60_000_000).max(now));
        }
        self.wheel_gift.remaining_micros = hours * 60 * 60 * 1_000_000;
        self.wheel_gift.anchored_at = now;
        self.wheel_gift.generation = generation;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Vocation, actor_conditions::stage_creature_hit, cast::CharacterCastFacts};
    use super::*;

    fn actor(stage: u8) -> Result<PlayerSpellState, String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::EliteKnight,
                level: 1000,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 200,
            },
            0,
            0,
        )
        .ok_or_else(|| "invalid fixture".to_owned())?;
        state.premium_current = true;
        state.stored_promotion = true;
        state.premium_valid_until_micros = Some(u64::MAX);
        let mut effects =
            oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
        effects
            .revelation_stages
            .insert("gift_of_life".to_owned(), stage);
        state.install_wheel_effects(effects);
        assert!(state.restore_wheel_gift(0, 0));
        state.install_wheel_gift_spellbook(&super::super::cast::v1_spell_book()?);
        state.health = 100;
        state.mana = 0;
        Ok(state)
    }

    #[test]
    fn gift_lethal_hit_heals_hp_mana_and_reduces_spell_timer_once() -> Result<(), String> {
        let mut state = actor(1)?;
        let spell_key = super::super::cast::v1_spell_book()?
            .spells
            .first()
            .ok_or_else(|| "empty admitted book".to_owned())?
            .key
            .clone();
        state.cooldowns.spells.insert(
            spell_key.clone(),
            SemanticTimeMicros::from_micros(90_000_000),
        );
        state.cooldowns.groups.insert(
            "healing".to_owned(),
            SemanticTimeMicros::from_micros(2_000_000),
        );
        let (next, hit) =
            stage_creature_hit(&state, 250, 1_000_000).map_err(|e| format!("{e:?}"))?;
        assert_eq!(hit.health_after, 50);
        assert_eq!(next.vitals().mana, 100);
        assert_eq!(next.cooldowns.spells[&spell_key].get(), 30_000_000);
        assert_eq!(next.cooldowns.groups["healing"].get(), 2_000_000);
        assert_eq!(
            next.wheel_gift_save(1_000_000, false),
            Some((1, MAX_GIFT_COOLDOWN_MICROS))
        );
        assert_eq!(next.revision(), state.revision() + 1);
        let (dead, _) = stage_creature_hit(&next, 250, 2_000_000).map_err(|e| format!("{e:?}"))?;
        assert_eq!(dead.health, 0);
        assert_eq!(dead.wheel_gift.generation, 1);
        Ok(())
    }

    #[test]
    fn gift_reduces_each_active_instant_secondary_group_without_touching_other_groups()
    -> Result<(), String> {
        use super::super::{Carrier, CooldownGroup};
        let mut state = actor(3)?;
        let mut book = super::super::cast::v1_spell_book()?;
        let keys: Vec<_> = book.spells.iter().map(|spell| spell.key.clone()).collect();
        assert_eq!(keys.len(), 4);
        for (spell, group) in book
            .spells
            .iter_mut()
            .zip(["shared", "shared", "expired", "short"])
        {
            spell.groups.push(CooldownGroup {
                key: group.to_owned(),
                cooldown_micros: 180_000_000,
            });
        }
        let mut rune = book.spells[0].clone();
        rune.key = "gift_test_rune".to_owned();
        rune.carrier = Carrier::Rune {
            item: 9999,
            charges: 1,
            magic_level: 0,
        };
        book.spells.push(rune);
        state.install_wheel_gift_spellbook(&book);
        for key in &keys {
            state
                .cooldowns
                .spells
                .insert(key.clone(), SemanticTimeMicros::from_micros(100_000_000));
        }
        state
            .cooldowns
            .spells
            .insert(keys[2].clone(), SemanticTimeMicros::from_micros(1_000_000));
        for key in ["gift_test_rune", "unknown_spell"] {
            state
                .cooldowns
                .spells
                .insert(key.to_owned(), SemanticTimeMicros::from_micros(100_000_000));
        }
        for (key, deadline) in [
            ("shared", 180_000_000),
            ("expired", 180_000_000),
            ("short", 40_000_000),
            ("unrelated", 180_000_000),
            ("healing", 2_000_000),
            ("movement", 10_000_000),
        ] {
            state
                .cooldowns
                .groups
                .insert(key.to_owned(), SemanticTimeMicros::from_micros(deadline));
        }
        let (next, _) = stage_creature_hit(&state, 250, 1_000_000).map_err(|e| format!("{e:?}"))?;
        assert_eq!(next.cooldowns.groups["shared"].get(), 60_000_000);
        assert_eq!(next.cooldowns.groups["short"].get(), 1_000_000);
        for (key, deadline) in [
            ("expired", 180_000_000),
            ("unrelated", 180_000_000),
            ("healing", 2_000_000),
            ("movement", 10_000_000),
        ] {
            assert_eq!(next.cooldowns.groups[key].get(), deadline, "{key}");
        }
        assert_eq!(next.cooldowns.spells[&keys[0]].get(), 40_000_000);
        assert_eq!(next.cooldowns.spells[&keys[1]].get(), 40_000_000);
        assert_eq!(next.cooldowns.spells[&keys[2]].get(), 1_000_000);
        assert_eq!(next.cooldowns.spells[&keys[3]].get(), 40_000_000);
        assert_eq!(next.cooldowns.spells["gift_test_rune"].get(), 100_000_000);
        assert_eq!(next.cooldowns.spells["unknown_spell"].get(), 100_000_000);
        Ok(())
    }

    #[test]
    fn gift_refuses_missing_load_expired_premium_nonlethal_and_excess_overkill()
    -> Result<(), String> {
        for scenario in 0..4 {
            let mut state = actor(3)?;
            let damage = match scenario {
                0 => {
                    state.wheel_gift.loaded = false;
                    150
                }
                1 => {
                    state.premium_current = false;
                    150
                }
                2 => 99,
                _ => 410,
            };
            let (next, _) = stage_creature_hit(&state, damage, 0).map_err(|e| format!("{e:?}"))?;
            assert_eq!(next.wheel_gift.generation, 0, "scenario {scenario}");
            assert_eq!(next.vitals().mana, 0);
        }
        Ok(())
    }

    #[test]
    fn gift_saved_remainder_excludes_offline_time_and_reaches_zero() -> Result<(), String> {
        let mut original = actor(2)?;
        original
            .apply_wheel_gift(200, 0)
            .map_err(|e| format!("{e:?}"))?;
        let (generation, remaining) = original.wheel_gift_save(10_000_000, true).unwrap_or((0, 0));
        original.wheel_gift_saved(generation.saturating_sub(1));
        assert!(original.wheel_gift_save(10_000_000, false).is_some());
        original.wheel_gift_saved(generation);
        assert!(original.wheel_gift_save(10_000_000, false).is_none());
        let mut restored = actor(2)?;
        restored.wheel_gift = GiftOfLifeState::default();
        assert!(restored.restore_wheel_gift(remaining, 5));
        assert_eq!(
            restored.wheel_gift.remaining(5),
            Some(20 * 60 * 60 * 1_000_000 - 10_000_000)
        );
        assert_eq!(restored.wheel_gift.remaining(5 + remaining), Some(0));
        assert_eq!(restored.wheel_gift.remaining(4), None);
        assert!(!restored.restore_wheel_gift(0, 10));
        Ok(())
    }

    #[test]
    fn gift_never_triggers_for_agony() -> Result<(), String> {
        let state = actor(3)?;
        let (next, _) = super::super::actor_conditions::stage_agony_hit(&state, 150, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(next.health, 0);
        assert_eq!(next.wheel_gift.generation, 0);
        Ok(())
    }

    #[test]
    fn gift_paid_successor_cannot_overwrite_a_later_durable_timer_load() -> Result<(), String> {
        let mut before = actor(1)?;
        before.wheel_gift = GiftOfLifeState::default();
        let mut paid = before.clone();
        paid.advance_batch_revision()
            .map_err(|error| format!("{error:?}"))?;
        let anchor = paid
            .payment_anchor_from(&before)
            .ok_or("valid common payment")?;
        assert!(paid.paid_successor_of(&before, &anchor));

        let mut current = before.clone();
        assert!(current.restore_wheel_gift(7_000_000, 1_000_000));
        assert_eq!(current.vitals(), before.vitals());
        assert_eq!(current.revision(), before.revision());
        assert_eq!(
            current.wheel_gift_save(2_000_000, true),
            Some((0, 6_000_000))
        );
        assert!(
            !paid.paid_successor_of(&current, &anchor),
            "an old paid candidate must not replace the loaded cooldown with its unloaded snapshot"
        );
        assert_eq!(
            current.wheel_gift_save(2_000_000, true),
            Some((0, 6_000_000))
        );

        let mut fresh = current.clone();
        fresh
            .advance_batch_revision()
            .map_err(|error| format!("{error:?}"))?;
        assert!(fresh.payment_anchor_from(&current).is_some());
        Ok(())
    }

    #[test]
    fn gift_paid_successor_cannot_undo_a_later_checkpoint_acknowledgement() -> Result<(), String> {
        let (before, _) =
            stage_creature_hit(&actor(1)?, 150, 0).map_err(|error| format!("{error:?}"))?;
        let (generation, remaining) = before
            .wheel_gift_save(0, false)
            .ok_or("triggered Gift needs a checkpoint")?;
        assert_eq!(generation, 1);
        let mut paid = before.clone();
        paid.mana = paid.mana.checked_sub(1).ok_or("Gift restored mana")?;
        paid.advance_batch_revision()
            .map_err(|error| format!("{error:?}"))?;
        let anchor = paid
            .payment_anchor_from(&before)
            .ok_or("valid common payment")?;
        assert!(paid.paid_successor_of(&before, &anchor));

        let mut current = before.clone();
        current.wheel_gift_saved(generation);
        assert_eq!(current.vitals(), before.vitals());
        assert_eq!(current.revision(), before.revision());
        assert_eq!(current.wheel_gift_save(0, false), None);
        assert_eq!(
            current.wheel_gift_save(0, true),
            Some((generation, remaining))
        );
        assert!(
            !paid.paid_successor_of(&current, &anchor),
            "the old paid candidate cannot restore an already acknowledged dirty generation"
        );
        assert_eq!(current.wheel_gift_save(0, false), None);

        let mut fresh = current.clone();
        fresh.mana = fresh.mana.checked_sub(1).ok_or("Gift restored mana")?;
        fresh
            .advance_batch_revision()
            .map_err(|error| format!("{error:?}"))?;
        assert!(fresh.payment_anchor_from(&current).is_some());
        Ok(())
    }
}
