use super::*;
pub(super) async fn market(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::market, client_ui::market as ui};
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
