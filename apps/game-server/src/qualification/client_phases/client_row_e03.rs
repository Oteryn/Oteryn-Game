//! E03 client replay capture: using a coin stack converts it to the next denomination. Client
//! owner: the CharacterInventory domain in `crates/session` (`Session::inventory`). The inventory
//! views are session-owned and re-encoded with the production encoder (C09/I11 practice).
//! A dedicated character receives the exact gold stack before live conversion.
use super::*;
use oteryn_protocol_oteryn::item_view;

use super::client_row_e02::{backpack_coins, coin_refs, wait_coins};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "E03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn pairs(client: &oteryn_dev_client::DevClientSession) -> String {
    client.inventory().map_or_else(String::new, |inventory| {
        inventory
            .entries
            .iter()
            .map(|item| format!("{}:{};", item.item_definition_ref, item.count))
            .collect()
    })
}

fn encoded(client: &oteryn_dev_client::DevClientSession) -> TestResult<Vec<u8>> {
    let inventory = client.inventory().ok_or("inventory absent")?;
    item_view::encode_character_inventory(inventory)
        .map_err(|error| format!("inventory encode: {error:?}").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_E03_CLIENT_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    character_prep::declare_seeded(
        "E03.client",
        "100 carried gold coins",
        "coin stack conversion",
    );
    let mut client = character_prep::Seeded {
        profile: "item-3031-100",
        reason: "coin stack conversion",
    }
    .connect(clients, 0x34, 0)
    .await?;
    let refs = coin_refs()?;
    let before = backpack_coins(&client, refs)?;
    let stack = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == refs[0] && item.count.get() == 100)
        })
        .copied()
        .ok_or("E03 served inventory has no single stack of 100 gold coins")?;
    let before_pairs = pairs(&client);
    let before_payload = encoded(&client)?;
    let outcome = client.use_item(stack.handle).await?;
    if outcome.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("E03 coin use refused: {:?}", outcome.disposition).into());
    }
    let after = [
        before[0]
            .checked_sub(100)
            .ok_or("E03 gold count below selected stack")?,
        before[1]
            .checked_add(1)
            .ok_or("E03 platinum count overflow")?,
        before[2],
    ];
    wait_coins(&mut client, refs, after, "E03 gold-to-platinum").await?;
    let after_pairs = pairs(&client);
    capture("e03-before.bin", &before_payload);
    capture("e03-after.bin", &encoded(&client)?);
    capture(
        "e03-expected.txt",
        format!("before={before_pairs}\nafter={after_pairs}\n").as_bytes(),
    );
    let _ = client.session_mut().logout().await;
    Ok(format!("before={before_pairs} after={after_pairs}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E03.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E03.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E03.client status=FAIL evidence=timeout"),
    }
}
