use crate::imbue::*;

fn panel() -> ImbuePanel {
    ImbuePanel {
        selected: true,
        details_selected: true,
        item: Some(ItemHandle::MIN),
        window: Some(imbuement::ImbuementWindow {
            slot_count: 3,
            applied: vec![imbuement::AppliedImbuement {
                slot: 1,
                family: 2,
                tier: 1,
                remaining_ms: 7_200_000,
            }],
            offers: vec![
                imbuement::ImbuementOffer {
                    family: 4,
                    tier: 2,
                    gold: 90,
                    requires_premium: false,
                },
                imbuement::ImbuementOffer {
                    family: 5,
                    tier: 3,
                    gold: 250,
                    requires_premium: true,
                },
            ],
            clear_gold: 12,
        }),
        ..ImbuePanel::default()
    }
}

#[test]
fn t115_slot_view_selects_only_existing_slots_and_labels_empty_and_applied_slots() {
    let mut panel = panel();
    assert_eq!(slot_label(&panel, 0), "Slot 1: empty");
    assert!(slot_label(&panel, 1).starts_with("Slot 2: family"));
    assert!(slot_label(&panel, 1).contains("7200 seconds left"));
    assert_eq!(slot_label(&panel, 3), "");
    assert!(panel.select_slot(1));
    assert!(panel.can_clear());
    assert!(!panel.can_apply());
    assert!(panel.select_slot(2));
    assert!(!panel.can_clear());
    assert!(!panel.select_slot(3));
    assert_eq!(panel.slot, 2);
    panel.pending = true;
    assert!(!panel.select_slot(0));
    assert_eq!(panel.slot, 2);
}

#[test]
fn t115_cost_and_chance_detail_follows_the_selected_offer_and_its_quote() {
    let mut panel = panel();
    assert!(panel.select_offer(1));
    assert!(!panel.select_offer(2));
    let text = selected_detail_text(&panel);
    assert!(text.contains("Clear cost: 12 gold"));
    assert!(text.contains("Apply cost: 250 gold (premium required)"));
    assert!(text.contains("Success chance: 100%"));
    assert!(text.contains("Material details are unavailable"));
    panel.details = Some(oteryn_client_runtime::imbuement_details::Reply {
        item: ItemHandle::MIN,
        quotes: vec![oteryn_client_runtime::imbuement_details::Quote {
            family: 5,
            tier: 3,
            gold: 250,
            requires_premium: true,
            recipe_available: true,
            success_percent: 100,
            materials: vec![oteryn_client_runtime::imbuement_details::Material {
                definition_key: "oteryn:item.gold_token".into(),
                quantity: 4,
            }],
        }],
    });
    let text = selected_detail_text(&panel);
    assert!(text.contains("Quoted cost: 250 gold; success chance: 100%"));
    assert!(text.contains("4 × oteryn:item.gold_token"));
    assert!(panel.select_slot(0));
    assert!(panel.can_apply());
    panel.details = None;
    assert!(!panel.can_apply());
}
