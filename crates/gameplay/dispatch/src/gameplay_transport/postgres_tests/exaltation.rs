//! Existing admitted metadata and real fenced6116 dispatcher; outer fixture compares all assets.
use super::{
    TestResult,
    item_forge::support::{self, Case, send_command},
};
use oteryn_protocol_oteryn::{CommandStatus, forge_preview as wire, item_view::ItemHandle};
#[test]
fn exaltation_postgres_real_effect_descriptions_are_readonly_and_handle_fenced() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("exaltation", || support::scenario(Case::Preview))
}
pub(super) async fn read_effects(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    command: &mut u64,
    handle: ItemHandle,
) -> TestResult {
    for (nonce, handle, valid) in [
        (21, handle, true),
        (22, ItemHandle::new(u64::MAX).ok_or("handle")?, false),
    ] {
        let payload = wire::Request {
            nonce,
            selection: wire::Selection::Effects(handle),
        }
        .encode()
        .ok_or("request")?;
        let (status, bytes) = send_command(
            client,
            generation,
            *command,
            wire::COMMAND_TYPE_FORGE_PREVIEW,
            &payload,
        )
        .await?;
        *command = command.checked_add(1).ok_or("effect command overflow")?;
        assert_eq!(status, CommandStatus::Accepted);
        let reply = wire::Reply::decode(&bytes).ok_or("effect reply")?;
        assert_eq!(reply.nonce, nonce);
        if !valid {
            assert_eq!(reply.payload, wire::Payload::Refused);
            continue;
        }
        let wire::Payload::Effects(e) = reply.payload else {
            return Err("real effect payload".into());
        };
        assert_eq!(e.item.handle, handle);
        assert_eq!(e.item.tier, 0);
        assert_eq!(e.item.maximum_tier, 1);
        assert_eq!(e.tiers, ["0", "1 (0.50% Onslaught)"]);
    }
    eprintln!(
        "EXALTATION POSTGRES_READ_PROOF admitted_metadata_exact_tiers_unissued_handle_refused_readonly"
    );
    Ok(())
}
