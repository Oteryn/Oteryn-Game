//! C19a client replay capture: a freshly admitted Character receives its starter backpack in the
//! CharacterInventory join snapshot. Client owner: `Session::admit` snapshot decode in
//! `crates/session` (`Session::inventory`).
use super::*;
use oteryn_protocol_oteryn::item_view;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("C19A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let definitions = provisioning::item_index()?;
    let backpack_ref = provisioning::item_reference(&definitions, 2854)?;
    let mut client = connect_world(clients, 0x32).await?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("inventory join snapshot absent")?
        .clone();
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
        snapshot.revision,
        backpack.handle,
        backpack.item_definition_ref,
        backpack.count
    );
    for entry in &inventory.entries {
        expected.push_str(&format!(
            "entry={} {} {}\n",
            entry.handle, entry.item_definition_ref, entry.count
        ));
    }
    capture("c19a-snapshot.bin", &snapshot.payload);
    capture("c19a-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "backpack_ref={} entries={}",
        backpack.item_definition_ref,
        inventory.entries.len()
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
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
