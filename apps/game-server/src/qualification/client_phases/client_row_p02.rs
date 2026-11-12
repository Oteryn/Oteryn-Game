//! P02 client replay capture: real BOSS_COOLDOWNS (Bosstiary progress and points) and
//! BOSSTIARY_SLOTS replies, including a refused Equip of a creature that is not a boss. The
//! consumer is `apply_result` in `crates/client-ui/src/features/0117-boss-encounter/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter;
use oteryn_protocol_oteryn::bosstiary_slots::{
    BossSlotReply, BossSlotRequest, COMMAND_TYPE_BOSSTIARY_SLOTS,
};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("P02_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn slots(
    client: &mut DevClientSession,
    request: BossSlotRequest,
    name: &str,
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
    capture(name, &reply.payload);
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
    capture("p02-cooldowns.bin", &reply.payload);
    let snapshot = boss_encounter::decode(&reply.payload).ok_or("malformed cooldown snapshot")?;
    let slot_state = slots(&mut client, BossSlotRequest::Query, "p02-slots.bin").await?;
    let refused = slots(
        &mut client,
        BossSlotRequest::Equip {
            slot: 1,
            boss: "oteryn:creature.rat".into(),
        },
        "p02-refused.bin",
    )
    .await?;
    if refused.status == 0 || refused.bosses != slot_state.bosses {
        return Err(format!("non-boss Equip was not refused: {refused:?}").into());
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
    capture("p02-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P02.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P02.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P02.client status=FAIL evidence=timeout"),
    }
}
