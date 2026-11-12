//! C16c capture gap: observed entity projections cannot substitute for the
//! Convince/Animate delta transcript. Synthetic unit fixtures stay in replay
//! tests; a real-capture run never writes manufactured entity identities.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>, _world: &BundleWorld) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let result: TestResult = verdict::observe(async {
        let mut client = connect_world(clients, 0xd5).await?;
        let observed = client.world_entities().map_or(0, |entities| entities.iter().count());
        let logout = client.session_mut().logout().await?;
        if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err("C16c capture admission logout refused".into());
        }
        evidence(&format!("C16C_CAPTURE_GAP entities={observed} needed=raw-convince-animate-deltas"));
        Ok(())
    }).await;
    match result {
        Ok(()) => evidence("e2e_all scenario=C16c.client status=MISSING-CLIENT seconds=0 evidence=source:crates/session/src/domain_dispatch.rs:1"),
        Err(error) => evidence(&format!("e2e_all scenario=C16c.client status=FAIL seconds=0 evidence={error}")),
    }
}
