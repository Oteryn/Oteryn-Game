//! C20c client replay capture: query the immutable receipt left by the real blessing-death row.

use super::*;
use oteryn_protocol_oteryn::death_protection::{
    CAPABILITY_DEATH_PROTECTION_V1, COMMAND_TYPE_DEATH_PROTECTION_QUERY, DeathProtection, DeathProtectionReply,
};

pub(super) async fn run_packet(prerequisite: impl Future<Output = TestResult>) {
    let result = verdict::observe(prerequisite).await;
    let (status, evidence_value) = match result {
        Ok(()) => ("PASS", "current-run-twist-death-receipt".to_owned()),
        Err(error) => ("FAIL", error.to_string()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C20c.client status={status} evidence={evidence_value}"
    ));
}

fn twist_receipt_matches(receipt: &DeathProtection, expected: [u8; 16]) -> bool {
    receipt.occurrence == expected && receipt.consumed == ["oteryn:blessing.twist_of_fate"]
}

pub(super) async fn capture_receipt(client: &mut oteryn_dev_client::DevClientSession, expected_occurrence: [u8; 16]) -> TestResult {
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
    if !twist_receipt_matches(&receipt, expected_occurrence) {
        return Err(format!("C20c expected Twist-only death {expected_occurrence:?}, observed {receipt:?}").into());
    }
    let visible = receipt.consumed.join("\n");

    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = Path::new(&directory);
        for (name, bytes) in [
            ("c20c-death-protection.bin", result.payload.as_slice()),
            ("c20c-expected.txt", visible.as_bytes()),
        ] {
            std::fs::write(directory.join(name), bytes)?;
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_twist_receipt_cannot_replace_the_captured_death() {
        let receipt = DeathProtection {
            occurrence: [2; 16],
            consumed: vec!["oteryn:blessing.twist_of_fate".into()],
        };
        assert!(!twist_receipt_matches(&receipt, [1; 16]));
        assert!(twist_receipt_matches(&receipt, [2; 16]));
    }
}
