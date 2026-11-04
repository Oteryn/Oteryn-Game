use super::*;
pub(super) async fn imbue(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::imbue, client_ui::imbue as ui};
    if !client.selected_capabilities().contains(&68) {
        evidence(
            "mechanic=imbue skipped=capability_68_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    let items = client
        .inventory()
        .map(|inventory| inventory.entries.clone())
        .unwrap_or_default();
    if items.is_empty() {
        evidence(
            "mechanic=imbue skipped=no_direct_backpack_item_to_imbue source=crates/gameplay/dispatch/src/gameplay_transport/imbuement.rs:496",
        );
        return Ok(());
    }
    let mut chosen = None;
    for item in items {
        let mut panel = ui::ImbuePanel {
            selected: true,
            item: Some(item.handle),
            ..Default::default()
        };
        let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("imbue request")?;
        let reply = imbue::send(client.session_mut(), &command).await?;
        ui::apply_result(&mut panel, &reply);
        if reply.reply.is_none() || panel.pending {
            return Err("imbue query was not decoded".into());
        }
        if panel
            .window
            .as_ref()
            .is_some_and(|window| window.slot_count > 0 && !window.offers.is_empty())
        {
            chosen = Some(panel);
            break;
        }
    }
    let Some(mut panel) = chosen else {
        evidence(
            "mechanic=imbue skipped=no_direct_backpack_item_has_admitted_imbuement_slots_and_offers source=crates/gameplay/dispatch/src/gameplay_transport/imbuement.rs:216",
        );
        return Ok(());
    };
    let window = panel.window.as_ref().ok_or("imbue window")?;
    let offer = window.offers[0];
    let command = ui::request(&mut panel, ui::Intent::Apply).ok_or("imbue apply")?;
    let reply = imbue::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let result = reply
        .reply
        .as_ref()
        .ok_or("imbue apply had no decoded result")?
        .result;
    if result != ui::imbuement::ImbuementResult::Ok {
        let recipe = oteryn_gameplay_items::imbuement::imbuement_recipe(offer.family, offer.tier)
            .ok_or("imbue recipe")?;
        evidence(&format!(
            "mechanic=imbue skipped=apply_refused_{result:?} recipe={} materials={:?} gold={} source=crates/gameplay/dispatch/src/gameplay_transport/imbuement.rs:600",
            recipe.key, recipe.materials, recipe.apply_gold
        ));
        return Ok(());
    }
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("imbue verify")?;
    ui::apply_result(
        &mut panel,
        &imbue::send(client.session_mut(), &command).await?,
    );
    if panel.window.as_ref().is_none_or(|window| {
        !window.applied.iter().any(|applied| {
            applied.slot == 0
                && applied.family == offer.family
                && applied.tier == offer.tier
                && applied.remaining_ms > 0
        })
    }) {
        return Err("imbue apply absent from current item window".into());
    }
    evidence(
        "mechanic=imbue apply=committed current_slot_family_and_tier=matched remaining_ms=positive",
    );
    Ok(())
}
