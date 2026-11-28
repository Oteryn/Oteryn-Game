//! C07 full-row journey, derived from the matrix, the Crystal reaudit, and the
//! `rune_use_bridge`/field owner exercised by `postgres_tests/field_runes.rs`.
//!
//! 1. Prepare a dedicated level-15 sorcerer with two Fire Field runes and
//!    verify the served build and inventory after relog.
//! 2. While still on a protection-zone tile, send a real rune USE at an adjacent
//!    tile. Require `ProtectionZone`, unchanged vitals, and unchanged rune count.
//! 3. Walk to a reachable non-PZ two-tile corridor and send the same USE at its
//!    adjacent tile. Require `Committed`, a pushed map delta containing field
//!    item 2118, and the exact one-rune inventory debit.
//! 4. Send STEP onto that safe field. Require movement and no health debit.
//!    A damaging-field contact remains a separate required observation.
//! 5. Wait on native liveness for the server-owned decay and require a pushed
//!    map delta removing item 2118; step off and back on, requiring movement
//!    but no further field-contact health loss.
//! 6. Logout, perform a fresh admission, and require a different game session,
//!    the retained position, and the remaining rune count. This re-checks the
//!    durable payment/custody state; the expired field must not reappear.

use super::full_row_c01::admission_error;
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_map::MapDefinition;

const RUNE_ID: u32 = 3188;
const FIELD_ID: u32 = 2118;
const LEVEL: u32 = 15;
const RUNE_COUNT: u64 = 2;
const PREPARATION: character_prep::Seeded = character_prep::Seeded {
    profile: "c07-sorcerer",
    reason: "Fire Field eligibility and consumables for the field lifecycle",
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded("C07.full", "level-15-sorcerer,magic-level-1,two-fire-field-runes", PREPARATION.reason);
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(560), verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout-560s need=server-complete-field-lifecycle".into()),
    };
    if let Err(error) = &result {
        evidence(&format!("C07_GAP {error}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C07.full status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        if result.is_ok() { "" } else { " evidence=required-actions-unproven-see-C07_GAP" },
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_C07_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let mut setup = PREPARATION.connect(clients, 0xc7, 0).await?;
    let setup_session = setup.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(setup_session.as_bytes()),
        "--profile",
        format!("item-{RUNE_ID}-{RUNE_COUNT}"),
    ).await?;
    logout(&mut setup).await?;
    drop(setup);
    let mut client = connect_character(clients, 0xc7, 0).await
        .map_err(|error| admission_error("initial-admission", error))?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("step=prepare observed=reused-session".into());
    }
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer", LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    ).ok_or("step=prepare observed=no-source-vitals")?;
    use oteryn_protocol_oteryn::cyclopedia_character as character;
    let stats = client.session_mut().gated_command(
        character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[character::CharacterTab::GeneralStats as u8, 0, 0, 50],
    ).await?;
    if stats.status != CommandStatus::Accepted
        || stats.payload.first() != Some(&(character::CharacterTab::GeneralStats as u8))
        || stats.payload.get(9..13) != Some(LEVEL.to_le_bytes().as_slice())
        || stats.payload.get(13..15) != Some(1u16.to_le_bytes().as_slice())
        || client.actor_vitals().is_none_or(|v| v.max_health != max_health || v.max_mana != max_mana)
    {
        return Err("step=prepare observed=wrong-level-or-derived-vitals".into());
    }
    let reference = provisioning::item_reference(&provisioning::item_index()?, RUNE_ID)?;
    let field = provisioning::item_reference(&provisioning::item_index()?, FIELD_ID)?;
    if quantity(&client, reference)? != RUNE_COUNT {
        return Err("step=prepare observed=wrong-fire-field-rune-count".into());
    }
    check("prepared", client.last_server_sequence());

    use oteryn_protocol_oteryn::actor_spell_catalog as spell_wire;
    let catalogue = client.session_mut().gated_snapshot(spell_wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step=rune-eligibility observed=no-served-spell-catalog")?;
    let entry = spell_wire::decode_catalog(&catalogue.payload)
        .map_err(|error| format!("step=rune-eligibility observed=invalid-catalog:{error:?}"))?
        .into_iter().find(|entry| entry.rune_item == Some(RUNE_ID))
        .ok_or("step=rune-eligibility observed=fire-field-absent need=served-fire-field-rune")?;
    if !entry.available {
        return Err(format!("step=rune-eligibility item={RUNE_ID} observed=unavailable required-level={} need=served-fire-field-level-and-magic-level", entry.level).into());
    }

    let pz_path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|t| t.protection_zone())
            && adjacent(world, at).is_some()
    })
    .ok_or("step=protection-zone-refusal observed=no-reachable-pz-corridor need=compiled-pz-route")?;
    walk_first_attempt(&mut client, world, &pz_path).await?;
    let pz_target = adjacent(world, tile(client.world_spatial().actor_position)?)
        .ok_or("step=protection-zone-refusal observed=no-adjacent-tile need=walkable-pz-target")?.1;
    let before_count = quantity(&client, reference)?;
    let before_vitals = *client.actor_vitals().ok_or("step=protection-zone-refusal observed=no-vitals-frame need=actor-vitals")?;
    let (disposition, refusal_sequence) =
        use_rune(&mut client, reference, actor_position(pz_target)).await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::ProtectionZone {
        return Err(format!("step=protection-zone-refusal observed={disposition:?} need=ProtectionZone").into());
    }
    later_frame(&mut client).await?;
    if quantity(&client, reference)? != before_count || client.actor_vitals() != Some(&before_vitals) {
        return Err("step=protection-zone-refusal observed=state-changed need=no-payment-or-vitals-delta".into());
    }
    check("pz-refusal", refusal_sequence);

    let site_path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|t| !t.protection_zone())
            && adjacent(world, at).is_some_and(|(_, next)| world.base().tile(next.x, next.y, next.floor).is_some_and(|t| !t.protection_zone()))
    }).ok_or("step=field-cast observed=no-reachable-non-pz-corridor need=compiled-field-site")?;
    walk_first_attempt(&mut client, world, &site_path).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let (direction, target_tile) = adjacent(world, from).ok_or("step=field-cast observed=no-adjacent-target need=walkable-target")?;
    let target = actor_position(target_tile);
    let map = client.session_mut().world_map_snapshot().ok_or("step=field-site observed=no-live-map")?;
    if map.tiles.iter().find(|tile| tile.position == target).is_none_or(|tile| tile.more || tile.items.len() != 1) {
        return Err("step=field-site observed=nonempty-target-stack need=bare-ground-before-placement".into());
    }
    client.take_events();
    let (disposition, cast_sequence) = use_rune(&mut client, reference, target).await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=field-cast observed={disposition:?} need=Committed-and-field-delta").into());
    }
    let placed = cast_deltas(&mut client, target, field, reference, before_count - 1, cast_sequence).await?;
    check("attack-rune", placed);

    use oteryn_protocol_oteryn::condition_status as condition;
    let conditions = client.session_mut().gated_command(
        condition::CAPABILITY_CONDITION_STATUS_V1, condition::COMMAND_TYPE_CONDITION_STATUS_QUERY, &[],
    ).await?;
    if conditions.status != CommandStatus::Accepted
        || condition::ConditionStatus::decode(&conditions.payload).is_none_or(|status| !status.conditions.is_empty())
        || !field_site_isolated(&client, target)
    {
        return Err("step=field-step observed=unisolated-site need=no-other-actors-or-active-conditions".into());
    }
    let health_before = client.actor_vitals().ok_or("step=field-step observed=no-vitals need=vitals-frame")?.health;
    client.take_events();
    let step = client.step(direction).await?;
    if step.disposition != StepDisposition::Moved || tile(client.world_spatial().actor_position)? != target_tile {
        return Err(format!("step=field-step observed={step:?} need=Moved-position-delta").into());
    }
    observe_safe_contact(&mut client, health_before, target, step.result_server_sequence).await?;
    let after = client.session_mut().gated_command(
        condition::CAPABILITY_CONDITION_STATUS_V1, condition::COMMAND_TYPE_CONDITION_STATUS_QUERY, &[],
    ).await?;
    if after.status != CommandStatus::Accepted
        || condition::ConditionStatus::decode(&after.payload).is_none_or(|status| !status.conditions.is_empty())
        || client.actor_vitals().is_none_or(|vitals| !safe_field_health(health_before, vitals.health))
    {
        return Err("step=safe-field-contact observed=condition-or-vital-debit need=harmless-2118".into());
    }
    check("safe-field-contact-no-damage", client.last_server_sequence());
    // Regular Fire decays through 200 s, 148 s and 98 s stages.
    let expired = field_frame(&mut client, target, field, false, placed, Duration::from_secs(465)).await?;
    check("field-expiry", expired);

    let back = route(world, target_tile, |at| at == from).ok_or("step=post-expiry-step observed=no-return-route need=adjacent-route")?;
    walk_first_attempt(&mut client, world, &back).await?;
    let stable = client.actor_vitals().ok_or("step=post-expiry-step observed=no-vitals need=vitals-frame")?.health;
    walk_first_attempt(&mut client, world, &[direction]).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    if client.actor_vitals().is_none_or(|v| v.health != stable) {
        return Err("step=post-expiry-step observed=health-changed need=expired-field-no-contact-tick".into());
    }
    check("field-step-expiry", client.last_server_sequence());

    let old_session = client.join_snapshot().game_session_id;
    let retained_position = client.world_spatial().actor_position;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xc8, 0)
        .await
        .map_err(|error| admission_error("fresh-admission", error))?;
    if fresh.join_snapshot().game_session_id == old_session
        || fresh.world_spatial().actor_position != retained_position
        || quantity(&fresh, reference)? != before_count - 1
    {
        return Err("step=fresh-admission observed=session-position-or-rune-mismatch need=durable-payment-custody".into());
    }
    let map = fresh.session_mut().world_map_snapshot()
        .ok_or("step=fresh-admission observed=no-map-snapshot need=world-map-view")?;
    let former_field = map.tiles.iter().find(|tile| tile.position == target)
        .ok_or("step=fresh-admission observed=expired-field-tile-not-visible need=observed-retired-site")?;
    if former_field.items.iter().any(|item| item.definition == MapDefinition::Item(field)) {
        return Err("step=fresh-admission observed=expired-field-present need=decay-remain-retired".into());
    }
    check("fresh-admission", fresh.last_server_sequence());
    logout(&mut fresh).await?;
    Err("step=field-contact-damage observed=harmless-2118-on-baseline-world need=separate-damaging-field-and-PvP-eligible-target".into())
}

fn adjacent(world: &BundleWorld, at: TilePos) -> Option<(StepDirection, TilePos)> {
    [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
        .into_iter().find_map(|direction| destination(world, at, direction).filter(|next| next.floor == at.floor).map(|next| (direction, next)))
}
fn actor_position(at: TilePos) -> ActorPosition { ActorPosition { x: i32::from(at.x), y: i32::from(at.y), floor: -i16::from(at.floor) } }
fn check(step: &str, sequence: u64) { evidence(&format!("C07_CHECK {step} PASS server_sequence={sequence}")); }
fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client.inventory().ok_or("C07 inventory absent")?.entries.iter().filter(|e| e.item_definition_ref == reference).map(|e| u64::from(e.count.get())).sum())
}
fn handle(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<oteryn_protocol_oteryn::item_view::ItemHandle> {
    client.inventory().and_then(|v| v.entries.iter().find(|e| e.item_definition_ref == reference)).map(|e| e.handle).ok_or_else(|| "step=rune-use observed=no-carried-handle need=prepared-rune-inventory-frame".into())
}

async fn walk_first_attempt(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("step=walk observed=no-landing")?;
        let outcome = client.step(direction).await?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!("step=walk from={from:?} direction={direction:?} observed={outcome:?}").into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
        client.service_liveness(Duration::from_secs(1)).await?;
    }
    Ok(())
}

async fn use_rune(client: &mut DevClientSession, reference: std::num::NonZeroU32, target: ActorPosition) -> TestResult<(oteryn_protocol_oteryn::world_object::UseDisposition,u64)> {
    use oteryn_protocol_oteryn::{actor_spell::SpellTargetPosition, world_object as wire};
    let payload = wire::encode_item_use_intent(&wire::UseIntent { target: wire::UseTarget::Item(handle(client, reference)?), use_with: None, use_with_map_item: None, use_with_position: Some(SpellTargetPosition { x: target.x, y: target.y, floor: target.floor }) }).map_err(|e| format!("C07 rune encode: {e:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload).map_err(|e| format!("C07 refusal: {e:?}"))?;
    let disposition = wire::decode_use_result(payload).map_err(|e| format!("C07 use result: {e:?}"))?;
    if reply.status != CommandStatus::Accepted { return Err(format!("step=rune-use observed=status:{:?}/disposition:{disposition:?} need=Accepted-command-frame", reply.status).into()); }
    Ok((disposition, reply.server_sequence))
}

async fn cast_deltas(
    client: &mut DevClientSession,
    at: ActorPosition,
    field: std::num::NonZeroU32,
    rune: std::num::NonZeroU32,
    expected_count: u64,
    cast_sequence: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut placed = None;
    let mut paid = false;
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::WorldMap(delta) if delta.server_sequence > cast_sequence => {
                    if delta.value.tiles.iter().any(|tile| {
                        tile.position == at && tile.items.iter().any(|item| {
                            item.definition == MapDefinition::Item(field)
                        })
                    }) {
                        placed = Some(delta.server_sequence);
                    }
                }
                SessionEvent::Inventory(delta) if delta.server_sequence > cast_sequence => {
                    let count = delta.value.entries.iter()
                        .filter(|entry| entry.item_definition_ref == rune)
                        .map(|entry| u64::from(entry.count.get()))
                        .sum::<u64>();
                    if count != expected_count {
                        return Err(format!("step=inventory-delta observed={count} need={expected_count}").into());
                    }
                    paid = true;
                }
                _ => {}
            }
        }
        if let Some(sequence) = placed.filter(|_| paid) {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=field-cast observed=placement:{placed:?}/debit:{paid} need=both-pushed-deltas").into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn later_frame(client: &mut DevClientSession) -> TestResult { client.service_liveness(Duration::from_millis(100)).await.map_err(Into::into) }
fn field_actor_interferes(
    entry: &oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity,
    own_identity: &[u8; 16],
    at: ActorPosition,
) -> bool {
    entry.entity.identity != *own_identity
        && matches!(entry.kind, oteryn_dev_client::EntityKind::Creature | oteryn_dev_client::EntityKind::Player)
        && entry.position.floor == at.floor
        && entry.position.x.abs_diff(at.x).max(entry.position.y.abs_diff(at.y)) <= 8
}

fn field_site_isolated(client: &DevClientSession, at: ActorPosition) -> bool {
    client.world_entities().is_some_and(|entities| {
        entities.iter().all(|entry| !field_actor_interferes(entry, entities.own_identity(), at))
    })
}

async fn observe_safe_contact(client: &mut DevClientSession, mut health: u32, at: ActorPosition, step_sequence: u64) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    loop {
        let isolated = field_site_isolated(client, at) && client.world_spatial().actor_position == at;
        if !isolated {
            return Err("step=field-step observed=other-actor-or-movement need=isolated-field-contact".into());
        }
        let own_identity = *client.world_entities().ok_or("step=field-step observed=no-spatial-entities")?.own_identity();
        let events = client.take_events();
        if events.iter().any(|event| match event {
            SessionEvent::WorldSpatialEntities(delta) => delta.value.enter.iter().chain(&delta.value.update)
                .any(|entry| field_actor_interferes(entry, &own_identity, at)),
            _ => false,
        }) {
            return Err("step=field-step observed=other-actor-entered-observation need=isolated-field-contact".into());
        }
        for event in events {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.server_sequence > step_sequence
            {
                if !safe_field_health(health, delta.value.health) {
                    return Err("step=safe-field-contact observed=health-debit need=harmless-2118".into());
                }
                health = delta.value.health;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}
async fn field_frame(client: &mut DevClientSession, at: ActorPosition, reference: std::num::NonZeroU32, present: bool, after: u64, wait: Duration) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + wait;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop { for event in client.take_events() { if let SessionEvent::WorldMap(delta) = event && delta.server_sequence > after && let Some(tile) = delta.value.tiles.iter().find(|t| t.position == at) && (if present { tile.items.iter().any(|item| item.definition == MapDefinition::Item(reference)) } else { !tile.more && tile.items.len() == 1 }) { return Ok(delta.server_sequence); } } if tokio::time::Instant::now() >= deadline { return Err(format!("step={} observed=missing-map-delta tile={at:?} need=field-{present}", if present {"field-cast"} else {"field-expiry"}).into()); } client.service_liveness(Duration::from_millis(25)).await?; }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={:?} need=Accepted", reply.result).into());
    }
    Ok(())
}

fn safe_field_health(before: u32, after: u32) -> bool { after >= before }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_review_c07_rejects_false_evidence() {
        assert!(safe_field_health(100, 100));
        assert!(!safe_field_health(100, 95));
    }

    #[test]
    fn field_ids_resolve_from_the_production_item_index() -> TestResult { let index = provisioning::item_index()?; let rune = provisioning::item_reference(&index, RUNE_ID)?; let field = provisioning::item_reference(&index, FIELD_ID)?; assert_ne!(rune, field); Ok(()) }
}
