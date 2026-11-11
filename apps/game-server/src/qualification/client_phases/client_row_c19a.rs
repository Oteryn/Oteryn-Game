//! C19a client replay capture: a freshly admitted Character receives its starter backpack in the
//! CharacterInventory join snapshot. Client owner: `Session::admit` snapshot decode in
//! `crates/session` (`Session::inventory`).
use super::*;
use oteryn_protocol_oteryn::item_view;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let definitions = provisioning::item_index()?;
    let backpack_ref = provisioning::item_reference(&definitions, 2854)?;
    let mut client = connect_world(clients, 0x32).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        // Re-encode the installed core store. Revision 1 below is the replay peer's
        // local baseline, not a revision invented for the live server.
        let snapshot = item_view::encode_character_inventory_with_equip_drop(
            client.inventory().ok_or("inventory state absent")?,
            true,
        )
        .map_err(|error| format!("inventory snapshot encode: {error:?}"))?;
        let inventory = client.inventory().cloned().ok_or("inventory not decoded")?;
        let backpack = inventory.main_backpack.ok_or("no starter backpack")?;
        if backpack.item_definition_ref != backpack_ref {
            return Err(format!(
                "main backpack definition {} is not item 2854 ({backpack_ref})",
                backpack.item_definition_ref
            )
            .into());
        }
        let mut expected = format!(
            "revision={}\nbackpack={} {} {}\n",
            1, backpack.handle, backpack.item_definition_ref, backpack.count
        );
        for entry in &inventory.entries {
            expected.push_str(&format!(
                "entry={} {} {}\n",
                entry.handle, entry.item_definition_ref, entry.count
            ));
        }
        capture("c19a-snapshot.bin", &snapshot)?;
        capture("c19a-expected.txt", expected.as_bytes())?;

        Ok(format!(
            "backpack_ref={} entries={}",
            backpack.item_definition_ref,
            inventory.entries.len()
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
            "e2e_all scenario=C19a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C19a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C19a.client status=FAIL evidence=timeout"),
    }
}
