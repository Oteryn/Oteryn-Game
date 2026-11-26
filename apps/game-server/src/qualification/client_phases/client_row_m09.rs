//! Client replay capture M09 — fishing and skinning/gathering yield actions.
//!
//! Concrete player scenario derived from the M09 matrix/re-audit row and the shared
//! inventory-delta seam used by fishing and skinning/gathering yields:
//! 1. Admit the node-boot Character and BUY a fishing rod (3483) and worms (3492).
//! 2. Walk beside authored water and send native USE-WITH until `Committed`; assert one worm
//!    consumed, one fish (3578) minted, and the rod retained, then capture the applied payload.
//! 3. Drain the bait and assert USE-WITH returns `NothingToUse` without changing fish or rod
//!    custody. The full M09 journey separately covers the skinning producer of this same payload.
use super::*;
use oteryn_dev_client::{DevClientSession, ItemEntry};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, decode_use_result, encode_item_use_intent,
};
use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroU32;

const ACTIONS: &[&str] = &[
    "player-acquired-rod-and-bait",
    "fishing-replies-and-deltas",
    "no-bait-refusal",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("M09_CHECK {action} PASS observed={observed:?}"));
        }
    }
    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("M09_CHECK {action} FAIL assertion-not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("M09_GAP {error}"));
        }
        evidence(&format!(
            "e2e_all scenario=M09.client status={} seconds={:.3}",
            if result.is_ok() && ACTIONS.iter().all(|a| self.passed.contains(a)) {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(600), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| {
            Err("step deadline: M09 journey exceeded 600s; last server reply unavailable".into())
        });
    row.finish(result);
}

fn reference(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    id: u32,
) -> TestResult<NonZeroU32> {
    provisioning::item_reference(index, id)
}

fn units(client: &DevClientSession, wanted: NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .chain(inventory.main_backpack.iter())
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .filter(|entry| entry.item_definition_ref == wanted)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

fn carried(client: &DevClientSession, wanted: NonZeroU32) -> Option<ItemEntry> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == wanted)
        .copied()
}

async fn settle(client: &mut DevClientSession) -> TestResult {
    client.service_liveness(Duration::from_millis(50)).await?;
    client.take_events();
    Ok(())
}

async fn use_on(
    client: &mut DevClientSession,
    source: oteryn_protocol_oteryn::item_view::ItemHandle,
    target: MapUseTarget,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(target),
        use_with_position: None,
    })
    .map_err(|error| format!("M09 USE-WITH encode: {error:?}"))?;
    let command = client.next_command_id();
    evidence(&format!(
        "M09_CAST command={command} position={:?} last_sequence={}",
        client.world_spatial().actor_position,
        client.last_server_sequence()
    ));
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("M09 USE-WITH reply status={:?}: {error:?}", reply.status))?;
    evidence(&format!(
        "M09_CAST_RESULT command={command} status={:?} disposition={disposition:?}",
        reply.status
    ));
    Ok((reply.status, disposition))
}

fn capture_inventory(client: &mut DevClientSession) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let inventory = client
        .take_events()
        .into_iter()
        .rev()
        .find_map(|event| match event {
            oteryn_dev_client::SessionEvent::Inventory(delta) => Some(delta.value),
            _ => None,
        })
        .ok_or("M09 capture missing inventory delta after committed fishing")?;
    let payload = oteryn_protocol_oteryn::item_view::encode_character_inventory(&inventory)
        .map_err(|error| format!("M09 capture encode: {error:?}"))?;
    let expected = inventory
        .entries
        .iter()
        .map(|entry| format!("{} {}\n", entry.item_definition_ref, entry.count))
        .collect::<String>();
    let root = std::path::Path::new(&root);
    std::fs::write(root.join("m09-inventory.bin"), payload)?;
    std::fs::write(root.join("m09-expected.txt"), expected)?;
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let inputs = full_row_m09::FishingInputs::load(&bytes)?;
    let pin = serde_json::from_slice(&fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_M09_CLIENT_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    character_prep::declare_seeded(
        "M09.client",
        "190 carried gold",
        "rod and 40 bait purchases",
    );
    let mut client = character_prep::funded_character(
        clients,
        0xb9,
        190,
        &[(3031, 100), (3031, 20), (3031, 19), (3031, 1)],
    )
    .await?;
    let (rod_ref, worm_ref) =
        full_row_m09::buy_fishing_gear(&mut client, world, &index, &inputs, 40).await?;
    let fish_ref = reference(&index, 3578)?;
    row.pass(
        "player-acquired-rod-and-bait",
        (units(&client, rod_ref), units(&client, worm_ref)),
    );

    let (water, water_tile) =
        full_row_m09::water_approach(&mut client, world, &inputs, None).await?;
    let rod = carried(&client, rod_ref).ok_or("step 2 rod not directly carried")?;
    let before_fish = units(&client, fish_ref);
    let mut committed = None;
    for attempt in 1..=40 {
        client.take_events();
        let worms = units(&client, worm_ref);
        let fish = units(&client, fish_ref);
        let reply = use_on(&mut client, rod.handle, MapUseTarget::Object(water.clone())).await?;
        if reply == (CommandStatus::Accepted, UseDisposition::Committed) {
            full_row_m09::wait_for_catch_delta(&mut client, worm_ref, fish_ref, worms, fish)
                .await?;
        } else {
            settle(&mut client).await?;
        }
        match reply {
            (CommandStatus::Accepted, UseDisposition::NothingToUse) if units(&client, worm_ref) == worms && units(&client, fish_ref) == fish => {}
            (CommandStatus::Accepted, UseDisposition::Committed) if units(&client, worm_ref) + 1 == worms && units(&client, fish_ref) == fish + 1 => { committed = Some((attempt, reply)); break; }
            observed => return Err(format!("step 2 fishing reply/delta mismatch: observed={observed:?}; worms={worms}->{}; fish={fish}->{}", units(&client, worm_ref), units(&client, fish_ref)).into()),
        }
    }
    let committed = committed.ok_or_else(|| {
        format!(
            "step 2 fishing: 40 native attempts produced no Committed reply; fish={}->{}",
            before_fish,
            units(&client, fish_ref)
        )
    })?;
    if units(&client, rod_ref) != 1 {
        return Err("step 2 committed fishing consumed rod".into());
    }
    row.pass("fishing-replies-and-deltas", committed);
    capture_inventory(&mut client)?;
    full_row_m09::clear_spare_bait(&mut client, worm_ref).await?;
    let (unspent_water, _) =
        full_row_m09::water_approach(&mut client, world, &inputs, Some(water_tile)).await?;
    let fish = units(&client, fish_ref);
    let refusal = use_on(&mut client, rod.handle, MapUseTarget::Object(unspent_water)).await?;
    settle(&mut client).await?;
    if refusal != (CommandStatus::Accepted, UseDisposition::NothingToUse)
        || units(&client, fish_ref) != fish
        || units(&client, rod_ref) != 1
    {
        return Err(format!("step 3 no-bait refusal mismatch: reply={refusal:?}").into());
    }
    row.pass("no-bait-refusal", refusal);

    Ok(())
}
