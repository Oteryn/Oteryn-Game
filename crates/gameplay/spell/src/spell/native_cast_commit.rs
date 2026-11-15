//! Native caster-side changes on the already paid, private player successor.
//! This never commits state. The transport must join the returned anchor to the
//! physical batch, compare both real owners, and install both in one owner turn.

use super::cast::{PaidNativeCast, PlayerSpellState};
use super::combat_batch::{CastOwnerChanges, LoweredNativeCombat, SpellAnchor};
use super::{SpellBook, SpellDefinition};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;

fn rejected<T>() -> Result<T, SpellCastDisposition> {
    Err(SpellCastDisposition::Rejected)
}
fn deadline(
    now: SemanticTimeMicros,
    milliseconds: u64,
) -> Result<SemanticTimeMicros, SpellCastDisposition> {
    now.checked_add(
        milliseconds
            .checked_mul(1000)
            .ok_or(SpellCastDisposition::Rejected)?,
    )
    .map_err(|_| SpellCastDisposition::Rejected)
}

impl PlayerSpellState {
    /// The source runs combat (including Beam cooldown reductions) before
    /// InstantSpell::postCastSpell adds this cast's exhaustion. Reconstruct
    /// that order from the actual predecessor rather than shortening the newly
    /// rearmed own cooldown or its newly rearmed groups.
    pub fn apply_native_owner_changes(
        &mut self,
        before: &Self,
        spell: &SpellDefinition,
        book: &SpellBook,
        changes: &CastOwnerChanges,
        now: SemanticTimeMicros,
    ) -> Result<SpellAnchor, SpellCastDisposition> {
        if self.payment_anchor_from(before).is_none()
            || book
                .spells
                .iter()
                .filter(|entry| entry.key == spell.key)
                .count()
                != 1
        {
            return rejected();
        }
        // Resolve every fallible binding/deadline before modifying even this
        // private successor. A source name is not a canonical identity key.
        let own_deadline = changes
            .cooldown_ms
            .map(|ms| deadline(now, ms))
            .transpose()?;
        let peer = match &changes.shared_cooldown {
            Some((name, milliseconds)) => {
                let mut matches = book
                    .spells
                    .iter()
                    .filter(|entry| entry.name.eq_ignore_ascii_case(name));
                let entry = matches.next().ok_or(SpellCastDisposition::Rejected)?;
                if matches.next().is_some() || entry.key == spell.key {
                    return rejected();
                }
                Some((entry.key.clone(), deadline(now, *milliseconds)?))
            }
            None => None,
        };
        let reduction = changes
            .reduce_all_spell_cooldowns_ms
            .checked_mul(1000)
            .ok_or(SpellCastDisposition::Rejected)?;
        let mut cooldowns = self.cooldowns.clone();
        if reduction > 0 {
            cooldowns.groups = before.cooldowns.groups.clone();
            for (key, previous) in &before.cooldowns.spells {
                if previous.get() <= now.get() {
                    continue;
                }
                let Some(instant) = book.spells.iter().find(|entry| {
                    &entry.key == key && matches!(entry.carrier, super::Carrier::Instant { .. })
                }) else {
                    continue;
                };
                if let Some(group) = instant.groups.get(1)
                    && let Some(ready) = cooldowns.groups.get_mut(&group.key)
                    && ready.get() > now.get()
                {
                    *ready = SemanticTimeMicros::from_micros(
                        ready.get().saturating_sub(reduction).max(now.get()),
                    );
                }
                if key != &spell.key {
                    cooldowns.spells.insert(
                        key.clone(),
                        SemanticTimeMicros::from_micros(
                            previous.get().saturating_sub(reduction).max(now.get()),
                        ),
                    );
                }
            }
            // Common payment already rearmed this cast. Restore those groups
            // after reducing predecessor timers, just as source postCastSpell does.
            for group in &spell.groups {
                if let Some(ready) = self.cooldowns.groups.get(&group.key) {
                    cooldowns.groups.insert(group.key.clone(), *ready);
                }
            }
        }
        if let Some(ready) = own_deadline {
            cooldowns.spells.insert(spell.key.clone(), ready);
        }
        if let Some((key, ready)) = peer {
            cooldowns.spells.insert(key, ready);
        }
        let mut successor = self.clone();
        successor.cooldowns = cooldowns;
        let anchor = successor
            .payment_anchor_from(before)
            .ok_or(SpellCastDisposition::Rejected)?;
        *self = successor;
        Ok(anchor)
    }
}

/// Update the common caster anchor and the physical batch with the same final
/// cooldown successor. Magnitude/armor/critical and weapon-charge requirements
/// remain in `lowered.owner_changes` for their actual owners; this function
/// does not certify or silently discard them.
pub fn finalize_paid_combat_cast(
    before: &PlayerSpellState,
    spell: &SpellDefinition,
    book: &SpellBook,
    paid: &mut PaidNativeCast,
    lowered: &mut LoweredNativeCombat,
    now: SemanticTimeMicros,
) -> Result<(), SpellCastDisposition> {
    let super::native::Plan::Combat(plan) = &paid.plan else {
        return rejected();
    };
    if lowered.batch.anchor.as_ref() != Some(&paid.anchor)
        || lowered.batch.now_us != Some(now)
        || lowered.batch.now_ms != now.get() / 1000
        || !paid.next.paid_successor_of(before, &paid.anchor)
    {
        return rejected();
    }
    // These derived fields cannot be independently changed after planning.
    let valid = match plan.as_ref() {
        super::native_combat::NativeCombatPlan::Combat(combat) => {
            lowered.owner_changes.cooldown_ms == combat.cooldown_ms
                && lowered.owner_changes.shared_cooldown == combat.shared_cooldown
                && lowered.owner_changes.reduce_all_spell_cooldowns_ms
                    == combat.reduce_all_spell_cooldowns_ms
                && lowered.owner_changes.block_armor == combat.block_armor
                && lowered.owner_changes.use_weapon_charges == combat.use_weapon_charges
                && lowered.owner_changes.weapon_missile == combat.weapon_missile
                && lowered.owner_changes.resolve_critical_and_fatal_once
                    == combat.resolve_critical_and_fatal_once
        }
        super::native_combat::NativeCombatPlan::Avatar(avatar) => {
            lowered.owner_changes.cooldown_ms == Some(avatar.cooldown_ms)
        }
        super::native_combat::NativeCombatPlan::MassSpiritMend { cooldown_ms, .. } => {
            lowered.owner_changes.cooldown_ms == Some(*cooldown_ms)
        }
        super::native_combat::NativeCombatPlan::MonsterAi { cooldown_ms, .. } => {
            lowered.owner_changes.cooldown_ms == *cooldown_ms
        }
        super::native_combat::NativeCombatPlan::ManaShield { .. } => {
            lowered.owner_changes.cooldown_ms.is_none()
        }
    };
    if !valid {
        return rejected();
    }
    let anchor =
        paid.next
            .apply_native_owner_changes(before, spell, book, &lowered.owner_changes, now)?;
    paid.anchor = anchor.clone();
    lowered.batch.anchor = Some(anchor);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{AbilityOccurrence, RevisionSet};
    use crate::spell::cast::{CastContext, CharacterCastFacts, cast, v1_spell_book};
    use crate::spell::{Carrier, CooldownGroup, Vocation};
    use oteryn_protocol_oteryn::actor_spell::{SpellCastIntent, SpellTarget};
    use std::num::NonZeroU32;

    #[test]
    #[allow(
        clippy::expect_used,
        reason = "regression fixture construction must succeed"
    )]
    fn beam_reduces_active_instant_secondary_groups_before_rearming() {
        let mut book = v1_spell_book().expect("book");
        for spell in book.spells.iter_mut().skip(1) {
            spell.groups.push(CooldownGroup {
                key: "focus".into(),
                cooldown_micros: 2_000_000,
            });
        }
        let mut rune = book.spells[1].clone();
        rune.key = "test:rune".into();
        rune.carrier = Carrier::Rune {
            item: 9999,
            charges: 1,
            magic_level: 0,
        };
        book.spells.push(rune);
        let mut before = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::ElderDruid,
                level: 1000,
                magic_level: 100,
                max_health: 1000,
                max_mana: 1000,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("state");
        let now = SemanticTimeMicros::from_micros(1_000_000);
        for (key, time) in [
            (book.spells[1].key.as_str(), 10_000_000),
            (book.spells[2].key.as_str(), 9_000_000),
            (book.spells[3].key.as_str(), 500_000),
            ("test:rune", 10_000_000),
        ] {
            before
                .cooldowns
                .spells
                .insert(key.into(), SemanticTimeMicros::from_micros(time));
        }
        before
            .cooldowns
            .groups
            .insert("focus".into(), SemanticTimeMicros::from_micros(20_000_000));
        // The current cast must be admissible before its groups are rearmed.
        book.spells[0].groups.push(CooldownGroup {
            key: "current".into(),
            cooldown_micros: 2_000_000,
        });
        let mut next = cast(
            &book,
            &before,
            &SpellCastIntent {
                spell: NonZeroU32::new(1).expect("index"),
                target: SpellTarget::None,
                aim_at_target: false,
            },
            CastContext {
                caster: "actor:test",
                owner_scope: "test",
                occurrence: AbilityOccurrence::new(
                    "beam:test",
                    RevisionSet::new("r:1", "c:1", "w:1", "f:1", "s:1").expect("revisions"),
                )
                .expect("occurrence"),
                now,
                draw: &mut |low, _| low,
            },
        )
        .expect("paid cast");
        let changes = CastOwnerChanges {
            cooldown_ms: Some(4000),
            reduce_all_spell_cooldowns_ms: 3000,
            ..Default::default()
        };
        next.apply_native_owner_changes(&before, &book.spells[0], &book, &changes, now)
            .expect("owner changes");
        assert_eq!(next.cooldowns.groups["focus"].get(), 14_000_000);
        assert_eq!(next.cooldowns.spells[&book.spells[1].key].get(), 7_000_000);
        assert_eq!(next.cooldowns.spells[&book.spells[2].key].get(), 6_000_000);
        assert_eq!(next.cooldowns.spells[&book.spells[3].key].get(), 500_000);
        assert_eq!(next.cooldowns.spells["test:rune"].get(), 10_000_000);
        assert_eq!(next.cooldowns.spells[&book.spells[0].key].get(), 5_000_000);
        assert_eq!(next.cooldowns.groups["healing"].get(), 2_000_000);
        assert_eq!(next.cooldowns.groups["current"].get(), 3_000_000);
    }
}
