//! EQMOD2: content -> fenced equipment -> framed attack intent -> real combat owners.
use super::equipment_mods_fixtures::{activate};

use super::party_xp_fixtures::{self as party_xp, Case};
use crate::bestiary_postgres_harness::{TestResult, debug, id};
use crate::foundation::WorldId;

#[test]
fn eqmod2_postgres_content_cleave_reflection_perfect_shot_and_amplification() -> TestResult {
    party_xp::run(Case::Eqmod2)
}

#[test]
fn eqmod2_content_forge_tiers_and_modifiers_are_projected() -> TestResult {
    use crate::durability::character_equipment::{EquipmentSnapshot, EquippedItem};
    use crate::durability::item_mint::TypedDefinitionRef;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = activate(world)?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let mut equipment = EquipmentSnapshot {
        character: id(41),
        content_digest: native.source_digest(),
        character_revision: 1,
        revision: 1,
        combat_mode: None,
        items: [(6, "3264", 10), (4, "6529", 10)]
            .into_iter()
            .map(|(slot, key, tier)| EquippedItem {
                slot,
                tier,
                item_instance_id: id(slot),
                quantity: 1,
                state_revision: 1,
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: format!("oteryn:item.tibia.i{key}"),
                    revision_ref: "definition-r1".into(),
                },
            })
            .collect(),
    };
    let offence =
        crate::gameplay_transport::equipment_move::equipment_effects_owner::equipped_offence(
            &equipment, native,
        )
        .ok_or("offence")?;
    assert_eq!(
        (offence.weapon_tier, offence.boots_tier, offence.cleave),
        (10, 10, 10_000)
    );
    assert_eq!(
        (
            offence.critical_chance,
            offence.life_leech,
            offence.reflect_percent,
            offence.reflect_flat
        ),
        (10_000, 5_000, 1_000, 99)
    );
    assert_eq!(offence.perfect_damage(1, 0), 42);
    equipment.items[1].tier = 11;
    assert!(
        crate::gameplay_transport::equipment_move::equipment_effects_owner::equipped_offence(
            &equipment, native
        )
        .is_none()
    );
    Ok(())
}
