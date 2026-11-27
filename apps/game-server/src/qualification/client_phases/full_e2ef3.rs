//! E2EF3: ordinary starter-character observations, never privileged provisioning.
//!
//! Full rows remain FAIL until every listed action has a real client assertion.
//! In particular, a successful read-only inspection is not I16 qualification.
use super::*;
use oteryn_protocol_oteryn::{container_tree as tree, inspection as inspect};
use std::{collections::BTreeMap, future::Future};

#[path = "full_e2ef3_e2ei-a.rs"]
mod e2ei_a;

#[path = "full_e2ef3_e2ei-c.rs"]
pub(super) mod e2ei_c;

#[path = "full_e2ef3_e2ei-b.rs"]
mod full_e2ef3_e2ei_b;

#[path = "full_e2ef3_e2ei-d.rs"]
mod full_e2ef3_e2ei_d;

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

fn result(row: &Row, seconds: f64, reason: &str, cause: &verdict::Cause) {
    // The first unexecuted action is the root; the starter observation's own
    // failure, when there is one, is the cause.
    let first = row.missing.split(',').next().unwrap_or("lifecycle");
    let suffix = cause.suffix(&format!("{}/{first}", row.id));
    evidence(&format!(
        "e2e_all scenario={}.full status=FAIL seconds={seconds:.3} evidence=source:{}{suffix}",
        row.id, row.source
    ));
    // Separate detail from the strict report record; never treat these words as checks.
    evidence(&format!(
        "E2EF3_GAP row={} unexecuted={} reason={}",
        row.id,
        row.missing,
        reason.replace(['\r', '\n'], " ")
    ));
}

// The served route evidence identifies this occupied western landing.
pub(super) fn route_available(at: TilePos) -> bool {
    at != (TilePos {
        x: 32449,
        y: 32148,
        floor: -7,
    })
}

pub(super) fn occupied_tiles(
    client: &oteryn_dev_client::DevClientSession,
) -> std::collections::BTreeSet<TilePos> {
    client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.others())
        .filter(|entity| {
            matches!(
                entity.kind,
                oteryn_dev_client::EntityKind::Creature
                    | oteryn_dev_client::EntityKind::Npc
                    | oteryn_dev_client::EntityKind::Player
            )
        })
        .filter_map(|entity| tile(entity.position).ok())
        .collect()
}

pub(super) fn ordinary_spawn_points(
    table: &oteryn_world_bundle::spawn::Table,
) -> impl Iterator<Item = (TilePos, &str)> {
    use oteryn_world_bundle::spawn::Period;

    table.sources.iter().flat_map(move |source| {
        source.points.iter().filter_map(move |point| {
            let creature = table.creatures.get(point.creature as usize)?;
            (creature.period == Period::All && point.boss_policy.is_none()).then_some((
                TilePos {
                    x: point.x,
                    y: point.y,
                    floor: source.floor,
                },
                creature.key.as_str(),
            ))
        })
    })
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
            "E2EF3_OBSERVATION reward-bag={constant} definition={} carried-quantity=0 acquisition-route=absent-from-starter-custody",
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
    let views = selected_container_views(client.selected_capabilities(), &delta.payload)
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
    client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let start = tile(client.world_spatial().actor_position)?;
    run_phase(
        start,
        observe_starter(client, clients),
        async |client, primal, covet, seconds| {
            let client = e2ei_a::run(client, clients).await;
            let client = full_e2ef3_e2ei_b::run(client, clients).await;
            let client = e2ei_c::run(client, clients).await;
            full_e2ef3_e2ei_d::run(client, clients, primal, covet, seconds).await
        },
        |client| tile(client.world_spatial().actor_position),
        |from, start| route_avoiding(world, from, |at| at == start, route_available),
        async |client, path| walk(client, world, path).await,
    )
    .await
}

async fn observe_starter(
    mut client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<(oteryn_dev_client::DevClientSession, bool, bool)> {
    let started = std::time::Instant::now();
    let primal = reward_bag(&mut client, "PRIMAL_BAG").await?;
    let covet = reward_bag(&mut client, "BAG_YOU_COVET").await?;
    let saved_quantities = quantities(&client)?;
    open_backpack(&mut client).await?;
    let before = backpack_inspection(&mut client).await?;
    e2ei_a::relogin_starter(&mut client, clients).await?;
    open_backpack(&mut client).await?;
    if backpack_inspection(&mut client).await? != before || quantities(&client)? != saved_quantities
    {
        return Err("E2EF3 starter inspection changed across fresh admission".into());
    }
    for scenario in ["I01.root-view-relogin", "I16.starter-weight-relogin"] {
        evidence(&format!(
            "E2EF3_OBSERVATION scenario={scenario} status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        ));
    }
    Ok((client, primal, covet))
}

/// The phase owns both the observation barrier and the return to the core route.
async fn run_phase<C>(
    start: TilePos,
    observations: impl Future<Output = TestResult<(C, bool, bool)>>,
    rows: impl AsyncFnOnce(C, bool, bool, f64) -> TestResult<C>,
    position: impl FnOnce(&C) -> TestResult<TilePos>,
    find: impl FnOnce(TilePos, TilePos) -> Option<Vec<StepDirection>>,
    return_steps: impl AsyncFnOnce(&mut C, &[StepDirection]) -> TestResult,
) -> TestResult<C> {
    let started = std::time::Instant::now();
    let observations = observations.await;
    let seconds = started.elapsed().as_secs_f64();
    report_starter(
        observations.as_ref().map(|_| ()).map_err(Box::as_ref),
        seconds,
    );
    let (client, primal, covet) = observations?;
    let mut client = rows(client, primal, covet, seconds).await?;
    let path = core_return_path(position(&client)?, start, find)?;
    return_steps(&mut client, &path).await?;
    Ok(client)
}

fn report_starter(observations: Result<(), &(dyn std::error::Error + 'static)>, seconds: f64) {
    let (cause, reason) = match observations {
        Ok(()) => (
            verdict::Cause::Assertion,
            "row has no mounted journey".to_owned(),
        ),
        Err(error) => (
            verdict::classify(error),
            format!("starter observation failed: {error}"),
        ),
    };
    for row in starter_rows(observations.is_err()) {
        result(row, seconds, &reason, &cause);
    }
}

fn core_return_path(
    from: TilePos,
    start: TilePos,
    find: impl FnOnce(TilePos, TilePos) -> Option<Vec<StepDirection>>,
) -> TestResult<Vec<StepDirection>> {
    if from == start {
        return Ok(Vec::new());
    }
    find(from, start)
        .ok_or_else(|| format!("E2EF3 has no return route from {from:?} to {start:?}").into())
}

#[tokio::test]
async fn qualification_e2ef3_starter_error_fails_phase() {
    let rows_called = std::cell::Cell::new(false);
    let result = run_phase(
        TOWN,
        async { Err::<(TilePos, bool, bool), _>("starter inspection changed".into()) },
        async |client, _, _, _| {
            rows_called.set(true);
            Ok(client)
        },
        |at| Ok(*at),
        |_, _| Some(Vec::new()),
        async |_, _| Ok(()),
    )
    .await;
    assert!(result.is_err_and(|error| error.to_string() == "starter inspection changed"));
    assert!(!rows_called.get());
}

#[tokio::test]
async fn qualification_e2ef3_displaced_client_requires_return_route() {
    let displaced = TilePos {
        x: TOWN.x + 1,
        ..TOWN
    };
    let steps_called = std::cell::Cell::new(false);
    let result = run_phase(
        TOWN,
        async { Ok((TOWN, true, false)) },
        async |_, primal, covet, _| {
            assert!(primal && !covet);
            Ok(displaced)
        },
        |at| Ok(*at),
        |from, to| {
            assert_eq!((from, to), (displaced, TOWN));
            None
        },
        async |_, _| {
            steps_called.set(true);
            Ok(())
        },
    )
    .await;
    assert!(result.is_err_and(|error| error.to_string().contains("has no return route")));
    assert!(!steps_called.get());
}

#[tokio::test]
async fn qualification_e2ef3_returns_displaced_client_before_handoff() -> TestResult {
    let displaced = TilePos {
        x: TOWN.x + 1,
        ..TOWN
    };
    let result = run_phase(
        TOWN,
        async { Ok((TOWN, false, true)) },
        async |_, primal, covet, _| {
            assert!(!primal && covet);
            Ok(displaced)
        },
        |at| Ok(*at),
        |from, to| {
            assert_eq!((from, to), (displaced, TOWN));
            Some(vec![StepDirection::West])
        },
        async |at, steps| {
            assert_eq!(steps, [StepDirection::West]);
            *at = TOWN;
            Ok(())
        },
    )
    .await?;
    assert_eq!(result, TOWN);

    let planner_called = std::cell::Cell::new(false);
    let at_start = run_phase(
        TOWN,
        async { Ok((TOWN, false, false)) },
        async |at, _, _, _| Ok(at),
        |at| Ok(*at),
        |_, _| {
            planner_called.set(true);
            None
        },
        async |_, steps| {
            assert!(steps.is_empty());
            Ok(())
        },
    )
    .await?;
    assert_eq!(at_start, TOWN);
    assert!(!planner_called.get());
    Ok(())
}

const MODULE_ROWS: &[&[&str]] = &[
    e2ei_a::COVERED_ROWS,
    full_e2ef3_e2ei_b::COVERED_ROWS,
    e2ei_c::COVERED_ROWS,
    full_e2ef3_e2ei_d::COVERED_ROWS,
];

fn starter_rows(failed: bool) -> impl Iterator<Item = &'static Row> {
    ROWS.iter()
        .filter(move |row| failed || !MODULE_ROWS.iter().any(|ids| ids.contains(&row.id)))
}

#[test]
fn full_e2ef3_reports_each_row_once() {
    let mut covered: Vec<_> = MODULE_ROWS
        .iter()
        .flat_map(|ids| ids.iter().copied())
        .chain(starter_rows(false).map(|row| row.id))
        .collect();
    covered.sort_unstable();
    let mut expected: Vec<_> = ROWS.iter().map(|row| row.id).collect();
    expected.sort_unstable();
    assert!(covered.windows(2).all(|pair| pair[0] != pair[1]));
    assert_eq!(covered, expected);
}
