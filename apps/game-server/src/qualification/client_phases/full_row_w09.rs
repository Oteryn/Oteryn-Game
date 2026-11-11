//! W09 concoction effects/cooldowns and Drome Cube consume/reward, derived from the item-use owner `crates/gameplay/dispatch/src/gameplay_transport/drome_cube_use.rs`.
//!
//! 1. Admit the node-boot character and retain its complete native inventory view.
//! 2. Look for a Drome Cube and a concoction in that inventory; a player obtains neither through
//!    any route in the content (no NPC sells them and no creature drops them).
//! 3. Send ordinary USE for each by definition, exactly as a player's hotkey would; expect the
//!    server to refuse (nothing to consume) and the inventory to stay unchanged.
//! 4. Log out. The row cannot reach the consume/reward and cooldown steps without the items, so it
//!    reports FAIL naming the precondition; it never provisions state.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object as use_wire;
use std::collections::BTreeMap;

const DROME_CUBE: u32 = 36827;
const CONCOCTION: u32 = 36723;
const PROBES: [(&str, u32); 2] = [
    ("Drome Cube", DROME_CUBE),
    ("Kooldown Aid concoction", CONCOCTION),
];
const NEEDED: &str = "needed:drome-cube-36827-and-concoction-36723-in-inventory;no-player-route(no-npc-seller-in-content/services/trade;no-loot-source-in-content/loot)";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(90), journey(clients)).await;
    let detail = match result {
        // A held item would need the row's effect checks, which this driver does not implement.
        Ok(Ok(())) => "step=precondition-observed=item-present-effect-driver-missing".to_owned(),
        Ok(Err(error)) => compact(&error.to_string()),
        Err(_) => "step=journey-observed=timeout".to_owned(),
    };
    evidence(&format!(
        "e2e_all scenario=W09.full status=FAIL seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(400)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut result = BTreeMap::new();
    for entry in &client
        .inventory()
        .ok_or("step=admit observed=no-inventory")?
        .entries
    {
        *result.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    Ok(result)
}

/// The server's answer to a USE of an item the character does not carry.
fn describe(status: CommandStatus, payload: &[u8]) -> String {
    let split = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(payload);
    let disposition = split
        .ok()
        .and_then(|(body, _)| use_wire::decode_use_result(body).ok());
    format!("status:{status:?},disposition:{disposition:?}")
}

async fn use_by_definition(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<(CommandStatus, String)> {
    let payload = use_wire::encode_item_use_intent(&use_wire::UseIntent {
        target: use_wire::UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=use observed=encode-{error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            use_wire::CAPABILITY_ITEM_USE_V1,
            use_wire::COMMAND_TYPE_USE_INTENT,
            &payload,
        )
        .await?;
    Ok((reply.status, describe(reply.status, &reply.payload)))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0xd2).await?;
    let before = quantities(&client)?;
    let index = provisioning::item_index()?;
    let mut observed = Vec::new();
    for (label, id) in PROBES {
        let Ok(reference) = provisioning::item_reference(&index, id) else {
            observed.push(format!("{label}:not-in-pinned-item-index"));
            continue;
        };
        if before.contains_key(&reference.get()) {
            // The character holds it: the effect steps of this row are not driven here. Close the
            // session with an accepted logout so the next admission does not wait on loss/grace.
            if client.row_logout().await?.result
                != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
            {
                return Err("step=logout observed=refused".into());
            }
            return Ok(());
        }
        let (status, text) = use_by_definition(&mut client, reference).await?;
        if status == CommandStatus::Accepted && text.contains("Committed") {
            return Err(format!("step=use-{label} observed=committed-without-item:{text}").into());
        }
        observed.push(format!("{label}:{text}"));
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    if quantities(&client)? != before {
        return Err("step=refused-use observed=inventory-changed".into());
    }
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Err(format!("step=use observed={} {NEEDED}", observed.join(",")).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needed_precondition_is_one_report_token() {
        assert!(NEEDED.starts_with("needed:"));
        assert!(!compact(NEEDED).contains(char::is_whitespace));
    }

    #[test]
    fn compact_bounds_and_joins() {
        assert_eq!(compact("a  b\nc"), "a-b-c");
        assert_eq!(compact(" "), "unknown");
        assert_eq!(compact(&"x".repeat(900)).len(), 400);
    }
}
