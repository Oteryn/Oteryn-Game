//! I07 client replay capture: real quick-loot filter, Gold Pouch destination and loot-all replies.
use super::*;
use oteryn_protocol_oteryn::quick_loot::{
    QuickLootAction, QuickLootCategory, QuickLootClickOutcome, QuickLootFilterMode,
    QuickLootRecipient, QuickLootRefusal, QuickLootRequest, decode_quick_loot_refusal,
    decode_quick_loot_result, encode_quick_loot_request,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(30),
        verdict::observe(capture_journey(clients)),
    )
    .await;
    let result = result.unwrap_or_else(|_| Err("I07 client capture timed out".into()));
    evidence(&format!(
        "e2e_all scenario=I07.client status={}",
        if result.is_ok() { "PASS" } else { "FAIL" }
    ));
    if let Err(error) = result {
        evidence(&format!("I07_CLIENT_GAP {error}"));
    }
}

#[derive(Default)]
struct CaptureTiming(Option<(&'static str, tokio::time::Instant)>);

impl CaptureTiming {
    fn enter(&mut self, stage: &'static str) {
        self.record("complete");
        self.0 = Some((stage, tokio::time::Instant::now()));
    }

    fn record(&mut self, status: &str) {
        if let Some((stage, started)) = self.0.take() {
            evidence(&format!(
                "I07_CAPTURE_STAGE stage={stage} status={status} elapsed_ms={}",
                started.elapsed().as_millis(),
            ));
        }
    }
}

impl Drop for CaptureTiming {
    fn drop(&mut self) {
        self.record("interrupted");
    }
}

// Fixture work runs on the node control socket. Continue reading gameplay
// frames and acknowledging liveness while that separate exchange is pending.
async fn provision_while_serving(
    client: &mut oteryn_dev_client::DevClientSession,
    profile: String,
) -> TestResult<String> {
    let request = request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        profile,
    );
    tokio::pin!(request);
    loop {
        let servicing = client.service_liveness(Duration::from_millis(100));
        tokio::pin!(servicing);
        tokio::select! {
            result = &mut servicing => result?,
            result = &mut request => {
                // A liveness read may have consumed the first byte of a frame.
                // Finish that window before the next command reads the socket.
                servicing.await?;
                return result.map_err(Into::into);
            }
        }
    }
}

// The serial command loop finishes sending the preceding command's deltas
// before it can answer this read. Waiting out a quiet interval is unnecessary.
async fn quick_loot_reply_barrier(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult {
    use oteryn_protocol_oteryn::quick_loot as wire;
    let payload = wire::encode_quick_loot_request(&request(QuickLootAction::ReviewDestinations))
        .map_err(|error| format!("I07 Quick Loot barrier request: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
            wire::COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || wire::decode_quick_loot_destinations(&reply.payload).is_err()
    {
        return Err(format!("I07 Quick Loot barrier refused: {reply:?}").into());
    }
    Ok(())
}

fn request(action: QuickLootAction) -> QuickLootRequest {
    QuickLootRequest {
        action,
        corpse: None,
        selected_item: None,
        review_filters: false,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    }
}

fn write_capture(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    std::fs::File::create_new(path)?.write_all(bytes)
}

fn capture_envelope(
    session: &[u8; 16],
    command_id: u64,
    server_sequence: u64,
    status: CommandStatus,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    if session[6] >> 4 != 7 || session[8] >> 6 != 2 {
        return Err("I07 capture session must be an RFC-variant UUIDv7".into());
    }
    Ok(serde_json::to_vec(&serde_json::json!({
        "command_id": command_id,
        "game_session_id": uuid_text(session),
        "payload": payload,
        "server_sequence": server_sequence,
        "status": status as u32,
    }))?)
}

#[test]
fn i07_writer_emits_exact_uuidv7_envelope() -> TestResult {
    #[derive(serde::Deserialize, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        command_id: u64,
        game_session_id: String,
        payload: Vec<u8>,
        server_sequence: u64,
        status: u32,
    }

    let mut session = [0; 16];
    session[6] = 0x70;
    for variant in [0x80, 0x90, 0xa0, 0xb0] {
        session[8] = variant;
        let wire = capture_envelope(&session, 4, 9, CommandStatus::Accepted, &[0, 255])?;
        let observed: Envelope = serde_json::from_slice(&wire)?;
        assert_eq!(
            observed,
            Envelope {
                command_id: 4,
                game_session_id: uuid_text(&session),
                payload: vec![0, 255],
                server_sequence: 9,
                status: 1,
            }
        );
        let json = std::str::from_utf8(&wire)?;
        for extra in [r#""command_id":4"#, r#""unexpected":0"#] {
            let invalid = format!("{{{extra},{}", &json[1..]);
            assert!(serde_json::from_str::<Envelope>(&invalid).is_err());
        }
    }
    for (version, variant) in [(0x40, 0x80), (0x70, 0x00), (0x70, 0xc0)] {
        session[6] = version;
        session[8] = variant;
        assert!(capture_envelope(&session, 4, 9, CommandStatus::Accepted, &[]).is_err());
    }
    Ok(())
}

async fn require_filters(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
    expected_mode: bool,
    expected: &[(bool, &str)],
) -> TestResult {
    let mode: Option<bool> = sqlx::query_scalar(
        "SELECT accept_only FROM game_character_quick_loot_settings
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_optional(&mut *database)
    .await?;
    let filters: Vec<(bool, String)> = sqlx::query_as(
        "SELECT accepted, item_production_key FROM game_character_quick_loot_filters
         WHERE character_id=encode($1,'hex')::uuid ORDER BY accepted, item_production_key LIMIT 4",
    )
    .bind(character.as_slice())
    .fetch_all(database)
    .await?;
    if mode.unwrap_or(false) != expected_mode
        || !filters
            .iter()
            .map(|(accept, key)| (*accept, key.as_str()))
            .eq(expected.iter().copied())
    {
        return Err(format!(
            "I07 durable filters expected mode={expected_mode} entries={expected:?}; observed mode={mode:?} entries={filters:?}"
        ).into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Placement {
    item: Vec<u8>,
    key: String,
    quantity: i64,
    slot: i64,
}

#[derive(Debug, Clone, Copy)]
enum Storage {
    Corpse,
    Backpack,
    InboxPouch,
    Quiver,
}

async fn placements(
    database: &mut sqlx::PgConnection,
    storage: Storage,
    owner: &[u8],
    parent: &[u8],
) -> TestResult<Vec<Placement>> {
    let (table, ownership) = match storage {
        Storage::Corpse => ("game_item_corpse_container_entries", ""),
        Storage::Backpack => (
            "game_item_container_entries",
            "AND e.character_id=encode($2,'hex')::uuid",
        ),
        Storage::InboxPouch | Storage::Quiver => ("game_item_nested_entries", ""),
    };
    let query = format!(
        "SELECT uuid_send(i.item_instance_id), i.definition_production_key, i.quantity, e.placement_ordinal::bigint
         FROM {table} e JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 {ownership}
         ORDER BY e.placement_ordinal LIMIT 501"
    );
    let mut query =
        sqlx::query_as::<_, (Vec<u8>, String, i64, i64)>(sqlx::AssertSqlSafe(query)).bind(parent);
    if matches!(storage, Storage::Backpack) {
        query = query.bind(owner);
    }
    let rows = query.fetch_all(database).await?;
    if rows.len() > 500 {
        return Err("I07 placement snapshot exceeded its bound".into());
    }
    Ok(rows
        .into_iter()
        .map(|(item, key, quantity, slot)| Placement {
            item,
            key,
            quantity,
            slot,
        })
        .collect())
}

async fn require_placements(
    database: &mut sqlx::PgConnection,
    storage: Storage,
    owner: &[u8],
    parent: &[u8],
    expected: &[Placement],
) -> TestResult {
    let actual = placements(database, storage, owner, parent).await?;
    if actual != expected {
        return Err(
            format!("I07 {storage:?} custody: expected={expected:?} observed={actual:?}").into(),
        );
    }
    Ok(())
}

async fn owned_containers(
    database: &mut sqlx::PgConnection,
    character: &[u8],
    world: &[u8],
) -> TestResult<([u8; 16], [u8; 16])> {
    let pouch: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id) FROM game_item_character_inbox_locations l
         JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE l.character_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid
           AND i.lifecycle=1 AND i.quantity=1 AND i.definition_production_key='oteryn:item.tibia.i23721' LIMIT 2",
    ).bind(character).bind(world).fetch_all(&mut *database).await?;
    let [pouch] = pouch.as_slice() else {
        return Err("I07 must own exactly one live Gold Pouch in the Inbox".into());
    };
    let backpack: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id) FROM game_item_container_slots s
         JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid
           AND i.lifecycle=1 AND i.quantity=1",
    )
    .bind(character)
    .bind(world)
    .fetch_one(database)
    .await?;
    Ok((
        pouch.as_slice().try_into()?,
        backpack.as_slice().try_into()?,
    ))
}

async fn require_retired_donor(database: &mut sqlx::PgConnection, item: &[u8]) -> TestResult {
    let state: (i16, i64) = sqlx::query_as(
        "SELECT lifecycle, quantity FROM game_item_instances
         WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item)
    .fetch_one(database)
    .await?;
    if state != (2, 0) {
        return Err(format!("I07 merge donor remains live or nonempty: {state:?}").into());
    }
    Ok(())
}

async fn require_unassigned_pouch(
    database: &mut sqlx::PgConnection,
    character: &[u8],
    pouch: &[u8],
) -> TestResult {
    let actual: (bool, bool, Option<Vec<u8>>) = sqlx::query_as(
        "SELECT fallback_to_main, unassigned_gold_pouch, uuid_send(unassigned_container_id)
         FROM game_character_quick_loot_settings WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character)
    .fetch_one(database)
    .await?;
    if actual != (false, true, Some(pouch.to_vec())) {
        return Err(
            "I07 expected Unassigned to resolve to the exact Inbox pouch with fallback disabled"
                .into(),
        );
    }
    Ok(())
}

async fn require_routes(
    database: &mut sqlx::PgConnection,
    character: &[u8],
    pouch: &[u8],
    fallback: bool,
    accept_only: bool,
    full: bool,
    gold_to_pouch: bool,
) -> TestResult {
    let settings: (bool, bool, bool, Option<Vec<u8>>) = sqlx::query_as(
        "SELECT accept_only, fallback_to_main, unassigned_gold_pouch, uuid_send(unassigned_container_id)
         FROM game_character_quick_loot_settings WHERE character_id=encode($1,'hex')::uuid",
    ).bind(character).fetch_one(&mut *database).await?;
    let routes: Vec<(i16, bool, Option<Vec<u8>>)> = sqlx::query_as(
        "SELECT category, gold_pouch, uuid_send(container_id)
         FROM game_character_quick_loot_categories WHERE character_id=encode($1,'hex')::uuid ORDER BY category",
    ).bind(character).fetch_all(database).await?;
    let mut expected = vec![
        (0, gold_to_pouch, gold_to_pouch.then(|| pouch.to_vec())),
        (5, true, Some(pouch.to_vec())),
    ];
    if full && !accept_only {
        expected.push((10, false, None));
    }
    if settings != (accept_only, fallback, false, None) || routes != expected {
        return Err(format!(
            "I07 durable routing differs: settings={settings:?} routes={routes:?}"
        )
        .into());
    }
    Ok(())
}

struct Capture<'a> {
    directory: &'a Path,
    full: bool,
    visible_top: Option<[u8; 16]>,
    previous: (u64, u64),
}

impl Capture<'_> {
    async fn loot_all(
        &mut self,
        client: &mut oteryn_dev_client::DevClientSession,
        key: usize,
        east_stack: bool,
        expected: &[(&Placement, bool)],
    ) -> TestResult {
        use oteryn_protocol_oteryn::world_spatial_entities::EntityKind;
        let actor = client.world_spatial().actor_position;
        let x = actor
            .x
            .checked_add(i32::from(east_stack))
            .ok_or("I07 corpse tile overflow")?;
        let nearby: Vec<_> = client
            .world_entities()
            .ok_or("I07 live corpse view missing")?
            .others()
            .filter(|entity| {
                entity.kind == EntityKind::Corpse
                    && (entity.position.x == actor.x
                        || actor.x.checked_add(1) == Some(entity.position.x))
                    && entity.position.y == actor.y
                    && entity.position.floor == actor.floor
            })
            .map(|entity| {
                (
                    entity.position.x,
                    entity.entity.identity,
                    entity.entity.generation,
                )
            })
            .collect();
        let clicked: Vec<_> = nearby
            .iter()
            .filter(|(tile_x, _, _)| *tile_x == x)
            .collect();
        if nearby.len() != 3 || clicked.len() != if east_stack { 2 } else { 1 } {
            return Err(
                format!("I07 probe {key} expected the exact corpse tiles and stack").into(),
            );
        }
        let anchor = clicked.last().ok_or("I07 missing selected corpse")?.1;
        if east_stack && Some(anchor) != self.visible_top {
            return Err(
                "I07 the last published corpse must be the visible top of the east stack".into(),
            );
        }
        self.probe(
            client,
            key,
            QuickLootRequest {
                corpse: Some(anchor),
                ..request(QuickLootAction::LootAll)
            },
            expected,
        )
        .await?;
        write_capture(
            &self.directory.join(format!("i07-{key}-tile.json")),
            &serde_json::to_vec(&serde_json::json!({
                "game_session_id": uuid_text(client.join_snapshot().game_session_id.as_bytes()),
                "command_id": self.previous.0,
                "server_sequence": self.previous.1,
                "actor_x": actor.x, "x": x, "y": actor.y, "floor": actor.floor,
                "corpses": nearby,
            }))?,
        )?;
        Ok(())
    }

    async fn probe(
        &mut self,
        client: &mut oteryn_dev_client::DevClientSession,
        key: usize,
        request: QuickLootRequest,
        expected: &[(&Placement, bool)],
    ) -> TestResult {
        let payload = encode_quick_loot_request(&request)
            .map_err(|error| format!("I07 probe {key} cannot encode {request:?}: {error:?}"))?;
        let session = client.join_snapshot().game_session_id;
        let command = request.action.command_type();
        let capability = if command
            == oteryn_protocol_oteryn::quick_loot::COMMAND_TYPE_QUICK_LOOT_DESTINATIONS
        {
            oteryn_protocol_oteryn::quick_loot::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1
        } else {
            oteryn_protocol_oteryn::quick_loot::CAPABILITY_QUICK_LOOT_V1
        };
        let started = tokio::time::Instant::now();
        let reply = client
            .session_mut()
            .gated_command(capability, command, &payload)
            .await?;
        evidence(&format!(
            "I07_CAPTURE_PROBE probe={key} elapsed_ms={}",
            started.elapsed().as_millis()
        ));
        let expected_status = if matches!(key, 19 | 20 | 25) {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        if reply.status != expected_status
            || client.join_snapshot().game_session_id != session
            || reply.command_id <= self.previous.0
            || reply.server_sequence <= self.previous.1
        {
            return Err(format!(
                "I07 first probe {key} {request:?} refused or misbound: request_frame={payload:02x?} {reply:?}"
            )
            .into());
        }
        self.previous = (reply.command_id, reply.server_sequence);
        if expected_status == CommandStatus::Rejected {
            let reason = if key == 19 {
                QuickLootRefusal::NotSupported
            } else {
                QuickLootRefusal::Stale
            };
            if decode_quick_loot_refusal(&reply.payload) != Ok(reason) {
                return Err(
                    format!("I07 probe {key} expected refusal {reason:?}: {reply:?}").into(),
                );
            }
        } else if request.selected_item.is_some() {
            if QuickLootClickOutcome::decode(&reply.payload) != Ok(QuickLootClickOutcome::Moved) {
                return Err(
                    format!("I07 selected Click probe {key} expected Moved: {reply:?}").into(),
                );
            }
        } else {
            let result =
                decode_quick_loot_result(&reply.payload).map_err(|error| format!("{error:?}"))?;
            let mut actual: Vec<_> = result
                .items
                .iter()
                .map(|item| (item.item.as_slice(), item.accepted))
                .collect();
            let mut expected: Vec<_> = expected
                .iter()
                .map(|(item, accepted)| (item.item.as_slice(), *accepted))
                .collect();
            if request.action == QuickLootAction::LootAll && request.corpse.is_none() {
                actual.sort_unstable();
                expected.sort_unstable();
            }
            if actual != expected {
                return Err(format!(
                    "I07 probe {key} outcomes differ: expected={expected:?} actual={actual:?}"
                )
                .into());
            }
        }
        if self.full
            && matches!(
                request.action,
                QuickLootAction::Click | QuickLootAction::LootAll
            )
        {
            evidence(&format!(
                "I07_FULL_ACTION action={:?} probe={key} status={:?} checked_item_outcomes={}",
                request.action,
                reply.status,
                expected.len()
            ));
        }
        {
            let directory = self.directory;
            write_capture(&directory.join(format!("i07-{key}-request.bin")), &payload)?;
            write_capture(
                &directory.join(format!("i07-{key}.json")),
                &capture_envelope(
                    session.as_bytes(),
                    reply.command_id,
                    reply.server_sequence,
                    reply.status,
                    &reply.payload,
                )?,
            )?;
        }
        Ok(())
    }
}

async fn capture_journey(clients: &SeamClients<'_>) -> TestResult {
    let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("I07 requires a fresh E2E_ALL_CAPTURE_DIR")?;
    journey(clients, Path::new(&directory), false).await
}

async fn capture_restore(
    client: &mut oteryn_dev_client::DevClientSession,
    directory: &Path,
    tag: &str,
) -> TestResult<(u64, u64)> {
    let session = client.join_snapshot().game_session_id;
    let restore_request = encode_quick_loot_request(
        &oteryn_protocol_oteryn::quick_loot::quick_loot_filter_review_request(),
    )
    .map_err(|error| format!("I07 restore request: {error:?}"))?;
    let restore_reply = client
        .session_mut()
        .gated_command(
            oteryn_protocol_oteryn::quick_loot::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
            oteryn_protocol_oteryn::quick_loot::COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            &restore_request,
        )
        .await?;
    if restore_reply.status != CommandStatus::Accepted {
        return Err(format!("I07 {tag} restore refused: {restore_reply:?}").into());
    }
    write_capture(
        &directory.join(format!("i07-{tag}-request.bin")),
        &restore_request,
    )?;
    write_capture(
        &directory.join(format!("i07-{tag}.json")),
        &capture_envelope(
            session.as_bytes(),
            restore_reply.command_id,
            restore_reply.server_sequence,
            restore_reply.status,
            &restore_reply.payload,
        )?,
    )?;

    Ok((restore_reply.command_id, restore_reply.server_sequence))
}

pub(super) async fn journey(
    clients: &SeamClients<'_>,
    capture_dir: &Path,
    full: bool,
) -> TestResult {
    use sqlx::Connection as _;
    const FILTERS: [(bool, &str); 2] = [
        (false, "oteryn:item.tibia.i3577"),
        (true, "oteryn:item.tibia.i3357"),
    ];
    let mut timing = CaptureTiming::default();
    timing.enter("resolve_fixture");
    let index = provisioning::item_index()?;
    let skip_reference = provisioning::item_reference(&index, 3577)?.get();
    let gold_reference = provisioning::item_reference(&index, 3031)?.get();
    let accept_reference = provisioning::item_reference(&index, 3357)?.get();
    let dedicated = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_I07_ACCOUNT",
        Some(if !full {
            "Quick Loot Tester"
        } else {
            "Quick Loot Full"
        }),
    )
    .await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let scoped = SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let clients = &scoped;
    timing.enter("picker_admission");
    let mut client = connect_world(clients, 0x07).await?;
    timing.enter("picker_grants");
    let initial_inventory = client.inventory().ok_or("I07 starter inventory missing")?;
    let missing: Vec<_> = [
        (3577, skip_reference),
        (3031, gold_reference),
        (3357, accept_reference),
    ]
    .into_iter()
    .filter(|(_, definition)| {
        !initial_inventory
            .entries
            .iter()
            .any(|item| item.item_definition_ref.get() == *definition)
    })
    .collect();
    for &(id, _) in &missing {
        let started = tokio::time::Instant::now();
        provision_while_serving(&mut client, format!("item-{id}-1")).await?;
        evidence(&format!(
            "I07_CAPTURE_GRANT definition={id} elapsed_ms={}",
            started.elapsed().as_millis(),
        ));
    }
    timing.enter("picker_logout");
    let picker_seed_session = client.join_snapshot().game_session_id;
    require_logout(&mut client).await?;
    drop(client);
    timing.enter("loot_admission");
    let mut client = connect_world(clients, 0x07).await?;
    if client.join_snapshot().game_session_id == picker_seed_session {
        return Err("I07 picker inventory admission reused its provisioning session".into());
    }
    let admitted_position = client.world_spatial().actor_position;
    if tile(admitted_position)? != TOWN {
        return Err("I07 fixture requires admission at the seeded corpse tile".into());
    }
    timing.enter("loot_provisioning");
    let provisioning_log = provision_while_serving(&mut client, "i07-loot".into()).await?;
    if !provisioning_log.is_empty() {
        evidence(&provisioning_log);
    }
    timing.enter("fixture_readback");
    evidence("FIXTURE I07 premium=granted purpose=category_prerequisite");
    let character = &clients.characters[1];
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    require_filters(&mut database, character, false, &[]).await?;
    let settings_exist: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_quick_loot_settings
         WHERE character_id=encode($1,'hex')::uuid)",
    )
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    if settings_exist {
        return Err("I07 dedicated character already has quick-loot settings".into());
    }
    let mut skip = request(QuickLootAction::FilterAdd);
    skip.filter_mode = Some(QuickLootFilterMode::Skip);
    skip.filter_definition_ref = Some(skip_reference);
    let mut accept = request(QuickLootAction::FilterAdd);
    accept.filter_mode = Some(QuickLootFilterMode::Accept);
    accept.filter_definition_ref = Some(accept_reference);
    let (pouch, backpack) = owned_containers(&mut database, character, &clients.world).await?;
    let deliveries: Vec<String> = sqlx::query_scalar(
        "SELECT fulfilment_unit_id FROM store_delivery_receipts
         WHERE character_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid
           AND world_id=encode($3,'hex')::uuid AND result='COMMITTED' LIMIT 2",
    )
    .bind(character.as_slice())
    .bind(pouch.as_slice())
    .bind(clients.world.as_slice())
    .fetch_all(&mut database)
    .await?;
    let [delivery] = deliveries.as_slice() else {
        return Err("I07 expected one committed Store delivery for its Inbox pouch".into());
    };
    let fulfilment = *sqlx::types::Uuid::parse_str(delivery)?.as_bytes();
    if fulfilment == pouch {
        return Err("I07 fulfilment and item instance identifiers must be distinct".into());
    }
    evidence("FIXTURE I07 pouch=store_inbox fulfilment_id=distinct_from_item");

    let cell = |x: i32| {
        [
            x.to_be_bytes().as_slice(),
            admitted_position.y.to_be_bytes().as_slice(),
            admitted_position.floor.to_be_bytes().as_slice(),
        ]
        .concat()
    };
    let on_tile = cell(admitted_position.x);
    let adjacent = cell(
        admitted_position
            .x
            .checked_add(1)
            .ok_or("I07 adjacent tile overflow")?,
    );
    let corpses: Vec<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT uuid_send(r.item_instance_id), g.spatial_position FROM game_item_mint_receipts r
         JOIN game_item_ground_locations g USING (item_instance_id)
         WHERE death_world_id=encode($1,'hex')::uuid AND death_channel_id=encode($2,'hex')::uuid
           AND death_scope_ownership_generation=$3::text::numeric
           AND corpse_top_damage_character_id=encode($4,'hex')::uuid
           AND loot_table_production_key='oteryn:qualification.i07.loot'
           AND loot_purpose_key='CORPSE_MATERIALIZATION' AND draw_ordinal=0
           AND g.spatial_position=ANY($5)
         ORDER BY g.spatial_position, r.item_instance_id
         LIMIT 4",
    )
    .bind(clients.world.as_slice())
    .bind(clients.channel.as_slice())
    .bind(clients.scope_generation.to_string())
    .bind(character.as_slice())
    .bind(vec![on_tile.clone(), adjacent.clone()])
    .fetch_all(&mut database)
    .await?;
    let [
        (corpse, primary_cell),
        (adjacent_a, cell_a),
        (adjacent_b, cell_b),
    ] = corpses.as_slice()
    else {
        return Err("I07 requires exactly three nearby fixture corpses".into());
    };
    if primary_cell != &on_tile || cell_a != &adjacent || cell_b != &adjacent {
        return Err("I07 requires a primary corpse and two corpses stacked one tile east".into());
    }
    let before = placements(&mut database, Storage::Corpse, character, corpse).await?;
    let a = placements(&mut database, Storage::Corpse, character, adjacent_a).await?;
    let b = placements(&mut database, Storage::Corpse, character, adjacent_b).await?;
    let (stacked, stacked_before, listed_corpse, listed_before) = if a
        .first()
        .is_some_and(|entry| entry.key == "oteryn:item.tibia.i3578")
    {
        (adjacent_a, a, adjacent_b, b)
    } else {
        (adjacent_b, b, adjacent_a, a)
    };
    let ([coin, armor, filtered, spare], [other], [listed]) = (
        before.as_slice(),
        stacked_before.as_slice(),
        listed_before.as_slice(),
    ) else {
        return Err("I07 fixture corpse count differs".into());
    };
    evidence(if full {
        "I07_FULL_INPUT corpses=3 east_stack=2 source=qualification_fixture"
    } else {
        "FIXTURE I07.client corpses=3 east_stack=2 source=qualification_fixture"
    });
    if !before
        .iter()
        .chain(&stacked_before)
        .chain(&listed_before)
        .map(|row| (row.key.as_str(), row.quantity, row.slot))
        .eq([
            ("oteryn:item.tibia.i3031", 7, 1),
            ("oteryn:item.tibia.i3357", 1, 2),
            ("oteryn:item.tibia.i3577", 1, 3),
            ("oteryn:item.tibia.i3577", 1, 4),
            ("oteryn:item.tibia.i3578", 1, 1),
            ("oteryn:item.tibia.i3577", 1, 1),
        ])
    {
        return Err("I07 fixture item definition, count or slot differs".into());
    }
    // The fixture publishes the Skip-listed corpse after the food corpse.
    // The most recently published corpse must be traversed first.
    let east_outcomes = |move_food| [(listed, false), (other, move_food)];
    let picker_inventory = client
        .inventory()
        .ok_or("I07 admitted picker inventory missing")?;
    let mut picker_handles = Vec::new();
    for definition in [skip_reference, gold_reference, accept_reference] {
        let mut entries = picker_inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref.get() == definition);
        let entry = entries
            .next()
            .ok_or("I07 live backpack lacks a filter picker item")?;
        if missing.iter().any(|&(_, granted)| granted == definition) {
            if entry.count.get() != 1 || entries.next().is_some() {
                return Err(format!(
                    "I07 picker fixture produced ambiguous definition {definition}"
                )
                .into());
            }
            picker_handles.push(entry.handle);
        }
    }
    write_capture(
        &capture_dir.join("i07-picker-inventory.bin"),
        &oteryn_protocol_oteryn::item_view::encode_character_inventory_with_appearances(
            picker_inventory,
            true,
            true,
        )
        .map_err(|error| format!("I07 picker inventory: {error:?}"))?,
    )?;
    let mut expected_backpack =
        placements(&mut database, Storage::Backpack, character, &backpack).await?;
    require_placements(&mut database, Storage::InboxPouch, character, &pouch, &[]).await?;
    let picker_keys: Vec<_> = missing
        .iter()
        .map(|(id, _)| format!("oteryn:item.tibia.i{id}"))
        .collect();
    let seeded_count = expected_backpack.len();
    expected_backpack.retain(|item| !picker_keys.contains(&item.key));
    if expected_backpack.len() + picker_keys.len() != seeded_count {
        return Err("I07 picker fixture custody differs from granted inputs".into());
    }
    let mut bag_armor = armor.clone();
    bag_armor.slot = expected_backpack.last().map_or(1, |item| item.slot + 1);
    let mut pouch_coin = coin.clone();
    pouch_coin.slot = 1;
    let expected_pouch = [pouch_coin];
    let expected_filters = [FILTERS[0], (true, "oteryn:item.tibia.i3031"), FILTERS[1]];
    let original_session = client.join_snapshot().game_session_id;
    let session = uuid_text(original_session.as_bytes());
    use oteryn_protocol_oteryn::store as store_wire;
    let inbox_request = store_wire::encode_inbox_query(None);
    let inbox_reply = client
        .session_mut()
        .gated_command(
            store_wire::CAPABILITY_STORE_V1,
            store_wire::COMMAND_TYPE_STORE_INBOX_CLAIM,
            &inbox_request,
        )
        .await?;
    let inbox_page = store_wire::decode_inbox_page(&inbox_reply.payload)
        .map_err(|error| format!("I07 Store Inbox page: {error:?}"))?;
    if inbox_reply.status != CommandStatus::Accepted
        || !inbox_page
            .entries
            .iter()
            .any(|entry| entry.item_id == 23721 && entry.fulfilment_unit_id == fulfilment)
    {
        return Err(
            "I07 authenticated Store Inbox omitted the fixture Gold Pouch fulfilment".into(),
        );
    }
    write_capture(&capture_dir.join("i07-inbox-request.bin"), &inbox_request)?;
    write_capture(
        &capture_dir.join("i07-inbox.json"),
        &capture_envelope(
            original_session.as_bytes(),
            inbox_reply.command_id,
            inbox_reply.server_sequence,
            inbox_reply.status,
            &inbox_reply.payload,
        )?,
    )?;
    let mut capture = Capture {
        directory: capture_dir,
        full,
        visible_top: None,
        previous: (inbox_reply.command_id, inbox_reply.server_sequence),
    };
    let route = |category, recipient, recipient_id, fallback_to_main| QuickLootRequest {
        category,
        recipient: Some(recipient),
        recipient_id,
        fallback_to_main,
        ..request(QuickLootAction::AssignLoot)
    };
    let armor_route = |fallback| {
        route(
            Some(QuickLootCategory::Armor),
            QuickLootRecipient::GoldPouch,
            Some(fulfilment),
            fallback,
        )
    };
    let gold_route = route(
        Some(QuickLootCategory::Gold),
        QuickLootRecipient::GoldPouch,
        Some(fulfilment),
        false,
    );
    let unassigned = route(None, QuickLootRecipient::MainContainer, None, false);

    timing.enter("initial_loot_commands");
    capture.probe(&mut client, 1, skip, &[]).await?;
    require_filters(&mut database, character, false, &FILTERS[..1]).await?;
    capture
        .probe(
            &mut client,
            2,
            QuickLootRequest {
                filter_definition_ref: Some(gold_reference),
                ..accept
            },
            &[],
        )
        .await?;
    require_filters(&mut database, character, true, &expected_filters[..2]).await?;
    let unassigned_pouch = route(None, QuickLootRecipient::GoldPouch, Some(fulfilment), false);
    capture.probe(&mut client, 3, unassigned_pouch, &[]).await?;
    let (fallback, assigned, recipient): (bool, bool, Option<Vec<u8>>) = sqlx::query_as(
        "SELECT fallback_to_main, unassigned_gold_pouch, uuid_send(unassigned_container_id)
         FROM game_character_quick_loot_settings WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    if (fallback, assigned, recipient.as_deref()) != (false, true, Some(pouch.as_slice())) {
        return Err("I07 Initial Unassigned assignment did not persist the exact pouch".into());
    }
    evidence("I07_UNASSIGNED_ASSIGNMENT initial=true recipient=exact_inbox_pouch fallback=false");
    capture.probe(&mut client, 4, gold_route, &[]).await?;
    let gold_target: (bool, Option<Vec<u8>>) = sqlx::query_as(
        "SELECT gold_pouch, uuid_send(container_id) FROM game_character_quick_loot_categories
         WHERE character_id=encode($1,'hex')::uuid AND category=0",
    )
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    if gold_target != (true, Some(pouch.to_vec())) {
        return Err("I07 Gold assignment did not resolve its fulfilment to the exact pouch".into());
    }
    evidence("I07_POUCH_ASSIGNMENT wire=fulfilment_id stored=exact_item_instance");
    // Restore a distinct Unassigned route before testing Gold transfer.
    capture.probe(&mut client, 5, unassigned, &[]).await?;
    let unassigned_state: (bool, bool, Option<Vec<u8>>) = sqlx::query_as(
        "SELECT fallback_to_main, unassigned_gold_pouch, uuid_send(unassigned_container_id)
         FROM game_character_quick_loot_settings WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    if unassigned_state != (false, false, None) {
        return Err(
            "I07 Gold isolation: Unassigned still points at a container before transfer".into(),
        );
    }
    evidence("I07_GOLD_ISOLATION unassigned=main fallback=off before_transfer=true");
    capture
        .probe(
            &mut client,
            6,
            request(QuickLootAction::LootAll),
            &[
                (coin, true),
                (armor, false),
                (filtered, false),
                (spare, false),
                (listed, false),
                (other, false),
            ],
        )
        .await?;
    require_placements(
        &mut database,
        Storage::InboxPouch,
        character,
        &pouch,
        &expected_pouch,
    )
    .await?;
    evidence("I07_LOOT_CUSTODY category=gold exact_coins=7 destination=inbox_pouch");

    // Reset Gold to main before exercising Equipment, so misclassifying armor as Gold fails.
    let gold_main = route(
        Some(QuickLootCategory::Gold),
        QuickLootRecipient::MainContainer,
        None,
        false,
    );
    capture.probe(&mut client, 7, gold_main, &[]).await?;
    capture
        .probe(&mut client, 8, armor_route(false), &[])
        .await?;
    capture.probe(&mut client, 9, accept, &[]).await?;
    // Remove the three temporary picker inputs before the capacity-sensitive transfer.
    let discard = oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: admitted_position
            .x
            .checked_sub(1)
            .ok_or("I07 fixture discard tile overflow")?,
        y: admitted_position.y,
        floor: admitted_position.floor,
    };
    for source in picker_handles {
        let moved = client
            .session_mut()
            .move_item_counted(oteryn_dev_client::client_runtime::inventory::MoveRequest {
                source,
                destination: oteryn_dev_client::ItemMoveDestination::Ground(discard),
                count: None,
            })
            .await?;
        if moved.outcome != oteryn_dev_client::ItemMoveOutcome::Moved {
            return Err(format!("I07 first picker fixture discard refused: {moved:?}").into());
        }
    }
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    evidence("FIXTURE I07 picker inputs=discarded backpack=exact_starter_custody");
    require_filters(&mut database, character, true, &expected_filters).await?;
    require_routes(&mut database, character, &pouch, false, true, full, false).await?;
    capture
        .loot_all(
            &mut client,
            10,
            false,
            &[(armor, false), (filtered, false), (spare, false)],
        )
        .await?;
    let remaining = [armor.clone(), filtered.clone(), spare.clone()];
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        corpse,
        &remaining,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        stacked,
        &stacked_before,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::InboxPouch,
        character,
        &pouch,
        &expected_pouch,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    evidence(
        "I07_LOOT_CUSTODY fallback=off category=equipment armor=refused unassigned=main stacked=retained",
    );

    capture
        .probe(&mut client, 11, armor_route(true), &[])
        .await?;
    require_routes(&mut database, character, &pouch, true, true, full, false).await?;
    timing.enter("loot_logout");
    require_logout(&mut client).await?;
    drop(client);
    timing.enter("relog_admission");
    let mut fresh = connect_world(clients, 0x07).await?;
    timing.enter("relog_loot_commands");
    let fresh_session = fresh.join_snapshot().game_session_id;
    if fresh_session == original_session
        || owned_containers(&mut database, character, &clients.world).await? != (pouch, backpack)
    {
        return Err("I07 relog changed the original Inbox pouch or equipped backpack custody, or reused a session".into());
    }
    require_filters(&mut database, character, true, &expected_filters).await?;
    require_routes(&mut database, character, &pouch, true, true, full, false).await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        corpse,
        &remaining,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        stacked,
        &stacked_before,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::InboxPouch,
        character,
        &pouch,
        &expected_pouch,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    capture.previous = capture_restore(&mut fresh, capture_dir, "relog-restore").await?;
    capture
        .loot_all(
            &mut fresh,
            12,
            false,
            &[(armor, true), (filtered, false), (spare, false)],
        )
        .await?;
    expected_backpack.push(bag_armor);
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        corpse,
        &[filtered.clone(), spare.clone()],
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        stacked,
        &stacked_before,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::InboxPouch,
        character,
        &pouch,
        &expected_pouch,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    evidence(
        "I07_LOOT_CUSTODY fallback=on relog=true armor=backpack filtered=corpse stacked=retained",
    );

    timing.enter("corpse_commands");
    // Manual click bypasses the active Accept filter and targets only this corpse.
    let (corpse_entity, corpse_handle, selected, sibling, opened) =
        open_primary_corpse(&mut fresh, skip_reference).await?;
    let click = QuickLootRequest {
        corpse: Some(corpse_entity.entity.identity),
        selected_item: Some(selected),
        ..request(QuickLootAction::Click)
    };
    capture
        .probe(&mut fresh, 13, click, &[(filtered, true)])
        .await?;
    let after_click = require_corpse_view(&mut fresh, corpse_handle, &[sibling]).await?;
    evidence("I07_HANDLE_CLICK status=moved corpse_view_delta=exact_sibling");
    let mut bag_filtered = filtered.clone();
    bag_filtered.slot = expected_backpack
        .last()
        .ok_or("I07 backpack lost armor")?
        .slot
        + 1;
    expected_backpack.push(bag_filtered);
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        corpse,
        std::slice::from_ref(spare),
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        stacked,
        &stacked_before,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::InboxPouch,
        character,
        &pouch,
        &expected_pouch,
    )
    .await?;
    // Route the otherwise unclassified food to the coin-only pouch with fallback off.
    // Ignoring this assignment would silently move it to the default backpack.
    capture
        .probe(
            &mut fresh,
            14,
            QuickLootRequest {
                filter_mode: Some(QuickLootFilterMode::Skip),
                ..request(QuickLootAction::FilterSelect)
            },
            &[],
        )
        .await?;
    require_filters(&mut database, character, false, &expected_filters).await?;
    // Spatial corpse identities name actor projections, not durable item UUIDs.
    // Open exactly the native top candidate and independently identify it by the
    // fixture's unique listed item before using it as the source.
    let position = fresh.world_spatial().actor_position;
    let east_x = position.x.checked_add(1).ok_or("I07 east tile overflow")?;
    let visible_top = fresh
        .world_entities()
        .ok_or("I07 east stack unavailable")?
        .others()
        .filter(|entity| {
            entity.kind == oteryn_protocol_oteryn::world_spatial_entities::EntityKind::Corpse
                && entity.position.x == east_x
                && entity.position.y == position.y
                && entity.position.floor == position.floor
        })
        .map(|entity| entity.entity.identity)
        .last()
        .ok_or("I07 east stack has no top corpse")?;
    open_foreign_corpse(&mut fresh, visible_top, skip_reference).await?;
    capture.visible_top = Some(visible_top);
    if full {
        capture
            .probe(&mut fresh, 21, unassigned_pouch.clone(), &[])
            .await?;
        require_unassigned_pouch(&mut database, character, &pouch).await?;
        capture
            .loot_all(&mut fresh, 22, true, &east_outcomes(false))
            .await?;
        require_placements(
            &mut database,
            Storage::Corpse,
            character,
            stacked,
            &stacked_before,
        )
        .await?;
        require_placements(
            &mut database,
            Storage::Backpack,
            character,
            &backpack,
            &expected_backpack,
        )
        .await?;
        capture
            .probe(
                &mut fresh,
                23,
                route(None, QuickLootRecipient::MainContainer, None, false),
                &[],
            )
            .await?;
        evidence("I07_FULL_UNASSIGNED_TRANSFER destination=exact_pouch food=retained fallback=off");
    }
    let late_pouch = if full {
        route(
            Some(QuickLootCategory::Food),
            QuickLootRecipient::GoldPouch,
            Some(fulfilment),
            false,
        )
    } else {
        unassigned_pouch
    };
    capture.probe(&mut fresh, 15, late_pouch, &[]).await?;
    if full {
        let food: (bool, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT gold_pouch, uuid_send(container_id)
             FROM game_character_quick_loot_categories
             WHERE character_id=encode($1,'hex')::uuid AND category=10",
        )
        .bind(character.as_slice())
        .fetch_one(&mut database)
        .await?;
        let unassigned_to_pouch: bool = sqlx::query_scalar(
            "SELECT unassigned_gold_pouch FROM game_character_quick_loot_settings
             WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(character.as_slice())
        .fetch_one(&mut database)
        .await?;
        if food != (true, Some(pouch.to_vec())) || unassigned_to_pouch {
            return Err("I07 full expected a Food-only route to the exact Gold Pouch".into());
        }
    } else {
        require_unassigned_pouch(&mut database, character, &pouch).await?;
    }
    capture
        .loot_all(&mut fresh, 16, true, &east_outcomes(false))
        .await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        stacked,
        &stacked_before,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    let late_main = if full {
        route(
            Some(QuickLootCategory::Food),
            QuickLootRecipient::MainContainer,
            None,
            false,
        )
    } else {
        unassigned
    };
    capture.probe(&mut fresh, 17, late_main, &[]).await?;
    capture
        .loot_all(&mut fresh, 18, true, &east_outcomes(true))
        .await?;
    // The starter backpack already contains one of this food. The transfer merges
    // into that existing identity and slot rather than placing the corpse identity.
    let mut stacks = expected_backpack
        .iter_mut()
        .filter(|entry| entry.key == other.key);
    let stack = stacks.next().ok_or("I07 starter food stack is absent")?;
    if stacks.next().is_some() || stack.quantity != 1 {
        return Err("I07 expected one starter food stack containing exactly one item".into());
    }
    stack.quantity += other.quantity;
    require_retired_donor(&mut database, &other.item).await?;
    require_placements(&mut database, Storage::Corpse, character, stacked, &[]).await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        listed_corpse,
        std::slice::from_ref(listed),
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::InboxPouch,
        character,
        &pouch,
        &expected_pouch,
    )
    .await?;
    evidence(if full {
        "I07_LOOT_CUSTODY food=pouch_refused_then_main_moved"
    } else {
        "I07_LOOT_CUSTODY unassigned=pouch_refused_then_main_moved"
    });
    capture
        .probe(
            &mut fresh,
            19,
            route(None, QuickLootRecipient::GoldPouch, Some(backpack), true),
            &[],
        )
        .await?;
    require_routes(&mut database, character, &pouch, false, false, full, false).await?;
    require_filters(&mut database, character, false, &expected_filters).await?;
    evidence("I07_INVALID_POUCH status=rejected settings=unchanged");
    let _ = fresh.take_events();
    capture.probe(&mut fresh, 20, click, &[]).await?;
    quick_loot_reply_barrier(&mut fresh).await?;
    if fresh
        .take_events()
        .into_iter()
        .any(|event| matches!(event, oteryn_dev_client::SessionEvent::OpenContainer(_)))
    {
        return Err("I07 stale Click emitted an open-container delta".into());
    }
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        corpse,
        std::slice::from_ref(spare),
    )
    .await?;
    require_placements(&mut database, Storage::Corpse, character, stacked, &[]).await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        listed_corpse,
        std::slice::from_ref(listed),
    )
    .await?;
    evidence("I07_SELECTED_CLICK sibling=retained stale_handle=rejected");
    let quiver = if full {
        Some(exercise_quiver(&mut fresh, &mut database, character, &backpack).await?)
    } else {
        None
    };
    capture
        .probe(
            &mut fresh,
            24,
            route(
                Some(QuickLootCategory::Gold),
                QuickLootRecipient::GoldPouch,
                Some(fulfilment),
                false,
            ),
            &[],
        )
        .await?;
    require_routes(&mut database, character, &pouch, false, false, full, true).await?;
    let foreign_view = open_foreign_corpse(&mut fresh, visible_top, skip_reference).await?;
    let foreign = foreign_view.value.entries[0].handle;
    if foreign == selected || foreign == sibling {
        return Err("I07 foreign item aliases the primary corpse item".into());
    }
    capture
        .probe(
            &mut fresh,
            25,
            QuickLootRequest {
                selected_item: Some(foreign),
                ..click
            },
            &[],
        )
        .await?;
    quick_loot_reply_barrier(&mut fresh).await?;
    if fresh
        .take_events()
        .into_iter()
        .any(|event| matches!(event, oteryn_dev_client::SessionEvent::OpenContainer(_)))
    {
        return Err("I07 a live foreign handle changed a container".into());
    }
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        listed_corpse,
        std::slice::from_ref(listed),
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        corpse,
        std::slice::from_ref(spare),
    )
    .await?;
    evidence("I07_FOREIGN_CLICK refusal=stale live_item=foreign_corpse custody=unchanged");
    timing.enter("corpse_logout");
    require_logout(&mut fresh).await?;
    drop(fresh);
    timing.enter("readback_admission");
    let mut readback = connect_world(clients, 0x07).await?;
    timing.enter("final_readback");
    let readback_session = readback.join_snapshot().game_session_id;
    if readback_session == original_session
        || readback_session == fresh_session
        || owned_containers(&mut database, character, &clients.world).await? != (pouch, backpack)
    {
        return Err("I07 final relog changed container custody or reused a session".into());
    }
    require_placements(
        &mut database,
        Storage::InboxPouch,
        character,
        &pouch,
        &expected_pouch,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Backpack,
        character,
        &backpack,
        &expected_backpack,
    )
    .await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        corpse,
        std::slice::from_ref(spare),
    )
    .await?;
    require_placements(&mut database, Storage::Corpse, character, stacked, &[]).await?;
    require_placements(
        &mut database,
        Storage::Corpse,
        character,
        listed_corpse,
        std::slice::from_ref(listed),
    )
    .await?;
    require_filters(&mut database, character, false, &expected_filters).await?;
    require_routes(&mut database, character, &pouch, false, false, full, true).await?;
    require_retired_donor(&mut database, &other.item).await?;
    evidence("I07_MERGE_DONOR lifecycle=retired quantity=0 after_relog=true");
    capture_restore(&mut readback, capture_dir, "restore").await?;
    let inventory = readback
        .inventory()
        .ok_or("I07 fresh client inventory snapshot absent")?;
    let mut actual: Vec<_> = inventory
        .entries
        .iter()
        .map(|entry| {
            (
                entry.item_definition_ref.get(),
                i64::from(entry.count.get()),
            )
        })
        .collect();
    let mut expected = Vec::new();
    for entry in &expected_backpack {
        let tibia_id: u32 = entry
            .key
            .strip_prefix("oteryn:item.tibia.i")
            .ok_or("I07 expected authored Tibia item")?
            .parse()?;
        expected.push((
            provisioning::item_reference(&index, tibia_id)?.get(),
            entry.quantity,
        ));
    }
    actual.sort_unstable();
    expected.sort_unstable();
    if actual != expected {
        return Err(format!(
            "I07 relog inventory hydration differs: expected={expected:?} actual={actual:?}"
        )
        .into());
    }
    evidence("I07_CLIENT_INVENTORY final_relog=exact_definitions_and_quantities");
    if let Some((quiver, ammunition)) = quiver {
        require_quiver(&mut database, character, &quiver, &ammunition).await?;
        let index = provisioning::item_index()?;
        let definition = provisioning::item_reference(&index, 35562)?;
        if !inventory.equipment.iter().any(|entry| {
            entry.item.item_definition_ref == definition
                && Some(entry.slot) == oteryn_protocol_oteryn::item_view::wire_slot(6)
                && entry.item.count.get() == 1
        }) {
            return Err("I07 full relog did not expose the quiver in the shield-hand slot".into());
        }
        evidence("I07_FULL_QUIVER bolts=7 backpack_debit=7 after_relog=true");
    }
    timing.enter("final_logout");
    require_logout(&mut readback).await?;
    timing.record("complete");
    evidence(
        "I07_LOOT_CUSTODY final_relog=true exact_ids_counts_slots=true pouch=inbox backpack=equipped click_sibling=retained skip_listed=retained",
    );
    let fresh_session = uuid_text(fresh_session.as_bytes());
    {
        write_capture(
            &capture_dir.join("i07-expected.txt"),
            &serde_json::to_vec(&serde_json::json!({
                "final_session": uuid_text(readback_session.as_bytes()),
                "skip": skip_reference, "gold": gold_reference, "listed": listed.item, "backpack": backpack, "accept": accept_reference, "pouch": pouch, "east_top": visible_top, "foreign_view": captured_container(foreign_view)?, "fulfilment": fulfilment, "selected_handle": selected.get(),
                "corpse_kind": corpse_entity.kind as u8,
                "corpse": corpse_entity.entity.identity, "corpse_generation": corpse_entity.entity.generation,
                "corpse_position": [corpse_entity.position.x, corpse_entity.position.y, i32::from(corpse_entity.position.floor)],
                "corpse_definition": provisioning::item_reference(&index, 5973)?.get(),
                "opened": captured_container(opened)?, "after_click": captured_container(after_click)?,
                "coin": coin.item, "armor": armor.item, "filtered": filtered.item, "spare": spare.item, "other": other.item,
            }))?,
        )?;
        let marker = if full {
            "I07_FULL_LIVE_CAPTURE"
        } else {
            "I07_LIVE_CAPTURE"
        };
        evidence(&format!(
            "{marker} session={session} relog={fresh_session} final={}",
            uuid_text(readback_session.as_bytes())
        ));
    }
    Ok(())
}

async fn require_logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let logout = client.session_mut().logout().await?;
    if logout.status != CommandStatus::Accepted
        || logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("I07 first-attempt logout refused: {logout:?}").into());
    }
    Ok(())
}

async fn require_quiver(
    database: &mut sqlx::PgConnection,
    character: &[u8],
    quiver: &[u8],
    arrows: &[Placement],
) -> TestResult {
    let worn = worn_quiver(database, character).await?;
    if worn.as_deref() != Some(quiver) {
        return Err(format!("I07 quiver equipment custody differs: {worn:?}").into());
    }
    require_placements(database, Storage::Quiver, character, quiver, arrows).await
}

async fn worn_quiver(
    database: &mut sqlx::PgConnection,
    character: &[u8],
) -> TestResult<Option<Vec<u8>>> {
    // Storage slot 6 is the shield hand, independent of wire slot numbers.
    Ok(sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id) FROM game_character_equipment_slots s
         JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE s.character_id=encode($1,'hex')::uuid AND s.slot=6
           AND i.lifecycle=1 AND i.quantity=1 AND i.definition_production_key='oteryn:item.tibia.i35562'",
    )
    .bind(character)
    .fetch_optional(database)
    .await?)
}

async fn exercise_quiver(
    client: &mut oteryn_dev_client::DevClientSession,
    database: &mut sqlx::PgConnection,
    character: &[u8],
    backpack: &[u8],
) -> TestResult<(Vec<u8>, Vec<Placement>)> {
    use oteryn_dev_client::{
        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome,
        client_runtime::hotkey_equip::equip_definition,
    };

    let quiver = worn_quiver(database, character)
        .await?
        .ok_or("I07 full Paladin starter quiver is absent from the shield hand")?;
    let mut expected = placements(database, Storage::Quiver, character, &quiver).await?;
    if !expected
        .iter()
        .map(|item| (item.key.as_str(), item.quantity, item.slot))
        .eq([
            ("oteryn:item.tibia.i3447", 100, 1),
            ("oteryn:item.tibia.i3447", 100, 2),
        ])
    {
        return Err(format!("I07 full starter quiver custody differs: {expected:?}").into());
    }
    let before = placements(database, Storage::Backpack, character, backpack).await?;
    character_prep::grant_item(client, 3446, 7).await?;
    evidence("FIXTURE I07.full bolts=3446 count=7");
    let seeded = placements(database, Storage::Backpack, character, backpack).await?;
    let added: Vec<_> = seeded
        .iter()
        .filter(|item| !before.contains(item))
        .collect();
    let [bolts] = added.as_slice() else {
        return Err("I07 full fixture must add exactly one stack to the backpack".into());
    };
    if (bolts.key.as_str(), bolts.quantity) != ("oteryn:item.tibia.i3446", 7)
        || seeded.len() != before.len() + 1
    {
        return Err(format!("I07 full bolt fixture custody differs: {added:?}").into());
    }
    require_quiver(database, character, &quiver, &expected).await?;

    let index = provisioning::item_index()?;
    let definition = provisioning::item_reference(&index, 3446)?;
    let quiver_definition = provisioning::item_reference(&index, 35562)?;
    let shield_slot = oteryn_protocol_oteryn::item_view::wire_slot(6)
        .ok_or("I07 shield-hand slot is not defined")?;
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .equipment
                .iter()
                .find(|entry| {
                    entry.slot == shield_slot && entry.item.item_definition_ref == quiver_definition
                })
                .map(|entry| entry.item.handle)
        })
        .ok_or("I07 missing shield-hand quiver handle")?;
    let unequip = client
        .move_item(&ItemMoveIntent {
            source,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if unequip.outcome != ItemMoveOutcome::Moved
        || worn_quiver(database, character).await?.is_some()
    {
        return Err(
            format!("I07 quiver unequip did not clear the shield hand: {unequip:?}").into(),
        );
    }
    let mut unworn = seeded.clone();
    unworn.push(Placement {
        key: "oteryn:item.tibia.i35562".into(),
        item: quiver.clone(),
        quantity: 1,
        slot: seeded.last().map_or(1, |entry| entry.slot + 1),
    });
    require_placements(database, Storage::Backpack, character, backpack, &unworn).await?;
    let refused = equip_definition(client.session_mut(), definition).await?;
    if refused != ItemMoveOutcome::SlotMismatch || worn_quiver(database, character).await?.is_some()
    {
        return Err(format!(
            "I07 bolts without a worn quiver should receive SlotMismatch: {refused:?}"
        )
        .into());
    }
    require_placements(database, Storage::Backpack, character, backpack, &unworn).await?;
    require_placements(database, Storage::Quiver, character, &quiver, &expected).await?;
    let equipped = equip_definition(client.session_mut(), quiver_definition).await?;
    if equipped != ItemMoveOutcome::Moved {
        return Err(format!("I07 first quiver re-equip refused: {equipped:?}").into());
    }
    require_quiver(database, character, &quiver, &expected).await?;
    require_placements(database, Storage::Backpack, character, backpack, &seeded).await?;
    evidence(
        "I07_FULL_QUIVER unequip=moved unarmed_ammo=slot_mismatch re_equip=moved slot=shield_hand",
    );

    let reply = equip_definition(client.session_mut(), definition).await?;
    if reply != ItemMoveOutcome::Moved {
        return Err(format!("I07 equip bolts into the worn quiver: {reply:?}").into());
    }
    let mut credited = (*bolts).clone();
    credited.slot = 3;
    expected.push(credited);
    require_quiver(database, character, &quiver, &expected).await?;
    require_placements(database, Storage::Backpack, character, backpack, &before).await?;
    evidence("I07_FULL_QUIVER bolts=7 backpack_debit=7 first_attempt=true");
    Ok((quiver, expected))
}

type CorpseDelta = oteryn_dev_client::AppliedDelta<oteryn_dev_client::OpenContainer>;

fn captured_container(delta: CorpseDelta) -> TestResult<serde_json::Value> {
    Ok(serde_json::json!({
        "server_sequence": delta.server_sequence,
        "base_revision": delta.base_revision,
        "new_revision": delta.new_revision,
        "payload": oteryn_protocol_oteryn::item_view::encode_open_container_with_appearances(&delta.value, true)
            .map_err(|error| format!("I07 open-corpse capture: {error:?}"))?,
    }))
}

fn matching_corpse_delta(
    client: &mut oteryn_dev_client::DevClientSession,
    corpse: oteryn_dev_client::ItemHandle,
) -> TestResult<Option<CorpseDelta>> {
    let mut matching = client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            oteryn_dev_client::SessionEvent::OpenContainer(delta) => Some(delta),
            _ => None,
        });
    let first = matching.next();
    if matching.next().is_some()
        || first
            .as_ref()
            .is_some_and(|delta| delta.value.container_handle != Some(corpse))
    {
        return Err("I07 command emitted an extra or wrong-container delta".into());
    }
    Ok(first)
}

async fn open_foreign_corpse(
    client: &mut oteryn_dev_client::DevClientSession,
    identity: [u8; 16],
    item_definition: u32,
) -> TestResult<CorpseDelta> {
    use oteryn_protocol_oteryn::world_spatial_entities::{EntityDetail, EntityKind};
    let corpse = client
        .world_entities()
        .ok_or("I07 foreign corpse view unavailable")?
        .others()
        .find(|entity| entity.kind == EntityKind::Corpse && entity.entity.identity == identity)
        .and_then(|entity| match entity.detail {
            EntityDetail::Object { item_handle, .. }
            | EntityDetail::ObjectWithAppearance { item_handle, .. } => item_handle,
            _ => None,
        })
        .ok_or("I07 foreign corpse handle unavailable")?;
    let reply = client.use_item(corpse).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("I07 foreign corpse open refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(delta) = matching_corpse_delta(client, corpse)? {
            let [entry] = delta.value.entries.as_slice() else {
                return Err("I07 foreign corpse must expose its one live item".into());
            };
            if entry.item_definition_ref.get() != item_definition || entry.count.get() != 1 {
                return Err(format!("I07 visible top corpse has definition {}; expected last-published definition {item_definition}", entry.item_definition_ref).into());
            }
            return Ok(delta);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I07 foreign corpse omitted its container view".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn open_primary_corpse(
    client: &mut oteryn_dev_client::DevClientSession,
    item_definition: u32,
) -> TestResult<(
    oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity,
    oteryn_dev_client::ItemHandle,
    oteryn_dev_client::ItemHandle,
    oteryn_dev_client::ItemHandle,
    CorpseDelta,
)> {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use oteryn_protocol_oteryn::world_spatial_entities::EntityDetail;
    let definition = provisioning::item_reference(&provisioning::item_index()?, 5973)?.get();
    let position = client.world_spatial().actor_position;
    let corpses: Vec<_> = client
        .world_entities()
        .ok_or("I07 corpse entities unavailable")?
        .others()
        .filter_map(|entity| match entity.detail {
            EntityDetail::Object {
                item_definition_ref,
                item_handle,
                ..
            }
            | EntityDetail::ObjectWithAppearance {
                item_definition_ref,
                item_handle,
                ..
            } if item_definition_ref == definition => item_handle.map(|handle| (*entity, handle)),
            _ => None,
        })
        .collect();
    let (primary, stack): (Vec<_>, Vec<_>) = corpses
        .into_iter()
        .partition(|(entity, _)| entity.position == position);
    let [(entity, corpse)] = primary.as_slice() else {
        return Err(
            format!("I07 expected one published corpse at the admitted tile: {primary:?}").into(),
        );
    };
    if entity.kind != oteryn_protocol_oteryn::world_spatial_entities::EntityKind::Corpse {
        return Err(format!("I07 published the loot target as {:?}", entity.kind).into());
    }

    let east = position
        .x
        .checked_add(1)
        .ok_or("I07 corpse position overflow")?;
    if stack
        .iter()
        .filter(|(entity, _)| {
            entity.kind == oteryn_protocol_oteryn::world_spatial_entities::EntityKind::Corpse
                && entity.position.x == east
                && entity.position.y == position.y
                && entity.position.floor == position.floor
        })
        .count()
        != 2
    {
        return Err("I07 world view omitted the two east-tile stacked corpses".into());
    }
    let reply = client.use_item(*corpse).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != UseDisposition::Committed {
        return Err(format!("I07 first corpse open refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(delta) = matching_corpse_delta(client, *corpse)? {
            let [selected, sibling] = delta.value.entries.as_slice() else {
                return Err("I07 opened corpse must contain the selected item and sibling".into());
            };
            if [selected, sibling].iter().any(|item| {
                item.item_definition_ref.get() != item_definition || item.count.get() != 1
            }) || selected.handle == sibling.handle
            {
                return Err(
                    "I07 opened corpse did not expose two distinct expected item handles".into(),
                );
            }
            evidence("I07_CORPSE_OPEN status=committed view_entries=2");
            return Ok((*entity, *corpse, selected.handle, sibling.handle, delta));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I07 committed corpse open omitted its container delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn require_corpse_view(
    client: &mut oteryn_dev_client::DevClientSession,
    corpse: oteryn_dev_client::ItemHandle,
    expected: &[oteryn_dev_client::ItemHandle],
) -> TestResult<CorpseDelta> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(delta) = matching_corpse_delta(client, corpse)? {
            if !delta
                .value
                .entries
                .iter()
                .map(|item| item.handle)
                .eq(expected.iter().copied())
            {
                return Err("I07 selected Click emitted a wrong corpse view delta".into());
            }
            quick_loot_reply_barrier(client).await?;
            if matching_corpse_delta(client, corpse)?.is_some() {
                return Err("I07 selected Click emitted an extra corpse delta".into());
            }
            return Ok(delta);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I07 selected Click omitted its exact corpse view delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}
