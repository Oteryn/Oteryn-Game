use super::*;
pub(super) async fn vip(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::vip, client_ui::vip as ui};
    if !client.selected_capabilities().contains(&43) {
        evidence(
            "mechanic=vip skipped=capability_43_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    let mut panel = ui::VipPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Query(0)).ok_or("vip request")?;
    let reply = vip::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.result.code != 1 || panel.pending || panel.view.is_none() {
        return Err("vip reducer did not load view".into());
    }
    evidence("mechanic=vip client_codec=real reducer=real state=loaded");
    Ok(())
}

pub(super) async fn add(
    client: &mut oteryn_dev_client::DevClientSession,
    name: &str,
) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::vip, client_ui::vip as ui};
    vip(client).await?;
    let mut panel = ui::VipPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Add(name.to_owned())).ok_or("VIP add")?;
    let reply = vip::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.result.code == 9 {
        evidence(
            "mechanic=vip skipped=write_service_unavailable_code_9_root_or_presence_relay source=crates/gameplay/dispatch/src/gameplay_transport/vip.rs:111",
        );
        return Ok(());
    }
    if reply.result.code != 1 {
        return Err(format!("VIP add refused: {:?}", reply.result).into());
    }
    let command = ui::request(&mut panel, ui::Intent::Query(0)).ok_or("VIP query after add")?;
    let reply = vip::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let view: serde_json::Value = serde_json::from_str(panel.view.as_deref().ok_or("VIP view")?)?;
    if reply.result.code != 1
        || !view["entries"].as_array().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry["name"].as_str() == Some(name))
        })
    {
        return Err("VIP added name absent from current query".into());
    }
    evidence("mechanic=vip add=committed current_list_contains_added_name=true");
    Ok(())
}
