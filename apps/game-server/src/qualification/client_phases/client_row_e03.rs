//! E03 client replay capture: using a coin stack converts it to the next denomination. Client
//! owner: the CharacterInventory domain in `crates/session` (`Session::inventory`). The inventory
//! views are session-owned and re-encoded with the production encoder (C09/I11 practice).
//! The ordinary starter carries no coin stack, so the row reports FAIL until one is reachable.
use super::*;
use oteryn_protocol_oteryn::item_view;

const COINS: [u32; 3] = [3031, 3035, 3043];

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("E03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
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
    let definitions = provisioning::item_index()?;
    let mut client = connect_world(clients, 0x34).await?;
    let mut stack = None;
    for id in COINS {
        let reference = provisioning::item_reference(&definitions, id)?;
        stack = client.inventory().and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference && item.count.get() >= 100)
                .copied()
        });
        if stack.is_some() {
            break;
        }
    }
    let stack = stack.ok_or("no coin stack of 100 in the ordinary starter inventory; conversion needs a legitimate gold source (see E02)")?;
    let before_pairs = pairs(&client);
    let before_payload = encoded(&client)?;
    let outcome = client.use_item(stack.handle).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while pairs(&client) == before_pairs {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("coin use changed nothing disposition={:?}", outcome.disposition).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
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
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
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
