//! G08: taming mounts (server package SRVPKG1, `gameplay_transport/mount_taming.rs`).
//! Construction self-use (Crystal `mounts.lua`): USE the carried construction object by
//! definition; the creature path needs the same carried taming item plus an adjacent source
//! creature.
//! 1. Admit the node-boot character and list the authored self-use construction rules from the
//!    active taming catalogue (`content/cosmetics/taming.json`).
//! 2. For each rule item send the real definition-targeted USE. Accepted + `Committed` must be
//!    followed by the state delta: the account appearance unlock row for the rule's grant key
//!    (read-only SQL), one unit of the item consumed, and the unlock still present after logout
//!    and a fresh admission.
//! 3. A reply of `NothingToUse` means the character carries none of the items; no player route
//!    in the run supplies them (they are quest/loot/store rewards), so the row ends FAIL naming
//!    the carried-item precondition. It never mints items.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::{self, UseDisposition, UseIntent, UseTarget};
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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
    .bind(clients.characters[1].as_slice())
    .bind(grant)
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(count > 0)
}

async fn use_by_definition(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
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
    let index = provisioning::item_index()?;
    let rules = crate::content::native_gameplay::mount_taming::rules();
    let mut client = connect_world(clients, 0xcb).await?;
    let mut tried = 0_u32;
    let mut observed = String::new();
    for rule in rules {
        let self_use = rule.target_item_key.as_deref() == Some(rule.item_key.as_str());
        let Some(id) = native_id(&rule.item_key).filter(|_| self_use) else {
            continue;
        };
        let Ok(definition) = provisioning::item_reference(&index, id) else {
            continue;
        };
        tried += 1;
        if unlocked(clients, &rule.grant_key).await? {
            // A grant that already exists cannot prove this USE created it.
            observed = format!("item={id},unlock-already-present");
            continue;
        }
        let before = carried(&client, definition);
        let (status, disposition) = use_by_definition(&mut client, definition).await?;
        if status == CommandStatus::Accepted && disposition == UseDisposition::Committed {
            if before == 0 || !unlocked(clients, &rule.grant_key).await? {
                return Err(format!(
                    "step=use item={id} observed=committed,carried:{before},unlock-row-absent"
                )
                .into());
            }
            // Debit side: exactly one unit leaves the character.
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            while carried(&client, definition) != before - 1 {
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "step=use item={id} observed=carried:{};needed={}",
                        carried(&client, definition),
                        before - 1
                    )
                    .into());
                }
                client.take_events();
                client.service_liveness(Duration::from_millis(50)).await?;
            }
            logout(&mut client).await?;
            drop(client);
            let mut fresh = connect_world(clients, 0xcc).await?;
            let durable = unlocked(clients, &rule.grant_key).await;
            let remaining = carried(&fresh, definition);
            logout(&mut fresh).await?;
            if !durable? {
                return Err(format!("step=fresh item={id} observed=unlock-lost").into());
            }
            if remaining != before - 1 {
                return Err(format!(
                    "step=fresh item={id} observed=carried:{remaining};needed={}",
                    before - 1
                )
                .into());
            }
            return Ok(());
        }
        observed =
            format!("item={id},carried={before},status={status:?},disposition={disposition:?}");
    }
    logout(&mut client).await?;
    Err(format!(
        "step=use observed=rules-tried:{tried};last:{observed};needed=carried-taming-item;content-trade-sells-none-of-the-51-taming-items"
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_id_reads_only_tibia_item_keys() {
        assert_eq!(native_id("oteryn:item.tibia.i12305"), Some(12305));
        assert_eq!(native_id("oteryn:creature.boar"), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
