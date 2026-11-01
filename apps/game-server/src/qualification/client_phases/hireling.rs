use super::*;
pub(super) async fn hireling(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::hireling, client_ui::hireling as ui};
    let mut panel = ui::HirelingPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("hireling request")?;
    let reply = hireling::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || panel.active.is_none() {
        return Err("hireling reducer did not load state".into());
    }
    evidence("mechanic=hireling client_codec=real reducer=real state=loaded");
    Ok(())
}
