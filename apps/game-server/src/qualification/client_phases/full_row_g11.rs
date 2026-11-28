//! G11 imbuement details query (capability 447, command 731).
//! Provision one imbuable weapon on the dedicated character, then query its
//! eligibility with the panel's wire command. Verify the echoed handle and
//! ordered, bounded quotes on both admissions and compare the quote lists.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::ItemHandle;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::imbuement_details as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "imbuement-quotes-validated-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G11.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const CAPABILITY: u32 = wire::CAPABILITY_IMBUEMENT_DETAILS_V1;

fn quotes_ok(quotes: &[wire::Quote]) -> bool {
    !quotes.is_empty()
        && quotes
            .windows(2)
            .all(|pair| (pair[0].family, pair[0].tier) < (pair[1].family, pair[1].tier))
        && quotes.iter().all(|quote| {
            quote.success_percent <= 100 && quote.materials.len() <= wire::MAX_MATERIALS
        })
}

const WEAPON: u32 = 3349;

fn carried(client: &DevClientSession, definition: u32) -> Vec<(ItemHandle, u32)> {
    client
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref.get() == definition)
                .map(|entry| (entry.handle, entry.count.get()))
                .collect()
        })
        .unwrap_or_default()
}

fn expected_quotes() -> Vec<wire::Quote> {
    // Item 3349 admits Strike, leech, distance and the five elemental families.
    [1, 11, 12, 16, 21, 22, 23, 24, 25]
        .into_iter()
        .flat_map(|family| {
            [(1, 7_500, false), (2, 60_000, true), (3, 250_000, true)]
                .into_iter()
                .map(move |(tier, gold, requires_premium)| {
                    let recipe = oteryn_gameplay_items::imbuement::imbuement_recipe(family, tier);
                    wire::Quote {
                        family,
                        tier,
                        gold,
                        requires_premium,
                        recipe_available: recipe.is_some(),
                        success_percent: 100,
                        materials: recipe.map_or_else(Vec::new, |recipe| {
                            recipe
                                .materials
                                .iter()
                                .map(|&(key, quantity)| wire::Material {
                                    definition_key: key.into(),
                                    quantity,
                                })
                                .collect()
                        }),
                    }
                })
        })
        .collect()
}

async fn verify_unissued_handle(client: &mut DevClientSession) -> TestResult {
    let handle = ItemHandle::new(u64::MAX).ok_or("invalid control handle")?;
    let before = client
        .inventory()
        .ok_or("missing control inventory")?
        .clone();
    let (status, payload) = send(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_IMBUEMENT_DETAILS,
        &wire::encode_query(handle),
    )
    .await?;
    if status != CommandStatus::Rejected || !payload.is_empty() {
        return Err(format!(
            "step=unissued-handle observed={status:?}/{payload:?} needed=empty-Rejected"
        )
        .into());
    }
    if client.inventory() != Some(&before) {
        return Err("step=unissued-handle observed=inventory-changed".into());
    }
    Ok(())
}

async fn quotes_for(
    client: &mut DevClientSession,
    definition: u32,
) -> TestResult<Vec<wire::Quote>> {
    let inventory_before = client.inventory().ok_or("missing query inventory")?.clone();
    let handles = carried(client, definition);
    let [(handle, 1)] = handles.as_slice() else {
        return Err("step=query needed=exactly-one-fixture-weapon".into());
    };
    let (status, payload) = send(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_IMBUEMENT_DETAILS,
        &wire::encode_query(*handle),
    )
    .await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=query observed={status:?} needed=Accepted").into());
    }
    let reply =
        wire::decode_reply(&payload).map_err(|error| format!("step=decode observed={error:?}"))?;
    if reply.item != *handle {
        return Err("step=query observed=reply-for-another-item".into());
    }
    if !quotes_ok(&reply.quotes) {
        return Err(format!("step=quotes observed=invalid:{:?}", reply.quotes).into());
    }
    if reply.quotes != expected_quotes() {
        return Err(format!(
            "step=quotes observed={:?} needed=exact-fixture-offers",
            reply.quotes
        )
        .into());
    }
    if client.inventory() != Some(&inventory_before) {
        return Err("step=query observed=inventory-changed".into());
    }
    Ok(reply.quotes)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    character_prep::declare_seeded(
        "G11",
        "one imbuable weapon 3349",
        "imbuement eligibility query",
    );
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G11_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let definition = provisioning::item_reference(&provisioning::item_index()?, WEAPON)?.get();
    let mut client = connect_character(clients, 0x6f, 0).await?;
    if !carried(&client, definition).is_empty() {
        return Err("step=fixture observed=weapon-already-present".into());
    }
    verify_unissued_handle(&mut client).await?;
    evidence(
        "G11_CHECK step=prestate fixture_weapon_count=0 unissued_handle=Rejected inventory=unchanged",
    );
    character_prep::grant_item(&client, WEAPON, 1).await?;
    finish(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0x70, 0).await?;
    let outcome = verdict::observe(quotes_for(&mut client, definition)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let quotes = outcome?;
    closed?;
    evidence(&format!(
        "G11_CHECK step=initial fixture_weapon_count=1 quotes={}",
        quotes.len()
    ));
    drop(client);

    let mut fresh = connect_character(clients, 0x71, 0).await?;
    let checked = verdict::observe(quotes_for(&mut fresh, definition)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    if checked? != quotes {
        return Err("step=relog needed=quotes-unchanged-after-fresh-admission".into());
    }
    closed?;
    evidence(&format!(
        "G11_CHECK step=relog fixture_weapon_count=1 quotes={}",
        quotes.len()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn quotes_must_be_ordered_and_bounded() {
        let quote = |family, tier, success_percent| wire::Quote {
            family,
            tier,
            gold: 1,
            requires_premium: false,
            recipe_available: true,
            success_percent,
            materials: Vec::new(),
        };
        assert!(quotes_ok(&[
            quote(1, 1, 90),
            quote(1, 2, 60),
            quote(2, 1, 100)
        ]));
        assert!(!quotes_ok(&[]));
        assert!(!quotes_ok(&[quote(1, 2, 90), quote(1, 1, 90)]));
        assert!(!quotes_ok(&[quote(1, 1, 101)]));
    }
}
