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

async fn relog(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let old = client.join_snapshot().game_session_id;
    let outcome = client.session_mut().logout().await?;
    evidence(&format!("C11_KNIGHT_LOGOUT attempt=1 reply={outcome:?}"));
    if outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("C11 Knight first logout refused: {outcome:?}").into());
    }
    *client = connect_character(clients, 0xcd, 1).await?;
    if client.join_snapshot().game_session_id == old {
        return Err("C11 Knight relog reused the old session".into());
    }
    Ok(())
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
    relog(&mut client, clients).await?;
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
    relog(&mut client, clients).await?;
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
