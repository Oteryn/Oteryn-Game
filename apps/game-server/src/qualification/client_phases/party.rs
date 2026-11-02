use super::*;
pub(super) async fn party(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::party, client_ui::party as ui};
    let mut panel = ui::PartyPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("party request")?;
    let reply = party::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) || panel.state.is_none() {
        return Err("party reducer did not load state".into());
    }
    evidence("mechanic=party client_codec=real reducer=real state=loaded");
    Ok(())
}
