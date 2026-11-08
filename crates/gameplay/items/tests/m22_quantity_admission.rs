use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use oteryn_gameplay_items::item_craft::{CarriedCookingConversion, CookingConversion};

fn conversion(captured: bool, debit: u32) -> CarriedCookingConversion {
    CarriedCookingConversion {
        conversion: CookingConversion {
            recipe: "oteryn:item-conversion.m22-fixture".into(),
            outputs: vec![(
                TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i22008".into(),
                    revision_ref: "definition-r1".into(),
                },
                1,
            )],
        },
        target_debit: debit,
        requires_quantity_witness: captured,
    }
}

#[test]
fn captured_whole_stack_cannot_fall_back_to_a_quantity_blind_owner() {
    for debit in [1, 7, 100] {
        assert!(
            conversion(true, debit)
                .into_quantity_blind_conversion()
                .is_none(),
            "a guessed count of one must not substitute for the locked target quantity"
        );
    }
}

#[test]
fn existing_fixed_single_unit_conversion_remains_admitted() -> Result<(), &'static str> {
    let resolved = conversion(false, 1)
        .into_quantity_blind_conversion()
        .ok_or("fixed recipe")?;
    assert_eq!(resolved.outputs.len(), 1);
    assert_eq!(resolved.outputs[0].1, 1);
    Ok(())
}

#[test]
fn quantity_blind_owner_also_refuses_inconsistent_fixed_debits() {
    for debit in [0, 2, 100, u32::MAX] {
        assert!(
            conversion(false, debit)
                .into_quantity_blind_conversion()
                .is_none()
        );
    }
}
