use super::*;
pub(super) async fn guild(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::guild, client_ui::guild as ui};
    let mut panel = ui::GuildPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::RefreshOwn).ok_or("guild request")?;
    let reply = guild::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending {
        return Err("guild reducer remained pending".into());
    }
    evidence("mechanic=guild client_codec=real reducer=real state=loaded");
    Ok(())
}
