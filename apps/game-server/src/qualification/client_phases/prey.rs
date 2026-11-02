use super::*;
pub(super) async fn prey(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::prey, client_ui::prey as ui};
    let mut panel = ui::PreyPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("prey request")?;
    let reply = prey::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || panel.reply.is_none() {
        return Err("prey reducer did not load state".into());
    }
    evidence("mechanic=prey client_codec=real reducer=real state=loaded");
    Ok(())
}
