//! Capture and replay the two out-of-house refusals and the authorised stale revision.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::house_wrap as wrap;
use std::path::PathBuf;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    match result.unwrap_or_else(|_| Err("I10 client capture timed out".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=I10.client status=PASS evidence=house-wrap-unwrap-and-stale-refusals",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=I10.client status=FAIL evidence={error}"
        )),
    }
}

const HOUSE: &str = "oteryn:content.house.upper_swamp_lane_2";
const AUCTION: &str = "01934f10-7c04-7024-805b-3b1122334491";

async fn seed_auction(clients: &SeamClients<'_>) -> TestResult {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/houses/houses-00500-00994.json"
    ))?;
    let house = catalogue["houses"]
        .as_array()
        .ok_or("I10 house fixture catalogue")?
        .iter()
        .find(|house| house["identity"]["key"] == HOUSE)
        .ok_or("I10 house fixture authored property missing")?;
    let rent = house["rent_gold"]
        .as_i64()
        .ok_or("I10 house fixture authored rent missing")?;
    if house["kind"] != "private_house" || rent <= 0 {
        return Err("I10 house fixture requires an authored private property with rent".into());
    }
    let world = uuid_text(&clients.world);
    let channel = uuid_text(&clients.channel);
    let character = uuid_text(&clients.characters[0]);
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = sqlx::Connection::begin(&mut database).await?;
    let property = sqlx::query(
        "INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest)
         SELECT world_id,$3,'private_house',2,$4,server_artifact_digest
         FROM game_content_activations WHERE world_id=$1::text::uuid AND channel_id=$2::text::uuid
         ORDER BY activation_sequence DESC LIMIT 1",
    )
    .bind(&world)
    .bind(&channel)
    .bind(HOUSE)
    .bind(rent)
    .execute(&mut *tx)
    .await?;
    if property.rows_affected() != 1 {
        return Err("I10 house fixture active content missing".into());
    }
    sqlx::query("INSERT INTO game_house_ownership VALUES($1::text::uuid,$2,NULL,1,game_character_uuid_v7())")
        .bind(&world).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at,next_order)
                 VALUES($1::text::uuid,$2::text::uuid,$3,$4,clock_timestamp()-interval '1 second',2)")
        .bind(AUCTION).bind(&world).bind(HOUSE).bind(rent).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_bids VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,1,$5,1,1,$5+1)")
        .bind(AUCTION).bind(&clients.accounts[0]).bind(&world).bind(&character).bind(rent).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn prepare(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "I10.client",
        "level 20, settled Upper Swamp Lane 2 auction and construction kit",
        "isolate the stale-revision probe from house auction and item acquisition",
    );
    let mut client = connect_character(clients, 0xb0, 0).await?;
    let session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(session.as_bytes()),
        "--profile",
        "house-i10-build".into(),
    )
    .await?;
    logout(&mut client).await?;
    drop(client);
    let client = connect_character(clients, 0xb0, 0).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("I10 seeded admission reused its original session".into());
    }
    let vitals = client.actor_vitals().ok_or("I10 fixture vitals missing")?;
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        20,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("I10 fixture has no source vitals")?;
    if (vitals.max_health, vitals.max_mana) != (health, mana) {
        return Err("step=fixture-admission observed=incorrect-level-20-vitals".into());
    }
    seed_auction(clients).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "house-i10".into(),
    )
    .await?;
    character_prep::grant_item(&client, 2775, 1).await?;
    Ok(client)
}

async fn walk_segment(client: &mut DevClientSession, to: TilePos) -> TestResult {
    let target = oteryn_protocol_oteryn::world_spatial::ActorPosition {
        x: i32::from(to.x),
        y: i32::from(to.y),
        floor: -i16::from(to.floor),
    };
    if client.world_spatial().actor_position == target {
        return Ok(());
    }
    evidence(&format!(
        "I10_CHECK step=walk-segment from={:?} target={target:?}",
        client.world_spatial().actor_position
    ));
    let reply = client.session_mut().target_walk(target).await?;
    if reply.status != CommandStatus::Accepted || reply.payload != [8, 1] {
        return Err(format!("step=walk-once target={target:?} observed={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    while client.world_spatial().actor_position != target {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=walk-once target={target:?} observed={:?}",
                client.world_spatial().actor_position
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

const ENTRANCE: TilePos = TilePos {
    x: 32348,
    y: 32252,
    floor: -7,
};
const DOOR: TilePos = TilePos {
    x: 32348,
    y: 32253,
    floor: -7,
};

async fn enter_house(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    let path = route_live(client, world, |at| at == ENTRANCE)
        .await?
        .ok_or("I10 authored route to Upper Swamp Lane 2 is unavailable")?;
    let mut at = start;
    // The live target-walk command admits targets within 12 tiles.
    for segment in path.chunks(8) {
        for &direction in segment {
            at = destination(world, at, direction).ok_or("I10 invalid authored route step")?;
        }
        walk_segment(client, at).await?;
    }
    transfer_house(client, false, DOOR).await
}

fn custody_position(client: &mut DevClientSession) -> TestResult<TilePos> {
    use oteryn_dev_client::client_runtime::house_transfer as transfer;
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    if let Some(scope) = client
        .session_mut()
        .gated_snapshot(transfer::STATE_DOMAIN_HOUSE_SCOPE)
    {
        let ack = transfer::decode_house_transfer_ack(&scope.payload)
            .map_err(|error| format!("I10 invalid house custody snapshot: {error:?}"))?;
        let [x, y, floor] = ack.position;
        return tile(ActorPosition {
            x,
            y,
            floor: i16::try_from(floor)?,
        });
    }
    tile(client.world_spatial().actor_position)
}

async fn transfer_house(client: &mut DevClientSession, exit: bool, to: TilePos) -> TestResult {
    use oteryn_dev_client::client_runtime::house_transfer as transfer;
    let before = custody_position(client)?;
    let source_session = client.join_snapshot().game_session_id;
    evidence(&format!(
        "I10_CHECK step=house-transfer-request exit={exit} from={before:?} destination={to:?}"
    ));
    let request = transfer::encode_house_transfer_intent(&transfer::HouseTransferIntent {
        exit,
        house: HOUSE.into(),
    })
    .map_err(|error| format!("step=house-entry encoding={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            &request,
        )
        .await?;
    let ack = transfer::decode_house_transfer_ack(&reply.payload).map_err(|error| {
        format!(
            "step=house-entry status={:?} decode={error:?}",
            reply.status
        )
    })?;
    let expected_house = (!exit).then_some(HOUSE);
    if reply.status != CommandStatus::Accepted
        || ack.house.as_deref() != expected_house
        || before == to
        || client.join_snapshot().game_session_id == source_session
        || custody_position(client)? != to
    {
        return Err(format!("step=house-entry status={:?} ack={ack:?}", reply.status).into());
    }
    evidence(&format!(
        "I10_CHECK step=house-transfer exit={exit} destination={to:?}"
    ));
    Ok(())
}

async fn catalogue(client: &mut DevClientSession) -> TestResult<wrap::Catalogue> {
    let payload = wrap::encode_catalogue_query(0).ok_or("I10 catalogue encoding")?;
    let reply = client
        .session_mut()
        .gated_command(
            wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            wrap::COMMAND_WRAP,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=furniture-catalogue status={:?} reply={:?}",
            reply.status,
            wrap::Reply::decode(&reply.payload)
        )
        .into());
    }
    let page =
        wrap::Catalogue::decode(&reply.payload).ok_or("step=furniture-catalogue invalid-body")?;
    if page.page != 0 || page.more {
        return Err("step=furniture-catalogue expected complete page zero".into());
    }
    Ok(page)
}

fn furniture(
    catalogue: &wrap::Catalogue,
    name: &str,
    action: wrap::Action,
) -> TestResult<wrap::Request> {
    let mut matches = catalogue
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.name == name && entry.action == action);
    let (index, _) = matches.next().ok_or_else(|| {
        format!(
            "step=furniture-catalogue missing={name:?} action={action:?} entries={:?}",
            catalogue.entries
        )
    })?;
    if matches.next().is_some() {
        return Err(format!("step=furniture-catalogue ambiguous={name:?}").into());
    }
    let mut request = catalogue
        .request(index)
        .ok_or("I10 furniture catalogue entry")?;
    request.position = [32348, 32254, 7];
    Ok(request)
}

async fn exchange(
    client: &mut DevClientSession,
    request: wrap::Request,
) -> TestResult<(CommandStatus, wrap::Reply)> {
    let reply = client
        .session_mut()
        .gated_command(
            wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            wrap::COMMAND_WRAP,
            &request.encode(),
        )
        .await?;
    let decoded = wrap::Reply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step=furniture-exchange action={:?} status={:?} payload={:?}",
            request.action, reply.status, reply.payload
        )
    })?;
    Ok((reply.status, decoded))
}

async fn committed(client: &mut DevClientSession, request: wrap::Request) -> TestResult<u64> {
    let observed = exchange(client, request).await?;
    match observed {
        (CommandStatus::Accepted, wrap::Reply::Committed(revision))
            if request.expected_revision.checked_add(1) == Some(revision) =>
        {
            Ok(revision)
        }
        _ => Err(format!(
            "step=furniture-commit action={:?} observed={observed:?}",
            request.action
        )
        .into()),
    }
}

async fn capture_refusal(
    client: &mut DevClientSession,
    directory: &Path,
    key: &str,
    request: wrap::Request,
    expected: wrap::Refusal,
) -> TestResult<String> {
    use std::io::Write as _;
    let session = client.join_snapshot().game_session_id;
    let position = custody_position(client)?;
    let inventory = client
        .inventory()
        .cloned()
        .ok_or("I10 inventory unavailable before refusal")?;
    let bytes = request.encode();
    let result = client
        .session_mut()
        .gated_command(
            wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            wrap::COMMAND_WRAP,
            &bytes,
        )
        .await?;
    let reply = wrap::Reply::decode(&result.payload);
    if result.status != CommandStatus::Rejected || reply != Some(wrap::Reply::Refused(expected)) {
        return Err(format!(
            "I10 {key}: expected {expected:?}, got {:?} {reply:?}",
            result.status
        )
        .into());
    }
    if client.join_snapshot().game_session_id != session
        || custody_position(client)? != position
        || client.inventory() != Some(&inventory)
    {
        return Err(format!("I10 {key}: refusal changed session, position or inventory").into());
    }
    let metadata = format!(
        "session={}\ncommand_id={}\nserver_sequence={}\n",
        uuid_text(session.as_bytes()),
        result.command_id,
        result.server_sequence
    );
    for (suffix, contents) in [
        ("request.bin", bytes.as_slice()),
        ("exchange.txt", metadata.as_bytes()),
        ("status.bin", &(result.status as u32).to_be_bytes()),
        ("bin", result.payload.as_slice()),
    ] {
        let name = if suffix == "bin" {
            format!("i10-{key}.bin")
        } else {
            format!("i10-{key}-{suffix}")
        };
        std::fs::File::create_new(directory.join(name))?.write_all(contents)?;
    }
    Ok(result
        .payload
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("I10 fixture logout refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    use std::io::Write as _;
    let directory = PathBuf::from(required("E2E_ALL_CAPTURE_DIR")?);
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let replay = required("NODE_BOOT_I10_REPLAY_BINARY")?;
    let account = character_prep::dedicated_character(clients, "NODE_BOOT_I10_ACCOUNT").await?;
    let scope = account.clients(clients);
    let mut prepared = prepare(&scope).await?;
    logout(&mut prepared).await?;
    drop(prepared);

    let mut client = connect_character(&scope, 0xb2, 0).await?;
    let position = client.world_spatial().actor_position;
    let request = wrap::Request {
        action: wrap::Action::Wrap,
        item: [0; 16],
        expected_revision: 0,
        acl_revision: 0,
        position: [position.x, position.y, i32::from(position.floor)],
    };
    let wrap = capture_refusal(
        &mut client,
        &directory,
        "wrap",
        request,
        wrap::Refusal::NotInHouse,
    )
    .await?;
    let unwrap = capture_refusal(
        &mut client,
        &directory,
        "unwrap",
        wrap::Request {
            action: wrap::Action::Unwrap,
            ..request
        },
        wrap::Refusal::NotInHouse,
    )
    .await?;

    enter_house(&mut client, world).await?;
    let kit = furniture(
        &catalogue(&mut client).await?,
        "red cushioned chair kit",
        wrap::Action::Unwrap,
    )?;
    let revision = committed(&mut client, kit).await?;
    let before_refusal = catalogue(&mut client).await?;
    let current = furniture(&before_refusal, "red cushioned chair", wrap::Action::Wrap)?;
    if current.item != kit.item || current.expected_revision != revision {
        return Err("I10 catalogue did not expose the newly installed chair".into());
    }
    let refusal = capture_refusal(
        &mut client,
        &directory,
        "refusal",
        wrap::Request {
            // The kit revision predates the committed installation of this same item.
            expected_revision: kit.expected_revision,
            ..current
        },
        wrap::Refusal::Stale,
    )
    .await?;
    if catalogue(&mut client).await? != before_refusal {
        return Err("I10 stale refusal changed the furniture catalogue".into());
    }
    transfer_house(&mut client, true, ENTRANCE).await?;
    let prior_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&scope, 0xb3, 0).await?;
    if fresh.join_snapshot().game_session_id == prior_session {
        return Err("I10 durability check reused its original session".into());
    }
    enter_house(&mut fresh, world).await?;
    let after_relog = catalogue(&mut fresh).await?;
    if after_relog.entries != before_refusal.entries
        || after_relog.acl_revision != before_refusal.acl_revision
    {
        return Err("I10 installed furniture or ACL changed across relog".into());
    }
    logout(&mut fresh).await?;
    std::fs::File::create_new(directory.join("i10-expected.txt"))?
        .write_all(format!("wrap={wrap}\nunwrap={unwrap}\nrefusal={refusal}\n").as_bytes())?;
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(replay)
            .arg("e2e_i10_replays_wrap_unwrap_and_stale_refusals")
            .env("E2E_ALL_CAPTURE_DIR", directory)
            .output()
    })
    .await??;
    let stdout = String::from_utf8(output.stdout)?;
    if !output.status.success() || !stdout.contains("1 passed; 0 failed; 0 ignored") {
        return Err(format!(
            "I10 named replay failed: {stdout} {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    evidence(&format!(
        "I10_REPLAY {}",
        stdout
            .lines()
            .find(|line| line.contains("1 passed"))
            .unwrap_or_default()
    ));
    Ok(())
}
