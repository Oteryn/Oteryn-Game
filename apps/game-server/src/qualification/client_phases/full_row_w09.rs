//! W09 exercises the real USE owner with two seeded Kooldown Aids and four Drome Cubes.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_gameplay_items::concoctions::Concoction;
use oteryn_gameplay_items::drome_cube::DROME_CUBE_KEY;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::presentation::{self, ActorCooldown, ActorCooldownKind};
use oteryn_protocol_oteryn::{tutorial_notices::TutorialNotice, world_object as wire};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

const RESET_NOTICE: &str = "Your spells are no longer on cooldown.";
const GIFT_INPUT_MICROS: i64 = 3_600_000_000;

async fn platform_premium_evidence(
    scope: &SeamClients<'_>,
    client: &mut DevClientSession,
    previous: Option<(&str, i64)>,
) -> TestResult<(String, i64)> {
    use sqlx::Connection;

    let mut connection = sqlx::PgConnection::connect(scope.url).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let current = sqlx::query_as::<_, (String, i64)>(
            "SELECT e.entitlement_id, e.authority_revision::bigint
             FROM game_premium_account_fence f
             JOIN game_premium_evidence e USING (account_id, authority_revision)
             WHERE f.account_id=$1::text::uuid AND f.conflict_authority_revision IS NULL
               AND e.entitlement_state=1 AND e.entitlement_id IS NOT NULL
               AND e.effective_from_us <= extract(epoch FROM statement_timestamp()) * 1000000
               AND LEAST(e.effective_until_us, e.authority_valid_until_us, e.refresh_after_us)
                   > extract(epoch FROM statement_timestamp()) * 1000000",
        )
        .bind(scope.accounts[0].as_str())
        .fetch_optional(&mut connection)
        .await?;
        if let Some((entitlement, revision)) = current {
            if let Some((expected, _)) = previous
                && entitlement != expected
            {
                return Err("W09 fresh admission received a different Premium entitlement".into());
            }
            if previous.is_none_or(|(_, old_revision)| revision > old_revision) {
                evidence(&format!("W09_CHECK platform-premium revision={revision}"));
                return Ok((entitlement, revision));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "W09 Platform Premium fixture snapshot was not accepted before the cast".into(),
            );
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

async fn seed_gift_input(scope: &SeamClients<'_>) -> TestResult {
    use sha2::{Digest, Sha256};
    use sqlx::Connection;

    let character = &scope.characters[0];
    let binding = Sha256::new()
        .chain_update(b"w09-gift-of-life-fixture-input-v1")
        .chain_update(character)
        .finalize();
    let mut occurrence: [u8; 16] = binding[..16].try_into()?;
    occurrence[6] = (occurrence[6] & 0x0f) | 0x70;
    occurrence[8] = (occurrence[8] & 0x3f) | 0x80;
    let mut connection = sqlx::PgConnection::connect(scope.url).await?;
    let mut transaction = connection.begin().await?;
    // This dedicated input must be installed before admission loads the live timer.
    sqlx::query(
        "INSERT INTO game_character_wheel_gift_receipts
         (character_id, occurrence, request_binding, before_revision, after_revision, remaining_micros)
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, 0, 1, $4)",
    )
    .bind(character.as_slice())
    .bind(occurrence.as_slice())
    .bind(binding.as_slice())
    .bind(GIFT_INPUT_MICROS)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_wheel_gift_state (character_id, revision, remaining_micros, occurrence)
         VALUES (encode($1,'hex')::uuid, 1, $2, encode($3,'hex')::uuid)",
    )
    .bind(character.as_slice())
    .bind(GIFT_INPUT_MICROS)
    .bind(occurrence.as_slice())
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    evidence("FIXTURE row=W09.full input=gift-of-life-timer");
    Ok(())
}

// Only the dedicated, logged-out fixture character is placed. The saved
// coordinate frame, world, vitals and lease generation remain intact.
async fn place_fixture(scope: &SeamClients<'_>, from: TilePos, to: TilePos) -> TestResult {
    use sqlx::Connection as _;
    wait_character_released(scope, 0).await?;
    let mut db = sqlx::PgConnection::connect(scope.url).await?;
    let updated = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$1, tile_y=$2, tile_floor=$3, write_sequence=write_sequence+1
         WHERE character_id=encode($4,'hex')::uuid AND world_id=encode($5,'hex')::uuid
           AND tile_x=$6 AND tile_y=$7 AND tile_floor=$8",
    )
    .bind(i32::from(to.x)).bind(i32::from(to.y)).bind(-i16::from(to.floor))
    .bind(scope.characters[0].as_slice()).bind(scope.world.as_slice())
    .bind(i32::from(from.x)).bind(i32::from(from.y)).bind(-i16::from(from.floor))
    .execute(&mut db).await?;
    if updated.rows_affected() != 1 {
        return Err(format!("W09 fixture placement missing saved tile {from:?}").into());
    }
    evidence(&format!("FIXTURE row=W09.full input=placement from={from:?} to={to:?}"));
    Ok(())
}

async fn carried_gold(client: &mut DevClientSession) -> TestResult<u64> {
    Ok(oteryn_dev_client::client_runtime::npc_shop::balances(client.session_mut())
        .await?.ok_or("W09 promotion balance absent")?.carried_gold)
}

async fn gift_remaining_ms(client: &mut DevClientSession, capture_label: Option<&str>) -> TestResult<u64> {
    use oteryn_protocol_oteryn::wheel_presentation as wheel;

    let (reply, capture) = oteryn_dev_client::capture_qualification_exchange(client
        .session_mut()
        .gated_command(
            wheel::CAPABILITY_WHEEL_PRESENTATION_V1,
            wheel::COMMAND_TYPE_WHEEL_GIFT_OF_LIFE_STATUS,
            &[],
        )
    ).await;
    let reply = reply?;
    if reply.status != CommandStatus::Accepted
        || reply.payload.len() != wheel::GIFT_OF_LIFE_STATUS_BYTES
    {
        return Err(format!("W09 Gift of Life status refused or malformed: {reply:?}").into());
    }
    if reply.payload[0] != 1 {
        return Err(format!("W09 expected active Gift of Life stage 1, got {}", reply.payload[0]).into());
    }
    if let (Some(label), Some(dir)) = (capture_label, std::env::var_os("E2E_ALL_CAPTURE_DIR")) {
        let dir = std::path::PathBuf::from(dir);
        std::fs::write(dir.join(format!("w09-gift-{label}-server.bin")), capture.inbound)?;
        std::fs::write(dir.join(format!("w09-gift-{label}-client.bin")), capture.outbound)?;
        std::fs::write(dir.join(format!("w09-gift-{label}-session.bin")), client.join_snapshot().game_session_id.as_bytes())?;
    }
    Ok(u64::from_le_bytes(reply.payload[1..].try_into()?))
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<NonZeroU32, u64>> {
    let mut quantities = BTreeMap::new();
    let inventory = client.inventory().ok_or("W09 inventory view missing")?;
    for entry in &inventory.entries {
        *quantities.entry(entry.item_definition_ref).or_default() += u64::from(entry.count.get());
    }
    Ok(quantities)
}

async fn use_item(
    client: &mut DevClientSession,
    reference: NonZeroU32,
    expected: (CommandStatus, wire::UseDisposition),
) -> TestResult<u64> {
    let payload = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("W09 USE encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await
        .map_err(|error| {
            verdict::context(
                format!("W09 USE reference={reference} expected={expected:?} observed={error}"),
                error,
            )
        })?;
    let (body, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("W09 USE refusal detail: {error:?}"))?;
    let disposition =
        wire::decode_use_result(body).map_err(|error| format!("W09 USE decode: {error:?}"))?;
    if (reply.status, disposition) != expected {
        return Err(format!(
            "W09 USE reference={reference} expected={expected:?} observed={:?}/{disposition:?}",
            reply.status
        )
        .into());
    }
    Ok(reply.server_sequence)
}

fn cube_reward(
    mut expected: BTreeMap<NonZeroU32, u64>,
    actual: &BTreeMap<NonZeroU32, u64>,
    pool: &[NonZeroU32],
) -> Option<NonZeroU32> {
    let reward = pool.iter().copied().find(|reference| {
        actual.get(reference).copied().unwrap_or_default()
            == expected.get(reference).copied().unwrap_or_default() + 1
    })?;
    *expected.entry(reward).or_default() += 1;
    (expected == *actual).then_some(reward)
}

async fn expected_cube_draws(
    scope: &SeamClients<'_>,
    pool: &[NonZeroU32],
) -> TestResult<Vec<NonZeroU32>> {
    use sha2::{Digest, Sha256};
    use sqlx::Connection;

    // Read only the identity of the fixture input. All outcome checks use the live inventory.
    let mut connection = sqlx::PgConnection::connect(scope.url).await?;
    let stacks: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT uuid_send(i.item_instance_id), i.quantity::bigint
         FROM game_item_container_entries e
         JOIN game_item_container_slots b USING (character_id, world_id)
         JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id AND i.world_id=e.world_id
         WHERE e.character_id=encode($1,'hex')::uuid
           AND e.parent_item_instance_id=b.item_instance_id
           AND i.definition_production_key=$2 AND i.lifecycle=1
         ORDER BY e.placement_ordinal DESC
         LIMIT 5",
    )
    .bind(scope.characters[0].as_slice())
    .bind(DROME_CUBE_KEY)
    .fetch_all(&mut connection)
    .await?;
    connection.close().await?;
    if stacks.len() != 4 || stacks.iter().any(|(_, quantity)| *quantity != 1) {
        return Err(format!(
            "W09 fixture requires four single-unit Cubes in the main backpack; observed quantities {:?}",
            stacks.iter().map(|(_, quantity)| quantity).collect::<Vec<_>>()
        ).into());
    }
    let draws = stacks
        .iter()
        .map(|(source, quantity)| {
            // Independently calculate the source-unit occurrence in hotkey display order.
            let mut occurrence = b"oteryn.drome-cube.reward.v1".to_vec();
            occurrence.extend_from_slice(source);
            occurrence.extend_from_slice(&u64::try_from(*quantity)?.to_be_bytes());
            let sample = Sha256::digest(&occurrence)
                .into_iter()
                .find(|byte| *byte < 240)
                .ok_or("W09 Cube fixture draw exhausted entropy")?;
            Ok(pool[usize::from(sample % 20)])
        })
        .collect::<TestResult<Vec<_>>>()?;
    if draws.windows(2).all(|pair| pair[0] == pair[1]) {
        return Err("W09 Cube fixture draws do not distinguish a fixed reward".into());
    }
    Ok(draws)
}

async fn wait_inventory(
    client: &mut DevClientSession,
    expected: &BTreeMap<NonZeroU32, u64>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while quantities(client)? != *expected {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "W09 inventory mismatch: expected {expected:?}, observed {:?}",
                quantities(client)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

async fn cast_paid(client: &mut DevClientSession, spell: NonZeroU32, mana_cost: u32) -> TestResult {
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("W09 caster vitals missing")?;
    let expected_mana = before
        .mana
        .checked_sub(mana_cost)
        .ok_or("W09 insufficient mana for cooldown probe")?;
    let cast = client.cast_spell(spell, SpellTarget::None, false).await?;
    if cast.status != CommandStatus::Accepted
        || cast.disposition != SpellCastDisposition::Cast
        || cast
            .actor_vitals_delta
            .as_ref()
            .map(|delta| delta.value.mana)
            != Some(expected_mana)
    {
        return Err(format!(
            "W09 spell {spell} did not spend {mana_cost} mana: before={before:?} cast={cast:?}"
        )
        .into());
    }
    Ok(())
}

async fn assert_cast_cooldowns(
    client: &mut DevClientSession,
    spell: NonZeroU32,
    cast_started: tokio::time::Instant,
    spell_ms: u64,
    stance: bool,
) -> TestResult<[ActorCooldown; 2]> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ActorCooldowns(frame) = event {
                let now = frame.value.server_now_ms;
                // Global: https://www.tibiawiki.com.br/wiki/Conjure_Wand_of_Darkness
                // The frame samples server time after commit. Both cooldowns share the cast time.
                let entries = frame.value.entries.as_slice();
                let [spell_entry, group_entry, additional @ ..] = entries else {
                    return Err(format!(
                        "W09 cooldown entry count diverged sequence={} frame={:?}",
                        frame.server_sequence, frame.value
                    )
                    .into());
                };
                let elapsed_ms =
                    u64::try_from(cast_started.elapsed().as_millis())?.saturating_add(1);
                let group_remaining = group_entry.expires_at_ms.checked_sub(now);
                let expected_stance = ActorCooldown {
                    kind: ActorCooldownKind::Group,
                    id: 9,
                    expires_at_ms: spell_entry.expires_at_ms,
                };
                let expected_additional = if stance {
                    std::slice::from_ref(&expected_stance)
                } else {
                    &[]
                };
                if (spell_entry.kind, spell_entry.id) != (ActorCooldownKind::Spell, spell.get())
                    || (group_entry.kind, group_entry.id) != (ActorCooldownKind::Group, 3)
                    || additional != expected_additional
                    || spell_entry
                        .expires_at_ms
                        .checked_sub(group_entry.expires_at_ms)
                        != spell_ms.checked_sub(2_000)
                    || !group_remaining.is_some_and(|ms| {
                        ms > 0 && (2_000_u64.saturating_sub(elapsed_ms)..=2_000).contains(&ms)
                    })
                {
                    return Err(format!(
                        "W09 cooldown update diverged sequence={} frame={:?} cast_elapsed_ms={elapsed_ms}",
                        frame.server_sequence, frame.value
                    ).into());
                }
                evidence(&format!(
                    "W09_CHECK cooldown-update sequence={} spell_ms={spell_ms} support_ms=2000",
                    frame.server_sequence
                ));
                return Ok([*spell_entry, *group_entry]);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "W09 cooldown update missing through sequence={}",
                client.last_server_sequence()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn reconnect_cooldowns(
    client: DevClientSession,
    scope: &SeamClients<'_>,
    expected: [ActorCooldown; 2],
) -> TestResult<DevClientSession> {
    let session = client.join_snapshot().game_session_id;
    let sequence = client.last_server_sequence();
    let next_command = client.next_command_id();
    let generation = platform_generation(scope.descriptor, &scope.accounts[0]).await?;
    let (nonce, _) = fresh_grant_material(0xd2)?;
    let previous = client.disconnect();
    // Silent closes become reconnectable after the server liveness window.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
    while !matches!(
        session_loss_row(scope.url, *session.as_bytes()).await?,
        Some((1, _, _))
    ) {
        if tokio::time::Instant::now() >= deadline {
            return Err("W09 transport never became reconnectable".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let credential = sign_recovery(
        scope.recovery,
        &scope.accounts[0],
        &scope.characters[0],
        &scope.world,
        generation,
        now_seconds()?,
        nonce,
    );
    let mut resumed = oteryn_dev_client::resume_session(
        oteryn_dev_client::RecoveryTransport {
            address: scope.address,
            server_name: "localhost",
            root_certificate: scope.certificate,
            deadline: Duration::from_secs(20),
        },
        previous,
        oteryn_dev_client::Recovery {
            reconnect_material: credential.as_bytes(),
            schema_revision: 1,
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
            appearance_items: None,
        },
    )
    .await?;
    let resumed_sequence = resumed.last_server_sequence();
    let resumed_next_command = resumed.next_command_id();
    let resumed_session = resumed.join_snapshot().game_session_id;
    let continuity = format!(
        "original_session={session:?} resumed_session={resumed_session:?} original_sequence={sequence} resumed_sequence={resumed_sequence} original_next_command={next_command} resumed_next_command={resumed_next_command}"
    );
    let cooldowns = resumed
        .session_mut()
        .gated_snapshot(presentation::STATE_DOMAIN_ACTOR_COOLDOWNS)
        .ok_or("W09 reconnect omitted ACTOR_COOLDOWNS")
        .and_then(|snapshot| {
            presentation::decode_actor_cooldowns(&snapshot.payload)
                .map_err(|_| "W09 reconnect sent malformed ACTOR_COOLDOWNS")
        })
        .map_err(|error| format!("{error} {continuity}"))?;
    let active: Vec<_> = expected
        .into_iter()
        .filter(|entry| entry.expires_at_ms > cooldowns.server_now_ms)
        .collect();
    // FND-04B allows a resume snapshot at the retained sequence when no new frame was sent.
    if resumed_session != session
        || resumed_sequence < sequence
        || resumed_next_command != next_command
        || !active
            .iter()
            .any(|entry| entry.kind == ActorCooldownKind::Spell)
        || cooldowns.entries != active
    {
        return Err(format!(
            "W09 reconnect cooldown replay diverged {continuity} frame={cooldowns:?} expected={active:?}"
        ).into());
    }
    evidence(&format!(
        "W09_CHECK cooldown-reconnect {continuity} spell-deadline=preserved"
    ));
    Ok(resumed)
}

async fn assert_cooling_down(
    client: &mut DevClientSession,
    spell: NonZeroU32,
    expected: &BTreeMap<NonZeroU32, u64>,
) -> TestResult {
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("W09 caster vitals missing")?;
    let command_id = client.next_command_id();
    let sequence = client.last_server_sequence();
    let blocked = client.cast_spell(spell, SpellTarget::None, false).await?;
    if blocked.status != CommandStatus::Accepted
        || blocked.disposition != SpellCastDisposition::CoolingDown
        || blocked.command_id != command_id
        || blocked.result_server_sequence <= sequence
        || command_id.checked_add(1) != Some(client.next_command_id())
        || blocked.actor_vitals_delta.is_some()
        || client.actor_vitals().copied() != Some(before)
        || &quantities(client)? != expected
    {
        return Err(format!(
            "W09 Conjure cooldown refusal failed after command={command_id} sequence={sequence}: {blocked:?}"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let cube_id = character_prep::item_id(DROME_CUBE_KEY)?;
    let aid_id = Concoction::KooldownAid.item_id();
    let index = provisioning::item_index()?;
    let cube = provisioning::item_reference(&index, cube_id)?;
    let aid = provisioning::item_reference(&index, aid_id)?;
    // Global pool: https://www.tibiawiki.com.br/wiki/Drome_Cube.
    // Item IDs for those twenty names: Crystal 201189f8, data/scripts/actions/items/drome_cube.lua.
    let pool = (36723..=36742)
        .map(|id| provisioning::item_reference(&index, id))
        .collect::<TestResult<Vec<_>>>()?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W09_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    seed_gift_input(&scope).await?;
    let mut client = connect_character(&scope, 0xd2, 0).await?;
    let mut expected = quantities(&client)?;
    if expected.contains_key(&cube) || expected.contains_key(&aid) {
        return Err("W09 requires a fresh dedicated character".into());
    }
    let preparation_session = client.join_snapshot().game_session_id;
    character_prep::declare_seeded(
        "W09.full",
        "level-300 Premium Master Sorcerer, Gift of Life timer input, four Drome Cubes and two Kooldown Aids",
        "ordinary leveling exceeds the row budget and the inputs have no authored acquisition route",
    );
    request_fixture_control(
        uuid_text(preparation_session.as_bytes()),
        "--profile",
        "w09-aid-audit".into(),
    )
    .await?;
    use_item(
        &mut client,
        aid,
        (CommandStatus::Accepted, wire::UseDisposition::NothingToUse),
    )
    .await?;
    if quantities(&client)? != expected {
        return Err("W09 missing Aid refusal changed inventory".into());
    }
    character_prep::grant_item(&client, cube_id, 4).await?;
    character_prep::grant_item(&client, aid_id, 2).await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    logout(&mut client).await?;
    let mut client = connect_character(&scope, 0xd2, 0).await?;
    if client.join_snapshot().game_session_id == preparation_session {
        return Err("W09 preparation reused its original session".into());
    }
    evidence("FIXTURE row=W09.full input=platform-premium");
    let (initial_entitlement, initial_revision) =
        platform_premium_evidence(&scope, &mut client, None).await?;
    let level = full_e2ef7::player_level(&mut client).await?;
    let vitals = client.actor_vitals().ok_or("W09 seeded vitals missing")?;
    if (level, vitals.max_health, vitals.max_mana) != (300, 1645, 8850) {
        return Err(format!(
            "W09 expected level-300 vitals 1645/8850: level={level} vitals={vitals:?}"
        )
        .into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&scope.world)?, ChannelId::decode(&scope.channel)?)?;
    let cast_tile = tile(client.world_spatial().actor_position)?;
    let placement = metadata.npcs.npcs.iter()
        .find(|npc| npc.key == "oteryn:npc.king_tibianus")
        .and_then(|npc| npc.placements.first())
        .ok_or("W09 compiled King Tibianus placement absent")?;
    let king_tile = TilePos { x: placement.x, y: placement.y, floor: placement.floor };
    let mut nearest = cast_tile;
    let goal_distance = |at: TilePos| (
        at.floor.abs_diff(king_tile.floor),
        at.x.abs_diff(king_tile.x).max(at.y.abs_diff(king_tile.y)),
    );
    let original_route = find_route(cast_tile, |at| goal_distance(at) <= (0, 3), |at, direction| {
        if goal_distance(at) < goal_distance(nearest) {
            nearest = at;
        }
        destination(world, at, direction).filter(|next| map_journey::in_town(*next, cast_tile))
    });
    let frontier = [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
        .map(|direction| (direction, destination(world, nearest, direction)));
    evidence(&format!("W09_ROUTE start={cast_tile:?} king={king_tile:?} nearest={nearest:?} frontier={frontier:?} search={:?}", original_route.as_ref().map(Vec::len)));
    let castle_tile = TilePos { y: king_tile.y + 1, ..king_tile };
    if !world.base().tile(castle_tile.x, castle_tile.y, castle_tile.floor)
        .is_some_and(|cell| cell.walkable()) || !world.enterable(castle_tile)
    {
        return Err(format!("W09 designated castle placement is blocked: {castle_tile:?}").into());
    }
    logout(&mut client).await?;
    place_fixture(&scope, cast_tile, castle_tile).await?;
    let mut client = connect_character(&scope, 0xd2, 0).await?;
    if tile(client.world_spatial().actor_position)? != castle_tile {
        return Err("W09 admission did not honor castle placement".into());
    }
    let king = full_row_p20::approach_npc(&mut client, world, &metadata.npcs, "oteryn:npc.king_tibianus").await?;
    let before = carried_gold(&mut client).await?;
    if full_row_p20::talk(&mut client, king, "hi").await?.is_empty()
        || full_row_p20::talk(&mut client, king, "promotion").await? != ["Do you want to be promoted for 20000 gold coins?"]
        || full_row_p20::talk(&mut client, king, "yes").await? != ["You are now promoted."]
    {
        return Err("W09 Master Sorcerer promotion was refused".into());
    }
    let after = carried_gold(&mut client).await?;
    if before.checked_sub(after) != Some(20_000) {
        return Err(format!("W09 promotion fee mismatch: {before} -> {after}").into());
    }
    let departure = tile(client.world_spatial().actor_position)?;
    logout(&mut client).await?;
    place_fixture(&scope, departure, cast_tile).await?;
    let mut client = connect_character(&scope, 0xd2, 0).await?;
    if tile(client.world_spatial().actor_position)? != cast_tile {
        return Err("W09 admission did not restore the original cast tile".into());
    }
    expected.extend([(cube, 4), (aid, 2)]);
    if quantities(&client)? != expected {
        return Err("W09 fresh admission did not serve exactly the seeded items".into());
    }

    // Global Gift of Life requires a level-300 character and a 250-point green allocation.
    // https://tibia.fandom.com/wiki/Gift_of_Life
    use oteryn_protocol_oteryn::wheel;
    let mut points = [0; 36];
    points[14] = 50;
    points[8] = 75;
    points[13] = 75;
    points[7] = 50;
    let allocation = wheel::WheelAllocationRequest {
        action: 1,
        occurrence: [0x09, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1],
        expected_revision: 0,
        points,
        vessels: None,
    };
    let payload = wheel::encode_wheel_request(&allocation)
        .map_err(|error| format!("W09 Wheel fixture encoding: {error:?}"))?;
    let reply = client.session_mut().gated_command(
        wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        &payload,
    ).await?;
    let saved = wheel::decode_wheel_reply(&reply.payload)
        .map_err(|error| format!("W09 Wheel fixture reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || saved.refused || saved.revision != 1 || saved.points != points {
        return Err(format!("W09 Gift of Life allocation failed: {saved:?}").into());
    }
    let gift_input = u64::try_from(GIFT_INPUT_MICROS)? / 1_000;
    if gift_remaining_ms(&mut client, None).await? != gift_input {
        return Err("W09 active Gift timer changed outside battle".into());
    }
    evidence("FIXTURE row=W09.full input=gift-of-life-allocation-stage-1");

    let expected_draws = expected_cube_draws(&scope, &pool).await?;
    let committed = (CommandStatus::Accepted, wire::UseDisposition::Committed);
    let missing = (CommandStatus::Accepted, wire::UseDisposition::NothingToUse);
    let book = native_spell_book()?;
    let spell = crate::qualification::client_spell_journey::spell_index(book, "exevo gran mort")?;
    let natural_cooldown = Duration::from_micros(
        book.indexed(spell)
            .ok_or("W09 Conjure Wand of Darkness missing from native book")?
            .cooldown_micros,
    );
    // Global: https://www.tibiawiki.com.br/wiki/Conjure_Wand_of_Darkness
    if natural_cooldown != Duration::from_secs(1800) {
        return Err(format!("W09 unexpected donor spell cooldown: {natural_cooldown:?}").into());
    }
    let wand = provisioning::item_reference(&index, 25760)?;
    if expected.contains_key(&wand)
        || !client
            .inventory()
            .is_some_and(|inventory| inventory.main_backpack.is_some())
    {
        return Err("W09 requires a main backpack with no Wand of Darkness".into());
    }
    client.take_events();
    let first_cast = tokio::time::Instant::now();
    cast_paid(&mut client, spell, 250).await?;
    *expected.entry(wand).or_default() += 1;
    let cooldowns = assert_cast_cooldowns(&mut client, spell, first_cast, 1_800_000, false).await?;
    wait_inventory(&mut client, &expected).await?;
    let mut client = reconnect_cooldowns(client, &scope, cooldowns).await?;
    if quantities(&client)? != expected {
        return Err("W09 reconnect changed the first conjure inventory".into());
    }
    platform_premium_evidence(
        &scope,
        &mut client,
        Some((&initial_entitlement, initial_revision)),
    )
    .await?;
    assert_cooling_down(&mut client, spell, &expected).await?;
    let gift_before = gift_remaining_ms(&mut client, Some("before")).await?;
    if gift_before != gift_input {
        return Err("W09 Gift timer ticked without a battle sign before Aid".into());
    }
    // Global: https://www.tibiawiki.com.br/wiki/Master_of_Thunder.
    // Its 30-second stance group survives the live command latency.
    let stance = crate::qualification::client_spell_journey::spell_index(book, "uteta vis")?;
    client.take_events();
    let stance_cast = tokio::time::Instant::now();
    cast_paid(&mut client, stance, 400).await?;
    let [stance_spell, support] =
        assert_cast_cooldowns(&mut client, stance, stance_cast, 30_000, true).await?;
    let stance_group = ActorCooldown {
        kind: ActorCooldownKind::Group,
        id: 9,
        expires_at_ms: stance_spell.expires_at_ms,
    };
    let mut reset_prestate = [cooldowns[0], stance_spell, support, stance_group];
    reset_prestate.sort_by_key(|entry| (entry.kind, entry.id));
    client.take_events();
    let (reset, capture) = oteryn_dev_client::capture_qualification_exchange(async {
    let used_at = tokio::time::Instant::now();
    let aid_sequence = use_item(&mut client, aid, committed).await?;
    let committed_at = tokio::time::Instant::now();
    let reset_until = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut saw_reset = false;
    let mut saw_clear = false;
    while !saw_reset || !saw_clear {
        for event in client.take_events() {
            match event {
                SessionEvent::ActorCooldowns(frame) if frame.server_sequence > aid_sequence => {
                    let expected_clear = reset_prestate
                        .iter()
                        .map(|entry| ActorCooldown {
                            expires_at_ms: 0,
                            ..*entry
                        })
                        .collect::<Vec<_>>();
                    if reset_prestate
                        .iter()
                        .any(|entry| entry.expires_at_ms <= frame.value.server_now_ms)
                        || frame.value.entries != expected_clear
                    {
                        return Err(format!("W09 Aid clearance mismatch sequence={} expected={expected_clear:?} frame={:?}", frame.server_sequence, frame.value).into());
                    }
                    saw_clear = true;
                }
                SessionEvent::TutorialNotice(delta)
                    if delta.server_sequence > aid_sequence && matches!(&delta.value,
                    TutorialNotice::StatusText(text) if text == RESET_NOTICE) =>
                {
                    saw_reset = true
                }
                _ => {}
            }
        }
        if saw_reset && saw_clear {
            break;
        }
        if tokio::time::Instant::now() >= reset_until {
            return Err(format!(
                "W09 missing Aid clearance or notice after sequence={}",
                client.last_server_sequence()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
        Ok::<_, Box<dyn std::error::Error>>((used_at, committed_at))
    }).await;
    let (used_at, committed_at) = reset?;
    if let Some(dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::write(dir.join("w09-aid-server.bin"), capture.inbound)?;
        std::fs::write(dir.join("w09-aid-client.bin"), capture.outbound)?;
        std::fs::write(dir.join("w09-aid-session.bin"), client.join_snapshot().game_session_id.as_bytes())?;
    }
    expected.insert(aid, 1);
    wait_inventory(&mut client, &expected).await?;
    let gift_after = gift_remaining_ms(&mut client, Some("after")).await?;
    if gift_after != gift_before {
        return Err(format!("W09 Aid changed the paused Gift timer: {gift_before} -> {gift_after} ms").into());
    }
    evidence(&format!(
        "W09_CHECK aid-debit=one gift-before-ms={gift_before} gift-after-ms={gift_after}"
    ));

    let stance_recast = tokio::time::Instant::now();
    cast_paid(&mut client, stance, 400).await?;
    let [new_stance, _] =
        assert_cast_cooldowns(&mut client, stance, stance_recast, 30_000, true).await?;
    if stance_cast.elapsed() >= Duration::from_secs(30) {
        return Err("W09 Master of Thunder recast did not precede natural Stance expiry".into());
    }
    let support_end = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < support_end {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let second_cast = tokio::time::Instant::now();
    cast_paid(&mut client, spell, 250).await?;
    let [new_conjure, new_support] =
        assert_cast_cooldowns(&mut client, spell, second_cast, 1_800_000, false).await?;
    let mut durable_cooldowns = [
        new_conjure,
        new_stance,
        new_support,
        ActorCooldown {
            expires_at_ms: new_stance.expires_at_ms,
            ..stance_group
        },
    ];
    durable_cooldowns.sort_by_key(|entry| (entry.kind, entry.id));
    if first_cast.elapsed() >= natural_cooldown {
        return Err(
            "W09 Conjure Wand of Darkness recast did not precede natural cooldown expiry".into(),
        );
    }
    *expected.entry(wand).or_default() += 1;
    wait_inventory(&mut client, &expected).await?;
    for (offset, reward) in expected_draws.into_iter().enumerate() {
        use_item(&mut client, cube, committed).await?;
        let remaining = 3 - u64::try_from(offset)?;
        if remaining == 0 {
            expected.remove(&cube);
        } else {
            expected.insert(cube, remaining);
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let actual = quantities(&client)?;
            if cube_reward(expected.clone(), &actual, &[reward]) == Some(reward) {
                expected = actual;
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "W09 Cube draw {offset} expected one reward ref={reward} with {remaining} Cubes left"
                ).into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
        evidence(&format!(
            "W09_CHECK cube-draw={offset} reward-ref={reward} remaining={remaining}"
        ));
    }
    let settled = expected;
    let used_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    let mut fresh = connect_character(&scope, 0xd2, 0).await?;
    if fresh.join_snapshot().game_session_id == used_session {
        return Err("W09 durable verification reused the USE session".into());
    }
    if quantities(&fresh)? != settled {
        return Err("W09 fresh admission did not serve the settled inventory".into());
    }
    if gift_remaining_ms(&mut fresh, Some("relog")).await? != gift_after {
        return Err("W09 fresh admission changed the persisted Gift of Life remainder".into());
    }
    evidence(&format!("W09_CHECK gift-stage=1 paused-and-persisted-ms={gift_after}"));
    use_item(&mut fresh, cube, missing).await?;
    if quantities(&fresh)? != settled {
        return Err("W09 missing Cube refusal changed inventory".into());
    }
    fresh.take_events();
    let minimum_age = committed_at.elapsed().as_secs();
    let upper = 86_400_u64.saturating_sub(minimum_age);
    use_item(
        &mut fresh,
        aid,
        (CommandStatus::Accepted, wire::UseDisposition::Exhausted),
    )
    .await?;
    if quantities(&fresh)? != settled {
        return Err("W09 refused Kooldown Aid consumed the second unit".into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let events = fresh.take_events();
        let remaining = events.iter().find_map(|event| {
            let SessionEvent::TutorialNotice(delta) = event else {
                return None;
            };
            let TutorialNotice::StatusText(text) = &delta.value else {
                return None;
            };
            text.strip_prefix("Kooldown Aid can be used again in ")?
                .strip_suffix(" seconds.")?
                .parse::<u64>()
                .ok()
        });
        if let Some(seconds) = remaining {
            let lower = 86_400_u64.saturating_sub(used_at.elapsed().as_secs().saturating_add(1));
            if upper == 86_400 || !(lower..=upper).contains(&seconds) {
                return Err(
                    format!("W09 Aid remaining seconds {seconds} outside elapsed-time bounds {lower}..={upper}").into(),
                );
            }
            evidence(&format!(
                "W09_CHECK cooldown-aid-reuse remaining_seconds={seconds}"
            ));
            break;
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!(
                "W09 missing remaining-time notice after sequence={}",
                fresh.last_server_sequence()
            )
            .into());
        }
        fresh.service_liveness(Duration::from_millis(50)).await?;
    }
    let refused_session = fresh.join_snapshot().game_session_id;
    let (entitlement, revision) = platform_premium_evidence(&scope, &mut fresh, None).await?;
    if entitlement != initial_entitlement {
        return Err("W09 Premium entitlement changed before the final relog".into());
    }
    logout(&mut fresh).await?;
    let mut verified = connect_character(&scope, 0xd2, 0).await?;
    if verified.join_snapshot().game_session_id == refused_session
        || quantities(&verified)? != settled
    {
        return Err("W09 fresh admission did not preserve inventory through both refusals".into());
    }
    let sequence = verified.last_server_sequence();
    let cooldowns = verified
        .session_mut()
        .gated_snapshot(presentation::STATE_DOMAIN_ACTOR_COOLDOWNS)
        .ok_or("W09 full relog omitted ACTOR_COOLDOWNS")
        .and_then(|snapshot| {
            presentation::decode_actor_cooldowns(&snapshot.payload)
                .map_err(|_| "W09 full relog sent malformed ACTOR_COOLDOWNS")
        })?;
    let active: Vec<_> = durable_cooldowns
        .into_iter()
        .filter(|entry| entry.expires_at_ms > cooldowns.server_now_ms)
        .collect();
    if !active
        .iter()
        .any(|entry| entry.kind == ActorCooldownKind::Spell)
        || cooldowns.entries != active
    {
        return Err(format!(
            "W09 full relog changed the active deadlines sequence={sequence} expected={active:?} frame={cooldowns:?}"
        ).into());
    }
    platform_premium_evidence(&scope, &mut verified, Some((&entitlement, revision))).await?;
    assert_cooling_down(&mut verified, spell, &settled).await?;
    logout(&mut verified).await?;
    evidence(
        "W09_CHECK spell-reset=paid-recast durable-inventory=verified cooldown-refusal=observed",
    );
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W09 logout refused: {reply:?}").into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "reset-and-cube-transform".to_owned()),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W09.full status={status} seconds={:.3} evidence={}",
        started.elapsed().as_secs_f64(),
        detail.split_whitespace().collect::<Vec<_>>().join("-")
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_reward_requires_one_pool_unit_and_preserves_every_other_stack() -> TestResult {
        let reward = NonZeroU32::new(1).ok_or("reward ref")?;
        let unrelated = NonZeroU32::new(2).ok_or("unrelated ref")?;
        let before = BTreeMap::from([(reward, 2), (unrelated, 1)]);
        let after = BTreeMap::from([(reward, 3), (unrelated, 1)]);
        assert_eq!(cube_reward(before.clone(), &after, &[reward]), Some(reward));
        assert_eq!(cube_reward(before.clone(), &before, &[reward]), None);
        assert_eq!(cube_reward(before.clone(), &after, &[unrelated]), None);
        assert_eq!(
            cube_reward(before, &BTreeMap::from([(reward, 3)]), &[reward]),
            None
        );
        Ok(())
    }
}
