//! Restartable coin outbox and expiry/matching. No connected client is required.
use super::*;
pub(super) async fn run(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    world: WorldId,
    stop: &CancellationToken,
) {
    let mut client = match crate::native_admission_source::coins::CoinClient::configured() {
        Ok(Some(client)) => client,
        _ => {
            stop.cancelled().await;
            return;
        }
    };
    let mut ticks = tokio::time::interval(Duration::from_secs(1));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if first(ticks.tick(), stop.cancelled()).await.is_none() {
            break;
        }
        if let Ok(owner) = root.coin_owner(authority, node, world, None) {
            crate::gameplay_transport::reconcile_coin_market(&owner, &mut client).await;
        }
    }
}
