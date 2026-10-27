//! Numerical Item contributions from the same fenced cast equipment snapshot.
use crate::spell::{SpellDefinition, cast::PlayerSpellState, owned_cast_facts::OwnedCastFacts};
use oteryn_gameplay_content::content::item_spell_augments::{
    ItemSpellAugment, ItemSpellAugmentKind as K,
};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;

fn selected<'a>(
    owned: &'a OwnedCastFacts,
    spell: &'a SpellDefinition,
) -> impl Iterator<Item = &'a ItemSpellAugment> {
    owned
        .magnitude()
        .into_iter()
        .flat_map(|a| &a.item_augments)
        .filter(move |a| {
            matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
                && a.spell.eq_ignore_ascii_case(&spell.name)
        })
}

pub(super) fn cooldown(
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    next: &mut PlayerSpellState,
    anchor: &mut crate::spell::combat_batch::SpellAnchor,
    now: SemanticTimeMicros,
) -> Result<(), SpellCastDisposition> {
    let reduction = selected(owned, spell)
        .filter(|a| a.kind == K::Cooldown)
        .try_fold(0u64, |sum, a| sum.checked_add(u64::from(a.value) * 1000))
        .ok_or(SpellCastDisposition::Rejected)?;
    if reduction > 0 {
        let current = next
            .cooldowns
            .spell_ready_at(&spell.key)
            .ok_or(SpellCastDisposition::Rejected)?;
        let duration = current
            .get()
            .saturating_sub(now.get())
            .saturating_sub(reduction)
            .max(spell.cooldown_micros / 2);
        next.cooldowns.spells.insert(
            spell.key.clone(),
            now.checked_add(duration)
                .map_err(|_| SpellCastDisposition::Rejected)?,
        );
        anchor.cooldown_deadlines = next.cooldowns.canonical_deadlines();
    }
    Ok(())
}

pub(super) fn leech(
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    before: &PlayerSpellState,
) -> (u32, u32) {
    let mut rates = (0u32, 0u32);
    for a in selected(owned, spell) {
        match a.kind {
            K::LifeLeech => rates.0 = rates.0.saturating_add(a.value),
            K::ManaLeech => rates.1 = rates.1.saturating_add(a.value),
            _ => {}
        }
    }
    if rates != (0, 0) {
        let base = before.equipped_offence();
        rates.0 = rates.0.saturating_add(u32::from(base.life_leech));
        rates.1 = rates.1.saturating_add(u32::from(base.mana_leech));
    }
    rates
}

pub(super) fn alters_base(owned: &OwnedCastFacts, spell: &SpellDefinition) -> bool {
    selected(owned, spell).any(|a| a.value > 0 && matches!(a.kind, K::BaseDamage | K::BaseHealing))
}
