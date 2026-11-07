//! Crystal201189f871d23597b82caf88fd611454a2ddefb8:
//! data/scripts/actions/items/{blueberry,strawberry}_cupcake.lua:9 and
//! lemon_cupcake.lua:1-5,15: full pools or forced distance+10 for3600s.
use super::*;

/// Prepare a source-bound successor; the native one-unit USE owns its publication.
pub fn stage_cupcake(
    state: &PlayerSpellState,
    item: u32,
    now_ms: u64,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    if state.health == 0 {
        return rejected();
    }
    let mut next = match item {
        28484 => {
            state
                .after_mana_gain(u64::from(state.facts.max_mana))
                .ok_or(SpellCastDisposition::Rejected)?
                .0
        }
        28485 => {
            state
                .after_health_gain(u64::from(state.facts.max_health))
                .ok_or(SpellCastDisposition::Rejected)?
                .0
        }
        28486 => {
            use oteryn_gameplay_domain::domain::condition::{
                ConflictKey,
                attributes::{AttributeModifier, AttributeModifiers},
            };
            let mut next = state.clone();
            // Source FORCEUPDATE replaces the current default attribute condition.
            next.conditions.dispel_key(ConflictKey::Attributes);
            let modifiers = AttributeModifiers {
                distance: Some(AttributeModifier::Add(10)),
                ..Default::default()
            };
            apply_definition(
                &mut next,
                definition(
                    "item.seasonal.cupcake.lemon",
                    ConditionValues::SourceAttributes {
                        modifiers,
                        duration_ms: 3_600_000,
                    },
                )?,
                now_ms,
                0,
            )?;
            next
        }
        _ => return rejected(),
    };
    next.revision = state
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_cupcakes_fill_pools_and_force_exact_persistent_distance_condition()
    -> Result<(), SpellCastDisposition> {
        let mut state = PlayerSpellState::new(
            super::super::super::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::Knight,
                level: 100,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .ok_or(SpellCastDisposition::Rejected)?;
        state.health = 7;
        state.mana = 9;
        assert_eq!(stage_cupcake(&state, 28484, 0)?.mana, 500);
        assert_eq!(stage_cupcake(&state, 28485, 0)?.health, 1000);
        let lemon = stage_cupcake(&state, 28486, 0)?;
        let attributes = lemon.conditions.effective_attributes(1);
        assert_eq!(
            attributes.distance,
            Some(oteryn_gameplay_domain::domain::condition::attributes::AttributeModifier::Add(10))
        );
        let snapshot = lemon
            .conditions
            .persistent_snapshot(600_000_000)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let restored=oteryn_gameplay_foundation::foundation::condition::ConditionStore::from_persistent_snapshot(&snapshot,0).map_err(|_|SpellCastDisposition::Rejected)?;
        assert!(
            restored
                .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Attributes)
                .is_some_and(|condition| condition.remaining_duration_ms(0) == Some(3_000_000))
        );
        assert_eq!(restored.effective_attributes(3_000_000_000).distance, None);
        assert!(
            lemon
                .conditions
                .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Attributes)
                .is_some_and(|condition| condition.remaining_duration_ms(0) == Some(3_600_000))
        );
        assert!(stage_cupcake(&state, 1, 0).is_err());
        Ok(())
    }
}
