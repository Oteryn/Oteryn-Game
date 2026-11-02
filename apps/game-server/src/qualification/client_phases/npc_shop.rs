use super::*;
pub(super) async fn npc_shop(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::npc_shop, client_ui::npc_shop as ui};
    if !client
        .selected_capabilities()
        .contains(&npc_shop::npc_shop::CAPABILITY_NPC_SHOP_V1)
    {
        evidence("mechanic=npc_shop skipped=client_prerequisite_routes_not_supported");
        return Ok(());
    }
    let value = npc_shop::npc_shop::NpcShopIntent {
        npc_actor: npc_shop::EntityRef {
            identity: [0; 16],
            generation: 0,
        },
        catalogue_revision: 0,
        offer_index: 0,
        side: npc_shop::NpcTradeSide::Sell,
        action: npc_shop::npc_shop::NpcShopAction::Look,
        expected_unit_price: 0,
    };
    let mut panel = ui::NpcShopPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Look(value)).ok_or("npc shop request")?;
    let reply = npc_shop::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending || panel.result.is_none() {
        return Err("npc shop reducer did not load result".into());
    }
    evidence("mechanic=npc_shop client_codec=real reducer=real state=decoded");
    Ok(())
}
