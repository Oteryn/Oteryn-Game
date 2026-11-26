//! P19 requires a deterministic live Forge marker before the session can capture it.
use super::*;

pub(super) async fn run_packet(_clients: &SeamClients<'_>) {
    evidence("e2e_all scenario=P19.client status=FAIL evidence=BLOCKED-SERVER:qualification-spawn-has-no-deterministic-forge-classification");
}
