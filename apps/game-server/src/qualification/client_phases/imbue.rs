use super::*;
pub(super) async fn imbue(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::imbue, client_ui::imbue as ui};
    let item = std::num::NonZeroU64::new(1).ok_or("imbue handle")?;
    let mut panel = ui::ImbuePanel {
        selected: true,
        item: Some(item),
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("imbue request")?;
    let reply = imbue::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending {
        return Err("imbue reducer remained pending".into());
    }
    evidence("mechanic=imbue client_codec=real reducer=real state=decoded");
    Ok(())
}
