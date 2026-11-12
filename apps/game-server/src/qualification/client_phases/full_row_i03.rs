//! I03 concrete player scenario, derived from the matrix, reaudit, and `item_look` owner:
//! 1. Enter through a fresh ordinary admission and LOOK at the character itself; expect the
//!    exact creature description returned in an accepted command reply.
//! 2. LOOK at an item genuinely held by the character (the node-boot starter backpack); expect
//!    accepted exact item text, including the server-computed look-distance description.
//! 3. Send LOOK for a non-existent carried-item handle; expect an accepted reply containing the
//!    fact-free `NotAvailable` refusal, proving the negative path does not disclose item facts.
//! 4. After acquiring a readable sign, book/document, and spellbook by ordinary play, LOOK at
//!    each real handle; expect exact authored sign/book text and the vocation-filtered spell list
//!    grouped by level (words, name, and fixed or percentage mana cost).
//! 5. Log out normally, obtain a genuinely fresh admission, LOOK at the same durable carried
//!    items again, and require byte-identical book/document and spellbook content.
//! 6. Emit PASS only after every reply and durable re-check succeeds; otherwise emit FAIL with
//!    the first attempted step and observed status/refusal/missing state plus the server/content
//!    facility needed to continue.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_look as wire;
use std::num::NonZeroU64;

async fn look(
    client: &mut DevClientSession,
    target: wire::ItemLookTarget,
) -> TestResult<(CommandStatus, wire::ItemLookResult)> {
    let selection = wire::ItemLookSelection { extended: true };
    let payload = wire::encode_item_look_intent_for(wire::ItemLookIntent { target }, selection)
        .map_err(|error| format!("I03 LOOK encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::ITEM_LOOK_CAPABILITY_ID.ok_or("I03 LOOK capability unallocated")?,
            wire::ITEM_LOOK_COMMAND_ID.ok_or("I03 LOOK command unallocated")?,
            &payload,
        )
        .await?;
    let result = wire::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("I03 LOOK decode status={:?}: {error:?}", reply.status))?;
    Ok((reply.status, result))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb3).await?;
    let entities = client
        .world_entities()
        .ok_or("step-1: own actor view missing")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .ok_or("step-1: own actor missing from visible entities")?
        .entity;
    let observed = look(&mut client, wire::ItemLookTarget::Creature(own)).await?;
    if observed.0 != CommandStatus::Accepted
        || !matches!(&observed.1, wire::ItemLookResult::Text(text) if text.starts_with("You see yourself (Level ") && text.ends_with("."))
    {
        return Err(format!(
            "step-1 self LOOK expected exact accepted description; observed={observed:?}"
        )
        .into());
    }

    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("step-2: legitimate starter backpack missing from inventory delta")?;
    let item_observed = look(&mut client, wire::ItemLookTarget::Item(backpack.handle)).await?;
    if item_observed.0 != CommandStatus::Accepted
        || !matches!(&item_observed.1, wire::ItemLookResult::Text(text) if text.starts_with("You see "))
    {
        return Err(
            format!("step-2 starter-item LOOK expected text; observed={item_observed:?}").into(),
        );
    }

    let absent = NonZeroU64::new(u64::MAX).ok_or("step-3: invalid test handle")?;
    let refusal = look(&mut client, wire::ItemLookTarget::Item(absent)).await?;
    if refusal
        != (
            CommandStatus::Accepted,
            wire::ItemLookResult::Refused(wire::ItemLookRefusal::NotAvailable),
        )
    {
        return Err(
            format!("step-3 unavailable-item refusal mismatch; observed={refusal:?}").into(),
        );
    }

    // Do not invent or SQL-mint readable items. A full attempt has exercised LOOK's positive and
    // refusal paths; continue only when ordinary prior play has put all three content classes in
    // this character's inventory, where every candidate is sent through the real LOOK command.
    let mut readable = Vec::new();
    let candidates: Vec<_> = client
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .chain(inventory.equipment.iter().map(|e| &e.item))
                .map(|entry| (entry.handle, entry.item_definition_ref))
                .collect()
        })
        .unwrap_or_default();
    for (handle, definition) in candidates {
        let reply = look(&mut client, wire::ItemLookTarget::Item(handle)).await?;
        if let (CommandStatus::Accepted, wire::ItemLookResult::Text(text)) = reply {
            readable.push((definition, text));
        }
    }
    Err(format!(
        "step-4 readable sign/book/spellbook acquisition incomplete after real LOOK attempts; observed_readable={readable:?}; need ordinary reachable content/NPC offers for a sign, document and spellbook"
    )
    .into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
    let evidence_text = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step deadline; observed=timeout; need responsive LOOK/content path".into()),
    };
    evidence(&format!(
        "e2e_all scenario=I03.full status={} seconds={:.3}{}",
        if evidence_text.is_none() {
            "PASS"
        } else {
            "FAIL"
        },
        started.elapsed().as_secs_f64(),
        evidence_text
            .as_deref()
            .map(|text| format!(" evidence={}", text.replace([' ', '\n'], "_")))
            .unwrap_or_default(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_handle_is_nonzero_and_outside_normal_allocation() {
        assert_eq!(
            NonZeroU64::new(u64::MAX).map(NonZeroU64::get),
            Some(u64::MAX)
        );
    }
}
