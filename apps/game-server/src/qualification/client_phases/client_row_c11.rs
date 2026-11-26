//! C11 live Battle Instinct query and native panel verification.

use super::*;
use oteryn_dev_client::client_ui::battle_instinct_status::{
    BattleInstinctStatusPanel, panel_text, request,
};
use oteryn_protocol_oteryn::battle_instinct_status as wire;
use std::io::Write;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(capture(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C11.client status=FAIL evidence={error}"
        ));
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xcd).await?;
    let mut panel = BattleInstinctStatusPanel {
        selected: client
            .selected_capabilities()
            .contains(&wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1),
        ..Default::default()
    };
    let payload = request(&mut panel).ok_or("C11 client did not build a Battle Instinct query")?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
            wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Battle Instinct query returned {:?}", reply.status).into());
    }
    let status = wire::BattleInstinctStatus::decode(&reply.payload)
        .ok_or("Battle Instinct query returned a malformed payload")?;
    if status != wire::BattleInstinctStatus::default() {
        return Err(format!(
            "C11 fresh character did not report inactive Battle Instinct: {status:?}"
        )
        .into());
    }
    if !panel.finish(true, &reply.payload)
        || panel.snapshot != Some(status)
        || panel.pending
        || panel.error.is_some()
        || panel_text(&panel) != "Battle Instinct: Inactive\r\nMelee: +0\r\nShielding: +0\r\n"
    {
        return Err(
            format!("C11 client did not render the live inactive snapshot: {panel:?}").into(),
        );
    }

    request(&mut panel).ok_or("C11 client did not build a refresh after the first query")?;
    let rejected = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
            wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
            &[0],
        )
        .await?;
    if rejected.status != CommandStatus::Rejected {
        return Err(format!("C11 malformed query returned {:?}", rejected.status).into());
    }
    if panel.finish(false, &rejected.payload)
        || panel.snapshot != Some(status)
        || panel.pending
        || panel.error != Some("Battle Instinct could not be refreshed. Try Refresh.")
    {
        return Err(format!("C11 client lost its snapshot on rejected refresh: {panel:?}").into());
    }

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = Path::new(&path);
        for (name, bytes) in [
            ("c11-battle-instinct.bin", reply.payload.as_slice()),
            ("c11-expected.txt", b"active=0 melee=0 shielding=0\n"),
        ] {
            std::fs::File::create_new(path.join(name))
                .and_then(|mut file| file.write_all(bytes))
                .map_err(|error| format!("C11 capture {name} failed: {error}"))?;
        }
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C11.client status=PASS evidence=active-{}-melee-{}-shielding-{}-native-panel-rejected-refresh",
        status.active, status.melee, status.shielding
    ));
    Ok(())
}
