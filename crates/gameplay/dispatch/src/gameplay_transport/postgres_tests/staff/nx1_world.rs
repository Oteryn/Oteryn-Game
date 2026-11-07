//! NX-1 production command560 -> signed audit -> real pinned-world mutation/visibility.
use super::*;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::monk_save::postgres_tests::appearance_change_postgres::observer_visibility_tests as observer;
use crate::map::{
    boot::{BootPins, boot},
    overlay::TilePos,
};
use oteryn_protocol_oteryn::staff_tools::{StaffToolsReply, decode_reply};

struct Chain<'a> {
    signing: &'a Signing,
    seq: u64,
    previous: String,
    operation: u8,
}
impl Chain<'_> {
    async fn action(
        &mut self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        admitted: AdmittedSession,
        action: rules::Action,
    ) -> TestResult<(wire::SignedEnvelope, StaffToolsReply)> {
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        let request =
            self.signing
                .request(self.seq + 1, &self.previous, self.operation, now, action);
        self.operation += 1;
        let signed = self.signing.sign(&canonical(&request)?, false)?;
        let reply = send_tools(owner, admitted, &signed, false).await?;
        self.seq += 1;
        self.previous = sha(&signed.canonical);
        if reply.result.disposition == 1 && request.action.kind() != "INSPECT" {
            let receipt = rules::Receipt {
                activation_sha256: self.signing.config.newest_activation_sha256.clone(),
                control_seq: (self.seq + 1).to_string(),
                disposition: rules::ReceiptDisposition::Committed,
                environment: self.signing.config.environment.clone(),
                operation_id: request.operation_id,
                outcome_sha256: sha(&wire::encode_result(&reply.result)?),
                previous_link_sha256: self.previous.clone(),
                request_sha256: sha(&signed.canonical),
                roster_sha256: self.signing.config.allowed_signers_sha256.clone(),
            };
            let receipt = self.signing.sign(&canonical(&receipt)?, true)?;
            assert_eq!(
                send_tools(owner, admitted, &receipt, true)
                    .await?
                    .result
                    .disposition,
                3
            );
            self.seq += 1;
            self.previous = sha(&receipt.canonical);
        }
        Ok((signed, reply))
    }
}
async fn send_tools(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    signed: &wire::SignedEnvelope,
    receipt: bool,
) -> TestResult<StaffToolsReply> {
    let command = if receipt { 561 } else { 560 };
    let (status, bytes) = exchange_raw(owner, admitted, command, &wire::encode_envelope(signed)?)
        .await?
        .ok_or("staff tools response")?;
    assert_eq!(status, protocol::CommandStatus::Accepted);
    Ok(decode_reply(&bytes)?)
}
async fn applied(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    signed: &wire::SignedEnvelope,
) -> TestResult {
    let request: rules::Request = serde_json::from_slice(&signed.canonical)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current fence")?
        .item_fence();
    assert_eq!(
        owner
            .root
            .staff_world_effect_state(owner.character, owner.holder, fence, &request.operation_id)
            .await?,
        Some(crate::durability::staff::StaffWorldEffectState::Applied)
    );
    Ok(())
}
#[test]
fn nx1_staff_tools_teleport_bring_ghost_invisibility_and_notices_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario::run().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "NX1 world thread panicked")??;
    eprintln!(
        "NX-1 world tools PostgreSQL duration: {:?}",
        began.elapsed()
    );
    Ok(())
}

#[path = "nx1_world/scenario.rs"]
mod scenario;
