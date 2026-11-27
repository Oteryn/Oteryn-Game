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
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::account_achievements as wire;

// The live sword-350 run took 36.5 seconds to remove the first 35% of a demon's
// health. Allow 150 seconds per target for damage variation and server ticks, then
// 120 seconds for party setup, the combat lock, the reward and relog.
const TARGET_DEADLINE: Duration = Duration::from_secs(150);
pub(super) const JOURNEY_DEADLINE: Duration = Duration::from_secs(420);

pub(super) async fn run_packet(clients: &SeamClients<'_>, core: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let placed_at = std::cell::Cell::new(None);
    let result = verdict::indexed_timeout(
        JOURNEY_DEADLINE,
        verdict::observe(journey(clients, core, &placed_at)),
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
            evidence(&format!("P11_LEVER_FRAME member_x={} {event:?}", staged.x));
            let (sequence, position) = match event {
                SessionEvent::WorldSpatial(delta) => {
                    (delta.server_sequence, delta.value.actor_position)
                }
                SessionEvent::WorldSpatialEntities(delta) => {
                    (delta.server_sequence, delta.value.actor_position)
                }
                _ => continue,
            };
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
    Some(
        client
            .inventory()?
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
    full_row_p05::wait_for_inventory(client, "P11 weapon equip", equipped).await
}

async fn corridor_step(
    client: &mut DevClientSession,
    direction: StepDirection,
    x: u16,
    y: u16,
) -> TestResult {
    let expected = TilePos { x, y, floor: -13 };
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
            client.take_events();
        }
    }
    Ok(())
}

async fn clear_corridor(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::{EntityDetail, EntityKind};
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let mut targets = Vec::new();
    let entities = client
        .world_entities()
        .ok_or("P11 corridor entity snapshot absent")?;
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
    let potion = provisioning::item_reference(&provisioning::item_index()?, POTION_ID)?;
    for target in targets {
        let existing_corpses: Vec<_> = client
            .world_entities()
            .ok_or("P11 corridor entities absent")?
            .iter()
            .filter(|row| row.kind == EntityKind::Corpse)
            .map(|row| row.entity)
            .collect();
        client.take_events();
        evidence(&format!(
            "P11_COMBAT attack_start session={} command={} after_sequence={} target={:?}",
            uuid_text(client.join_snapshot().game_session_id.as_bytes()),
            client.next_command_id(),
            client.last_server_sequence(),
            target.entity
        ));
        let reply = client
            .session_mut()
            .attack_target(Some(&target.entity))
            .await?;
        if reply.status != oteryn_protocol_oteryn::CommandStatus::Accepted
            || reply.disposition != AttackIntentDisposition::Ok
        {
            return Err(format!(
                "P11 first corridor attack failed target={:?}: {reply:?}",
                target.entity
            )
            .into());
        }
        let deadline = tokio::time::Instant::now() + TARGET_DEADLINE;
        let mut damaged = false;
        let mut last_health = 100;
        let mut lowest_health = last_health;
        let attack_started = std::time::Instant::now();
        let mut last = target.position;
        loop {
            client.service_liveness(Duration::from_millis(100)).await?;
            for event in client.take_events() {
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
                }
            }
            let visible = client
                .world_entities()
                .ok_or("P11 corridor entity state disappeared")?;
            if damaged
                && visible.get(&target.entity).is_none()
                && visible.iter().any(|row| {
                    row.kind == EntityKind::Corpse
                        && row.position == last
                        && !existing_corpses.contains(&row.entity)
                })
            {
                evidence(&format!(
                    "P11_COMBAT killed={:?} corpse_at={last:?}",
                    target.entity
                ));
                break;
            }
            let vitals = client.actor_vitals().ok_or("P11 corridor vitals missing")?;
            if vitals.health == 0 || tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "P11 corridor kill failed target={:?} damaged={damaged} remaining_percent={last_health} vitals={vitals:?}",
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
                full_row_p05::wait_for_inventory(client, "P11 potion", |client| {
                    bag_count(client, potion)
                        .is_some_and(|after| after.checked_add(1) == Some(before))
                        && client
                            .actor_vitals()
                            .is_some_and(|vitals| vitals.health > health)
                })
                .await?;
            }
        }
        corridor_step(
            client,
            StepDirection::East,
            u16::try_from(target.position.x)?,
            CORRIDOR_Y,
        )
        .await?;
    }
    Ok(())
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
    client.take_events();
    let [second, third, fourth] = companions.as_mut_slice() else {
        return Err("P11 requires exactly three lever companions".into());
    };
    let [second_sequence, third_sequence, fourth_sequence] =
        [&mut *second, &mut *third, &mut *fourth].map(|member| {
            member.take_events();
            member.last_server_sequence()
        });
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
            let result = clear_corridor(client).await;
            combat_finished.set(true);
            result
        },
        async {
            while !combat_finished.get() {
                for peer in &mut companions {
                    peer.service_liveness(Duration::from_millis(100)).await?;
                    peer.take_events();
                }
            }
            Ok::<(), oteryn_dev_client::DevClientError>(())
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
) -> TestResult<(Vec<u8>, String, &'static BundleWorld)> {
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
    let mut matches = report
        .served
        .into_iter()
        .flat_map(|claim| claim.placements)
        .filter(|placement| {
            placement.cell.x == REWARD_CHEST.x
                && placement.cell.y == REWARD_CHEST.y
                && placement.cell.floor == REWARD_CHEST.floor
                && placement.placement.achievement.as_deref() == Some(ACHIEVEMENT)
        });
    let placement = matches
        .next()
        .ok_or("step=granter missing fixed authored Annihilator chest at 33227,31656,13")?;
    let [item] = placement.placement.items.as_slice() else {
        return Err("P11 bound chest must award exactly one item".into());
    };
    if item.count != 1 || item.item.key().as_str() != "oteryn:item.tibia.i3388" {
        return Err("P11 bound chest does not award one Demon Armor".into());
    }
    if matches.next().is_some() {
        return Err("step=granter ambiguous authored Annihilator chest at 33227,31656,13".into());
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
    let mut key = world.base().digest().to_vec();
    key.extend_from_slice(&placement.bundle_placement_key.to_be_bytes());
    Ok((key, ACHIEVEMENT.to_owned(), world))
}

async fn leave_reward_room(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
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
    corridor_step(client, South, 33228, 31658).await?;
    for x in 33229..=33233 {
        corridor_step(client, East, x, 31658).await?;
    }
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
    let (result, detail, _, _) = fixture_logout_retrying_busy(&mut initial).await?;
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
}

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

    client.take_events();
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    if wire::decode_account_achievements_query(&[0x08]).is_ok()
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
    {
        return Err("step=local-codec-refusal observed=submitted-or-not-refused".into());
    }
    if client
        .take_events()
        .iter()
        .any(|event| matches!(event, SessionEvent::AchievementEarned(_)))
        || page(&mut client, 0).await? != before
    {
        return Err("step=local-codec-refusal observed=unexpected-achievement-delta".into());
    }
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
    let (placement, achievement, world) = authored_granter(&mut client, clients, core).await?;
    if achievement != ACHIEVEMENT {
        return Err(
            format!("P11 expected the authored Annihilator claim, got {achievement}").into(),
        );
    }
    let earliest = achievement_clock_ms()?;
    client.take_events();
    let use_reply = client.session_mut().use_object(&placement, 0).await?;
    if use_reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!(
            "step=granter achievement={achievement} authored chest declined: {use_reply:?}"
        )
        .into());
    }
    // The Server may defer notice delivery to its next liveness refresh.
    let notice_deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let earned = loop {
        if let Some(earned) = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::AchievementEarned(delta) => Some(delta.value),
                _ => None,
            })
        {
            break earned;
        }
        let remaining = notice_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "step=granter committed chest did not send AchievementEarned: {use_reply:?}"
            )
            .into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(250)))
            .await?;
    };
    full_row_p05::wait_for_inventory(&mut client, "P11 chest reward", |client| {
        bag_count(client, reward) == Some(1)
    })
    .await?;
    let granted = page(&mut client, 0).await?;
    let latest = achievement_clock_ms()?;
    if earned.key != ACHIEVEMENT
        || earned.name != "Annihilator"
        || earned.watermark.fact_count != 1
        || earned.watermark.total_points != 5
        || !expected_grant(&granted, earliest, latest)
    {
        return Err(format!("step=granter delta/query mismatch: {earned:?} {granted:?}").into());
    }

    client.take_events();
    let duplicate_reply = client.session_mut().use_object(&placement, 0).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    if duplicate_reply.disposition
        != oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse
        || client
            .take_events()
            .iter()
            .any(|event| matches!(event, SessionEvent::AchievementEarned(_)))
        || page(&mut client, 0).await? != granted
        || bag_count(&client, reward) != Some(1)
    {
        return Err(
            format!("step=duplicate observed_reply={duplicate_reply:?},duplicate-grant").into(),
        );
    }
    leave_reward_room(&mut client, world).await?;
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
        client.take_events();
    }
    logout(&mut client).await?;
    drop(client);
    Ok(GrantProof {
        page: granted,
        session,
        reward,
    })
}

async fn journey(
    clients: &SeamClients<'_>,
    core: &SeamClients<'_>,
    placed_at: &std::cell::Cell<Option<std::time::Instant>>,
) -> TestResult {
    client_row_p11::malformed_live(clients, 1).await?;
    let proof = earn_and_logout(clients, core, placed_at).await?;
    let mut fresh = connect_world(clients, 0xc2).await?;
    if fresh.join_snapshot().game_session_id.as_bytes() == &proof.session
        || page(&mut fresh, 0).await? != proof.page
        || bag_count(&fresh, proof.reward) != Some(1)
    {
        return Err("step=relogin durable achievement page changed".into());
    }
    logout(&mut fresh).await?;
    Ok(())
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
    fn p11_uses_only_the_authored_annihilator_reward_position() {
        let source = include_str!("full_row_p11.rs");
        let Some((_, granter)) = source.split_once("async fn authored_granter(") else {
            panic!("missing P11 granter");
        };
        let Some((granter, _)) = granter.split_once("\n}") else {
            panic!("missing P11 granter body");
        };
        assert!(granter.contains("placement.cell.x == REWARD_CHEST.x"));
        assert!(granter.contains("placement.cell.y == REWARD_CHEST.y"));
        assert!(granter.contains("placement.cell.floor == REWARD_CHEST.floor"));
        assert!(
            granter.contains("placement.placement.achievement.as_deref() == Some(ACHIEVEMENT)")
        );
        assert!(granter.contains("matches.next().is_some()"));
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
