//! Visible inventory rows carry live handles internally; users select rows, never handles.
use super::{EntityRef, Intent, PlayerTradePanel};
use oteryn_client_runtime::player_trade::ItemEntry;
pub use oteryn_client_runtime::player_trade::TradeItems;

pub fn rows(items: &TradeItems) -> Vec<(&'static str, ItemEntry)> {
    let mut rows = Vec::new();
    rows.extend(
        items
            .inventory
            .main_backpack
            .map(|item| (tr!("Backpack slot"), item)),
    );
    rows.extend(
        items
            .inventory
            .entries
            .iter()
            .map(|item| (tr!("Backpack"), *item)),
    );
    rows.extend(
        items
            .inventory
            .equipment
            .iter()
            .map(|item| (tr!("Equipment"), item.item)),
    );
    rows.extend(
        items
            .corpse
            .entries
            .iter()
            .map(|item| (tr!("Open container"), *item)),
    );
    for view in &items.containers.views {
        rows.extend(view.entries.iter().map(|item| (tr!("Container"), *item)));
    }
    let mut seen = std::collections::BTreeSet::new();
    rows.retain(|(_, item)| seen.insert(item.handle));
    rows
}

pub fn select_partner(panel: &mut PlayerTradePanel, partner: EntityRef) -> bool {
    if !panel.selected || panel.pending || panel.items.pending {
        return false;
    }
    panel.partner = Some(partner);
    panel.offer = None;
    panel.feedback = Some(tr!("Choose an inventory item, then Offer selected"));
    true
}

pub fn select_item(panel: &mut PlayerTradePanel, row: usize) -> bool {
    let Some(partner) = panel.partner.or_else(|| panel.state.as_ref()?.partner) else {
        panel.feedback = Some(tr!("Choose Trade with from a player's world context menu"));
        return false;
    };
    let Some((_, item)) = rows(&panel.items).get(row).copied() else {
        return false;
    };
    panel.offer = Some((partner, item.handle.get()));
    true
}

pub fn offer_selected(panel: &mut PlayerTradePanel) -> Option<super::PlayerTradeCommand> {
    let (partner, item) = panel.offer?;
    if !rows(&panel.items)
        .iter()
        .any(|(_, entry)| entry.handle.get() == item)
    {
        panel.offer = None;
        panel.feedback = Some(tr!("That item is no longer available; choose again"));
        return None;
    }
    super::request(panel, Intent::Offer { partner, item })
}

pub fn picker_text(panel: &PlayerTradePanel) -> Vec<String> {
    rows(&panel.items)
        .iter()
        .map(|(source, item)| {
            lfmt!(
                "{source}: item {} x{} (subtype {})",
                item.item_definition_ref,
                item.count,
                item.sub_type,
                source = source
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::player_trade::{CommandStatus, PlayerTradeResponse, player_trade};

    fn item() -> ItemEntry {
        ItemEntry {
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        }
    }
    fn partner() -> EntityRef {
        EntityRef {
            identity: [4; 16],
            generation: 9,
        }
    }

    #[test]
    fn picker_encodes_live_row_and_rejects_removed_item_and_busy_session()
    -> Result<(), &'static str> {
        let mut panel = PlayerTradePanel {
            selected: true,
            ..Default::default()
        };
        panel.items.inventory.entries.push(item());
        assert!(select_partner(&mut panel, partner()));
        assert!(select_item(&mut panel, 0));
        let command = offer_selected(&mut panel).ok_or("offer missing")?;
        assert_eq!(
            command.intent,
            player_trade::Intent::Offer {
                partner: partner(),
                item: 1
            }
        );
        assert!(offer_selected(&mut panel).is_none());
        assert!(!select_partner(&mut panel, partner()));
        panel.pending = false;
        panel.items.inventory.entries.clear();
        assert!(offer_selected(&mut panel).is_none());
        assert!(panel.offer.is_none());
        assert!(
            panel
                .feedback
                .is_some_and(|text| text.contains("no longer available"))
        );
        panel.items.pending = true;
        assert!(!select_partner(&mut panel, partner()));
        Ok(())
    }

    #[test]
    fn incoming_partner_can_choose_counter_offer_without_trade_id() {
        let mut panel = PlayerTradePanel {
            selected: true,
            state: Some(player_trade::State {
                partner: Some(partner()),
                phase: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        panel.items.inventory.entries.push(item());
        assert!(select_item(&mut panel, 0));
        assert_eq!(panel.offer, Some((partner(), 1)));
        assert!(!select_item(&mut panel, 99));
    }

    #[test]
    fn picker_merges_visible_sources_without_duplicate_handles() {
        let mut items = TradeItems::default();
        items.inventory.entries.push(item());
        items.corpse.entries.push(item());
        assert_eq!(rows(&items), vec![("Backpack", item())]);
    }

    #[test]
    fn all_authoritative_refusals_and_terminal_states_reach_text() {
        for code in [
            player_trade::ResultCode::TooFar,
            player_trade::ResultCode::NotTradeable,
            player_trade::ResultCode::PartnerBusy,
            player_trade::ResultCode::NoRoom,
            player_trade::ResultCode::NoBackpack,
            player_trade::ResultCode::Stale,
            player_trade::ResultCode::NotSupported,
            player_trade::ResultCode::Rejected,
            player_trade::ResultCode::Cancelled,
        ] {
            let mut panel = PlayerTradePanel {
                selected: true,
                pending: true,
                partner: Some(partner()),
                offer: Some((partner(), 1)),
                ..Default::default()
            };
            super::super::apply_result(
                &mut panel,
                &PlayerTradeResponse {
                    status: CommandStatus::Accepted,
                    result: code,
                    state: Default::default(),
                },
            );
            assert!(!panel.pending);
            assert_eq!(panel.offer, Some((partner(), 1)));
            assert!(super::super::panel_text(&panel).contains(code.message()));
        }
    }
}

#[cfg(test)]
#[test]
fn refusal_keeps_partner_so_another_inventory_item_can_be_offered() -> Result<(), &'static str> {
    use oteryn_client_runtime::player_trade::{CommandStatus, PlayerTradeResponse, player_trade};
    let partner = EntityRef {
        identity: [4; 16],
        generation: 2,
    };
    let mut panel = PlayerTradePanel {
        selected: true,
        ..Default::default()
    };
    panel.items.inventory.entries = [1_u64, 2]
        .into_iter()
        .map(|handle| ItemEntry {
            handle: std::num::NonZeroU64::new(handle).unwrap_or(std::num::NonZeroU64::MIN),
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        })
        .collect();
    assert!(select_partner(&mut panel, partner));
    assert!(select_item(&mut panel, 0));
    assert!(offer_selected(&mut panel).is_some());
    super::apply_result(
        &mut panel,
        &PlayerTradeResponse {
            status: CommandStatus::Accepted,
            result: player_trade::ResultCode::NotTradeable,
            state: Default::default(),
        },
    );
    assert!(select_item(&mut panel, 1));
    assert_eq!(
        offer_selected(&mut panel).ok_or("retry missing")?.intent,
        player_trade::Intent::Offer { partner, item: 2 }
    );
    Ok(())
}
