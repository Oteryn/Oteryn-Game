//! P18 client replay capture: the real Forge JSON replies for HISTORY, FUSION, DUST_TO_SLIVERS and
//! DUST_LIMIT. The consumer is `apply_result` and `panel_text` in
//! `crates/client-ui/src/features/0152-item-forge`.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{
    server_mechanic::ServerMechanicCommand, world_interaction,
};
use oteryn_dev_client::client_ui::item_forge::{self as forge_ui, ItemForgePanel};
use oteryn_protocol_oteryn::forge_preview as wire;
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

use oteryn_protocol_oteryn::item_forge::COMMAND_TYPE_FORGE_DUST_QUERY;

const FORGE_STATION: TilePos = TilePos {
    x: 32209,
    y: 32274,
    floor: -7,
};
const FORGE_APPROACH: TilePos = TilePos {
    x: 32209,
    y: 32275,
    floor: -7,
};

// Global 15.33 probe: docs/reference/content-prep/forge-content-1/live-client-forge-probe.json
pub(super) const GLOBAL_FIRST_DUST_CAP: u64 = 120;

const ADVENTURERS_STONE: u32 = 16277;
const THAIS_TEMPLE: TilePos = TilePos {
    x: 32369,
    y: 32238,
    floor: -7,
};
const GUILD_LANDING: TilePos = TilePos {
    x: 32210,
    y: 32300,
    floor: -6,
};

pub(super) async fn prepare_station_travel(client: &DevClientSession, row: &str) -> TestResult {
    let position = tile(client.world_spatial().actor_position)?;
    if row == "P18.full" && position == FORGE_APPROACH {
        // Full can continue with the same character after its first Stone trip.
        return Ok(());
    }
    if position != THAIS_TEMPLE {
        return Err(format!("P18 expected the Thais temple fixture, observed {position:?}").into());
    }
    character_prep::grant_item(client, ADVENTURERS_STONE, 1).await?;
    evidence(&format!(
        "FIXTURE row={row} input=adventurers_stone item={ADVENTURERS_STONE} temple={THAIS_TEMPLE:?}"
    ));
    Ok(())
}

async fn enter_guild(client: &mut DevClientSession) -> TestResult {
    // Global city-temple entry: https://tibia.fandom.com/wiki/Adventurer%27s_Stone
    let definitions = provisioning::item_index()?;
    let stone = provisioning::item_reference(&definitions, ADVENTURERS_STONE)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == stone)
        })
        .ok_or("P18 carried Adventurer's Stone missing")?
        .handle;
    let command = client.next_command_id();
    let frame = client.last_server_sequence();
    evidence(&format!(
        "P18_STONE_DISPATCH path=carried_use command_id={command} last_frame={frame}"
    ));
    let reply = client.use_item(handle).await.map_err(|error| {
        use oteryn_dev_client::{DevClientError, SessionError};
        let mut cause = &error;
        while let DevClientError::Context { source, .. } = cause {
            cause = source;
        }
        let (side, kind) = match cause {
            DevClientError::Session(SessionError::ServerProtocolError { .. }) => ("server", "protocol_error"),
            DevClientError::Io(io) | DevClientError::Session(SessionError::Io(io)) => {
                match io.kind() {
                    std::io::ErrorKind::UnexpectedEof => ("peer", "peer_eof"),
                    std::io::ErrorKind::ConnectionReset => ("unknown", "connection_reset"),
                    std::io::ErrorKind::TimedOut => ("unknown", "io_deadline"),
                    _ => ("unknown", "io_error"),
                }
            }
            _ => ("unknown", "command_error"),
        };
        evidence(&format!(
            "P18_STONE_COMMAND_ERROR observed_side={side} kind={kind} last_command={command} last_frame={} error={error}",
            client.last_server_sequence(),
        ));
        error
    })?;
    verdict::Harness::global().progress("stone-reply");
    let landing = tile(client.world_spatial().actor_position)?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        || landing != GUILD_LANDING
    {
        return Err(format!(
            "P18 Adventurer's Stone USE expected {GUILD_LANDING:?}, observed {landing:?}: {reply:?}"
        )
        .into());
    }
    evidence(&format!("P18_STONE_USE landing={landing:?}"));
    Ok(())
}

fn station_opened(
    status: CommandStatus,
    disposition: oteryn_protocol_oteryn::world_object::UseDisposition,
) -> bool {
    status == CommandStatus::Accepted
        && disposition == oteryn_protocol_oteryn::world_object::UseDisposition::ForgeOpened
}

#[test]
fn p18_station_requires_the_actual_forge_open_disposition() {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    assert!(station_opened(
        CommandStatus::Accepted,
        UseDisposition::ForgeOpened
    ));
    assert!(!station_opened(
        CommandStatus::Accepted,
        UseDisposition::Committed
    ));
    assert!(!station_opened(
        CommandStatus::Rejected,
        UseDisposition::ForgeOpened
    ));
}

fn first_station_entry(position: TilePos) -> TestResult {
    if position != THAIS_TEMPLE {
        return Err(format!(
            "P18 expected the Thais temple before Stone USE, observed {position:?}"
        )
        .into());
    }
    Ok(())
}

#[test]
fn p18_first_station_entry_requires_the_temple() {
    assert!(first_station_entry(THAIS_TEMPLE).is_ok());
    assert!(first_station_entry(FORGE_APPROACH).is_err());
}

pub(super) async fn open_station(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    continuing: bool,
) -> TestResult {
    walk_to_station(client, clients, continuing, None).await
}

async fn walk_to_station(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    continuing: bool,
    panel: Option<&mut ItemForgePanel>,
) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    verdict::Harness::global().progress("forge-map-loaded");
    let station = world
        .base()
        .tile(FORGE_STATION.x, FORGE_STATION.y, FORGE_STATION.floor)
        .ok_or("P18 physical Forge tile (32209,32274,7) absent from served region 125,126")?;
    let ordinal = station
        .ids()
        .iter()
        .zip(station.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (&id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            world
                .facts()
                .base_entry(FORGE_STATION, ordinal, id)
                .filter(|entry| entry.appearance_id == 37122 && !entry.pickupable)
                .map(|_| ordinal)
        })
        .ok_or("P18 real Exaltation Forge object i37122 absent from its tile")?;
    let key =
        crate::map::view::placement_key(FORGE_STATION, ordinal).ok_or("Forge placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let position = tile(client.world_spatial().actor_position)?;
    if !continuing || position != FORGE_APPROACH {
        first_station_entry(position)?;
        enter_guild(client).await?;
    }
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| at == FORGE_APPROACH)
        .ok_or("P18 no walkable route to the fixed Exaltation Forge approach")?;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, before, direction).ok_or("invalid Forge route step")?;
        let outcome = client.step(direction).await?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!("P18 Forge step failed at {before:?}: {outcome:?}").into());
        }
        client.take_events();
        verdict::Harness::global().progress("forge-walk");
    }
    let expected_revision = world.facts().object_revision(key);
    if let Some(panel) = panel {
        let session = client.session_mut();
        let header = session
            .world_map_snapshot()
            .ok_or("P18 Forge absent from client map snapshot")?
            .header;
        let command = world_interaction::Command {
            action: world_interaction::Action::Use,
            target: world_interaction::Target::Object(
                world_interaction::world_object::WorldObjectTarget {
                    placement,
                    expected_revision,
                },
            ),
            selection: Some(world_interaction::Selection {
                position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                    x: FORGE_STATION.x.into(),
                    y: FORGE_STATION.y.into(),
                    floor: -i16::from(FORGE_STATION.floor),
                },
                binding: Some((
                    header.content_generation,
                    header.bundle_digest,
                    header.reset_epoch,
                )),
            }),
        };
        let reply = world_interaction::send_framed_reply(session, &command, true).await?;
        if !matches!(reply, world_interaction::Reply::ForgeOpened) {
            return Err(format!("P18 native Forge USE: {}", reply.feedback()).into());
        }
        let browse = forge_ui::begin_open(panel).ok_or("P18 native Forge browse did not start")?;
        let response = session
            .gated_command(browse.capability, browse.kind, &browse.payload)
            .await?;
        forge_ui::preview::apply(panel, &browse, &response);
        if response.status != CommandStatus::Accepted || !panel.open || panel.preview.page.is_none()
        {
            return Err("P18 native Forge panel did not open with the browse page".into());
        }
    } else {
        let reply = client.use_object(&placement, expected_revision).await?;
        if !station_opened(reply.status, reply.disposition) {
            return Err(format!("P18 Forge USE did not open the station: {reply:?}").into());
        }
    }
    verdict::Harness::global().progress("forge-open");
    evidence(&format!(
        "P18_FORGE_OPEN from={start:?} at={FORGE_APPROACH:?}"
    ));
    Ok(())
}

type LiveReply = (u64, u64, u32, Vec<u8>, Vec<u8>);

pub(super) async fn dust(client: &mut DevClientSession) -> TestResult<(u64, u64)> {
    Ok(dust_capture(client).await?.0)
}

async fn dust_capture(client: &mut DevClientSession) -> TestResult<((u64, u64), LiveReply)> {
    let result = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_FORGE_DUST_QUERY, &[])
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("dust query refused: {:?}", result.status).into());
    }
    let body: serde_json::Value = serde_json::from_slice(&result.payload)?;
    let balance = (
        body["current_dust"]
            .as_u64()
            .ok_or("dust balance missing")?,
        body["dust_limit"].as_u64().ok_or("dust limit missing")?,
    );
    Ok((
        balance,
        (
            result.command_id,
            result.server_sequence,
            result.status as u32,
            result.payload,
            Vec::new(),
        ),
    ))
}

fn write_capture(root: &Path, session: [u8; 16], frames: &[LiveReply]) -> TestResult<String> {
    use sha2::Digest as _;
    use std::io::Write as _;
    if frames.len() != 14 {
        return Err(format!(
            "Forge capture requires fourteen live frames, got {}",
            frames.len()
        )
        .into());
    }
    let session = uuid_text(&session);
    let mut requests = Vec::with_capacity(frames.len());
    let mut envelopes = Vec::with_capacity(frames.len());
    for (index, (command_id, server_sequence, status, payload, submitted)) in
        frames.iter().enumerate()
    {
        let mut file = std::fs::File::create_new(root.join(format!("p18-frame-{index}.json")))?;
        let bytes = serde_json::to_vec(&serde_json::json!({
            "command_id": command_id,
            "game_session_id": session,
            "payload": payload,
            "server_sequence": server_sequence,
            "status": status,
        }))?;
        file.write_all(&bytes)?;
        file.flush()?;
        envelopes.push(bytes);
        requests.push((command_id, server_sequence, submitted));
    }
    let mut file = std::fs::File::create_new(root.join("p18-requests.json"))?;
    let manifest = serde_json::to_vec(&(&session, requests))?;
    file.write_all(&manifest)?;
    file.flush()?;
    let mut digest = sha2::Sha256::new();
    for bytes in std::iter::once(manifest).chain(envelopes) {
        digest.update((bytes.len() as u64).to_be_bytes());
        digest.update(bytes);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn occurrence(session: oteryn_protocol_oteryn::GameSessionId, ordinal: u8) -> [u8; 16] {
    use sha2::Digest;
    let digest = sha2::Sha256::new()
        .chain_update(b"E2E-P18-CLIENT/occurrence")
        .chain_update(session.as_bytes())
        .chain_update([ordinal])
        .finalize();
    let mut value = [0; 16];
    value.copy_from_slice(&digest[..16]);
    value[6] = (value[6] & 15) | 0x70;
    value[8] = (value[8] & 63) | 0x80;
    value
}

fn distinct_dagger_pair(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<(NonZeroU64, NonZeroU64)> {
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    let mut daggers = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition && entry.count.get() == 1);
    let first = daggers.next().ok_or("first Forge dagger missing")?.handle;
    let second = daggers.next().ok_or("second Forge dagger missing")?.handle;
    if first == second || daggers.next().is_some() {
        return Err("expected exactly two distinct Forge daggers".into());
    }
    Ok((first, second))
}

async fn forge(
    client: &mut DevClientSession,
    operation: Operation,
    occurrence: [u8; 16],
    pair: Option<(NonZeroU64, NonZeroU64)>,
    history_limit: u8,
) -> TestResult<(LiveReply, serde_json::Value)> {
    let payload = Request {
        operation,
        occurrence,
        first_item: pair.map(|value| value.0),
        second_item: pair.map(|value| value.1),
        boost: false,
        protect: false,
        history_limit,
    }
    .encode()
    .ok_or("forge request was not canonical")?;
    submit_forge(client, operation, forge_ui::command(payload.to_vec())).await
}

async fn submit_forge(
    client: &mut DevClientSession,
    operation: Operation,
    command: ServerMechanicCommand,
) -> TestResult<(LiveReply, serde_json::Value)> {
    if (command.capability, command.command_type)
        != (CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE)
    {
        return Err(format!("{operation:?} client command used incorrect routing").into());
    }
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.command_type, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("{operation:?} status={:?}", reply.status).into());
    }
    let body: serde_json::Value = serde_json::from_slice(&reply.payload)?;
    Ok((
        (
            reply.command_id,
            reply.server_sequence,
            reply.status as u32,
            reply.payload,
            command.payload,
        ),
        body,
    ))
}

async fn capture_preview(
    client: &mut DevClientSession,
    panel: &mut ItemForgePanel,
    selection: wire::Selection,
) -> TestResult<LiveReply> {
    let request =
        forge_ui::preview_request(panel, selection).ok_or("Forge preview did not start")?;
    let response = client
        .session_mut()
        .gated_command(request.capability, request.kind, &request.payload)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "Forge preview {selection:?} was refused: {:?}",
            response.status
        )
        .into());
    }
    forge_ui::preview::apply(panel, &request, &response);
    Ok((
        response.command_id,
        response.server_sequence,
        response.status as u32,
        response.payload,
        request.payload,
    ))
}

fn confirm_resource(
    panel: &mut ItemForgePanel,
    occurrence: [u8; 16],
) -> TestResult<ServerMechanicCommand> {
    let serial = panel.resource_dialog.serial;
    let confirmed = panel
        .confirm_resource(serial, occurrence)
        .ok_or("Forge resource review did not permit confirmation")?;
    if panel.confirm_resource(serial, occurrence).is_some() {
        return Err("Forge accepted the same confirmation twice".into());
    }
    let payload = confirmed
        .encode()
        .ok_or("Forge confirmation did not encode")?;
    let command = forge_ui::request(panel, forge_ui::command(payload.to_vec()))
        .ok_or("Forge confirmation was not submitted")?;
    Ok(command)
}

fn check_refusal(body: &serde_json::Value, reason: &str) -> TestResult {
    if body["succeeded"].as_bool() != Some(false)
        || body["applied"].as_bool() != Some(false)
        || body["reason"].as_str() != Some(reason)
        || body["bonus"].as_u64() != Some(0)
        || body.get("result") != Some(&serde_json::Value::Null)
    {
        return Err(format!(
            "expected Forge refusal {reason} with zero bonus and null result, got {body}"
        )
        .into());
    }
    Ok(())
}

pub(super) async fn seed_dust(row: &str, clients: &SeamClients<'_>) -> TestResult {
    let [character] = clients.characters else {
        return Err("funded Forge fixture requires one dedicated character".into());
    };
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = sqlx::Connection::begin(&mut connection).await?;
    let character = uuid_text(character);
    let entry = uuid_text(&v7(0x44, 0xa1));
    sqlx::query("INSERT INTO game_character_forge_dust(character_id,balance,dust_limit) VALUES($1::uuid,0,100)")
        .bind(&character).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_forge_dust_entries
        (entry_id,character_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after)
        VALUES($1::uuid,$2::uuid,'GAIN','creature_kill',$3::uuid,$4::uuid,100,0,0,100,100,100)")
        .bind(&entry).bind(&character).bind(uuid_text(&v7(0x44, 0xa2)))
        .bind(uuid_text(&v7(0x44, 0xa3))).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_forge_dust SET balance=100,last_entry_id=$1::uuid WHERE character_id=$2::uuid")
        .bind(&entry).bind(&character).execute(&mut *tx).await?;
    tx.commit().await?;
    evidence(&format!(
        "FIXTURE row={row} input=forge_dust before=0 after=100 dust_cap=100"
    ));
    Ok(())
}

fn check_funded(body: &serde_json::Value, operation: Operation) -> TestResult {
    let expected = match operation {
        Operation::DustToSlivers => serde_json::json!({
            "operation":"dust_to_slivers","succeeded":true,"applied":true,"reason":null,"bonus":0,
            "result":{"first_item":null,"first_tier":null,"second_item":null,"second_tier":null,
            "dust":40,"dust_limit":100,"gold_spent":0,"cores_spent":0,
            "slivers_spent":0,"slivers_created":3,"cores_created":0}
        }),
        Operation::DustLimit => serde_json::json!({
            "operation":"dust_limit","succeeded":true,"result":{"dust":15,"dust_limit":GLOBAL_FIRST_DUST_CAP}
        }),
        _ => return Err("unexpected funded Forge action".into()),
    };
    if *body != expected {
        return Err(
            format!("funded Forge {operation:?} result: expected {expected}, got {body}").into(),
        );
    }
    Ok(())
}

fn check_funded_inventory(
    before: &oteryn_protocol_oteryn::item_view::CharacterInventory,
    mut after: oteryn_protocol_oteryn::item_view::CharacterInventory,
    operation: Operation,
    sliver: std::num::NonZeroU32,
) -> TestResult {
    if operation == Operation::DustToSlivers {
        after
            .entries
            .retain(|entry| entry.item_definition_ref != sliver);
    }
    if *before != after {
        return Err(format!("funded {operation:?} changed unrelated inventory").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("Forge capture directory missing")?;
    let root = Path::new(&root);
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P18_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let clients = &scope;
    let mut fixture = connect_character(clients, 0x43, 0).await?;
    prepare_station_travel(&fixture, "P18.client").await?;
    character_prep::grant_item(&fixture, 3267, 2).await?;
    character_prep::grant_item(&fixture, 3043, 3).await?;
    evidence("FIXTURE row=P18.client input=carried_fusion_gold gold=30000");
    if fixture.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge dagger fixture logout refused".into());
    }
    drop(fixture);
    seed_dust("P18.client", clients).await?;
    let mut client = connect_character(clients, 0x44, 0).await?;
    let mut confirmation_panel = ItemForgePanel {
        selected: true,
        ..Default::default()
    };
    confirmation_panel.preview.selected = true;
    walk_to_station(&mut client, clients, false, Some(&mut confirmation_panel)).await?;
    let session = client.join_snapshot().game_session_id;
    let definitions = provisioning::item_index()?;
    let dagger = provisioning::item_reference(&definitions, 3267)?;
    let pair = distinct_dagger_pair(&client, dagger)?;

    let balance = dust(&mut client).await?;
    if balance != (100, 100) {
        return Err(
            format!("funded Forge expected fixture dust 100/100, observed {balance:?}").into(),
        );
    }
    let (history_bytes, history) =
        forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let rows = history
        .as_array()
        .ok_or("HISTORY payload was not an array")?;
    if !rows.is_empty() {
        return Err("fresh Forge history was not empty".into());
    }
    let mut frames = vec![history_bytes];
    let sliver_key = "oteryn:item.tibia.i37109";
    let revision = definitions
        .revision(sliver_key)
        .ok_or("Forge sliver definition missing")?;
    let sliver = definitions
        .definition_ref("Item", sliver_key, revision)
        .ok_or("Forge sliver reference missing")?;
    if super::full_e2ef7::units(&client, sliver) != 0 {
        return Err("funded Forge pre-state already contains slivers".into());
    }
    let baseline = client
        .inventory()
        .cloned()
        .ok_or("initial Forge inventory missing")?;
    let mut bank = sqlx::PgConnection::connect(clients.url).await?;
    if super::full_row_p18::bank_balance(&mut bank, clients).await? != 0 {
        return Err("fresh Forge fixture has unexpected bank gold".into());
    }
    let mut expected_inventory = baseline.clone();
    let mut quotes = Vec::new();
    let mut applied = Vec::new();
    let mut parity_failure = None;
    for (ordinal, operation, expected_balance) in [
        (10, Operation::DustToSlivers, (40, 100)),
        (11, Operation::DustLimit, (15, GLOBAL_FIRST_DUST_CAP)),
    ] {
        client.service_liveness(Duration::from_millis(490)).await?;
        let before = client
            .inventory()
            .cloned()
            .ok_or("funded inventory missing")?;
        if before != expected_inventory
            || super::full_row_p18::bank_balance(&mut bank, clients).await? != 0
        {
            return Err("Forge inventory changed before the next submitted action".into());
        }
        let attempt = occurrence(session, ordinal);
        let expected_before = if operation == Operation::DustToSlivers {
            (100, 100)
        } else {
            (40, 100)
        };
        if dust(&mut client).await? != expected_before
            || confirmation_panel
                .confirm_resource(confirmation_panel.resource_dialog.serial, attempt)
                .is_some()
        {
            return Err(
                "Forge resource pre-state invalid or confirmation available before preview".into(),
            );
        }
        let quote = capture_preview(
            &mut client,
            &mut confirmation_panel,
            wire::Selection::Resource(operation),
        )
        .await?;
        let (_, preview_history) =
            forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
        let expected_history = if operation == Operation::DustToSlivers {
            serde_json::json!([])
        } else {
            serde_json::json!([{"ordinal":1,"receipt":applied[0]}])
        };
        if client.inventory() != Some(&before)
            || dust(&mut client).await? != expected_before
            || super::full_row_p18::bank_balance(&mut bank, clients).await? != 0
            || preview_history != expected_history
        {
            return Err(format!(
                "Forge {operation:?} preview changed live state before confirmation"
            )
            .into());
        }
        quotes.push(quote);
        client.service_liveness(Duration::from_millis(490)).await?;
        let command = confirm_resource(&mut confirmation_panel, attempt)?;
        let (frame, body) = submit_forge(&mut client, operation, command).await?;
        use oteryn_dev_client::client_runtime::server_mechanic::{
            CommandStatus as Status, ServerMechanicResponse,
        };
        forge_ui::apply_result(
            &mut confirmation_panel,
            &ServerMechanicResponse {
                command_id: frame.0,
                server_sequence: frame.1,
                status: Status::Accepted,
                payload: frame.3.clone(),
            },
        );
        let actual_balance = dust(&mut client).await?;
        if super::full_e2ef7::units(&client, sliver) != 3 {
            return Err(format!("funded {operation:?} did not leave exactly three slivers").into());
        }
        let after = client
            .inventory()
            .cloned()
            .ok_or("funded inventory missing")?;
        check_funded_inventory(&expected_inventory, after.clone(), operation, sliver)?;
        if super::full_row_p18::bank_balance(&mut bank, clients).await? != 0 {
            return Err("Forge resource conversion changed bank gold".into());
        }
        expected_inventory = after;
        if let Err(error) = check_funded(&body, operation) {
            if operation != Operation::DustLimit {
                return Err(error);
            }
            parity_failure = Some(error);
        } else if actual_balance != expected_balance {
            return Err(format!(
                "funded {operation:?} expected {expected_balance:?}, observed {actual_balance:?}"
            )
            .into());
        }
        applied.push(body);
        frames.push(frame);
    }
    if let Some(error) = parity_failure {
        if client.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("Forge logout refused after Global parity mismatch".into());
        }
        drop(client);
        let mut fresh = connect_character(clients, 0x45, 0).await?;
        let observed = dust(&mut fresh).await?;
        let bank_gold = super::full_row_p18::bank_balance(&mut bank, clients).await?;
        let same = fresh
            .inventory()
            .cloned()
            .map(super::full_row_p18::durable_inventory)
            == Some(super::full_row_p18::durable_inventory(expected_inventory));
        let (_, history) = forge(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
        let history_ok = history
            == serde_json::json!([
                {"ordinal":2,"receipt":applied[1]}, {"ordinal":1,"receipt":applied[0]}
            ]);
        if fresh.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("Forge relog logout refused after Global parity mismatch".into());
        }
        return Err(format!(
            "{error}; relog dust={observed:?} inventory_same={same} history_same={history_ok} bank_gold={bank_gold}"
        )
        .into());
    }
    let balance = (15, GLOBAL_FIRST_DUST_CAP);
    let inventory = client
        .inventory()
        .cloned()
        .ok_or("funded inventory missing")?;
    if inventory != expected_inventory {
        return Err("Forge inventory changed before the refusal checks".into());
    }
    if super::full_row_p18::dagger_tiers(&mut client, dagger.get()).await? != [0, 0] {
        return Err("Forge refusal pre-state must contain two tier-zero daggers".into());
    }
    let selection = wire::Selection::Pair {
        operation: Operation::Fusion,
        first: pair.0,
        second: pair.1,
        boost: false,
        protect: false,
    };
    let panel = &mut confirmation_panel;
    if !panel.choose_operation(Operation::Fusion) {
        return Err("Forge UI did not select Fusion".into());
    }
    let browse = capture_preview(&mut client, panel, wire::Selection::Browse { after: 0 }).await?;
    let item = |handle| wire::Item {
        handle,
        definition_ref: dagger.get(),
        appearance: 3267,
        name: "dagger".into(),
        classification: 1,
        tier: 0,
        maximum_tier: 1,
        container: false,
    };
    if panel.preview.item(pair.0.get()) != Some(&item(pair.0))
        || panel.preview.item(pair.1.get()) != Some(&item(pair.1))
    {
        return Err("Forge browse displayed incorrect dagger metadata before selection".into());
    }
    if !panel.choose_item(true, pair.0.get())
        || !panel.choose_item(false, pair.1.get())
        || panel.selection() != Some(selection)
    {
        return Err("The Forge picker did not select the two live daggers".into());
    }
    let fusion_id = occurrence(session, 1);
    if panel.paid_request(fusion_id).is_some() {
        return Err("Fusion could be submitted before reviewing a quote".into());
    }
    quotes.push(capture_preview(&mut client, panel, selection).await?);
    quotes.push(browse);
    let expected_quote = wire::Quote {
        selection,
        first: item(pair.0),
        second: item(pair.1),
        resources: wire::Resources {
            carried_gold: 30_050,
            bank_gold: 0,
            bank_spendable: true,
            dust: 15,
            dust_limit: u32::try_from(GLOBAL_FIRST_DUST_CAP)?,
            cores: 0,
            slivers: 3,
        },
        cost: wire::Cost {
            gold: 25_000,
            dust: 100,
            cores: 0,
        },
        target_tier: 1,
        success_percent: 50,
        protection_percent: 0,
    };
    if panel.preview.quote.as_ref() != Some(&expected_quote) {
        return Err(format!("Fusion quote mismatch: {:?}", panel.preview.quote).into());
    }
    let fusion = panel
        .paid_request(fusion_id)
        .and_then(|request| request.encode())
        .ok_or("reviewed Fusion could not be submitted")?;
    let fusion = forge_ui::request(panel, forge_ui::command(fusion.to_vec()))
        .ok_or("reviewed Fusion request was rejected by the panel")?;
    for (ordinal, (name, operation, expected_reason)) in [
        // The valid, distinct pair is evaluated against the remaining dust.
        ("fusion", Operation::Fusion, "INSUFFICIENT_DUST"),
        (
            "dust_to_slivers",
            Operation::DustToSlivers,
            "INSUFFICIENT_DUST",
        ),
        ("dust_limit", Operation::DustLimit, "INSUFFICIENT_DUST"),
    ]
    .into_iter()
    .enumerate()
    {
        // The item-forge-v1 contract gives new attempts and history a shared 490 ms cooldown.
        client.service_liveness(Duration::from_millis(490)).await?;
        let (bytes, body) = if operation == Operation::Fusion {
            submit_forge(&mut client, operation, fusion.clone()).await?
        } else {
            let attempt = occurrence(session, u8::try_from(ordinal + 1)?);
            let preview =
                capture_preview(&mut client, panel, wire::Selection::Resource(operation)).await?;
            let dialog = panel
                .resource_dialog
                .content
                .as_ref()
                .ok_or("resource refusal dialog missing")?;
            if forge_ui::resource_dialog::dialog_text(dialog, 0)
                != "This resource conversion is unavailable with your current resources or dust limit."
                || panel
                    .confirm_resource(panel.resource_dialog.serial, attempt)
                    .is_some()
            {
                return Err(format!("{name} incorrectly offered or rendered confirmation").into());
            }
            quotes.push(preview);
            // Keep the independent wire refusal probe after verifying the UI blocks it.
            let payload = Request {
                operation,
                occurrence: attempt,
                first_item: None,
                second_item: None,
                boost: false,
                protect: false,
                history_limit: 0,
            }
            .encode()
            .ok_or("Forge refusal request was not canonical")?;
            let command = forge_ui::request(panel, forge_ui::command(payload.to_vec()))
                .ok_or("Forge refusal probe did not start")?;
            submit_forge(&mut client, operation, command).await?
        };
        use oteryn_dev_client::client_runtime::server_mechanic::{
            CommandStatus as Status, ServerMechanicResponse,
        };
        forge_ui::apply_result(
            panel,
            &ServerMechanicResponse {
                command_id: bytes.0,
                server_sequence: bytes.1,
                status: Status::Accepted,
                payload: bytes.3.clone(),
            },
        );
        check_refusal(&body, expected_reason).map_err(|error| format!("{name}: {error}"))?;
        if dust(&mut client).await? != balance
            || client.inventory() != Some(&inventory)
            || super::full_row_p18::bank_balance(&mut bank, clients).await? != 0
        {
            return Err(format!("{name} refusal changed inventory or Forge balance").into());
        }
        if super::full_row_p18::dagger_tiers(&mut client, dagger.get()).await? != [0, 0] {
            return Err(format!("{name} refusal changed dagger tiers").into());
        }
        frames.push(bytes);
    }
    client.service_liveness(Duration::from_millis(490)).await?;
    let (populated_history, final_history) =
        forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let expected_history = serde_json::json!([
        {"ordinal":2,"receipt":applied[1]},
        {"ordinal":1,"receipt":applied[0]}
    ]);
    if final_history != expected_history {
        return Err("Forge history does not contain exactly the funded outcomes".into());
    }
    frames.push(populated_history);
    let (captured_balance, current_dust) = dust_capture(&mut client).await?;
    if captured_balance != balance {
        return Err("Forge current dust changed before logout".into());
    }
    frames.push(current_dust);
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge capture logout refused".into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x45, 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh
            .inventory()
            .cloned()
            .map(super::full_row_p18::durable_inventory)
            != Some(super::full_row_p18::durable_inventory(inventory))
        || dust(&mut fresh).await? != balance
        || super::full_row_p18::bank_balance(&mut bank, clients).await? != 0
    {
        return Err("Forge funded state changed after relog".into());
    }
    if super::full_row_p18::dagger_tiers(&mut fresh, dagger.get()).await? != [0, 0] {
        return Err("Forge refusal changed persisted dagger tiers".into());
    }
    fresh.service_liveness(Duration::from_millis(490)).await?;
    let (_, persisted_history) =
        forge(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    if persisted_history != expected_history {
        return Err("Forge funded history changed after relog".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge relog session logout refused".into());
    }
    frames.extend(quotes);
    let digest = write_capture(root, *session.as_bytes(), &frames)?;
    Ok(format!(
        "live-capture session={} sha256={digest}",
        uuid_text(session.as_bytes())
    ))
}

// Include the validated Stone travel and Forge walk as well as the capture exchanges.
const CLIENT_JOURNEY_BUDGET: Duration = Duration::from_secs(360);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    evidence(&format!(
        "P18_CLIENT_BUDGET deadline_s={}",
        CLIENT_JOURNEY_BUDGET.as_secs()
    ));
    match verdict::indexed_timeout(CLIENT_JOURNEY_BUDGET, verdict::observe(journey(clients))).await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_capture scenario=P18.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P18.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P18.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::item_view::{CharacterInventory, ItemEntry};
    use std::num::NonZeroU32;

    #[tokio::test(start_paused = true)]
    async fn p18_client_budget_allows_the_live_walk_and_still_expires() {
        let live_walk = async {
            tokio::time::sleep(Duration::from_secs(121)).await;
            TestResult::<()>::Ok(())
        };
        assert!(
            verdict::indexed_timeout(CLIENT_JOURNEY_BUDGET, live_walk)
                .await
                .is_ok(),
            "the old 120-second deadline interrupted a progressing Forge walk"
        );
        assert!(
            verdict::indexed_timeout(
                CLIENT_JOURNEY_BUDGET,
                std::future::pending::<TestResult<()>>()
            )
            .await
            .is_err()
        );
    }

    #[test]
    fn funded_cap_requires_global_first_increase() {
        let mut receipt = serde_json::json!({
            "operation":"dust_limit","succeeded":true,"result":{"dust":15,"dust_limit":GLOBAL_FIRST_DUST_CAP}
        });
        assert!(check_funded(&receipt, Operation::DustLimit).is_ok());
        receipt["result"]["dust_limit"] = 101.into();
        assert!(check_funded(&receipt, Operation::DustLimit).is_err());
    }

    #[test]
    fn funded_inventory_rejects_collateral_additions_and_deletions() -> TestResult {
        let dagger = ItemEntry {
            appearance_id: 0,
            handle: NonZeroU64::MIN,
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        };
        let sliver = ItemEntry {
            handle: NonZeroU64::new(2).unwrap(),
            item_definition_ref: NonZeroU32::new(2).unwrap(),
            count: NonZeroU32::new(3).unwrap(),
            ..dagger
        };
        let unrelated = ItemEntry {
            handle: NonZeroU64::new(3).unwrap(),
            ..dagger
        };
        let before = CharacterInventory {
            entries: vec![dagger],
            ..Default::default()
        };
        for operation in [Operation::DustToSlivers, Operation::DustLimit] {
            let mut expected = before.clone();
            if operation == Operation::DustToSlivers {
                expected.entries.push(sliver);
            }
            let definition = sliver.item_definition_ref;
            check_funded_inventory(&before, expected.clone(), operation, definition)?;
            let mut added = expected.clone();
            added.entries.push(unrelated);
            assert!(check_funded_inventory(&before, added, operation, definition).is_err());
            expected.entries.remove(0);
            assert!(check_funded_inventory(&before, expected, operation, definition).is_err());
        }
        Ok(())
    }

    #[test]
    fn refusal_rejects_value_bonus_and_missing_result() -> TestResult {
        let valid = serde_json::json!({
            "succeeded": false, "applied": false, "reason": "INSUFFICIENT_DUST",
            "bonus": 0, "result": null,
        });
        check_refusal(&valid, "INSUFFICIENT_DUST")?;
        for (field, value) in [
            ("result", serde_json::json!({ "dust": 0 })),
            ("bonus", serde_json::json!(1)),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(check_refusal(&invalid, "INSUFFICIENT_DUST").is_err());
        }
        let mut missing = valid;
        missing
            .as_object_mut()
            .ok_or("refusal object")?
            .remove("result");
        assert!(check_refusal(&missing, "INSUFFICIENT_DUST").is_err());
        Ok(())
    }
}
