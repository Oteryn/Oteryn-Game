//! C20c client replay capture: query the immutable receipt left by the real blessing-death row.

use super::*;
use oteryn_protocol_oteryn::death_protection::{
    CAPABILITY_DEATH_PROTECTION_V1, COMMAND_TYPE_DEATH_PROTECTION_QUERY, DeathProtectionReply,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(capture(clients)).await;
    let (status, evidence_value) = match result {
        Ok(visible) => ("PASS", visible),
        Err(error) => ("FAIL", error.to_string()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C20c.client status={status} evidence={evidence_value}"
    ));
}

async fn capture(clients: &SeamClients<'_>) -> TestResult<String> {
    // full_e2ef6 leaves Character 1's latest receipt at the Twist-only PvP death.
    let mut client = connect_character(clients, 0xc2, 1).await?;
    let result = client
        .session_mut()
        .gated_command(
            CAPABILITY_DEATH_PROTECTION_V1,
            COMMAND_TYPE_DEATH_PROTECTION_QUERY,
            &[],
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("death-protection query refused: {:?}", result.status).into());
    }
    let reply = DeathProtectionReply::decode(&result.payload)
        .ok_or("death-protection server payload was not canonical")?;
    let receipt = reply.0.ok_or("death-protection server returned no latest death")?;
    let visible = receipt.consumed.join("\n");
    if visible != "oteryn:blessing.twist_of_fate" {
        return Err(format!("latest real death consumed unexpected protection: {visible}").into());
    }

    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = Path::new(&directory);
        for (name, bytes) in [
            ("c20c-death-protection.bin", result.payload.as_slice()),
            ("c20c-expected.txt", visible.as_bytes()),
        ] {
            if let Err(error) = std::fs::write(directory.join(name), bytes) {
                evidence(&format!(
                    "SEAM_EVIDENCE e2e_all scenario=C20c.client status=CAPTURE-WRITE-ERROR evidence={name}:{error}"
                ));
            }
        }
    }
    Ok(visible)
}
