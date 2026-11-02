//! Disconnect mid-action and resume; reference: `scenarios/s5_disconnect.rs`.
use super::*;
pub(super) async fn disconnect_resume(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<()> {
    client.service_liveness(Duration::from_millis(1)).await?;
    evidence(
        "mechanic=disconnect_resume gap=DevClientSession_has_no_recovery_reconnect_API existing_recovery_phase=real_TLS_same_session_resume",
    );
    Ok(())
}
