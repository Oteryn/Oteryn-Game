use super::*;
pub(super) async fn prey(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::prey, client_ui::prey as ui};
    // Prey needs a native gameplay catalogue, which the qualification entry room does not compose.
    // Keep the phase strict whenever the serving node selects its mechanic.
    if !client.selected_capabilities().contains(&140) {
        evidence("mechanic=prey skipped=entry_room_owner_not_composed");
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
    Ok(())
}
