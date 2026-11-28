//! Client replay capture for SCENARIO_MATRIX C13 (summons and owner end-of-life).
//!
//! An owner summons a Rat while a second character observes. The observer's session applies the
//! server's WORLD_SPATIAL_ENTITIES deltas: the one that brings the summon in, and the one that
//! takes it out again when the owner logs out. Both applied deltas are captured; the observer's
//! pre-summon entity store is captured only as the starting point to replay them from.
//! The consumer is `crates/session/src/domain_dispatch.rs::Session::apply_domain_delta`
//! (domain 1/type 2), which calls `crates/session/src/entities.rs::WorldEntities::after` and is
//! exposed by `Session::world_entities`.

use super::*;
use oteryn_dev_client::{EntityKind, EntityRef, SessionEvent, WorldSpatialEntitiesDelta};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLights, WorldSpatialEntitiesSnapshot,
    encode_world_spatial_entities_delta_with_fields,
    encode_world_spatial_entities_snapshot_with_fields,
};

const FIELD_CAPABILITIES: &[u32] = &[6, 286];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(capture(clients)))
            .await;
    match result.unwrap_or_else(|_| Err("C13.client journey timeout".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=C13.client status=FAIL evidence=original-frame-replay-required-live-assertions-passed",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C13.client status=FAIL evidence={error}"
        )),
    }
}

fn is_rat(entity: &oteryn_dev_client::WorldSpatialEntity) -> bool {
    entity.kind == EntityKind::Creature
        && entity
            .display_name
            .as_ref()
            .is_some_and(|name| name.as_str() == Ok("Rat"))
}

/// The next entity delta the observer applied that satisfies `wanted`.
async fn next_delta(
    observer: &mut oteryn_dev_client::DevClientSession,
    what: &str,
    wanted: impl Fn(&WorldSpatialEntitiesDelta) -> bool,
) -> TestResult<WorldSpatialEntitiesDelta> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        for event in observer.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && wanted(&delta.value)
            {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C13.client observer never applied {what}").into());
        }
        observer.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn logout_once(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C13.client first logout refused: {:?}", reply.result).into());
    }
    Ok(())
}

async fn walk_once(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("C13.client invalid planned step")?;
        let reply = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if reply.disposition != StepDisposition::Moved
            || reply.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!("C13.client first step {direction:?} from {before:?}: {:?}, expected {expected:?}, got {actual:?}", reply.disposition).into());
        }
    }
    Ok(())
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    const CASTER_LEVEL: u32 = 200;
    const SEED_REASON: &str = "Ordinary leveling exceeds the client capture budget";
    character_prep::declare_seeded("C13.client", "level-200 sorcerer", SEED_REASON);
    let owner_identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_F5_C13_ACCOUNT").await?;
    let owner_clients = owner_identity.clients(clients);
    let mut seed = connect_character(&owner_clients, 0x2d, 0).await?;
    let seed_session = seed.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(seed_session.as_bytes()),
        "--profile",
        "f5-c13".into(),
    )
    .await?;
    logout_once(&mut seed).await?;
    drop(seed);
    let mut owner = connect_character(&owner_clients, 0x2d, 0).await?;
    let owner_session = owner.join_snapshot().game_session_id;
    if owner_session == seed_session {
        return Err("C13.client seed logout did not end the session".into());
    }
    let level = full_e2ef7::player_level(&mut owner).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        CASTER_LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("C13.client caster has no source vitals")?;
    if level != CASTER_LEVEL
        || !owner
            .actor_vitals()
            .is_some_and(|vitals| (vitals.max_health, vitals.max_mana) == (max_health, max_mana))
    {
        return Err(format!(
            "C13.client caster preparation mismatch: level={level} vitals={:?}",
            owner.actor_vitals()
        )
        .into());
    }
    evidence(&format!(
        "C13_CHECK caster-preparation PASS level={level} max_health={max_health} max_mana={max_mana}"
    ));
    let mut observer = connect_character(clients, 0x2e, 0).await?;
    if !FIELD_CAPABILITIES
        .iter()
        .all(|capability| observer.selected_capabilities().contains(capability))
    {
        return Err("C13.client observer lacks entity capabilities 6 and 286".into());
    }
    let from = tile(owner.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|entry| !entry.protection_zone())
    })
    .ok_or("C13.client has no route outside protection zone")?;
    walk_once(&mut owner, world, &path).await?;
    let spot = tile(owner.world_spatial().actor_position)?;
    let near = |at: TilePos| {
        at != spot
            && at.floor == spot.floor
            && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 2
    };
    let approach = route(world, tile(observer.world_spatial().actor_position)?, near)
        .ok_or("C13.client observer has no route next to the owner")?;
    walk_once(&mut observer, world, &approach).await?;
    owner.take_events();
    observer
        .service_liveness(Duration::from_millis(100))
        .await?;
    observer.take_events();

    let owner_wire_identity = *owner
        .world_entities()
        .ok_or("C13.client caster store absent")?
        .own_identity();
    let before = observer
        .world_entities()
        .ok_or("C13.client observer store absent")?;
    let owner_entity = before
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Player && entity.entity.identity == owner_wire_identity
        })
        .ok_or("C13.client observer does not see the caster")?
        .entity;
    let baseline_identity = *before.own_identity();
    let baseline_position = observer.world_spatial().actor_position;
    let baseline_entities: Vec<_> = before.iter().copied().collect();
    if baseline_entities.iter().any(is_rat) {
        return Err("C13.client observer already sees a Rat before casting".into());
    }
    let mana_before = owner
        .actor_vitals()
        .ok_or("C13.client caster vitals absent")?
        .mana;
    let expected_mana = mana_before
        .checked_sub(200)
        .ok_or("C13.client caster lacks 200 mana")?;
    let reply = owner
        .chat(&oteryn_dev_client::client_runtime::chat::ChatIntent::Say {
            mode: oteryn_dev_client::client_runtime::chat::ChatSpeechMode::Say,
            text: "utevo res \"rat\"".into(),
        })
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C13.client summon refused: {:?}", reply.disposition).into());
    }
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let receipt: (Vec<u8>, String) = sqlx::query_as(
        "SELECT uuid_send(d.actor_placement_identity), d.actor_local_generation::text
         FROM game_spell_direct_companion_acquisition_receipts d
         JOIN game_spell_item_receipts r USING (transaction_id)
         WHERE d.master_game_session_id = $1::uuid
           AND r.game_session_id = d.master_game_session_id
           AND r.command_id = $2::text::numeric
           AND d.master_placement_identity = encode($3, 'hex')::uuid
           AND d.creature_key = 'oteryn:creature.rat'
           AND d.source_kind = 1",
    )
    .bind(uuid_text(owner_session.as_bytes()))
    .bind(reply.command_id.to_string())
    .bind(owner_wire_identity.as_slice())
    .fetch_one(&mut db)
    .await?;
    db.close().await?;
    let rat = EntityRef {
        identity: receipt.0.as_slice().try_into()?,
        generation: receipt.1.parse()?,
    };
    let spawn = next_delta(&mut observer, "the summon enter delta", |delta| {
        delta.enter.iter().any(|entity| entity.entity == rat)
    })
    .await?;
    let [entered] = spawn.enter.as_slice() else {
        return Err("C13.client summon enter includes unrelated entities".into());
    };
    if rat.generation == 0 || !is_rat(entered) || !spawn.leave.is_empty() {
        return Err("C13.client spawn delta is not an isolated summon enter".into());
    }
    full_e2ef7::player_level(&mut owner).await?;
    if owner
        .actor_vitals()
        .is_none_or(|vitals| vitals.mana != expected_mana)
    {
        return Err("C13.client cast did not debit exactly 200 mana".into());
    }

    logout_once(&mut owner).await?;
    drop(owner);
    let ended = next_delta(&mut observer, "the owner-end leave delta", |delta| {
        delta.leave.contains(&rat)
    })
    .await?;
    if ended.leave.len() != 2
        || !ended.leave.contains(&owner_entity)
        || !ended.enter.is_empty()
        || ended.update.iter().any(|updated| {
            !baseline_entities
                .iter()
                .any(|entity| entity.entity == updated.entity)
        })
    {
        return Err("C13.client owner-end delta is not isolated from other changes".into());
    }
    if observer.world_entities().is_none_or(|entities| {
        entities.get(&rat).is_some()
            || entities.get(&owner_entity).is_some()
            || entities.iter().any(is_rat)
    }) {
        return Err("C13.client observer did not remove the exact caster and Rat".into());
    }
    let fresh = connect_character(&owner_clients, 0x2d, 0).await?;
    if fresh.join_snapshot().game_session_id == owner_session
        || fresh
            .actor_vitals()
            .is_none_or(|vitals| vitals.mana != expected_mana)
        || fresh
            .world_entities()
            .is_none_or(|entities| entities.iter().any(is_rat))
    {
        return Err("C13.client relog restored the summon or lost the mana debit".into());
    }

    let baseline = WorldSpatialEntitiesSnapshot {
        content_generation: spawn.content_generation,
        actor_position: baseline_position,
        own_identity: baseline_identity,
        entities: baseline_entities,
    };
    let fields = EntityFields::selected(FIELD_CAPABILITIES);
    let lights = EntityLights::new();
    let encoded_baseline =
        encode_world_spatial_entities_snapshot_with_fields(&baseline, fields, &lights)
            .map_err(|error| format!("C13.client baseline codec: {error:?}"))?;
    let encoded_spawn = encode_world_spatial_entities_delta_with_fields(&spawn, fields, &lights)
        .map_err(|error| format!("C13.client spawn codec: {error:?}"))?;
    let encoded_ended = encode_world_spatial_entities_delta_with_fields(&ended, fields, &lights)
        .map_err(|error| format!("C13.client owner-end codec: {error:?}"))?;
    let hex = |identity: &[u8; 16]| {
        identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = Path::new(&path);
        std::fs::write(path.join("c13-baseline.bin"), &encoded_baseline)?;
        std::fs::write(path.join("c13-spawn.bin"), &encoded_spawn)?;
        std::fs::write(path.join("c13-owner-ended.bin"), &encoded_ended)?;
        std::fs::write(
            path.join("c13-expected.txt"),
            format!(
                "Rat\n{}\n{}\n{}\n",
                hex(&rat.identity),
                rat.generation,
                hex(&owner_entity.identity)
            ),
        )?;
    }
    let mut fresh = fresh;
    logout_once(&mut fresh).await?;
    logout_once(&mut observer).await?;
    Ok(())
}
