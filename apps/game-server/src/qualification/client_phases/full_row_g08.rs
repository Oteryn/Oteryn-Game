//! G08: use the authored Glooth Glider recipe and verify its ingredient debits and durable unlock.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::{self, UseDisposition, UseIntent, UseTarget};
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "construction-mount-unlocked-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G08.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn native_id(key: &str) -> Option<u32> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}

fn carried(client: &DevClientSession, definition: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

async fn unlocked(clients: &SeamClients<'_>, grant: &str) -> TestResult<bool> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlocks u JOIN game_character_roots r ON r.account_id = u.account_id WHERE r.character_id = encode($1, 'hex')::uuid AND u.unlock_key = $2",
    )
    .bind(clients.characters[0].as_slice())
    .bind(grant)
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(count > 0)
}

async fn use_construction_item(
    client: &mut DevClientSession,
    handle: oteryn_dev_client::ItemHandle,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=encode observed={error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("step=use observed=reply-decode:{error:?}"))?;
    Ok((reply.status, disposition))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    const ITEM: u32 = 21897;
    character_prep::declare_seeded(
        "G08",
        "one of each authored Glooth Glider construction ingredient",
        "construction taming",
    );
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G08_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let index = provisioning::item_index()?;
    let definition = provisioning::item_reference(&index, ITEM)?;
    let rule = crate::content::native_gameplay::mount_taming::rules()
        .iter()
        .find(|rule| {
            native_id(&rule.item_key) == Some(ITEM)
                && rule.target_item_key.as_deref() == Some(rule.item_key.as_str())
        })
        .ok_or("step=fixture observed=missing-authored-construction-rule")?;
    let recipe = oteryn_gameplay_items::item_cosmetics::rule(&rule.item_key)
        .ok_or("step=fixture observed=missing-item-construction-owner")?;
    let [choice] = recipe.choices.as_slice() else {
        return Err("step=fixture observed=expected-one-construction-result".into());
    };
    let [grant] = choice.grants.as_slice() else {
        return Err("step=fixture observed=expected-one-construction-grant".into());
    };
    let ingredients = recipe
        .required_items
        .iter()
        .map(|&id| Ok((id, provisioning::item_reference(&index, id)?)))
        .collect::<TestResult<Vec<_>>>()?;
    if ingredients.len() <= 1 || !ingredients.iter().any(|(_, item)| *item == definition) {
        return Err("step=fixture observed=invalid-glider-recipe".into());
    }

    let mut client = connect_character(clients, 0xcb, 0).await?;
    let matches_count = |client: &DevClientSession, count| {
        client.inventory().is_some()
            && ingredients
                .iter()
                .all(|(_, ingredient)| carried(client, *ingredient) == count)
    };
    if !matches_count(&client, 0) || unlocked(clients, grant).await? {
        return Err("step=fixture observed=ingredient-or-unlock-already-present".into());
    }
    for &(id, _) in &ingredients {
        character_prep::grant_item(&client, id, 1).await?;
    }
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xcc, 0).await?;
    if !matches_count(&client, 1) {
        return Err(
            "step=fixture observed=expected-exactly-one-served-unit-of-each-ingredient".into(),
        );
    }
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == definition)
        })
        .ok_or("step=fixture observed=construction-trigger-not-in-backpack")?
        .handle;
    let (status, disposition) = use_construction_item(&mut client, handle).await?;
    if status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("step=use item={ITEM} observed={status:?}/{disposition:?}").into());
    }
    if !unlocked(clients, grant).await? {
        return Err("step=use observed=committed-without-unlock".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !matches_count(&client, 0) {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=use observed=recipe-ingredients-not-all-consumed".into());
        }
        client.take_events();
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xcd, 0).await?;
    let durable = unlocked(clients, grant).await;
    let debited = matches_count(&fresh, 0);
    logout(&mut fresh).await?;
    if !durable? || !debited {
        return Err(format!("step=fresh observed=unlock-or-debit-lost;debited={debited}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_id_reads_only_tibia_item_keys() {
        assert_eq!(native_id("oteryn:item.tibia.i21897"), Some(21897));
        assert_eq!(native_id("oteryn:creature.boar"), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
