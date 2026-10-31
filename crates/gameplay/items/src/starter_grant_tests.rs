use super::*;
#[test]
fn newhaven_all_five_kits_have_exact_source_counts_slots_and_quiver_stacks() {
    for (vocation, mana, health, len) in [
        ("sorcerer", 10, 2, 12),
        ("druid", 10, 2, 12),
        ("paladin", 7, 5, 13),
        ("knight", 2, 10, 11),
        ("monk", 5, 7, 10),
    ] {
        let Some(rows) = newhaven_starter_items(vocation) else {
            assert!(false, "known vocation");
            return;
        };
        assert_eq!(rows.len(), len);
        for (id, slot) in [(3355, 1), (3561, 4), (3559, 7), (3552, 8), (34017, 10)] {
            assert!(rows.contains(&StarterItem {
                tibia_id: id,
                quantity: 1,
                destination: StarterDestination::Equipment(slot)
            }));
        }
        for (id, quantity) in [(268, mana), (266, health), (3031, 50), (3578, 1)] {
            assert!(rows.contains(&StarterItem {
                tibia_id: id,
                quantity,
                destination: StarterDestination::Backpack
            }));
        }
        assert_eq!(
            rows.iter()
                .filter(|r| r.destination == StarterDestination::Quiver)
                .count(),
            if vocation == "paladin" { 2 } else { 0 }
        );
        assert!(
            rows.iter()
                .filter(|r| r.destination == StarterDestination::Quiver)
                .all(|r| r.tibia_id == 3447 && r.quantity == 100)
        );
        assert!(!rows.iter().any(|r| r.tibia_id == 2854));
    }
    for key in ["none", "elite_knight", "unknown"] {
        assert!(newhaven_starter_items(key).is_none());
    }
}
#[test]
fn first_entry_backpack_template_is_unchanged() {
    let t = StarterTemplate::main();
    assert_eq!(t.key, "oteryn:starter-template.main");
    assert_eq!(t.definition.production_key, "oteryn:item.tibia.i2854");
    assert_eq!(t.quantity, 1);
}
