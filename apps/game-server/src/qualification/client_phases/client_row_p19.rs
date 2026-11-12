//! P19 client capture: MISSING-CLIENT. The client owners of the influenced/fiendish creature
//! marker are `Session::visible_appearances().forge_stacks` in
//! `crates/session/src/appearance_state.rs` and the scene mapping in
//! `apps/client/src/features/0006-world-spatial-entities`. The marker arrives in the appearance
//! binding snapshot and deltas, which the session decodes and drops: no raw payload is retained
//! (`Session::gated_snapshot` covers only gated capability domains), so nothing the server sent
//! can be replayed byte for byte, and a marked creature exists only after the 330 s spawn sweep
//! of E2EF7. This row drives the minimal real action (an admission with the forge capability
//! selected and the visible-appearance projection) and reports the gap.
use super::*;
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x53, 0).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("forge creature capability 104 was not selected".into());
    }
    let visible = client.session_mut().visible_appearances();
    let creatures = visible
        .iter()
        .filter(|entry| entry.entity.kind == oteryn_dev_client::EntityKind::Creature)
        .count();
    let marked = visible
        .iter()
        .filter(|entry| entry.forge_stacks.is_some())
        .count();
    let _ = client.session_mut().logout().await?;
    Ok(format!(
        "visible_entities={};creatures={creatures};marked={marked}",
        visible.len()
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=P19.client status=MISSING-CLIENT evidence={observed};searched=crates/session/src/appearance_state.rs,crates/session/src/tests/appearance_session_tests.rs,apps/client/src/features/0006-world-spatial-entities,crates/client-ui/src,crates/client-runtime/src,tools/dev-client/src;gap=no-raw-appearance-delta-retention"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P19.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P19.client status=FAIL evidence=timeout"),
    }
}
