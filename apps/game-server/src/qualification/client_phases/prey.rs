use super::*;
pub(super) async fn prey(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::prey, client_ui::prey as ui};
    if !client.selected_capabilities().contains(&140) {
        evidence(
            "mechanic=prey skipped=capability_140_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    let mut panel = ui::PreyPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("prey request")?;
    let reply = prey::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || panel.confirmed.is_none() {
        return Err("prey reducer did not load state".into());
    }
    evidence("mechanic=prey client_codec=real reducer=real state=loaded");
    let before = panel
        .confirmed
        .as_ref()
        .and_then(|state| state.slots.iter().find(|slot| slot.state != 0))
        .cloned();
    let Some(before) = before else {
        evidence(
            "mechanic=prey skipped=no_unlocked_slot source=crates/gameplay/durability/src/durability/prey/storage.rs:99",
        );
        return Ok(());
    };
    let command = ui::request(&mut panel, ui::Intent::ListReroll { slot: before.slot })
        .ok_or("prey reroll")?;
    let reply = prey::send(client.session_mut(), &command).await?;
    let result = prey::prey::decode_reply(&reply.payload)
        .map_err(|error| format!("prey reroll reply: {error:?}"))?;
    let applied = ui::apply_result(&mut panel, &reply);
    if result.status != 0 {
        evidence(&format!(
            "mechanic=prey skipped=list_reroll_refused_status_{} source=crates/gameplay/durability/src/durability/prey/command.rs:227",
            result.status
        ));
        return Ok(());
    }
    let after = panel.slot(before.slot).ok_or("prey reroll lost slot")?;
    if !applied || after.creatures.is_empty() || after.free_reroll_at <= before.free_reroll_at {
        return Err(
            "prey reroll did not change the current slot deadline and populate candidates".into(),
        );
    }
    evidence(
        "mechanic=prey list_reroll=committed current_slot_candidates=nonempty free_reroll_deadline=advanced",
    );
    Ok(())
}
