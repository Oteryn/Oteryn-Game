//! Pure carrier eligibility for the gated legacy USE qualification.
use super::ordinary_combat::has_world_geometry;
use oteryn_gameplay_content::content::spell_data::{
    Carrier, Execution, SpellDefinition, SpellEffect,
};

/// The legacy USE qualification admits the Light Magic Missile carrier only.
/// Additional Rune families require their own target/gate qualification.
pub fn direct_rune(spell: &SpellDefinition) -> bool {
    matches!(spell.carrier, Carrier::Rune { item: 3174, .. })
        && !has_world_geometry(spell)
        && matches!(&spell.execution, Execution::Effects(effects)
            if !effects.is_empty() && effects.iter().all(|effect|
                matches!(effect, SpellEffect::Damage { .. } | SpellEffect::Heal { .. })))
}
