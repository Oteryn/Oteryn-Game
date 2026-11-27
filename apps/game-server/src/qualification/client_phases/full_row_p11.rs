//! P11: achievements query/notification and quest/event granters.
//!
//! Concrete player scenario derived from the P11 matrix/re-audit row and the routed owner:
//! 1. Admit the provisioned character and send account-achievement page 0; expect an accepted,
//!    internally consistent page and an achievement-notice watermark snapshot.
//! 2. Assert the live malformed-query refusal and unchanged pre-grant state.
//! 3. Check page boundaries and re-admit the same character to verify the query
//!    remains stable. A grant requires a reachable authored source and its notice.
use super::*;
use oteryn_dev_client::client_runtime::account_achievements::{
    AccountAchievementsResult, Response,
};
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::account_achievements as wire;
use oteryn_protocol_oteryn::item_view::{ItemEntry, ItemHandle};
use oteryn_protocol_oteryn::zones_nologout::ServerMessage;

// The live sword-350 run took 36.5 seconds to remove the first 35% of a demon's
// health. Allow 150 seconds per target for damage variation and server ticks, then
// 120 seconds for party setup, the combat lock, the reward and relog.
const TARGET_DEADLINE: Duration = Duration::from_secs(150);
const STEP_CLEAR_DEADLINE: Duration = Duration::from_secs(20);
pub(super) const JOURNEY_DEADLINE: Duration = Duration::from_secs(420);

pub(super) async fn run_packet(clients: &SeamClients<'_>, core: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let placed_at = std::cell::Cell::new(None);
    let result = verdict::indexed_timeout(
        JOURNEY_DEADLINE,
        verdict::observe(async { journey(clients, core, &placed_at).await.map(|_| ()) }),
    )
    .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step deadline: achievement lifecycle timed out".into()),
    };
    let (status, evidence_text) = match result {
        Ok(()) => (
            "PASS",
            "query-notice-grant-refusal-relogin-validated".to_owned(),
        ),
        Err(error) => ("FAIL", sanitize(&error.to_string())),
    };
    if let Some(placed_at) = placed_at.get() {
        evidence(&format!(
            "P11_TIMING placement_to_row_end_ms={} status={status}",
            placed_at.elapsed().as_millis()
        ));
    }
    evidence(&format!("P11_GAP status={status} observed={evidence_text}"));
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=P11.full status={status} seconds={:.3} evidence={evidence_text}",
        started.elapsed().as_secs_f64()
    ));
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_:./=,".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

async fn page(client: &mut DevClientSession, number: u32) -> TestResult<AccountAchievementsResult> {
    match client.session_mut().account_achievements(number).await? {
        Response::Page(page)
            if page.page == number
                && page.fact_count >= u32::try_from(page.rows.len())?
                && page.total_points >= page.rows.iter().map(|row| row.points).sum() =>
        {
            Ok(page)
        }
        Response::Page(page) => Err(format!("step=query invalid server page: {page:?}").into()),
        Response::Refused => Err(format!("step=query page={number} observed=refused").into()),
    }
}

const STAGING: TilePos = TilePos {
    x: 33225,
    y: 31671,
    floor: -13,
};

async fn place_member(clients: &SeamClients<'_>, index: usize, at: TilePos) -> TestResult {
    wait_character_released(clients, index).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let changed = sqlx::query(
        "UPDATE game_character_last_positions AS p
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
           AND (p.tile_x,p.tile_y,p.tile_floor) IS DISTINCT FROM ($3,$4,$5)
           AND NOT EXISTS (
               SELECT 1 FROM game_durability_reconnect_sessions AS s
               WHERE s.character_id=p.character_id AND s.world_id=p.world_id AND s.session_state<>3
           )",
    )
    .bind(clients.characters[index].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(at.x))
    .bind(i32::from(at.y))
    .bind(i32::from(-at.floor))
    .execute(&mut db)
    .await?;
    db.close().await?;
    if changed.rows_affected() != 1 {
        return Err(
            format!("P11 fixture requires one different offline position for {at:?}").into(),
        );
    }
    evidence(&format!(
        "FIXTURE_PLACEMENT mechanic=P11 character={} x={} y={} floor={}",
        uuid_text(&clients.characters[index]),
        at.x,
        at.y,
        -at.floor
    ));
    Ok(())
}

fn assert_staging(client: &DevClientSession, expected: TilePos) -> TestResult {
    let actual = tile(client.world_spatial().actor_position)?;
    if actual != expected {
        return Err(
            format!("P11 expected fixture position {expected:?}, admitted at {actual:?}").into(),
        );
    }
    Ok(())
}

async fn wait_for_lever_arrival(
    member: &mut DevClientSession,
    staged: TilePos,
    expected: TilePos,
    after_sequence: u64,
    deadline: tokio::time::Instant,
) -> TestResult {
    loop {
        let events = member.take_events();
        let mut arrived = false;
        for event in events {
            check_unexpected_notice(&event)?;
            evidence(&format!("P11_LEVER_FRAME member_x={} {event:?}", staged.x));
            let SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            let (sequence, position) = (delta.server_sequence, delta.value.actor_position);
            if sequence <= after_sequence {
                continue;
            }
            let actual = tile(position)?;
            if actual == staged && !arrived {
                continue;
            }
            if actual != expected {
                return Err(format!(
                    "step=lever wrong_arrival expected={expected:?} actual={actual:?} sequence={sequence}"
                ).into());
            }
            arrived = true;
        }
        if arrived {
            assert_staging(member, expected)?;
            return Ok(());
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "step=lever missing_arrival expected={expected:?} after_sequence={after_sequence} last_sequence={} actual={:?}",
                member.last_server_sequence(), tile(member.world_spatial().actor_position)?
            ).into());
        }
        member
            .service_liveness(remaining.min(Duration::from_millis(200)))
            .await?;
    }
}

const CORRIDOR_Y: u16 = 31659;
const WEAPON_ID: u32 = 7384; // Mystic Blade has a fully specified one-hand policy.
const POTION_ID: u32 = 7643;
const REWARD_ID: u32 = 3388;

fn bag_count(client: &DevClientSession, definition: std::num::NonZeroU32) -> Option<u32> {
    inventory_count(client.inventory(), definition)
}

pub(super) fn inventory_count(
    inventory: Option<&oteryn_protocol_oteryn::item_view::CharacterInventory>,
    definition: std::num::NonZeroU32,
) -> Option<u32> {
    Some(
        inventory?
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == definition)
            .map(|item| item.count.get())
            .sum(),
    )
}

async fn equip_corridor_weapon(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::client_runtime::inventory::MoveRequest;
    use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};
    let definition = provisioning::item_reference(&provisioning::item_index()?, WEAPON_ID)?;
    let source = client
        .inventory()
        .and_then(|view| {
            view.entries
                .iter()
                .find(|item| item.item_definition_ref == definition)
        })
        .ok_or("P11 fixture sword missing")?
        .handle;
    let equipped = |client: &DevClientSession| {
        client.inventory().is_some_and(|view| {
            view.equipment.iter().any(|item| {
                item.slot == EquipmentSlot::RightHand && item.item.item_definition_ref == definition
            })
        })
    };
    if equipped(client) {
        return Err("P11 fixture sword already equipped".into());
    }
    let result = client
        .session_mut()
        .move_item_counted(MoveRequest {
            source,
            destination: oteryn_dev_client::ItemMoveDestination::Equipment(
                EquipmentSlot::RightHand,
            ),
            count: None,
        })
        .await?;
    if result.outcome != ItemMoveOutcome::Moved {
        return Err(format!("P11 first sword equip failed: {result:?}").into());
    }
    wait_for_inventory(client, "P11 weapon equip", equipped).await
}

async fn corridor_step(
    client: &mut DevClientSession,
    direction: StepDirection,
    x: u16,
    y: u16,
) -> TestResult {
    let expected = TilePos { x, y, floor: -13 };
    let origin = client.world_spatial().actor_position;
    let (dx, dy) = direction.delta();
    // A teleport's arrival differs from the tile the step actually enters.
    let next = ActorPosition {
        x: origin.x + dx,
        y: origin.y + dy,
        floor: origin.floor,
    };
    let started = tokio::time::Instant::now();
    let deadline = started + STEP_CLEAR_DEADLINE;
    let mut waiting = false;
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Err(format!(
                "P11 step tile did not clear within {STEP_CLEAR_DEADLINE:?}: {next:?}"
            )
            .into());
        }
        let entities = client
            .world_entities()
            .ok_or("P11 step entity state missing")?;
        let blocker = entities.iter().find(|row| {
            row.position == next
                && matches!(
                    row.kind,
                    EntityKind::Creature | EntityKind::Player | EntityKind::Npc
                )
        });
        let Some(blocker) = blocker else {
            if waiting {
                evidence(&format!(
                    "P11_STEP_CLEAR tile={next:?} waited_ms={}",
                    started.elapsed().as_millis()
                ));
            }
            break;
        };
        if !waiting {
            evidence(&format!(
                "P11_STEP_WAIT tile={next:?} blocker={:?} timeout_ms={}",
                blocker.entity,
                STEP_CLEAR_DEADLINE.as_millis()
            ));
            waiting = true;
        }
        client
            .service_liveness((deadline - now).min(Duration::from_millis(100)))
            .await?;
    }
    let reply = client.step(direction).await?;
    let actual = tile(client.world_spatial().actor_position)?;
    if reply.disposition != StepDisposition::Moved || actual != expected {
        return Err(format!(
            "P11 first corridor step expected={expected:?} actual={actual:?} reply={reply:?}"
        )
        .into());
    }
    if reply.step_wait_ms > 0 {
        let until =
            tokio::time::Instant::now() + Duration::from_millis(u64::from(reply.step_wait_ms));
        while tokio::time::Instant::now() < until {
            client
                .service_liveness(
                    until
                        .saturating_duration_since(tokio::time::Instant::now())
                        .min(Duration::from_millis(100)),
                )
                .await?;
        }
    }
    Ok(())
}

async fn select_corridor_target(
    client: &mut DevClientSession,
    target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    reason: &str,
) -> TestResult<u64> {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    evidence(&format!(
        "P11_COMBAT attack_start reason={reason} session={} command={} after_sequence={} target={:?}",
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        client.next_command_id(),
        client.last_server_sequence(),
        target
    ));
    let reply = client
        .session_mut()
        .attack_target(Some(&target))
        .await?;
    if reply.status != oteryn_protocol_oteryn::CommandStatus::Accepted
        || reply.disposition != AttackIntentDisposition::Ok
    {
        return Err(format!(
            "P11 corridor attack failed target={:?}: {reply:?}",
            target
        )
        .into());
    }
    Ok(reply.result_server_sequence)
}

fn corridor_target_lost(
    acknowledgment: u64,
    observation: Option<(u64, Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>)>,
    visible: bool,
) -> bool {
    visible && matches!(observation, Some((sequence, None)) if sequence > acknowledgment)
}

async fn owned_corridor_corpse(
    clients: &SeamClients<'_>,
    target: oteryn_dev_client::EntityRef,
) -> TestResult<[u8; 16]> {
    use crate::foundation::{ActorLocalGeneration, ActorLocalId, ActorRef, ExactActorRef, ScopeOwnershipGeneration};
    let world = WorldId::decode(&clients.world)?;
    let channel = ChannelId::decode(&clients.channel)?;
    let scope = ScopeOwnershipGeneration::new(clients.scope_generation)
        .map_err(|error| format!("P11 invalid corpse scope: {error:?}"))?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let slots: Vec<i64> = sqlx::query_scalar(
        "SELECT death_actor_local_id FROM game_item_mint_receipts
         WHERE death_world_id=encode($1,'hex')::uuid
           AND death_channel_id=encode($2,'hex')::uuid
           AND death_scope_ownership_generation=$3::text::numeric
           AND death_actor_local_generation=$4::text::numeric
           AND corpse_top_damage_character_id=encode($5,'hex')::uuid
           AND loot_purpose_key='CORPSE_MATERIALIZATION'",
    )
    .bind(clients.world.as_slice())
    .bind(clients.channel.as_slice())
    .bind(clients.scope_generation.to_string())
    .bind(target.generation.to_string())
    .bind(clients.characters[1].as_slice())
    .fetch_all(&mut db)
    .await?;
    db.close().await?;
    let mut matches = Vec::new();
    for slot in slots {
        let actor = ExactActorRef(ActorRef::from_owner_parts(
            world, channel, scope, ActorLocalId(u32::try_from(slot)?),
            ActorLocalGeneration(target.generation),
        ));
        if actor.placement_identity() == target.identity {
            matches.push(actor.corpse_identity());
        }
    }
    match matches.as_slice() {
        [corpse] => Ok(*corpse),
        _ => Err(format!("P11 expected one committed corpse owned by the killer for {target:?}").into()),
    }
}

fn unique_corridor_corpse(
    entered: &[oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity],
    position: ActorPosition,
    definition: std::num::NonZeroU32,
    expected: [u8; 16],
    old: &[oteryn_protocol_oteryn::world_spatial_entities::EntityRef],
) -> TestResult<ItemHandle> {
    use oteryn_dev_client::EntityDetail;
    let mut candidates = entered.iter().filter_map(|row| {
        if row.kind != EntityKind::Corpse || row.position != position || old.contains(&row.entity) {
            return None;
        }
        match row.detail {
            EntityDetail::Object { item_definition_ref, item_handle, .. }
            | EntityDetail::ObjectWithAppearance { item_definition_ref, item_handle, .. }
                if item_definition_ref == definition.get() => item_handle.map(|handle| (row.entity.identity, handle)),
            _ => None,
        }
    });
    match (candidates.next(), candidates.next()) {
        (Some(row_and_handle), None) if row_and_handle.0 == expected => Ok(row_and_handle.1),
        _ => Err("P11 target removal must publish exactly one matching new corpse in the same delta".into()),
    }
}

async fn clear_corridor(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    use oteryn_dev_client::EntityDetail;
    let item_index = provisioning::item_index()?;
    let corpse_item = provisioning::item_reference(&item_index, 5995)?;
    let mut targets = Vec::new();
    let entities = client
        .world_entities()
        .ok_or("P11 corridor entity snapshot absent")?;
    let mut room_demons: Vec<_> = entities
        .iter()
        .filter(|entity| {
            entity.kind == EntityKind::Creature
                && entity.position.floor == 13
                && (33219..=33224).contains(&entity.position.x)
                && (31657..=31661).contains(&entity.position.y)
                && entity
                    .display_name
                    .is_some_and(|name| name.as_str() == Ok("Angry Demon"))
        })
        .map(|entity| (entity.position.x, entity.position.y))
        .collect();
    room_demons.sort_unstable();
    if room_demons
        != [
            (33219, 31657),
            (33220, 31661),
            (33221, 31657),
            (33222, 31661),
            (33223, 31659),
            (33224, 31659),
        ]
    {
        return Err(format!("P11 expected all six authored Angry Demons: {room_demons:?}").into());
    }
    for x in [33223, 33224] {
        let mut found = entities.iter().filter(|entity| {
            entity.kind == EntityKind::Creature
                && entity.position.x == x
                && entity.position.y == i32::from(CORRIDOR_Y)
                && entity.position.floor == 13
        });
        let target = found
            .next()
            .copied()
            .ok_or_else(|| format!("P11 no authored demon at {x},{CORRIDOR_Y},13"))?;
        if found.next().is_some()
            || target
                .display_name
                .is_none_or(|name| name.as_str() != Ok("Angry Demon"))
            || !matches!(
                target.detail,
                EntityDetail::Actor {
                    health_percent: 100,
                    ..
                }
            )
        {
            return Err(format!(
                "P11 corridor target was not one full-health Angry Demon: {target:?}"
            )
            .into());
        }
        targets.push(target);
    }
    let potion = provisioning::item_reference(&item_index, POTION_ID)?;
    let mut targets = targets.into_iter().peekable();
    let mut next_step_x = u16::try_from(targets.peek().ok_or("P11 corridor target missing")?.position.x)?;
    while let Some(target) = targets.next() {
        let existing_corpses: Vec<_> = client
            .world_entities()
            .ok_or("P11 corridor entities absent")?
            .iter()
            .filter(|row| row.kind == EntityKind::Corpse)
            .map(|row| row.entity)
            .collect();
        reject_earned_notices(client)?;
        let attack_sequence = select_corridor_target(client, target.entity, "initial").await?;
        let mut corpse_handle = None;
        let mut removed = false;
        let deadline = tokio::time::Instant::now() + TARGET_DEADLINE;
        let mut damaged = false;
        let mut last_health = 100;
        let mut lowest_health = last_health;
        let attack_started = std::time::Instant::now();
        let mut last = target.position;
        let mut loot_report = String::new();
        loop {
            client.service_liveness(Duration::from_millis(100)).await?;
            for event in client.take_events() {
                check_unexpected_notice(&event)?;
                if let SessionEvent::ServerMessage(delta) = &event
                    && delta.server_sequence > attack_sequence
                    && let ServerMessage::StatusText(text) = &delta.value
                    && (text.starts_with("Loot of ")
                        || (!loot_report.is_empty() && !loot_report.ends_with('.')))
                {
                    if !loot_report.is_empty() {
                        loot_report.push(' ');
                    }
                    loot_report.push_str(text);
                }
                if let SessionEvent::ActorCombatState(delta) = &event
                    && corridor_target_lost(
                        attack_sequence,
                        Some((delta.server_sequence, delta.value.target)),
                        !removed,
                    )
                {
                    return Err(format!(
                        "P11 target cleared before removal: target={:?} sequence={}",
                        target.entity, delta.server_sequence
                    ).into());
                }
                if let SessionEvent::WorldSpatialEntities(delta) = event {
                    for updated in delta
                        .value
                        .update
                        .iter()
                        .filter(|row| row.entity == target.entity)
                    {
                        last = updated.position;
                        if let EntityDetail::Actor { health_percent, .. } = updated.detail
                            && health_percent != last_health
                        {
                            damaged |= health_percent < 100;
                            if health_percent < lowest_health {
                                lowest_health = health_percent;
                                verdict::Harness::global().progress("p11-corridor-damage");
                            }
                            last_health = health_percent;
                            evidence(&format!(
                                "P11_COMBAT health target={:?} percent={health_percent} elapsed_ms={}",
                                target.entity,
                                attack_started.elapsed().as_millis()
                            ));
                        }
                    }
                    if delta.value.leave.contains(&target.entity) {
                        if removed || !damaged || delta.server_sequence <= attack_sequence {
                            return Err("P11 unexpected corridor target removal".into());
                        }
                        let expected = owned_corridor_corpse(clients, target.entity).await?;
                        corpse_handle = Some(unique_corridor_corpse(
                            &delta.value.enter, last, corpse_item, expected, &existing_corpses,
                        )?);
                        removed = true;
                    }
                }
            }
            if let Some(handle) = corpse_handle.filter(|_| loot_report.ends_with('.')) {
                assert_corridor_loot(client, handle, &item_index, &loot_report).await?;
                evidence(&format!(
                    "P11_COMBAT killed={:?} corpse_at={last:?}",
                    target.entity
                ));
                break;
            }
            let vitals = client.actor_vitals().cloned().ok_or("P11 corridor vitals missing")?;
            if vitals.health == 0 || tokio::time::Instant::now() >= deadline {
                let lock = client.session_mut().combat_state().and_then(|state| state.target);
                return Err(format!(
                    "P11 corridor kill failed target={:?} lock={lock:?} attack_sequence={attack_sequence} damaged={damaged} remaining_percent={last_health} loot_report={loot_report:?} vitals={vitals:?}",
                    target.entity
                )
                .into());
            }
            if vitals.max_health - vitals.health >= 1200 {
                let health = vitals.health;
                let before = bag_count(client, potion).ok_or("P11 potion inventory missing")?;
                let handle = client
                    .inventory()
                    .and_then(|view| {
                        view.entries
                            .iter()
                            .find(|item| item.item_definition_ref == potion)
                    })
                    .ok_or("P11 fixture potions exhausted")?
                    .handle;
                evidence(&format!(
                    "P11_COMBAT potion_use session={} command={} after_sequence={} item={POTION_ID} hp={health} count={before}",
                    uuid_text(client.join_snapshot().game_session_id.as_bytes()),
                    client.next_command_id(),
                    client.last_server_sequence()
                ));
                let result = client.use_item(handle).await?;
                if result.disposition
                    != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
                {
                    return Err(format!("P11 first potion use refused: {result:?}").into());
                }
                wait_for_inventory(client, "P11 potion", |client| {
                    bag_count(client, potion)
                        .is_some_and(|after| after.checked_add(1) == Some(before))
                        && client
                            .actor_vitals()
                            .is_some_and(|vitals| vitals.health > health)
                })
                .await?;
            }
        }
        let entities = client.world_entities().ok_or("P11 corridor entity state missing")?;
        if let Some(next_target) = targets.peek()
            && entities
                .get(&next_target.entity)
                .is_some_and(|entity| entity.position == target.position)
        {
            evidence(&format!(
                "P11_ROUTE fight_before_step tile={:?} blocker={:?}",
                target.position, next_target.entity
            ));
            continue;
        }
        let reached_x = u16::try_from(target.position.x)?;
        for x in next_step_x..=reached_x {
            corridor_step(client, StepDirection::East, x, CORRIDOR_Y).await?;
        }
        next_step_x = reached_x + 1;
    }
    Ok(())
}

async fn assert_corridor_loot(
    client: &mut DevClientSession,
    handle: ItemHandle,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    observed: &str,
) -> TestResult {
    let reply = client.use_item(handle).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("P11 corpse owner USE refused: {reply:?}").into());
    }
    wait_for_inventory(client, "P11 corpse view", |client| {
        client.open_container().is_some_and(|view| view.container_handle == Some(handle))
    }).await?;
    let entries = &client.open_container().ok_or("P11 corpse view missing")?.entries;
    let descriptions = entries.iter().map(|entry| corridor_loot_description(index, entry))
        .collect::<Result<Vec<_>, _>>()?;
    let loot = if descriptions.is_empty() { "nothing".to_owned() } else { descriptions.join(", ") };
    let expected = format!("Loot of an angry demon: {loot}.");
    if observed != expected {
        return Err(format!("P11 corpse loot report expected={expected:?} observed={observed:?}").into());
    }
    evidence(&format!("P11_LOOT {observed}"));
    Ok(())
}

fn corridor_loot_description(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    entry: &ItemEntry,
) -> TestResult<String> {
    let (key, _) = index.definition_at(entry.item_definition_ref).ok_or("P11 corpse item ref missing")?;
    let id = key.strip_prefix("oteryn:item.tibia.i").ok_or("P11 corpse item lacks Global ID")?;
    let names = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/gameplay/dispatch/src/gameplay_transport/kill_reward/loot_item_names.tsv"
    ));
    let forms = names.lines().find_map(|line| {
        let (row_id, forms) = line.split_once('\t')?;
        (row_id == id).then_some(forms)
    }).ok_or_else(|| format!("P11 Global loot item name missing for {key}"))?;
    let mut forms = forms.split('\t');
    let (Some(name), Some(article), Some(plural)) = (forms.next(), forms.next(), forms.next()) else {
        return Err(format!("P11 invalid Global loot item forms for {key}").into());
    };
    Ok(match (entry.count.get(), article, plural) {
        (1, "-", _) => name.to_owned(),
        (1, article, _) => format!("{article} {name}"),
        (count, _, "-") => format!("{count} {name}"),
        (count, _, plural) => format!("{count} {plural}"),
    })
}

async fn operate_lever(
    client: &mut DevClientSession,
    world: &BundleWorld,
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
) -> TestResult {
    assert_staging(client, STAGING)?;
    let mut companions = Vec::with_capacity(3);
    for (scope, index, x) in [(clients, 0, 33224), (core, 0, 33223), (core, 1, 33222)] {
        let finished = std::cell::Cell::new(false);
        let (member, serviced) = tokio::join!(
            async {
                let result = async {
                    let member = connect_knight(scope, index, false).await?;
                    assert_staging(&member, TilePos { x, ..STAGING })?;
                    Ok::<_, Box<dyn std::error::Error>>(member)
                }
                .await;
                finished.set(true);
                result
            },
            async {
                while !finished.get() {
                    for peer in std::iter::once(&mut *client).chain(companions.iter_mut()) {
                        peer.service_liveness(Duration::from_millis(100)).await?;
                    }
                }
                Ok::<(), oteryn_dev_client::DevClientError>(())
            }
        );
        let member = member?;
        serviced?;
        companions.push(member);
    }
    let lever = TilePos {
        x: 33226,
        ..STAGING
    };
    let key =
        crate::map::view::placement_key(lever, 1).ok_or("Annihilator lever placement absent")?;
    let mut target = world.base().digest().to_vec();
    target.extend_from_slice(&key.to_be_bytes());
    reject_earned_notices(client)?;
    let [second, third, fourth] = companions.as_mut_slice() else {
        return Err("P11 requires exactly three lever companions".into());
    };
    let previous_sequence = |member: &mut DevClientSession| -> TestResult<u64> {
        reject_earned_notices(member)?;
        Ok(member.last_server_sequence())
    };
    let second_sequence = previous_sequence(second)?;
    let third_sequence = previous_sequence(third)?;
    let fourth_sequence = previous_sequence(fourth)?;
    let reply = client.session_mut().use_object(&target, 0).await?;
    evidence(&format!(
        "P11_ROUTE step=annihilator-lever tile={lever:?} wire_reply={reply:?}"
    ));
    if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        for member in &mut companions {
            let _ = logout(member).await;
        }
        return Err(format!("step=lever authored_lever={lever:?} first_reply={reply:?}").into());
    }
    let destination = TilePos {
        x: 33222,
        y: 31659,
        floor: -13,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let (leader, second, third, fourth) = tokio::join!(
        wait_for_lever_arrival(
            client,
            STAGING,
            destination,
            reply.result_server_sequence,
            deadline
        ),
        wait_for_lever_arrival(
            second,
            TilePos {
                x: 33224,
                ..STAGING
            },
            TilePos {
                x: 33221,
                ..destination
            },
            second_sequence,
            deadline
        ),
        wait_for_lever_arrival(
            third,
            TilePos {
                x: 33223,
                ..STAGING
            },
            TilePos {
                x: 33220,
                ..destination
            },
            third_sequence,
            deadline
        ),
        wait_for_lever_arrival(
            fourth,
            TilePos {
                x: 33222,
                ..STAGING
            },
            TilePos {
                x: 33219,
                ..destination
            },
            fourth_sequence,
            deadline
        ),
    );
    leader?;
    second?;
    third?;
    fourth?;
    evidence("P11_LEVER_ARRIVAL all_four=exact");
    let combat_finished = std::cell::Cell::new(false);
    let (combat, liveness) = tokio::join!(
        async {
            let result = clear_corridor(client, clients).await;
            combat_finished.set(true);
            result
        },
        async {
            while !combat_finished.get() {
                for peer in &mut companions {
                    peer.service_liveness(Duration::from_millis(100)).await?;
                    reject_earned_notices(peer)?;
                }
            }
            TestResult::Ok(())
        }
    );
    combat?;
    liveness?;
    Ok(())
}

async fn authored_granter(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
) -> TestResult<(Vec<u8>, [Vec<u8>; 3], String, String, &'static BundleWorld)> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let items = provisioning::item_index()?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|item| {
            item.key()
                .as_str()
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|id| id.parse().ok())
                .is_some_and(|id| provisioning::item_reference(&items, id).is_ok())
        },
    )?;
    operate_lever(client, world, clients, core).await?;
    const REWARD_CHEST: TilePos = TilePos {
        x: 33227,
        y: 31656,
        floor: -13,
    };
    let mut claims = report.served.iter().filter(|claim| {
        claim.placements.iter().any(|placement| {
            (placement.cell.x, placement.cell.y, placement.cell.floor)
                == (REWARD_CHEST.x, REWARD_CHEST.y, REWARD_CHEST.floor)
        })
    });
    let claim = claims.next().ok_or("P11 missing authored Annihilator claim")?;
    if claims.next().is_some() || claim.placements.len() != 4 {
        return Err("P11 expected one shared claim with four rewards".into());
    }
    let fixed_chest = |x| {
        let mut placements = claim.placements.iter().filter(|placement| {
            (placement.cell.x, placement.cell.y, placement.cell.floor)
                == (x, REWARD_CHEST.y, REWARD_CHEST.floor)
                && placement.placement.achievement.as_deref() == Some(ACHIEVEMENT)
        });
        let placement = placements.next().ok_or("P11 missing fixed authored chest")?;
        if placements.next().is_some() {
            return Err("P11 ambiguous fixed authored chest");
        }
        Ok(placement)
    };
    let placement = fixed_chest(REWARD_CHEST.x)?;
    let alternates = [fixed_chest(33229)?, fixed_chest(33231)?, fixed_chest(33233)?];
    let wire_target = |key: u64| {
        let mut target = world.base().digest().to_vec();
        target.extend_from_slice(&key.to_be_bytes());
        target
    };
    let key = wire_target(placement.bundle_placement_key);
    let alternate_keys = alternates.map(|placement| wire_target(placement.bundle_placement_key));
    // From the corridor, a bound reward chest must reject on distance.
    // The generic map fallback for a missing binding returns NothingToUse.
    let before_probe = client.inventory().ok_or("P11 missing inventory before binding probes")?.clone();
    for target in std::iter::once(&key).chain(&alternate_keys) {
        let reply = client.session_mut().use_object(target, 0).await?;
        if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::TooFar
            || client.inventory() != Some(&before_probe)
        {
            return Err(format!("P11 live reward binding probe expected TooFar: {reply:?}").into());
        }
        reject_earned_notices(client)?;
    }
    let [item] = placement.placement.items.as_slice() else {
        return Err("P11 bound chest must award exactly one item".into());
    };
    if item.count != 1 || item.item.key().as_str() != "oteryn:item.tibia.i3388" {
        return Err("P11 bound chest does not award one Demon Armor".into());
    }
    let door = TilePos {
        x: 33225,
        y: CORRIDOR_Y,
        floor: -13,
    };
    let door_key =
        crate::map::view::placement_key(door, 1).ok_or("P11 reward door placement missing")?;
    let mut door_target = world.base().digest().to_vec();
    door_target.extend_from_slice(&door_key.to_be_bytes());
    let door_reply = client.session_mut().use_object(&door_target, 0).await?;
    if door_reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("P11 first reward-room door USE failed: {door_reply:?}").into());
    }
    let interaction = TilePos {
        y: 31657,
        ..REWARD_CHEST
    };
    for x in 33225..=interaction.x {
        corridor_step(client, StepDirection::East, x, CORRIDOR_Y).await?;
    }
    for y in (interaction.y..CORRIDOR_Y).rev() {
        corridor_step(client, StepDirection::North, interaction.x, y).await?;
    }
    Ok((key, alternate_keys, ACHIEVEMENT.to_owned(), claim.key.as_str().to_owned(), world))
}

async fn leave_reward_room(
    client: &mut DevClientSession,
    world: &BundleWorld,
    alternates: &[Vec<u8>; 3],
    reward: std::num::NonZeroU32,
    granted: &AccountAchievementsResult,
) -> TestResult {
    let portal = TilePos {
        x: 33236,
        y: 31655,
        floor: -13,
    };
    let arrival = TilePos {
        x: 33213,
        y: 31671,
        floor: -13,
    };
    let from = tile(client.world_spatial().actor_position)?;
    if world.floors().teleport(portal) != Some(arrival)
        || !world
            .base()
            .tile(from.x, from.y, from.floor)
            .is_some_and(|tile| tile.no_logout())
        || !world
            .base()
            .tile(arrival.x, arrival.y, arrival.floor)
            .is_some_and(|tile| !tile.no_logout())
    {
        return Err(
            "P11 authored exit must lead from the reward room to the logout-enabled approach"
                .into(),
        );
    }

    // Global Annihilator Quest: use the teleport east of the rewards. The
    // captured display and statue require the walk below the display.
    use StepDirection::{East, North, South};
    corridor_step(client, East, 33228, 31657).await?;
    for (index, target) in alternates.iter().enumerate() {
        let x = [33228, 33230, 33232][index];
        if index != 0 {
            corridor_step(client, East, x - 1, 31658).await?;
            corridor_step(client, East, x, 31658).await?;
            corridor_step(client, North, x, 31657).await?;
        }
        let before_inventory = client.inventory().ok_or("P11 missing carried inventory")?.clone();
        let reply = client.session_mut().use_object(target, 0).await?;
        if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse
            || page(client, 0).await? != *granted
            || client.inventory() != Some(&before_inventory)
            || bag_count(client, reward) != Some(1)
        {
            return Err(format!("P11 alternative chest beside x={x} changed the reward: {reply:?}").into());
        }
        reject_earned_notices(client)?;
        evidence(&format!("P11_CHECK alternative_chest_adjacent_x={x} disposition=NothingToUse inventory=unchanged page=unchanged"));
        corridor_step(client, South, x, 31658).await?;
    }
    corridor_step(client, East, 33233, 31658).await?;
    corridor_step(client, South, 33233, 31659).await?;
    corridor_step(client, East, 33234, 31659).await?;
    let door = TilePos {
        x: 33235,
        y: 31659,
        floor: -13,
    };
    let key = crate::map::view::placement_key(door, 1).ok_or("P11 exit door placement missing")?;
    let mut target = world.base().digest().to_vec();
    target.extend_from_slice(&key.to_be_bytes());
    let reply = client.session_mut().use_object(&target, 0).await?;
    if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("P11 first exit-door USE failed: {reply:?}").into());
    }

    for x in 33235..=portal.x {
        corridor_step(client, East, x, 31659).await?;
    }
    for y in (31656..=31658).rev() {
        corridor_step(client, North, portal.x, y).await?;
    }
    corridor_step(client, North, arrival.x, arrival.y).await?;
    verdict::Harness::global().progress("p11-reward-room-exit");
    evidence("P11_ROUTE exit_teleport_arrival=33213,31671,13 no_logout=false");
    Ok(())
}

async fn prepare_knight(clients: &SeamClients<'_>, index: usize, leader: bool) -> TestResult {
    let (level, profile) = if leader {
        ("knight-level-1000", "p11-corridor-knight")
    } else {
        ("knight-level-100", "knight")
    };
    character_prep::declare_seeded(
        "P11.full",
        level,
        "Annihilator lever eligibility and corridor survival",
    );
    let mut initial = connect_character(clients, 0xc1, index).await?;
    let session = uuid_text(initial.join_snapshot().game_session_id.as_bytes());
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        session.clone(),
        "--profile",
        profile.into(),
    )
    .await?;
    if leader {
        for (id, count) in [(WEAPON_ID, 1), (POTION_ID, 20)] {
            request_fixture_control_at(
                "NODE_BOOT_CONTROL_SOCKET",
                session.clone(),
                "--profile",
                format!("item-{id}-{count}"),
            )
            .await?;
            evidence(&format!(
                "FIXTURE_P11 character={} item={id} count={count}",
                uuid_text(&clients.characters[index])
            ));
        }
    }
    let (result, detail, _, _) = logout_once(&mut initial).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("P11 fixture logout refused: {detail}").into());
    }
    Ok(())
}

async fn connect_knight(
    clients: &SeamClients<'_>,
    index: usize,
    leader: bool,
) -> TestResult<DevClientSession> {
    let level = if leader { 1000 } else { 100 };
    let mut fresh = connect_character(clients, 0xc1, index).await?;
    use oteryn_dev_client::client_ui::cyclopedia_character as panel;
    use oteryn_protocol_oteryn::cyclopedia_character::CharacterTab;
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        limit: 50,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(&fresh),
            death_details: false,
        },
        ..Default::default()
    };
    let command = panel::request(
        &mut view,
        panel::Intent::SelectTab(CharacterTab::GeneralStats),
    )
    .ok_or("P11 GeneralStats request absent")?;
    let reply = fresh
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    let sword = if leader { 350 } else { 10 };
    if !panel::apply_result(&mut view, &reply)
        || view
            .general
            .as_ref()
            .map(|stats| (stats.level, stats.skills[2]))
            != Some((level, sword))
    {
        return Err(format!(
            "P11 seeded Knight expected level {level} and sword skill {sword}; reply={reply:?}"
        )
        .into());
    }
    if leader {
        evidence(&format!(
            "FIXTURE_P11 character={} sword_skill={sword} verified_after_admission=true",
            uuid_text(&clients.characters[index]),
        ));
    }
    let vitals = fresh
        .actor_vitals()
        .ok_or("P11 seeded Knight missing vitals")?;
    evidence(&format!(
        "P11_ROUTE seeded_level={level} seeded_vitals={vitals:?}"
    ));
    if (vitals.max_health, vitals.max_mana) != (1565 + (level - 100) * 15, 550 + (level - 100) * 5)
    {
        return Err(format!("P11 Knight derived vitals mismatch: {vitals:?}").into());
    }
    Ok(fresh)
}

pub(super) struct GrantProof {
    pub(super) page: AccountAchievementsResult,
    pub(super) session: [u8; 16],
    pub(super) reward: std::num::NonZeroU32,
    pub(super) claim: String,
    pub(super) relog_chest: Vec<u8>,
}

const CONGRATULATIONS: &str = "Congratulations! You earned the achievement Annihilator.";
const ACHIEVEMENT: &str = "oteryn:achievement/annihilator";
const DESCRIPTION: &str = "You've daringly jumped into the infamous Annihilator and survived - taking home fame, glory and your reward.";

fn achievement_clock_ms() -> TestResult<i64> {
    Ok(i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?)
}

fn expected_grant(page: &AccountAchievementsResult, earliest: i64, latest: i64) -> bool {
    page.page == 0
        && page.total_points == 5
        && page.fact_count == 1
        && !page.has_more
        && matches!(page.rows.as_slice(), [entry]
            if entry.key == ACHIEVEMENT && entry.name == "Annihilator"
                && entry.description == DESCRIPTION && entry.grade == 2 && entry.points == 5
                && !entry.secret && (earliest..=latest).contains(&entry.earned_at))
}

pub(super) async fn earn_and_logout(
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
    placed_at: &std::cell::Cell<Option<std::time::Instant>>,
) -> TestResult<GrantProof> {
    // Complete setup with one live session at a time, before staging the party.
    for (scope, index, leader) in [
        (clients, 1, true),
        (clients, 0, false),
        (core, 0, false),
        (core, 1, false),
    ] {
        prepare_knight(scope, index, leader).await?;
    }
    for (scope, index, x) in [
        (clients, 1, 33225),
        (clients, 0, 33224),
        (core, 0, 33223),
        (core, 1, 33222),
    ] {
        place_member(scope, index, TilePos { x, ..STAGING }).await?;
        if placed_at.get().is_none() {
            placed_at.set(Some(std::time::Instant::now()));
        }
    }
    let mut client = connect_knight(clients, 1, true).await?;
    let session = *client.join_snapshot().game_session_id.as_bytes();
    let before = page(&mut client, 0).await?;
    if before != AccountAchievementsResult::default() {
        return Err(format!(
            "P11 fresh account expected no previous achievement facts; actual={before:?}"
        )
        .into());
    }
    let notice_snapshot = client
        .session_mut()
        .gated_snapshot(
            oteryn_protocol_oteryn::achievement_notices::STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        )
        .ok_or("step=query missing achievement-notice snapshot")?;
    let watermark =
        oteryn_protocol_oteryn::achievement_notices::decode_achievement_notices_snapshot(
            &notice_snapshot.payload,
        )
        .map_err(|error| format!("step=query malformed notice snapshot: {error:?}"))?;
    if watermark.fact_count != 0 || watermark.total_points != 0 {
        return Err(
            format!("P11 initial achievement watermark was not empty: {watermark:?}").into(),
        );
    }

    reject_earned_notices(&mut client)?;
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    if wire::decode_account_achievements_query(&[0x08]).is_ok()
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
    {
        return Err("step=local-codec-refusal observed=submitted-or-not-refused".into());
    }
    if page(&mut client, 0).await? != before {
        return Err("step=local-codec-refusal observed=unexpected-achievement-delta".into());
    }
    reject_earned_notices(&mut client)?;
    evidence(
        "P11_CHECK step=malformed-query boundary=local-codec-refusal wire_submission=false command-and-sequence=unchanged reread=unchanged",
    );

    let beyond = before
        .fact_count
        .div_ceil(wire::ACCOUNT_ACHIEVEMENTS_PAGE_ROWS as u32)
        .max(1);
    let empty = page(&mut client, beyond).await?;
    if empty.has_more
        || !empty.rows.is_empty()
        || empty.fact_count != before.fact_count
        || empty.total_points != before.total_points
    {
        return Err(format!("step=pagination page={beyond} observed={empty:?}").into());
    }
    equip_corridor_weapon(&mut client).await?;
    let reward = provisioning::item_reference(&provisioning::item_index()?, REWARD_ID)?;
    if bag_count(&client, reward) != Some(0) {
        return Err("P11 reward inventory pre-state must be zero".into());
    }
    let (placement, alternate, achievement, claim, world) = authored_granter(&mut client, clients, core).await?;
    if achievement != ACHIEVEMENT {
        return Err(
            format!("P11 expected the authored Annihilator claim, got {achievement}").into(),
        );
    }
    check_claim(clients, &claim, false).await?;
    let earliest = achievement_clock_ms()?;
    reject_earned_notices(&mut client)?;
    let before_claim = client.inventory().ok_or("P11 missing pre-claim inventory")?.clone();
    let use_reply = client.session_mut().use_object(&placement, 0).await?;
    if use_reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!(
            "step=granter achievement={achievement} authored chest declined: {use_reply:?}"
        )
        .into());
    }
    // The Server may defer notice delivery to its next liveness refresh.
    let notice_deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut earned = None;
    let mut congratulations = false;
    let earned = loop {
        for event in client.take_events() {
            match event {
                SessionEvent::AchievementEarned(delta) => {
                    if !fresh_notice(use_reply.result_server_sequence, delta.server_sequence, earned.is_some())
                    {
                        return Err("P11 earned notice was premature or duplicated".into());
                    }
                    earned = Some(delta.value);
                }
                event => {
                    if let Some((sequence, text)) = achievement_congratulations(&event) {
                        if !fresh_notice(use_reply.result_server_sequence, sequence, congratulations)
                            || text != CONGRATULATIONS
                        {
                            return Err(format!("P11 invalid congratulations sequence={sequence} text={text:?}").into());
                        }
                        congratulations = true;
                    }
                }
            }
        }
        if congratulations && let Some(earned) = earned.take() {
            break earned;
        }
        let remaining = notice_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "step=granter committed chest did not send both achievement notifications: {use_reply:?}"
            )
            .into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(250)))
            .await?;
    };
    wait_for_inventory(&mut client, "P11 chest reward", |client| {
        bag_count(client, reward) == Some(1)
    })
    .await?;
    let granted = page(&mut client, 0).await?;
    let latest = achievement_clock_ms()?;
    if !only_reward_added(&before_claim, client.inventory().ok_or("P11 missing post-claim inventory")?, reward) {
        return Err("P11 chest must add exactly one Demon Armor and no other item changes".into());
    }
    if earned.key != ACHIEVEMENT
        || earned.name != "Annihilator"
        || earned.watermark.fact_count != 1
        || earned.watermark.total_points != 5
        || !expected_grant(&granted, earliest, latest)
    {
        return Err(format!("step=granter delta/query mismatch: {earned:?} {granted:?}").into());
    }

    reject_earned_notices(&mut client)?;
    let before_duplicate = client.inventory().ok_or("P11 missing inventory before repeat USE")?.clone();
    let duplicate_reply = client.session_mut().use_object(&placement, 0).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    let unchanged = page(&mut client, 0).await?;
    reject_earned_notices(&mut client)?;
    if duplicate_reply.disposition
        != oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse
        || unchanged != granted
        || client.inventory() != Some(&before_duplicate)
        || bag_count(&client, reward) != Some(1)
    {
        return Err(
            format!("step=duplicate observed_reply={duplicate_reply:?},duplicate-grant").into(),
        );
    }
    leave_reward_room(&mut client, world, &alternate, reward, &granted).await?;
    reject_earned_notices(&mut client)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(65);
    while client
        .session_mut()
        .combat_state()
        .ok_or("P11 combat state missing")?
        .in_fight
    {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("P11 combat lock did not clear before logout".into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(200)))
            .await?;
        reject_earned_notices(&mut client)?;
    }
    logout(&mut client).await?;
    reject_earned_notices(&mut client)?;
    drop(client);
    Ok(GrantProof {
        page: granted,
        session,
        reward,
        claim,
        relog_chest: alternate[0].clone(),
    })
}

pub(super) async fn check_claim(clients: &SeamClients<'_>, claim: &str, expected: bool) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let present: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_reward_claims
         WHERE character_id=encode($1,'hex')::uuid AND claim_family='RewardClaim' AND claim_production_key=$2)",
    )
    .bind(clients.characters[1].as_slice())
    .bind(claim)
    .fetch_one(&mut db)
    .await?;
    db.close().await?;
    if present != expected {
        return Err(format!("P11 shared claim {claim} durable state expected={expected} actual={present}").into());
    }
    Ok(())
}

pub(super) async fn probe_relog_claim(clients: &SeamClients<'_>, proof: &GrantProof) -> TestResult {
    check_claim(clients, &proof.claim, true).await?;
    // The normal relog has already verified the persisted reward and page.
    // Position this released character at the alternate for a separate lockout probe.
    let at = TilePos { x: 33228, y: 31658, floor: -13 };
    place_member(clients, 1, at).await?;
    let mut client = connect_character(clients, 0xb1, 1).await?;
    if tile(client.world_spatial().actor_position)? != at {
        return Err("P11 relog lockout probe did not admit at the alternate chest".into());
    }
    let before = client.inventory().ok_or("P11 missing relog lockout inventory")?.clone();
    let distant = client.session_mut().use_object(&proof.relog_chest, 0).await?;
    if distant.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::TooFar
        || client.inventory() != Some(&before)
    {
        return Err(format!("P11 fresh session failed the alternate chest binding probe: {distant:?}").into());
    }
    reject_earned_notices(&mut client)?;
    corridor_step(&mut client, StepDirection::North, at.x, 31657).await?;
    let reply = client.session_mut().use_object(&proof.relog_chest, 0).await?;
    if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse
        || client.inventory() != Some(&before)
        || page(&mut client, 0).await? != proof.page
        || client.inventory() != Some(&before)
    {
        return Err(format!("P11 shared claim lockout failed in a fresh session: {reply:?}").into());
    }
    reject_earned_notices(&mut client)?;
    evidence("P11_CHECK persisted_shared_claim alternate_chest=NothingToUse inventory=unchanged page=unchanged");
    corridor_step(&mut client, StepDirection::South, at.x, at.y).await?;
    for x in 33229..=33233 {
        corridor_step(&mut client, StepDirection::East, x, 31658).await?;
    }
    corridor_step(&mut client, StepDirection::South, 33233, 31659).await?;
    for x in 33234..=33236 {
        corridor_step(&mut client, StepDirection::East, x, 31659).await?;
    }
    for y in (31656..=31658).rev() {
        corridor_step(&mut client, StepDirection::North, 33236, y).await?;
    }
    corridor_step(&mut client, StepDirection::North, 33213, 31671).await?;
    reject_earned_notices(&mut client)?;
    logout(&mut client).await?;
    reject_earned_notices(&mut client)

}

fn only_reward_added(
    before: &oteryn_protocol_oteryn::item_view::CharacterInventory,
    after: &oteryn_protocol_oteryn::item_view::CharacterInventory,
    reward: std::num::NonZeroU32,
) -> bool {
    let mut without_reward = after.clone();
    without_reward.entries.retain(|entry| entry.item_definition_ref != reward);
    inventory_count(Some(after), reward) == Some(1) && &without_reward == before
}

fn fresh_notice(result_sequence: u64, notice_sequence: u64, observed: bool) -> bool {
    notice_sequence > result_sequence && !observed
}

fn achievement_congratulations(event: &SessionEvent) -> Option<(u64, &str)> {
    let SessionEvent::TutorialNotice(delta) = event else { return None };
    let oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(text) = &delta.value else {
        return None;
    };
    text.starts_with("Congratulations! You earned the achievement ")
        .then_some((delta.server_sequence, text))
}

pub(super) fn check_unexpected_notice(event: &SessionEvent) -> TestResult {
    if matches!(event, SessionEvent::AchievementEarned(_)) || achievement_congratulations(event).is_some() {
        return Err("P11 emitted an unexpected earned notice".into());
    }
    Ok(())
}

fn reject_earned_notices(client: &mut DevClientSession) -> TestResult {
    for event in client.take_events() {
        check_unexpected_notice(&event)?;
    }
    Ok(())
}

async fn wait_for_inventory(
    client: &mut DevClientSession,
    step: &str,
    committed: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    reject_earned_notices(client)?;
    while !committed(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step={step} accepted-command-inventory-delta-missing").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        reject_earned_notices(client)?;
    }
    Ok(())
}

async fn bootstrap_sibling(clients: &SeamClients<'_>) -> TestResult<[u8; 16]> {
    const NAME: &str = "Annihilator Sibling";
    const OPERATION: &str = "01934f10-7c04-7b11-805b-3b1122334b11";
    let work = std::path::PathBuf::from(required("NODE_BOOT_PLATFORM_GRANT_DIR")?);
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let composition = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/game_platform/composition.py");
    let account = clients.accounts[1].clone();
    let world = uuid_text(&clients.world);
    let deploy = work.join("deploy");
    let intent = tokio::task::spawn_blocking(move || {
        std::process::Command::new("python3")
            .arg(composition).arg("bootstrap").arg(work)
            .args([&account, &world, OPERATION,
                "nb-profile-1", "nb-ruleset-1", "s3b-content-1", "newhaven-starter-r1", NAME, "Knight"])
            .output()
    }).await??;
    if !intent.status.success() {
        return Err(format!("P11 sibling fixture Platform intent failed: {}", intent.status).into());
    }
    let birth = tokio::task::spawn_blocking(move || {
        std::process::Command::new("timeout")
            .arg("30")
            .arg(deploy.join("bin/oteryn-game-ops"))
            .arg("--config").arg(deploy.join("ops/ops.toml"))
            .args(["character", "bootstrap", "--socket", &socket, "--operation-id", OPERATION])
            .output()
    }).await??;
    if !birth.status.success() || !String::from_utf8_lossy(&birth.stdout).contains("character_bootstrap=committed") {
        return Err(format!("P11 sibling fixture Game bootstrap failed: {}", birth.status).into());
    }
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let id: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_character_roots
         WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid AND name=$3",
    )
    .bind(&clients.accounts[1]).bind(clients.world.as_slice()).bind(NAME)
    .fetch_one(&mut db).await?;
    db.close().await?;
    let sibling = id.as_slice().try_into()?;
    if sibling == clients.characters[1] {
        return Err("P11 character-scope fixture must have two distinct characters on one account".into());
    }
    evidence(&format!("FIXTURE_P11 sibling_character={} same_account=true", uuid_text(&sibling)));
    Ok(sibling)
}

pub(super) async fn journey(
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
    placed_at: &std::cell::Cell<Option<std::time::Instant>>,
) -> TestResult<String> {
    let sibling_characters = [bootstrap_sibling(clients).await?];
    let sibling_scope = SeamClients {
        accounts: &clients.accounts[1..2],
        characters: &sibling_characters,
        ..*clients
    };
    let malformed = client_row_p11::malformed_live(clients, 1).await?;
    let proof = earn_and_logout(clients, core, placed_at).await?;
    let mut other = connect_character(&sibling_scope, 0xb2, 0).await?;
    let sibling_page = page(&mut other, 0).await?;
    if sibling_page != AccountAchievementsResult::default() {
        return Err(format!("P11 achievement leaked to the other character on the same account: {sibling_page:?}").into());
    }
    reject_earned_notices(&mut other)?;
    logout(&mut other).await?;
    reject_earned_notices(&mut other)?;
    evidence("P11_CHECK sibling_character=same_account page=empty points=0");
    client_row_p11::capture_relog(clients, proof, malformed).await
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("P11 mechanic logout refused: {detail}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn p11_flags_live_target_clear_after_the_acknowledgment() {
        use super::corridor_target_lost;
        use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
        let target = EntityRef { identity: [7; 16], generation: 1 };
        assert!(corridor_target_lost(10, Some((11, None)), true));
        assert!(!corridor_target_lost(10, Some((11, None)), false));
        assert!(!corridor_target_lost(10, Some((10, None)), true));
        assert!(!corridor_target_lost(10, Some((9, None)), true));
        assert!(!corridor_target_lost(10, Some((11, Some(target))), true));
        assert!(!corridor_target_lost(10, None, true));
    }

    #[test]
    fn p11_rejects_unrelated_or_ambiguous_corpse_entries() -> super::TestResult {
        use super::*;
        use oteryn_protocol_oteryn::world_spatial_entities::{EntityDetail, EntityRef, WorldSpatialEntity};
        let position = ActorPosition { x: 33223, y: 31659, floor: 13 };
        let definition = std::num::NonZeroU32::new(7).ok_or("corpse definition")?;
        let handle = ItemHandle::new(8).ok_or("corpse handle")?;
        let corpse = WorldSpatialEntity {
            motion: None,
            display_name: None,
            kind: EntityKind::Corpse,
            entity: EntityRef { identity: [1; 16], generation: 1 },
            position,
            detail: EntityDetail::Object { item_definition_ref: definition.get(), quantity: 1, item_handle: Some(handle) },
        };
        let other = WorldSpatialEntity {
            entity: EntityRef { identity: [2; 16], generation: 1 }, ..corpse
        };
        assert_eq!(unique_corridor_corpse(&[corpse], position, definition, corpse.entity.identity, &[])?, handle);
        assert!(unique_corridor_corpse(&[], position, definition, corpse.entity.identity, &[]).is_err());
        assert!(unique_corridor_corpse(&[corpse], position, definition, corpse.entity.identity, &[corpse.entity]).is_err());
        assert!(unique_corridor_corpse(&[other], position, definition, corpse.entity.identity, &[]).is_err());
        assert!(unique_corridor_corpse(&[corpse, other], position, definition, corpse.entity.identity, &[]).is_err());
        Ok(())
    }

    #[test]
    fn p11_rejects_extra_inventory_and_relog_congratulations() -> super::TestResult {
        use super::*;
        use oteryn_protocol_oteryn::item_view::CharacterInventory;
        use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
        let reward = std::num::NonZeroU32::new(7).ok_or("reward")?;
        let count = std::num::NonZeroU32::new(1).ok_or("count")?;
        let handle = ItemHandle::new(1).ok_or("handle")?;
        let before = CharacterInventory::default();
        let armor = ItemEntry { appearance_id: 0, handle, item_definition_ref: reward, count, sub_type: 0 };
        let mut after = CharacterInventory { entries: vec![armor], ..before.clone() };
        assert!(only_reward_added(&before, &after, reward));
        after.entries.push(ItemEntry { item_definition_ref: count, ..armor });
        assert!(!only_reward_added(&before, &after, reward));
        let event = SessionEvent::TutorialNotice(oteryn_dev_client::AppliedDelta {
            server_sequence: 11,
            base_revision: 0,
            new_revision: 1,
            value: TutorialNotice::StatusText(CONGRATULATIONS.into()),
        });
        assert_eq!(achievement_congratulations(&event), Some((11, CONGRATULATIONS)));
        assert!(check_unexpected_notice(&event).is_err());
        assert!(fresh_notice(10, 11, false));
        assert!(!fresh_notice(11, 11, false));
        assert!(!fresh_notice(12, 11, false));
        assert!(!fresh_notice(10, 11, true));
        Ok(())
    }

    #[test]
    fn p11_uses_only_the_authored_annihilator_reward_position() {
        let source = include_str!("full_row_p11.rs");
        let Some((_, granter)) = source.split_once("async fn authored_granter(") else {
            panic!("missing P11 granter");
        };
        let Some((granter, _)) = granter.split_once("\n}") else {
            panic!("missing P11 granter body");
        };
        assert!(granter.contains("(REWARD_CHEST.x, REWARD_CHEST.y, REWARD_CHEST.floor)"));
        assert!(granter.contains("fixed_chest(REWARD_CHEST.x)"));
        assert!(granter.contains("fixed_chest(33229)"));
        assert!(
            granter.contains("placement.placement.achievement.as_deref() == Some(ACHIEVEMENT)")
        );
        assert!(granter.contains("claims.next().is_some()"));
        assert!(granter.contains("corridor_step(client, StepDirection::North, interaction.x, y)"));
    }
    #[test]
    fn p11_serializes_fixture_logouts_before_party_staging() -> super::TestResult {
        let source = include_str!("full_row_p11.rs");
        let (_, staging) = source
            .split_once("async fn operate_lever(")
            .ok_or("lever staging helper")?;
        let (staging, _) = staging
            .split_once("async fn authored_granter(")
            .ok_or("granter helper")?;
        let (_, earn) = source
            .split_once("pub(super) async fn earn_and_logout(")
            .ok_or("earn helper")?;
        let (earn, _) = earn
            .split_once("async fn journey(")
            .ok_or("journey helper")?;
        assert!(!staging.contains("prepare_knight("));
        assert!(staging.contains("service_liveness("));
        let fixture = earn.find("prepare_knight(").ok_or("fixture preparation")?;
        let first_admission = earn
            .find("let mut client = connect_knight(")
            .ok_or("first live admission")?;
        let placement = earn.find("place_member(").ok_or("fixture placement")?;
        assert!(fixture < placement && placement < first_admission);
        assert!(!staging.contains("stage_member("));
        assert!(staging.contains("assert_staging("));
        Ok(())
    }

    #[test]
    fn p11_grant_proof_rejects_each_audit_false_positive() {
        use super::*;
        let entry = wire::AccountAchievementRow {
            key: ACHIEVEMENT.into(),
            name: "Annihilator".into(),
            description: DESCRIPTION.into(),
            grade: 2,
            points: 5,
            earned_at: 150,
            secret: false,
        };
        let correct = AccountAchievementsResult {
            rows: vec![entry],
            total_points: 5,
            fact_count: 1,
            ..Default::default()
        };
        assert!(expected_grant(&correct, 140, 160));
        assert!(!expected_grant(
            &AccountAchievementsResult::default(),
            140,
            160
        ));
        let mutate: [fn(&mut AccountAchievementsResult); 10] = [
            |page| page.total_points += 1,
            |page| page.fact_count += 1,
            |page| page.page += 1,
            |page| page.has_more = true,
            |page| page.rows[0].key.push('x'),
            |page| page.rows[0].name.push('x'),
            |page| page.rows[0].description.push('x'),
            |page| page.rows[0].grade += 1,
            |page| page.rows[0].points += 1,
            |page| page.rows[0].secret = true,
        ];
        for wrong in mutate {
            let mut page = correct.clone();
            wrong(&mut page);
            assert!(!expected_grant(&page, 140, 160));
        }
        assert!(!expected_grant(&correct, 151, 160));
        assert!(!expected_grant(&correct, 140, 149));
    }

    #[test]
    fn evidence_sanitizer_produces_one_report_token() {
        assert_eq!(
            super::sanitize("step failed\n(reply)"),
            "step_failed__reply_"
        );
    }
}

#[cfg(test)]
#[test]
fn row_11_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p11.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P11_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_p11_guards_the_journey() {
    assert!(
        include_str!("full_row_p11.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("duplicate_reply"))
    );
}
