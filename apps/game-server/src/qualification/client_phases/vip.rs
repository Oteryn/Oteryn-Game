use super::*;
pub(super) async fn vip(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::vip, client_ui::vip as ui};
    let mut panel = ui::VipPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Query(0)).ok_or("vip request")?;
    let reply = vip::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending || panel.view.is_none() {
        return Err("vip reducer did not load view".into());
    }
    evidence("mechanic=vip client_codec=real reducer=real state=loaded");
    Ok(())
}
