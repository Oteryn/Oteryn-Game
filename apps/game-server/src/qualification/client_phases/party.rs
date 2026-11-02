use super::*;
pub(super) async fn party(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::party, client_ui::party as ui};
    let mut panel = ui::PartyPanel {
        selected: true,
        ..Default::default()
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("party request")?;
        let reply = party::send(client.session_mut(), &command).await?;
        if ui::apply_result(&mut panel, &reply) && panel.state.is_some() {
            break;
        }
        // A busy root refuses this read explicitly. Retry a fresh query, preserving the
        // requirement to decode and reduce the real state before advancing the scenario.
        let result = ui::party::decode_party_result(&reply.operation)
            .map_err(|error| format!("party refresh result: {error:?}"))?;
        if result.disposition != ui::party::PartyDisposition::Unavailable
            || tokio::time::Instant::now() >= deadline
        {
            return Err("party reducer did not load state".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    evidence("mechanic=party client_codec=real reducer=real state=loaded");
    Ok(())
}
