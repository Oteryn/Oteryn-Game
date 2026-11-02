use super::*;
pub(super) async fn stash(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::stash, client_ui::stash as ui};
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
