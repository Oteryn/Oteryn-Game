use super::*;
use oteryn_dev_client::{DevClientSession, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapItem, MapOrigin};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::{item_inscription as inscription, mail};
use std::num::{NonZeroU32, NonZeroU64};

const I04: &str = "I04";
const I09: &str = "I09";
pub(super) const COVERED_ROWS: &[&str] = &[I04, I09];

pub(in crate::qualification::client_world) const CHAIR: TilePos = TilePos {
    x: 32355,
    y: 32206,
    floor: -7,
};
const REWRITABLE: TilePos = TilePos {
    x: 32493,
    y: 32246,
    floor: -8,
};
const WRITE_ONCE: TilePos = TilePos {
    x: 32320,
    y: 32254,
    floor: -6,
};
// The town compilation retains these loose sources but not a starter route to them.
const DOCUMENT_OBSTACLE: &str =
    "2818 is behind locked door 1631 at (32321, 32258, 6), with no key action";
const PARCHMENT_OBSTACLE: &str =
    "2814 has no connected approach in the town compilation, even with ordinary doors open";
const WOLF_STEP_DESTINATION: TilePos = TilePos {
    x: 32428,
    y: 32158,
    floor: -7,
};

fn report(
    row: &str,
    started: std::time::Instant,
    action: &str,
    error: Option<&(dyn std::error::Error + 'static)>,
) {
    let seconds = started.elapsed().as_secs_f64();
    match error {
        None => evidence(&format!(
            "e2e_all scenario={row}.full status=PASS seconds={seconds:.3} evidence={action}"
        )),
        Some(error) => {
            let suffix = verdict::classify(error).suffix(&format!("{row}/{action}"));
            evidence(&format!(
                "e2e_all scenario={row}.full status=FAIL seconds={seconds:.3}{suffix}"
            ));
            evidence(&format!(
                "E2EF3_GAP row={row} unexecuted={action} observed={error}"
            ));
        }
    }
}

pub(super) async fn run(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let started = std::time::Instant::now();
    let world = (|| {
        load_world(
            std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
            &serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )
    })();
    let world = match world {
        Ok(world) => world,
        Err(error) => {
            report(I04, started, "write-once-transform", Some(error.as_ref()));
            report(
                I09,
                started,
                "approach-unbound-nonhouse-furniture",
                Some(error.as_ref()),
            );
            return Ok(client);
        }
    };
    let mut action = "approach-unbound-nonhouse-furniture";
    let result = furniture(&mut client, world, &mut action).await;
    report(I09, started, action, result.as_ref().err().map(Box::as_ref));

    let started = std::time::Instant::now();
    let mut action = "write-once-transform";
    let mut logged_out = false;
    let result = documents(&mut client, clients, world, &mut action, &mut logged_out).await;
    report(I04, started, action, result.as_ref().err().map(Box::as_ref));
    if logged_out {
        result?;
    }
    Ok(client)
}

fn position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: impl std::fmt::Debug,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route_avoiding(world, from, goal, |at| {
        route_available(at) && at != WOLF_STEP_DESTINATION
    })
    .ok_or_else(|| {
        format!(
            "no served walk route from {from:?} to {target:?} avoiding {WOLF_STEP_DESTINATION:?}"
        )
    })?;
    walk(client, world, &path).await
}

async fn visible_item(
    client: &mut DevClientSession,
    at: TilePos,
    appearance: u16,
) -> TestResult<MapItem> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let map = client.session_mut().world_map_snapshot();
        let cell = map
            .as_ref()
            .and_then(|map| map.tiles.iter().find(|cell| cell.position == position(at)));
        if let Some(item) = cell.and_then(|cell| {
            cell.items
                .iter()
                .find(|item| item.appearance_id == appearance)
        }) {
            return Ok(*item);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("tile {at:?} expected appearance {appearance}, observed {cell:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

fn chair_steps(world: &BundleWorld, at: TilePos) -> Option<[StepDirection; 2]> {
    if at.floor != CHAIR.floor
        || !matches!(
            (at.x.abs_diff(CHAIR.x), at.y.abs_diff(CHAIR.y)),
            (1, 0) | (0, 1)
        )
    {
        return None;
    }
    [
        [StepDirection::North, StepDirection::South],
        [StepDirection::East, StepDirection::West],
        [StepDirection::South, StepDirection::North],
        [StepDirection::West, StepDirection::East],
    ]
    .into_iter()
    .find(|[step, back]| {
        destination(world, at, *step) == Some(CHAIR) && destination(world, CHAIR, *back) == Some(at)
    })
}

async fn furniture(
    client: &mut DevClientSession,
    world: &BundleWorld,
    action: &mut &'static str,
) -> TestResult {
    let definitions = provisioning::item_index()?;
    let before = provisioning::item_reference(&definitions, 2358)?;
    let after = provisioning::item_reference(&definitions, 2359)?;
    if let Some(house) = world.facts().house_id(CHAIR) {
        return Err(format!("chair 2358 at {CHAIR:?} belongs to house {house}").into());
    }
    approach(client, world, CHAIR, |at| chair_steps(world, at).is_some()).await?;
    let steps = chair_steps(world, tile(client.world_spatial().actor_position)?)
        .ok_or("chair collision probe has no adjacent return step")?;
    let item = visible_item(client, CHAIR, 2358).await?;
    let MapOrigin::BaseOrdinal {
        ordinal,
        object_revision: 0,
    } = item.origin
    else {
        return Err(format!("expected unbound base chair at {CHAIR:?}, observed {item:?}").into());
    };
    if item.definition != MapDefinition::Item(before) {
        return Err(format!("expected chair definition {before}, observed {item:?}").into());
    }
    walk(client, world, &steps).await?;

    *action = "use-rotate";
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(CHAIR, ordinal)
            .ok_or("invalid chair placement")?
            .to_be_bytes(),
    );
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::WorldObject(WorldObjectTarget {
            placement,
            expected_revision: 0,
        }),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("chair USE encoding {error:?}"))?;
    client.take_events();
    let reply = client.session_mut().world_context_use(&payload).await?;
    if reply.status != CommandStatus::Accepted
        || decode_use_result(&reply.payload) != Ok(UseDisposition::Committed)
    {
        return Err(format!("chair USE observed {reply:?}").into());
    }

    *action = "assert-definition-and-collision";
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldMap(delta) = event
                && delta.server_sequence > reply.server_sequence
                && let Some(cell) = delta
                    .value
                    .tiles
                    .iter()
                    .find(|cell| cell.position == position(CHAIR))
            {
                let expected = MapItem {
                    origin: MapOrigin::BaseOrdinal {
                        ordinal,
                        object_revision: 1,
                    },
                    definition: MapDefinition::Item(after),
                    appearance_id: 2359,
                    ..item
                };
                if !cell.items.contains(&expected) {
                    return Err(format!("rotation expected {expected:?}, observed {cell:?}").into());
                }
                walk(client, world, &steps).await?;
                *action = "walk-unbound-chair-2358-use-2359-map-delta-walkable-before-and-after";
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no rotation map delta at {CHAIR:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

fn carried(client: &DevClientSession, definition: NonZeroU32) -> TestResult<NonZeroU64> {
    let entries = &client
        .inventory()
        .ok_or("backpack inventory absent")?
        .entries;
    let mut items = entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition);
    match (items.next(), items.next()) {
        (Some(entry), None) if entry.count.get() == 1 => Ok(entry.handle),
        _ => Err(format!(
            "expected one carried unit of definition {definition}, observed {entries:?}"
        )
        .into()),
    }
}

async fn wait_carried(
    client: &mut DevClientSession,
    definition: NonZeroU32,
) -> TestResult<NonZeroU64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let observed = carried(client, definition);
        if observed.is_ok() || tokio::time::Instant::now() >= deadline {
            return observed;
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn pick_up(
    client: &mut DevClientSession,
    world: &BundleWorld,
    at: TilePos,
    appearance: u16,
    definition: NonZeroU32,
) -> TestResult<Option<NonZeroU64>> {
    let from = tile(client.world_spatial().actor_position)?;
    let Some(path) = route_avoiding(
        world,
        from,
        |cell| cell.floor == at.floor && cell.x.abs_diff(at.x).max(cell.y.abs_diff(at.y)) == 1,
        |cell| route_available(cell) && cell != WOLF_STEP_DESTINATION,
    ) else {
        return Ok(None);
    };
    walk(client, world, &path).await?;
    let item = visible_item(client, at, appearance).await?;
    let MapOrigin::Handle(source) = item.origin else {
        return Err(format!("ground document at {at:?} has no pickup handle: {item:?}").into());
    };
    if item.definition != MapDefinition::Item(definition) || item.count != 1 {
        return Err(format!(
            "ground document at {at:?} expected one {definition}, observed {item:?}"
        )
        .into());
    }
    let inventory = client
        .inventory()
        .ok_or("backpack inventory absent before pickup")?;
    if let Some(entry) = inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == definition)
    {
        return Err(format!(
            "document {appearance} already carried before ground pickup: {entry:?}"
        )
        .into());
    }
    let reply = client
        .move_item(&ItemMoveIntent {
            source,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if reply.status != CommandStatus::Accepted || reply.outcome != ItemMoveOutcome::Moved {
        return Err(format!("ground pickup {appearance} at {at:?}: {reply:?}").into());
    }
    wait_carried(client, definition).await.map(Some)
}

async fn write(
    client: &mut DevClientSession,
    handle: NonZeroU64,
    text: &str,
    expected: mail::MailOutcome,
) -> TestResult {
    let payload = mail::encode_intent(&mail::MailIntent::WriteText {
        item_handle: handle.get(),
        text: text.into(),
    })
    .map_err(|error| format!("MAIL encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            u32::from(mail::CAPABILITY_MAIL_V1),
            u32::from(mail::COMMAND_TYPE_MAIL),
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.payload != [expected as u8] {
        return Err(format!("MAIL expected {expected:?}, observed {reply:?}").into());
    }
    Ok(())
}

async fn read(
    client: &mut DevClientSession,
    handle: NonZeroU64,
    expected: &str,
    writable: bool,
) -> TestResult {
    let payload = inscription::Request {
        item_handle: handle.get(),
    }
    .encode()
    .ok_or("inscription encode")?;
    let reply = client
        .session_mut()
        .gated_command(
            inscription::CAPABILITY_ITEM_INSCRIPTION_READ_V1,
            inscription::COMMAND_TYPE_ITEM_INSCRIPTION_READ,
            &payload,
        )
        .await?;
    let observed = inscription::Reply::decode(&reply.payload);
    if reply.status != CommandStatus::Accepted
        || !observed.as_ref().is_some_and(|value| {
            value.text == expected && value.max_length == 1023 && value.writable == writable
        })
    {
        return Err(format!("inscription expected text {expected:?}, limit 1023, writable {writable}; observed status {:?} {observed:?}", reply.status).into());
    }
    Ok(())
}

const REVISED: &str = "Second inscription: żółw 🐢";

async fn acquire_document(
    client: &mut DevClientSession,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    at: TilePos,
    appearance: u16,
    definition: NonZeroU32,
    obstacle: &str,
) -> TestResult<NonZeroU64> {
    if let Ok(handle) = carried(client, definition) {
        return Ok(handle);
    }
    let ground = pick_up(client, world, at, appearance, definition).await;
    ground_or_trade(ground, async {
        let key = format!("oteryn:item.tibia.i{appearance}");
        full_e2ef7::acquire(client, world, index, &key, 1)
            .await
            .map_err(|trade| format!("ground source={obstacle}; ordinary NPC trade={trade}"))?;
        carried(client, definition)
    })
    .await
}

async fn ground_or_trade<T>(
    ground: TestResult<Option<T>>,
    trade: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    match ground? {
        Some(item) => Ok(item),
        None => trade.await,
    }
}

#[tokio::test]
async fn document_trade_cannot_mask_a_failed_ground_assertion() -> TestResult {
    let traded = std::cell::Cell::new(false);
    for error in [
        "MOVE refused despite inventory credit",
        "wrong ground definition or count",
    ] {
        let result = ground_or_trade(Err(error.into()), async {
            traded.set(true);
            Ok(1)
        })
        .await;
        assert!(result.is_err_and(|observed| observed.to_string() == error));
    }
    assert!(!traded.get());
    let item = ground_or_trade(Ok(None), async { Ok(1) }).await?;
    assert_eq!(item, 1);
    Ok(())
}

async fn rewrite_parchment(
    client: &mut DevClientSession,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    rewritable: NonZeroU32,
) -> TestResult {
    let handle = acquire_document(
        client,
        world,
        index,
        REWRITABLE,
        2814,
        rewritable,
        PARCHMENT_OBSTACLE,
    )
    .await?;
    read(client, handle, "", true).await?;
    write(
        client,
        handle,
        "First inscription",
        mail::MailOutcome::TextWritten,
    )
    .await?;
    read(client, handle, "First inscription", true).await?;
    write(client, handle, REVISED, mail::MailOutcome::TextWritten).await?;
    read(client, handle, REVISED, true).await?;
    if carried(client, rewritable)? != handle {
        return Err("rewritable item changed handle".into());
    }

    Ok(())
}

const SEALED: &str = "Sealed inscription: żółw 🐢";

async fn write_document(
    client: &mut DevClientSession,
    world: &BundleWorld,
    action: &mut &'static str,
    definitions: &crate::content::item_ref::ItemDefinitionIndex,
    once: NonZeroU32,
    sealed: NonZeroU32,
) -> TestResult {
    *action = "write-once-transform";

    let handle = acquire_document(
        client,
        world,
        definitions,
        WRITE_ONCE,
        2818,
        once,
        DOCUMENT_OBSTACLE,
    )
    .await?;

    read(client, handle, "", true).await?;
    *action = "text-limit";
    write(
        client,
        handle,
        &"x".repeat(1024),
        mail::MailOutcome::TooLong,
    )
    .await?;
    read(client, handle, "", true).await?;
    if carried(client, once)? != handle {
        return Err("over-limit write changed the document handle".into());
    }

    *action = "write-once-transform";
    write(client, handle, SEALED, mail::MailOutcome::TextWritten).await?;
    let transformed = wait_carried(client, sealed).await?;
    if transformed != handle {
        return Err(
            format!("write-once transform changed handle {handle} to {transformed}").into(),
        );
    }
    let entries = &client
        .inventory()
        .ok_or("transformed inventory absent")?
        .entries;
    if entries.iter().any(|item| item.item_definition_ref == once) {
        return Err(
            format!("source definition {once} remains after transform: {entries:?}").into(),
        );
    }
    read(client, transformed, SEALED, false).await?;

    *action = "rewrite-refusal";
    write(
        client,
        transformed,
        "Forbidden rewrite",
        mail::MailOutcome::NotWritable,
    )
    .await?;
    read(client, transformed, SEALED, false).await?;

    Ok(())
}

async fn documents(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    action: &mut &'static str,
    logged_out: &mut bool,
) -> TestResult {
    let definitions = provisioning::item_index()?;
    let rewritable = provisioning::item_reference(&definitions, 2814)?;
    let once = provisioning::item_reference(&definitions, 2818)?;
    let sealed = provisioning::item_reference(&definitions, 2834)?;

    let write_once = write_document(client, world, action, &definitions, once, sealed).await;
    let once_step = *action;

    *action = "write-read-rewritable";
    let rewrite = async {
        approach(client, world, TOWN, |at| at == TOWN).await?;
        rewrite_parchment(client, world, &definitions, rewritable).await
    }
    .await;
    let writes = [(once_step, write_once), ("write-read-rewritable", rewrite)];
    if writes.iter().all(|(_, result)| result.is_err()) {
        return document_outcome(action, writes);
    }

    let admission = async {
        approach(client, world, TOWN, |at| at == TOWN).await?;
        let session = client.join_snapshot().game_session_id;
        let (outcome, detail, attempts, busy) = super::logout_retrying_busy(client).await?;
        if outcome != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!(
                "logout observed {outcome:?} {detail}; attempts={attempts}, busy={busy}"
            )
            .into());
        }
        *logged_out = true;
        *client = connect_world(clients, 0xb5).await?;
        *logged_out = false;
        if client.join_snapshot().game_session_id == session {
            return Err(format!("fresh admission reused session {session:?}").into());
        }
        Ok(())
    }
    .await;
    if admission.is_err() {
        return document_outcome(
            action,
            writes
                .into_iter()
                .chain([("relogin-text-transform", admission)]),
        );
    }
    check_document_durability(
        client,
        action,
        writes,
        async |client| {
            let entries = &client.inventory().ok_or("fresh inventory absent")?.entries;
            if entries.iter().any(|item| item.item_definition_ref == once) {
                return Err(
                    format!("source definition {once} returned after login: {entries:?}").into(),
                );
            }
            read(client, carried(client, sealed)?, SEALED, false).await
        },
        async |client| read(client, carried(client, rewritable)?, REVISED, true).await,
    )
    .await?;
    *action = "rewrite-limit-transform-refusal-fresh-admission";
    Ok(())
}

fn document_outcome(
    action: &mut &'static str,
    checks: impl IntoIterator<Item = (&'static str, TestResult)>,
) -> TestResult {
    let mut failures = checks
        .into_iter()
        .filter_map(|(phase, result)| result.err().map(|error| (phase, error)))
        .peekable();
    let Some((phase, first)) = failures.next() else {
        return Ok(());
    };
    *action = phase;
    if failures.peek().is_none() {
        return Err(first);
    }
    let mut additional = String::new();
    for (phase, error) in failures {
        additional.push_str(&format!("; {phase}: {error}"));
    }
    Err(Box::new(DocumentFailures { first, additional }))
}

#[derive(Debug)]
struct DocumentFailures {
    first: Box<dyn std::error::Error>,
    additional: String,
}

impl std::fmt::Display for DocumentFailures {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}{}", self.first, self.additional)
    }
}

impl std::error::Error for DocumentFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.first.as_ref())
    }
}

async fn check_document_durability<C>(
    client: &mut C,
    action: &mut &'static str,
    writes: [(&'static str, TestResult); 2],
    read_sealed: impl AsyncFnOnce(&mut C) -> TestResult,
    read_parchment: impl AsyncFnOnce(&mut C) -> TestResult,
) -> TestResult {
    let sealed = if writes[0].1.is_ok() {
        read_sealed(client).await
    } else {
        Ok(())
    };
    let parchment = if writes[1].1.is_ok() {
        read_parchment(client).await
    } else {
        Ok(())
    };
    document_outcome(
        action,
        writes.into_iter().chain([
            ("relogin-write-once-text-transform", sealed),
            ("relogin-rewritable-text", parchment),
        ]),
    )
}

#[tokio::test]
async fn durability_checks_both_documents_on_sealed_mismatch() {
    let mut reads = Vec::new();
    let mut action = "";
    let result = check_document_durability(
        &mut reads,
        &mut action,
        [("write-once", Ok(())), ("rewrite", Ok(()))],
        async |reads| {
            reads.push("sealed");
            Err("sealed text mismatch".into())
        },
        async |reads| {
            reads.push("parchment");
            Err("parchment text mismatch".into())
        },
    )
    .await;
    assert_eq!(reads, ["sealed", "parchment"]);
    assert_eq!(action, "relogin-write-once-text-transform");
    assert!(result.is_err_and(|error| {
        let error = error.to_string();
        error.contains("sealed text mismatch") && error.contains("parchment text mismatch")
    }));
}

#[tokio::test]
async fn durability_keeps_an_earlier_write_failure_primary() {
    let mut reads = Vec::new();
    let mut action = "";
    let result = check_document_durability(
        &mut reads,
        &mut action,
        [
            ("write-once", Err("transform failed".into())),
            ("rewrite", Ok(())),
        ],
        async |reads| {
            reads.push("sealed");
            Ok(())
        },
        async |reads| {
            reads.push("parchment");
            Err("parchment text mismatch".into())
        },
    )
    .await;
    assert_eq!(reads, ["parchment"]);
    assert_eq!(action, "write-once");
    assert!(result.is_err_and(|error| {
        error.to_string() == "transform failed; relogin-rewritable-text: parchment text mismatch"
    }));
}
