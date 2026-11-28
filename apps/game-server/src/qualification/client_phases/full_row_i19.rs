//! I19: exercise both live enchanted ruby recipes and verify their pushed inventory.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use oteryn_protocol_oteryn::item_view::{CharacterInventory, ItemHandle};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, decode_use_result, encode_item_use_intent,
};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

const RUBY: u32 = 676;
const RECIPES: [(u32, u32); 2] = [(8077, 8078), (9020, 9019)];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    let detail = detail.split_whitespace().collect::<Vec<_>>().join("-");
    evidence(&format!(
        "e2e_all scenario=I19.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn quantities(inventory: &CharacterInventory) -> BTreeMap<NonZeroU32, u64> {
    let mut values = BTreeMap::new();
    for item in &inventory.entries {
        *values.entry(item.item_definition_ref).or_default() += u64::from(item.count.get());
    }
    values
}

fn item_handle(client: &DevClientSession, definition: NonZeroU32) -> TestResult<ItemHandle> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == definition)
        })
        .map(|item| item.handle)
        .ok_or_else(|| format!("missing carried item reference={definition}").into())
}

fn expected_recipe(
    inventory: &CharacterInventory,
    ruby: NonZeroU32,
    target: NonZeroU32,
    output: NonZeroU32,
) -> TestResult<BTreeMap<NonZeroU32, u64>> {
    let mut expected = quantities(inventory);
    for input in [ruby, target] {
        let count = expected.get_mut(&input).ok_or("recipe input missing")?;
        *count -= 1;
        if *count == 0 {
            expected.remove(&input);
        }
    }
    *expected.entry(output).or_default() += 1;
    Ok(expected)
}

fn matches_delta(
    reply: u64,
    sequence: u64,
    inventory: &CharacterInventory,
    expected: &BTreeMap<NonZeroU32, u64>,
) -> bool {
    sequence > reply && quantities(inventory) == *expected
}

fn check_vitals(event: &SessionEvent, reply: u64, before: &ActorVitals) -> TestResult {
    let vitals = match event {
        SessionEvent::ActorVitals(delta) | SessionEvent::GiftOfLife { vitals: delta, .. } => delta,
        _ => return Ok(()),
    };
    if vitals.server_sequence > reply && vitals.value != *before {
        return Err(format!(
            "recipe changed vitals at sequence {}",
            vitals.server_sequence
        )
        .into());
    }
    Ok(())
}

async fn await_recipe_inventory(
    client: &mut DevClientSession,
    reply: u64,
    expected: &BTreeMap<NonZeroU32, u64>,
    vitals: &ActorVitals,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let events = client.take_events();
        for event in &events {
            check_vitals(event, reply, vitals)?;
        }
        if let Some(sequence) = events.into_iter().find_map(|event| match event {
            SessionEvent::Inventory(delta)
                if matches_delta(reply, delta.server_sequence, &delta.value, expected) =>
            {
                Some(delta.server_sequence)
            }
            _ => None,
        }) {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "missing post-result recipe inventory delta after sequence {reply}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn look_at_output(client: &mut DevClientSession, handle: ItemHandle) -> TestResult {
    use oteryn_protocol_oteryn::item_look as wire;
    let selection = wire::ItemLookSelection { extended: true };
    let payload = wire::encode_item_look_intent_for(
        wire::ItemLookIntent {
            target: wire::ItemLookTarget::Item(handle),
        },
        selection,
    )
    .map_err(|error| format!("recipe output LOOK encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::ITEM_LOOK_CAPABILITY_ID.ok_or("LOOK capability absent")?,
            wire::ITEM_LOOK_COMMAND_ID.ok_or("LOOK command absent")?,
            &payload,
        )
        .await?;
    let result = wire::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("recipe output LOOK decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || !matches!(result, wire::ItemLookResult::Text(_)) {
        return Err(format!("recipe output LOOK observed={result:?}").into());
    }
    Ok(())
}

async fn use_recipe(
    client: &mut DevClientSession,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    target_id: u32,
    output_id: u32,
) -> TestResult<u64> {
    let ruby = provisioning::item_reference(index, RUBY)?;
    let target = provisioning::item_reference(index, target_id)?;
    let output = provisioning::item_reference(index, output_id)?;
    let expected = expected_recipe(
        client.inventory().ok_or("inventory absent")?,
        ruby,
        target,
        output,
    )?;
    let vitals = client
        .actor_vitals()
        .cloned()
        .ok_or("actor vitals absent")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(item_handle(client, ruby)?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(item_handle(client, target)?)),
        use_with_position: None,
    })
    .map_err(|error| format!("recipe USE encode: {error:?}"))?;
    client.take_events();
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("recipe USE decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "recipe target={target_id} observed={disposition:?} status={:?}",
            reply.status
        )
        .into());
    }
    let sequence =
        await_recipe_inventory(client, reply.server_sequence, &expected, &vitals).await?;
    // The next command's result follows all post-USE frames, including any
    // vitals frame sent after the inventory delta.
    look_at_output(client, item_handle(client, output)?).await?;
    for event in client.take_events() {
        check_vitals(&event, reply.server_sequence, &vitals)?;
    }
    if client.actor_vitals() != Some(&vitals)
        || quantities(client.inventory().ok_or("recipe inventory absent")?) != expected
    {
        return Err(
            format!("recipe target={target_id} changed vitals or unrelated inventory").into(),
        );
    }
    Ok(sequence)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let client = connect_world(clients, 0xdf).await?;
    let mut client = provisioning::provision_profile(client, clients, 1, 100, "gem-use").await?;
    let index = provisioning::item_index()?;
    let mut deltas = Vec::with_capacity(RECIPES.len());
    for (target, output) in RECIPES {
        deltas.push(use_recipe(&mut client, &index, target, output).await?);
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(format!(
        "rainbow-shield-delta={};firewalker-boots-delta={};vitals-unchanged;logout=Accepted",
        deltas[0], deltas[1]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::item_view::ItemEntry;

    fn inventory(items: &[(u32, u32)]) -> TestResult<CharacterInventory> {
        let entries = items
            .iter()
            .enumerate()
            .map(|(i, &(id, count))| {
                Ok::<_, &'static str>(ItemEntry {
                    appearance_id: 0,
                    handle: ItemHandle::new(i as u64 + 1).ok_or("zero handle")?,
                    item_definition_ref: NonZeroU32::new(id).ok_or("zero item reference")?,
                    count: NonZeroU32::new(count).ok_or("zero count")?,
                    sub_type: 0,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(CharacterInventory {
            entries,
            ..CharacterInventory::default()
        })
    }

    #[test]
    fn i19_requires_both_recipes_post_result_inventory() -> TestResult {
        let def = |id| NonZeroU32::new(id).ok_or("invalid item reference");
        let mut before = inventory(&[(676, 2), (8077, 1), (9020, 1)])?;
        for ((target, output), after) in RECIPES.into_iter().zip([
            inventory(&[(676, 1), (8078, 1), (9020, 1)])?,
            inventory(&[(8078, 1), (9019, 1)])?,
        ]) {
            let expected = expected_recipe(&before, def(RUBY)?, def(target)?, def(output)?)?;
            assert!(
                !matches_delta(10, 11, &before, &expected),
                "unchanged inventory must fail"
            );
            assert!(
                !matches_delta(10, 10, &after, &expected),
                "pre-result inventory must fail"
            );
            assert!(matches_delta(10, 11, &after, &expected));
            before = after;
        }
        Ok(())
    }
}
