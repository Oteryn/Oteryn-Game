//! Disconnect mid-action and resume; reference: `scenarios/s5_disconnect.rs`.
use super::*;
pub(super) async fn disconnect_resume(
    _client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<()> {
    // DevClientSession has no recovery/reconnect API. The separate recovery phase uses
    // real TLS and the same GameSession, but does not qualify this dev-client journey.
    // Keep this session intact for the caller's control-loss/terminal-release assertions.
    evidence("mechanic=disconnect_resume skipped=dev_client_has_no_recovery_reconnect_api");
    Ok(())
}
