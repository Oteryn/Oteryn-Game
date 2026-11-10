//! E2EF3: ordinary starter-character observations, never privileged provisioning.
//!
//! Full rows remain FAIL until every listed action has a real client assertion.
//! In particular, a successful read-only inspection is not I16 qualification.
use super::*;
use oteryn_protocol_oteryn::{container_tree as tree, inspection as inspect};
use std::collections::BTreeMap;

struct Row {
    id: &'static str,
    source: &'static str,
    missing: &'static str,
}

// Keep the audit's sub-actions explicit, including the durability assertions.
const ROWS: &[Row] = &[
    Row {
        id: "I01",
        source: "crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs:3",
        missing: "ground-inventory,nested-move,split-stack,merge-stack,capacity-refusal,relogin-quantities",
    },
    Row {
        id: "I04",
        source: "crates/protocol-oteryn/src/mail.rs:14",
        missing: "write-read-rewritable,text-limit,write-once-transform,rewrite-refusal,relogin-text-transform",
    },
    Row {
        id: "I06",
        source: "crates/gameplay/dispatch/src/gameplay_transport/corpse_decay.rs:7",
        missing: "kill,leave-loot,observe-decay,retired-loot-unavailable,relogin-retirement",
    },
    Row {
        id: "I07",
        source: "crates/protocol-oteryn/src/quick_loot.rs:48",
        missing: "manual-click,loot-all,accept-skip-filters,category-routing,gold-pouch,quiver-ammunition,relogin-policy-custody",
    },
    Row {
        id: "I08",
        source: "crates/gameplay/dispatch/src/gameplay_transport/reward_bag_use.rs:16",
        missing: "obtain-primal-bag,use-consumes-one,pinned-pool-reward,relogin-reward",
    },
    Row {
        id: "I09",
        source: "crates/gameplay/dispatch/src/gameplay_transport/item_transform.rs:16",
        missing: "approach-unbound-nonhouse-furniture,use-rotate,assert-definition-and-collision",
    },
    Row {
        id: "I12",
        source: "crates/gameplay/dispatch/src/gameplay_transport/mining/mod.rs:67",
        missing: "obtain-pick-and-shiny-stone,use-with,stone-consumed,retained-draw,relogin-refinement",
    },
    Row {
        id: "I13",
        source: "crates/gameplay/dispatch/src/gameplay_transport/reward_bag_use.rs:16",
        missing: "obtain-bag-you-covet,use-consumes-one,pinned-pool-reward,relogin-reward",
    },
    Row {
        id: "I15",
        source: "crates/gameplay/dispatch/src/gameplay_transport/timed_item_clock.rs:49",
        missing: "obtain-energy-ring,equip,damage-mana-at-two-to-one,health-spill,unequip-pause,expiry,relogin-timer",
    },
    Row {
        id: "I16",
        source: "crates/gameplay/dispatch/src/gameplay_transport/inspection.rs:644",
        missing: "inspect-applied-imbuement,assert-family-tier-time-and-weight,relogin-imbuement",
    },
];

fn result(row: &Row, seconds: f64, reason: &str) {
    evidence(&format!(
        "e2e_all scenario={}.full status=FAIL seconds={seconds:.3} evidence=source:{}",
        row.id, row.source
    ));
    // Separate detail from the strict report record; never treat these words as checks.
    evidence(&format!(
        "E2EF3_GAP row={} unexecuted={} reason={reason}",
        row.id, row.missing
    ));
}

fn quantities(client: &oteryn_dev_client::DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut totals = BTreeMap::new();
    for entry in &client.inventory().ok_or("E2EF3 inventory absent")?.entries {
        *totals.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    Ok(totals)
}

/// A bag must already be in real carried custody. This never calls `provision`.
/// Compare the entire direct inventory, so an unrelated mint cannot mask item loss.
async fn reward_bag(
    client: &mut oteryn_dev_client::DevClientSession,
    constant: &str,
) -> TestResult<bool> {
    let bags = oteryn_gameplay_items::reward_bag::reward_bags().ok_or("reward pool absent")?;
    let bag = bags
        .iter()
        .find(|bag| bag.constant == constant && bag.playable)
        .ok_or("playable reward bag absent")?;
    let index = provisioning::item_index()?;
    let revision = index.revision(&bag.bag).ok_or("bag revision absent")?;
    let reference = index
        .definition_ref("Item", &bag.bag, revision)
        .ok_or("bag definition absent")?;
    let item = client
        .inventory()
        .ok_or("reward inventory absent")?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == reference)
        .copied();
    let Some(item) = item else {
        evidence(&format!(
            "E2EF3_OBSERVATION reward-bag={constant} definition={} carried-quantity=0 acquisition-route=unimplemented",
            reference.get()
        ));
        return Ok(false);
    };
    let mut expected = quantities(client)?;
    let quantity = expected
        .get_mut(&reference.get())
        .ok_or("bag quantity absent")?;
    *quantity -= 1;
    if *quantity == 0 {
        expected.remove(&reference.get());
    }
    let mut candidates = Vec::new();
    for reward in &bag.rewards {
        if reward.weight == 0 {
            return Err("zero weight reward in playable pool".into());
        }
        let revision = index
            .revision(&reward.item)
            .ok_or("reward revision absent")?;
        let reference = index
            .definition_ref("Item", &reward.item, revision)
            .ok_or("reward ref absent")?;
        let mut candidate = expected.clone();
        *candidate.entry(reference.get()).or_default() += 1;
        candidates.push(candidate);
    }
    let response = client.use_item(item.handle).await?;
    if response.status != CommandStatus::Accepted
        || response.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("E2EF3 reward USE refused: {response:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if candidates.contains(&quantities(client)?) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "E2EF3 reward did not consume one bag and mint exactly one pinned reward".into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    evidence(&format!(
        "E2EF3_OBSERVATION reward-bag={constant} source-consumed=1 pinned-reward=1 pool-size={}",
        bag.rewards.len()
    ));
    Ok(true)
}

/// These reads intentionally use the server-issued instance handle, not Catalogue.
async fn backpack_inspection(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<inspect::InspectedItem> {
    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("E2EF3 starter backpack absent")?;
    let payload = inspect::encode_request(inspect::Request::Item(backpack.handle))
        .map_err(|error| format!("inspection encode: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            inspect::CAPABILITY_INSPECTION_V1,
            inspect::COMMAND_TYPE_INSPECTION,
            &payload,
        )
        .await?;
    let reply = inspect::decode_reply(&response.payload)
        .map_err(|error| format!("inspection decode: {error:?}"))?;
    if response.status != CommandStatus::Accepted || reply.status != 1 || reply.items.len() != 1 {
        return Err(format!("E2EF3 instance inspection refused: {reply:?}").into());
    }
    let item = reply.items.into_iter().next().ok_or("inspection item")?;
    if item.definition_ref != backpack.item_definition_ref.get()
        || item.count != backpack.count.get()
        || !item.attributes.iter().any(|(key, value)| {
            key == "Total Weight" && value.ends_with(" oz") && value != "0.00 oz"
        })
    {
        return Err("E2EF3 backpack identity/count/total weight mismatch".into());
    }
    Ok(item)
}

async fn open_backpack(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("E2EF3 backpack missing")?;
    let payload = tree::encode_container_view_intent(&tree::ContainerViewIntent::Open {
        handle: backpack.handle,
        replace_view: None,
    })
    .map_err(|error| format!("container encode: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            tree::CAPABILITY_CONTAINER_TREE_V1,
            tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted
        || tree::decode_container_view_result(&response.payload)
            .map_err(|error| format!("container outcome: {error:?}"))?
            != tree::ContainerViewOutcome::Opened
    {
        return Err("E2EF3 open starter backpack refused".into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    let views = tree::decode_container_views(&delta.payload)
        .map_err(|error| format!("container delta: {error:?}"))?;
    let view = views
        .views
        .iter()
        .find(|view| view.container_handle == backpack.handle)
        .ok_or("E2EF3 opened backpack not projected")?;
    let inventory = client.inventory().ok_or("E2EF3 inventory")?;
    if view.capacity == 0 || view.entries != inventory.entries || view.parent_handle.is_some() {
        return Err("E2EF3 root container capacity/entries/parent mismatch".into());
    }
    evidence(&format!(
        "E2EF3_OBSERVATION root-capacity={} direct-entries={}",
        view.capacity,
        view.entries.len()
    ));
    Ok(())
}

pub(super) async fn run(
    mut client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let started = std::time::Instant::now();
    let mut primal = false;
    let mut covet = false;
    let observations = async {
        primal = reward_bag(&mut client, "PRIMAL_BAG").await?;
        covet = reward_bag(&mut client, "BAG_YOU_COVET").await?;
        let saved_quantities = quantities(&client)?;
        open_backpack(&mut client).await?;
        let before = backpack_inspection(&mut client).await?;
        let session = client.join_snapshot().game_session_id;
        let saved_position = client.world_spatial().actor_position;
        let (result, _, _, _) = super::logout_retrying_busy(&mut client).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err("E2EF3 fresh admission logout refused".into());
        }
        drop(client);
        client = connect_world(clients, 0x79).await?;
        if client.join_snapshot().game_session_id == session
            || client.world_spatial().actor_position != saved_position
        {
            return Err("E2EF3 fresh admission session/position mismatch".into());
        }
        open_backpack(&mut client).await?;
        if backpack_inspection(&mut client).await? != before
            || quantities(&client)? != saved_quantities
        {
            return Err("E2EF3 starter inspection changed across fresh admission".into());
        }
        for scenario in ["I01.root-view-relogin", "I16.starter-weight-relogin"] {
            evidence(&format!(
                "e2e_all scenario={scenario} status=PASS seconds={:.3}",
                started.elapsed().as_secs_f64()
            ));
        }
        Ok::<_, Box<dyn std::error::Error>>(client)
    }
    .await;
    for row in ROWS {
        if observations.is_ok() && ((row.id == "I08" && primal) || (row.id == "I13" && covet)) {
            evidence(&format!(
                "e2e_all scenario={}.full status=PASS seconds={:.3} evidence=use-consume-pinned-reward-fresh-admission-quantities",
                row.id,
                started.elapsed().as_secs_f64()
            ));
            continue;
        }
        result(
            row,
            started.elapsed().as_secs_f64(),
            if observations.is_ok() {
                "ordinary-starter-observed;full-actions-unimplemented;no-provisioning-used"
            } else {
                "starter-observation-failed;full-actions-unimplemented"
            },
        );
    }
    observations
}
