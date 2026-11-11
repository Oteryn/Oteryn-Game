//! P19: the appearance snapshot is retained; missing marked-creature content is
//! a journey prerequisite failure, not evidence of a missing client implementation.
use super::*;
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut client = connect_character(clients, 0x53, 0).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        if !client
            .selected_capabilities()
            .contains(&CAPABILITY_FORGE_CREATURE_V1)
        {
            return Err("forge creature capability 104 was not selected".into());
        }
        let snapshot = client
            .session_mut()
            .gated_snapshot(oteryn_protocol_oteryn::appearance::STATE_DOMAIN_APPEARANCE)
            .ok_or("needed:appearance-snapshot")?
            .clone();
        if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            std::fs::write(
                Path::new(&root).join("p19-appearance.bin"),
                &snapshot.payload,
            )?;
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

        if marked == 0 {
            return Err("needed:visible-influenced-or-fiendish-creature".into());
        }
        let decoded = oteryn_protocol_oteryn::appearance::Snapshot::decode(&snapshot.payload)
            .map_err(|error| format!("appearance snapshot decode: {error:?}"))?;
        for entry in visible.iter().filter(|entry| entry.forge_stacks.is_some()) {
            if !decoded.bindings.iter().any(|binding| {
                binding.entity == entry.entity.entity && binding.forge_stacks == entry.forge_stacks
            }) {
                return Err(
                    "forge marker differs between received snapshot and visible appearance".into(),
                );
            }
        }
        Ok(format!(
            "visible_entities={};creatures={creatures};marked={marked}",
            visible.len()
        ))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=P19.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P19.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P19.client status=FAIL evidence=timeout"),
    }
}
