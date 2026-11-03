use super::*;
pub(super) async fn stash(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::stash, client_ui::stash as ui};
    if !client.selected_capabilities().contains(&62) {
        evidence(
            "mechanic=stash skipped=capability_62_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash request")?;
    let reply = stash::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending || reply.reply.outcome != ui::stash::StashOutcome::Opened {
        return Err("stash reducer remained pending".into());
    }
    evidence("mechanic=stash client_codec=real reducer=real state=loaded");
    let Some(item) = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
    else {
        evidence(
            "mechanic=stash skipped=no_carried_stack_to_stow source=crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs:373",
        );
        return Ok(());
    };
    let before: u64 = panel.rows.iter().map(|row| u64::from(row.quantity)).sum();
    let command = ui::request(&mut panel, ui::Intent::Stow(item.handle)).ok_or("stash stow")?;
    let reply = stash::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Committed {
        evidence(&format!(
            "mechanic=stash skipped=stow_refused_{:?}_definition_{} source=crates/gameplay/durability/src/durability/character_stash.rs:279",
            reply.reply.outcome, item.item_definition_ref
        ));
        return Ok(());
    }
    if reply.reply.stowed_quantity != item.count.get() {
        return Err("stash stow receipt quantity mismatch".into());
    }
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash verify")?;
    ui::apply_result(
        &mut panel,
        &stash::send(client.session_mut(), &command).await?,
    );
    let after: u64 = panel.rows.iter().map(|row| u64::from(row.quantity)).sum();
    if after != before + u64::from(item.count.get())
        || client.inventory().is_none_or(|inventory| {
            inventory
                .entries
                .iter()
                .any(|entry| entry.handle == item.handle)
        })
    {
        return Err(
            "stash conversion did not move the real inventory stack into current stash".into(),
        );
    }
    evidence(
        "mechanic=stash stow=committed inventory_handle=removed current_stash_quantity=increased",
    );
    Ok(())
}
