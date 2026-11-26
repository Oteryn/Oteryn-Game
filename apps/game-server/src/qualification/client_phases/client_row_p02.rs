//! P02 client replay capture: real BOSS_COOLDOWNS (Bosstiary progress and points) and
//! BOSSTIARY_SLOTS replies, including a refused Equip of a creature that is not a boss. The
//! consumer is `apply_result` in `crates/client-ui/src/features/0117-boss-encounter/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter;
use oteryn_protocol_oteryn::bosstiary_slots::{
    BossSlotReply, BossSlotRequest, COMMAND_TYPE_BOSSTIARY_SLOTS,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    for key in ["E2E_ALL_CAPTURE_DIR", "OTERYN_P02_CAPTURE_DIR"] {
        let Some(root) = std::env::var_os(key) else {
            continue;
        };
        std::fs::write(Path::new(&root).join(name), bytes)
            .map_err(|error| format!("P02 capture {name}: {error}"))?;
    }
    Ok(())
}

fn capture_reply(
    client: &DevClientSession,
    name: &str,
    reply: &oteryn_dev_client::native_session::GatedCommandResult,
) -> TestResult {
    let mut bytes = Vec::with_capacity(36 + reply.payload.len());
    bytes.extend_from_slice(client.join_snapshot().game_session_id.as_bytes());
    bytes.extend_from_slice(&reply.command_id.to_be_bytes());
    bytes.extend_from_slice(&reply.server_sequence.to_be_bytes());
    bytes.extend_from_slice(&(reply.status as u32).to_be_bytes());
    bytes.extend_from_slice(&reply.payload);
    capture(name, &bytes)
}

async fn slots(
    client: &mut DevClientSession,
    request: BossSlotRequest,
    capture_name: Option<&str>,
) -> TestResult<BossSlotReply> {
    let payload = request.encode().ok_or("slot request encoder")?;
    let reply = client
        .session_mut()
        .gated_command(
            boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            COMMAND_TYPE_BOSSTIARY_SLOTS,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("slots {request:?} status={:?}", reply.status).into());
    }
    if let Some(name) = capture_name {
        capture_reply(client, name, &reply)?;
    }
    BossSlotReply::decode(&reply.payload).ok_or_else(|| "malformed slot reply".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x22).await?;
    let query = boss_encounter::query_taints("").ok_or("cooldown encoder")?;
    let reply = client
        .session_mut()
        .gated_command(query.capability, query.kind, &query.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("cooldown query status={:?}", reply.status).into());
    }
    capture_reply(&client, "p02-cooldowns.bin", &reply)?;
    let snapshot = boss_encounter::decode(&reply.payload).ok_or("malformed cooldown snapshot")?;
    let slot_state = slots(&mut client, BossSlotRequest::Query, Some("p02-slots.bin")).await?;
    let refused = slots(
        &mut client,
        BossSlotRequest::Equip {
            slot: 1,
            boss: "oteryn:creature.rat".into(),
        },
        Some("p02-refused.bin"),
    )
    .await?;
    let expected_refusal = BossSlotReply {
        status: 1,
        ..slot_state.clone()
    };
    if slot_state.status != 0 || refused != expected_refusal {
        return Err(format!("expected unchanged slots with ineligible refusal: before={slot_state:?}, after={refused:?}").into());
    }

    // The read-only query and the refused Equip must leave the persisted slot state intact.
    let session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?.result;
    if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P02 logout: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0x23).await?;
    let persisted = slots(&mut fresh, BossSlotRequest::Query, None).await?;
    if fresh.join_snapshot().game_session_id == session || persisted != slot_state {
        return Err(
            format!("P02 relog slot mismatch: before={slot_state:?}, after={persisted:?}").into(),
        );
    }
    let progress: Vec<String> = snapshot
        .progress
        .iter()
        .map(|(key, kills)| format!("{key}={kills}"))
        .collect();
    let taints = snapshot.taints.map_or("none".to_owned(), |t| {
        format!(
            "{},{},{},{}",
            t.soul_war, t.rotten_blood, t.rotten_blood_flags, t.rotten_blood_final
        )
    });
    let expected = format!(
        "points={}\ntaints={taints}\nprogress={}\nslots_status={}\nslots_unlocked={}\nslots_bonus={}\nslots_points={}\nslots_fee={}\nslots_ready_at={}\nslot1={}\nslot2={}\nrefusal_status={}\n",
        snapshot.points,
        progress.join(","),
        slot_state.status,
        slot_state.unlocked,
        slot_state.bonus,
        slot_state.points,
        slot_state.remove_fee,
        slot_state.ready_at_ms,
        slot_state.bosses[0],
        slot_state.bosses[1],
        refused.status
    );
    let expected = format!("{expected}session={:02x?}\n", session.as_bytes());
    capture("p02-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P02.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P02.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P02.client status=FAIL evidence=timeout"),
    }
}
