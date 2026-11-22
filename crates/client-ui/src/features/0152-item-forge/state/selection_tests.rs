use super::*;
use oteryn_client_runtime::{cui3::item_forge::Operation, depot::ItemEntry, forge_preview as wire};
use std::num::{NonZeroU32, NonZeroU64};

fn item(handle: u64, name: &str, container: bool) -> wire::Item {
    wire::Item {
        handle: NonZeroU64::new(handle).unwrap_or(NonZeroU64::MIN),
        definition_ref: 1,
        appearance: 3264,
        name: name.into(),
        classification: 2,
        tier: 1,
        maximum_tier: 4,
        container,
    }
}
fn entry(handle: u64) -> ItemEntry {
    ItemEntry {
        appearance_id: 0,
        handle: NonZeroU64::new(handle).unwrap_or(NonZeroU64::MIN),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}
fn panel() -> ItemForgePanel {
    let mut panel = ItemForgePanel {
        selected: true,
        operation: Operation::Fusion,
        ..ItemForgePanel::default()
    };
    panel.preview.selected = true;
    panel.preview.remember(item(11, "Crimson blade", false));
    panel.preview.remember(item(12, "Iron helmet", false));
    panel.preview.remember(item(13, "Backpack", true));
    panel
}

#[test]
fn t116_item_selector_offers_named_non_container_items_and_rejects_unknown_handles() {
    let mut panel = panel();
    assert!(panel.choose_item(true, 11));
    assert!(panel.choose_item(false, 12));
    assert!(!panel.choose_item(true, 13), "a container is not forgeable");
    assert!(!panel.choose_item(true, 99), "an unknown handle is refused");
    assert_eq!((panel.first_item, panel.second_item), (11, 12));
}

#[test]
fn t116_item_preview_shows_names_and_tiers_instead_of_numeric_handles() {
    let mut panel = panel();
    assert!(panel.choose_item(true, 11));
    assert!(panel.choose_item(false, 12));
    let text = panel_text(&panel);
    assert!(
        text.contains(
            "Item: Crimson blade — class 2, tier 1 / 4 / Iron helmet — class 2, tier 1 / 4"
        )
    );
    assert!(!text.contains("11") && !text.contains("12"));
    let pair = selected_pair_text(&panel);
    assert!(pair.contains("Receiver: Crimson blade — class 2, tier 1 / 4"));
    assert!(pair.contains("Source / tier donor: Iron helmet"));
    let empty = ItemForgePanel::default();
    assert!(panel_text(&empty).contains("No current item selected / No current item selected"));
}

#[test]
fn t116_legacy_server_without_preview_labels_carried_items_without_the_handle() {
    let mut panel = ItemForgePanel {
        selected: true,
        operation: Operation::Fusion,
        ..ItemForgePanel::default()
    };
    panel.observe(vec![entry(7), entry(8)]);
    assert!(panel.choose_item(true, 7));
    let text = panel_text(&panel);
    assert!(text.contains("Carried item — name unavailable on this server"));
    assert!(!text.contains("7 /"));
}

#[test]
fn settled_pair_operations_invalidate_metadata_even_when_live_entries_are_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    for operation in [
        Operation::Fusion,
        Operation::ConvergenceFusion,
        Operation::Transfer,
        Operation::ConvergenceTransfer,
    ] {
        let mut panel = panel();
        panel.operation = operation;
        panel.occurrence = [1; 16];
        panel.occurrence[6] = 0x70;
        panel.occurrence[8] = 0x80;
        panel.first_item = 11;
        panel.second_item = 12;
        let live = vec![entry(11), entry(12)];
        panel.preview.live = live.clone();
        panel.preview.refresh = false;
        control_request(&mut panel).ok_or("pair request")?;
        let op = match operation {
            Operation::Fusion => "fusion",
            Operation::ConvergenceFusion => "convergence_fusion",
            Operation::Transfer => "transfer",
            Operation::ConvergenceTransfer => "convergence_transfer",
            _ => return Err("not a pair".into()),
        };
        let payload = serde_json::to_vec(&serde_json::json!({
            "operation": op, "succeeded": operation != Operation::Fusion, "applied": true, "reason": null, "bonus": 0,
            "result": { "dust": 0, "dust_limit": 100, "first_tier": 1, "second_tier": 0,
                "gold_spent": 100, "cores_spent": 0, "slivers_spent": 0, "slivers_created": 0, "cores_created": 0 }
        }))?;
        apply_result(
            &mut panel,
            &ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload,
            },
        );
        panel.observe(live);
        assert!(panel.preview.item(11).is_none());
        assert!(panel.preview.item(12).is_none());
        assert!(panel.preview.refresh);
        assert!(!selected_pair_text(&panel).contains("tier 1"));
    }
    Ok(())
}
