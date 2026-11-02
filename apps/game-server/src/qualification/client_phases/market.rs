use super::*;
pub(super) async fn market(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::market, client_ui::market as ui};
    // Market is bundle-world-only and requires Depot (37), whose Item view (4) is absent here.
    // Keep the phase strict whenever the serving node selects its mechanic.
    if !client.selected_capabilities().contains(&63) {
        evidence("mechanic=market skipped=entry_room_owner_not_composed");
        return Ok(());
    }
    let mut panel = ui::MarketPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Browse).ok_or("market request")?;
    let reply = market::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err("market reducer rejected browse".into());
    }
    evidence("mechanic=market client_codec=real reducer=real state=loaded");
    Ok(())
}
