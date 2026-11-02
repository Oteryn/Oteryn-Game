use super::*;
pub(super) async fn stash(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::stash, client_ui::stash as ui};
    // Stash is bundle-world-only and requires Depot (37), whose Item view (4) is absent here.
    // Keep the phase strict whenever the serving node selects its mechanic.
    if !client.selected_capabilities().contains(&62) {
        evidence("mechanic=stash skipped=entry_room_owner_not_composed");
        return Ok(());
    }
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash request")?;
    let reply = stash::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending {
        return Err("stash reducer remained pending".into());
    }
    evidence("mechanic=stash client_codec=real reducer=real state=loaded");
    Ok(())
}
