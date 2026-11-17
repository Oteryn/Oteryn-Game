//! Newhaven evidence derived from the admitted client's server snapshot.
use super::*;

pub(super) fn first_login(client: &oteryn_dev_client::DevClientSession) -> TestResult {
    let position = tile(client.world_spatial().actor_position)?;
    let passed = position == NEWHAVEN_ISLET_TEMPLE;
    evidence(&format!(
        "NEWHAVEN_STEP first-login {} server_position={position:?} vitals={:?}",
        if passed { "PASS" } else { "FAIL" },
        client.actor_vitals(),
    ));
    Ok(())
}
