//! Live Wheel persistence and Battle Instinct threshold for the native client replay.

use super::full_row_c11::{admitted_vocation, approach_npc, talk, wheel_command};
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{battle_instinct_status as wire, logout::LogoutResult, wheel};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Write;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(capture(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C11.client status=FAIL evidence={error}"
        ));
    }
}

async fn query(
    client: &mut DevClientSession,
    expected: wire::BattleInstinctStatus,
) -> TestResult<(u64, u64, u32, Vec<u8>)> {
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
            wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Battle Instinct query refused: {reply:?}").into());
    }
    require_status(&reply.payload, expected)?;
    Ok((
        reply.command_id,
        reply.server_sequence,
        reply.status as u32,
        reply.payload,
    ))
}

fn require_status(payload: &[u8], expected: wire::BattleInstinctStatus) -> TestResult {
    let observed =
        wire::BattleInstinctStatus::decode(payload).ok_or("invalid Battle Instinct payload")?;
    if observed != expected {
        return Err(format!("Battle Instinct observed {observed:?}, expected {expected:?}").into());
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn c11_threshold_rejects_always_inactive_and_zero_bonus() -> TestResult {
    let inactive = wire::BattleInstinctStatus::default();
    let eligible = wire::BattleInstinctStatus {
        active: true,
        melee: 0,
        shielding: 0,
    };
    let active = wire::BattleInstinctStatus {
        active: true,
        melee: 1,
        shielding: 6,
    };
    let encode =
        |status: wire::BattleInstinctStatus| status.encode().ok_or("invalid C11 test status");
    assert!(require_status(&encode(inactive)?, eligible).is_err());
    assert!(require_status(&encode(eligible)?, active).is_err());
    assert!(require_status(&encode(active)?, eligible).is_err());
    require_status(&encode(eligible)?, eligible)?;
    require_status(&encode(active)?, active)
}

async fn relog(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    placement: (&str, TilePos),
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let old = client.join_snapshot().game_session_id;
    let outcome = client.session_mut().logout().await?;
    evidence(&format!("C11_KNIGHT_LOGOUT attempt=1 reply={outcome:?}"));
    if outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("C11 Knight first logout refused: {outcome:?}").into());
    }
    wait_character_released(clients, 1).await?;
    let (label, target) = placement;
    stage_position(clients, from, target).await?;
    evidence(&format!(
        "C11_FIXTURE placement={label} from={from:?} to={target:?}"
    ));
    *client = connect_character(clients, 0xcd, 1).await?;
    let observed = tile(client.world_spatial().actor_position)?;
    if observed != target {
        return Err(format!(
            "C11 {label} fixture admission expected {target:?}, observed {observed:?}"
        )
        .into());
    }
    if client.join_snapshot().game_session_id == old {
        return Err("C11 Knight relog reused the old session".into());
    }
    Ok(())
}

// Operates only on the isolated node-boot database after terminal release. Keep all
// digests, vitals and lease fields from the real logout; advance its write order.
async fn stage_position(clients: &SeamClients<'_>, from: TilePos, to: TilePos) -> TestResult {
    use sqlx::Connection as _;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let result = sqlx::query(
        "UPDATE game_character_last_positions AS p
         SET tile_x=$6, tile_y=$7,
             tile_floor=CASE WHEN p.tile_floor=$5::smallint THEN $8::smallint ELSE -($8::smallint) END,
             write_sequence=p.write_sequence+1
         WHERE p.world_id=encode($1,'hex')::uuid
           AND p.character_id=encode($2,'hex')::uuid
           AND p.tile_x=$3 AND p.tile_y=$4 AND p.tile_floor IN ($5::smallint,-($5::smallint))
           AND p.house_key IS NULL
           AND NOT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s
                           WHERE s.character_id=p.character_id AND s.session_state<>3)",
    )
    .bind(clients.world.as_slice())
    .bind(clients.characters[1].as_slice())
    .bind(i32::from(from.x))
    .bind(i32::from(from.y))
    .bind(i16::from(from.floor))
    .bind(i32::from(to.x))
    .bind(i32::from(to.y))
    .bind(i16::from(to.floor))
    .execute(&mut db)
    .await?;
    db.close().await?;
    if result.rows_affected() != 1 {
        return Err(format!(
            "C11 placement expected one released saved position at {from:?}: matched {}",
            result.rows_affected()
        )
        .into());
    }
    Ok(())
}

fn combat_staging_tile(world: &BundleWorld) -> TestResult<TilePos> {
    let usable = |pos: TilePos| {
        world.enterable(pos)
            && world
                .base()
                .tile(pos.x, pos.y, pos.floor)
                .is_some_and(|tile| !tile.protection_zone() && !tile.no_logout())
    };
    world
        .base()
        .tiles()
        .filter(|(floor, x, y, _)| {
            *floor == TOWN.floor && x.abs_diff(TOWN.x).max(y.abs_diff(TOWN.y)) <= 40
        })
        .map(|(floor, x, y, _)| TilePos { x, y, floor })
        .filter(|pos| {
            usable(*pos)
                && (-1..=1).all(|dx| {
                    (-1..=1).all(|dy| {
                        pos.x
                            .checked_add_signed(dx)
                            .zip(pos.y.checked_add_signed(dy))
                            .is_some_and(|(x, y)| {
                                usable(TilePos {
                                    x,
                                    y,
                                    floor: pos.floor,
                                })
                            })
                    })
                })
        })
        .min_by_key(|pos| {
            (
                pos.x.abs_diff(TOWN.x).max(pos.y.abs_diff(TOWN.y)),
                pos.y,
                pos.x,
            )
        })
        .ok_or_else(|| "C11 no authored non-PZ combat staging tile near Thais".into())
}

fn adjacent_creatures(
    client: &DevClientSession,
) -> TestResult<BTreeSet<oteryn_dev_client::EntityRef>> {
    let here = client.world_spatial().actor_position;
    let entities = client
        .world_entities()
        .ok_or("C11 entity view unavailable")?;
    Ok(entities
        .iter()
        .filter(|entry| {
            entry.kind == oteryn_dev_client::EntityKind::Creature
                && entry.position.floor == here.floor
                && entry
                    .position
                    .x
                    .abs_diff(here.x)
                    .max(entry.position.y.abs_diff(here.y))
                    == 1
        })
        .map(|entry| entry.entity)
        .collect())
}

async fn spawn_adjacent(
    client: &mut DevClientSession,
    expected: &mut BTreeSet<oteryn_dev_client::EntityRef>,
) -> TestResult {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let reply = request_spawn(session, "oteryn:creature.rat".into()).await?;
    evidence(&format!("C11_FIXTURE adjacent_rat reply={reply}"));
    let actor = parse_spawned_entity(&reply)?;
    if !expected.insert(actor) {
        return Err("C11 spawn reused a previous actor".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while adjacent_creatures(client)? != *expected {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C11 expected {expected:?}, observed {:?}",
                adjacent_creatures(client)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    Ok(())
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let dir =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C11 fresh capture directory missing")?;
    let run = required("C11_CAPTURE_RUN")?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_C11_KNIGHT_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    character_prep::declare_seeded(
        "C11.client",
        "knight-level-625,premium,promotion-gold,adjacent-rats",
        "Wheel prerequisite inputs",
    );
    let mut client = connect_character(clients, 0xcd, 1).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "c11-knight".into(),
    )
    .await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    character_prep::grant_premium(&client).await?;
    // Authored King: (32311,32172,z=6). Stage on the adjacent throne-room floor.
    let throne = TilePos {
        x: 32311,
        y: 32173,
        floor: -6,
    };
    if !world.enterable(throne)
        || !world
            .base()
            .tile(throne.x, throne.y, throne.floor)
            .is_some_and(|tile| !tile.no_logout())
    {
        return Err(format!(
            "C11 throne-room fixture tile is not enterable or logout-safe: {throne:?}"
        )
        .into());
    }
    relog(&mut client, clients, ("throne-room", throne)).await?;
    let (vocation, level) = admitted_vocation(&mut client).await?;
    if Some(vocation) != oteryn_protocol_oteryn::cyclopedia_character::vocation_id("knight")
        || level != 625
    {
        return Err(
            format!("C11 preparation expected level-625 Knight: {vocation}/{level}").into(),
        );
    }
    let king = approach_npc(
        &mut client,
        world,
        &metadata.npcs,
        "oteryn:npc.king_tibianus",
    )
    .await?;
    if talk(&mut client, king, "Hail King").await?.is_empty()
        || talk(&mut client, king, "promotion").await?
            != ["Do you want to be promoted for 20000 gold coins?"]
        || talk(&mut client, king, "yes").await? != ["You are now promoted."]
    {
        return Err("C11 King Tibianus promotion dialogue did not confirm purchase".into());
    }

    query(&mut client, wire::BattleInstinctStatus::default()).await?;
    let read = wheel::WheelAllocationRequest {
        action: 0,
        occurrence: [0; 16],
        expected_revision: 0,
        points: [0; 36],
        vessels: None,
    };
    let before = wheel_command(&mut client, &read).await?;
    if before.refused || before.points != [0; 36] {
        return Err(format!("C11 expected clean eligible Wheel: {before:?}").into());
    }
    let mut points = [0; 36];
    for (slot, capacity) in [(15, 50), (9, 75), (8, 100), (7, 150), (1, 200)] {
        points[slot - 1] = capacity;
    }
    let saved = wheel_command(
        &mut client,
        &wheel::WheelAllocationRequest {
            action: 1,
            occurrence: v7(0xcd, 1),
            expected_revision: before.revision,
            points,
            vessels: None,
        },
    )
    .await?;
    if saved.refused
        || saved.points != points
        || Some(saved.revision) != before.revision.checked_add(1)
        || wheel_command(&mut client, &read).await? != saved
    {
        return Err(format!("C11 exact Wheel allocation failed: {saved:?}").into());
    }
    let eligible = wire::BattleInstinctStatus {
        active: true,
        melee: 0,
        shielding: 0,
    };
    query(&mut client, eligible).await?;
    let combat = combat_staging_tile(world)?;
    relog(&mut client, clients, ("combat", combat)).await?;
    if wheel_command(&mut client, &read).await? != saved {
        return Err("C11 exact Wheel allocation failed to survive relog".into());
    }
    query(&mut client, eligible).await?;
    let mut creatures = adjacent_creatures(&client)?;
    if !creatures.is_empty() {
        return Err(
            format!("C11 combat fixture begins with adjacent creatures: {creatures:?}").into(),
        );
    }
    for _ in 0..4 {
        spawn_adjacent(&mut client, &mut creatures).await?;
    }
    let four = query(&mut client, eligible).await?;
    if adjacent_creatures(&client)? != creatures {
        return Err("C11 exact four actors did not remain adjacent during the query".into());
    }
    spawn_adjacent(&mut client, &mut creatures).await?;
    let five = query(
        &mut client,
        wire::BattleInstinctStatus {
            active: true,
            melee: 1,
            shielding: 6,
        },
    )
    .await?;
    if adjacent_creatures(&client)? != creatures {
        return Err("C11 exact five actors did not remain adjacent during the query".into());
    }
    let rejected = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
            wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
            &[0],
        )
        .await?;
    if rejected.status != CommandStatus::Rejected || !rejected.payload.is_empty() {
        return Err(
            format!("C11 malformed query did not return empty Rejected: {rejected:?}").into(),
        );
    }
    query(
        &mut client,
        wire::BattleInstinctStatus {
            active: true,
            melee: 1,
            shielding: 6,
        },
    )
    .await?;

    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let capture = serde_json::to_vec(&serde_json::json!({
        "run": run,
        "session": session,
        "four": [four.0, four.1],
        "four_status": four.2,
        "four_payload": four.3,
        "five": [five.0, five.1],
        "five_status": five.2,
        "five_payload": five.3,
        "rejected": [rejected.command_id, rejected.server_sequence],
        "rejected_status": rejected.status as u32,
        "rejected_payload": rejected.payload,
    }))?;
    let digest = Sha256::digest(&capture)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    std::fs::File::create_new(Path::new(&dir).join("c11-capture.json"))?.write_all(&capture)?;
    evidence(&format!(
        "C11_CAPTURED run={run} session={session} sequences={},{},{} digest={digest}",
        four.1, five.1, rejected.server_sequence
    ));
    Ok(())
}
