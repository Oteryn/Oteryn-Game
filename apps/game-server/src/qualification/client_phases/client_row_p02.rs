//! P02 live request/reply capture for the production Bosstiary panel replay.
use super::*;
use full_row_p02::{ORDINARY_BOSS, kill, logout, prepare_combat};
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{boss_encounter, raw_feature::FeatureCommand};
use oteryn_protocol_oteryn::bosstiary_slots::{
    BossSlotReply, BossSlotRequest, COMMAND_TYPE_BOSSTIARY_SLOTS,
};

async fn exchange(
    client: &mut DevClientSession,
    request: FeatureCommand,
    capture: Option<&str>,
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(request.capability, request.kind, &request.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("P02 command {} status={:?}", request.kind, reply.status).into());
    }
    if let Some(name) = capture {
        let directory = required("OTERYN_P02_CAPTURE_DIR")?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(client.join_snapshot().game_session_id.as_bytes());
        bytes.extend_from_slice(&reply.command_id.to_be_bytes());
        bytes.extend_from_slice(&reply.server_sequence.to_be_bytes());
        bytes.extend_from_slice(&(reply.status as u32).to_be_bytes());
        bytes.extend_from_slice(&request.capability.to_be_bytes());
        bytes.extend_from_slice(&request.kind.to_be_bytes());
        bytes.extend_from_slice(&u32::try_from(request.payload.len())?.to_be_bytes());
        bytes.extend_from_slice(&request.payload);
        bytes.extend_from_slice(&reply.payload);
        std::fs::write(Path::new(&directory).join(name), bytes)?;
    }
    Ok(reply.payload)
}

async fn progress(
    client: &mut DevClientSession,
    capture: Option<&str>,
) -> TestResult<boss_encounter::Snapshot> {
    let request = boss_encounter::query_taints("").ok_or("P02 cooldown encoder")?;
    boss_encounter::decode(&exchange(client, request, capture).await?)
        .ok_or_else(|| "P02 malformed progress reply".into())
}

async fn slots(
    client: &mut DevClientSession,
    request: BossSlotRequest,
    capture: Option<&str>,
) -> TestResult<BossSlotReply> {
    let command = FeatureCommand {
        capability: boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
        kind: COMMAND_TYPE_BOSSTIARY_SLOTS,
        payload: request.encode().ok_or("P02 slot encoder")?,
    };
    BossSlotReply::decode(&exchange(client, command, capture).await?)
        .ok_or_else(|| "P02 malformed slot reply".into())
}

fn earned(snapshot: &boss_encounter::Snapshot) -> bool {
    snapshot.progress == [(ORDINARY_BOSS.to_owned(), 1)]
        && snapshot.progress_next.is_none()
        && snapshot.points == 10
}

fn unlocked_empty(reply: &BossSlotReply) -> bool {
    reply.status == 0 && reply.unlocked == 1 && reply.points == 10 && reply.bonus == 26
        && reply.ready_at_ms == 0 && reply.bosses == ["", ""]
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = prepare_combat(clients, "P02.client", 0x22).await?;
    let before = progress(&mut client, None).await?;
    let locked = slots(&mut client, BossSlotRequest::Query, None).await?;
    if !before.progress.is_empty() || before.points != 0 || locked.status != 0
        || locked.unlocked != 0 || locked.bosses != ["", ""]
    {
        return Err(format!("P02 expected fresh locked pre-state: {before:?} {locked:?}").into());
    }

    kill(&mut client, ORDINARY_BOSS).await?;
    let credit = progress(&mut client, Some("p02-cooldowns.bin")).await?;
    if !earned(&credit) || credit.taints != before.taints {
        return Err(format!("P02 expected one Rotworm Queen kill and 10 points: {credit:?}").into());
    }
    let empty = slots(&mut client, BossSlotRequest::Query, Some("p02-slots.bin")).await?;
    if !unlocked_empty(&empty) {
        return Err(format!("P02 expected eligible, empty and ready slot one: {empty:?}").into());
    }
    let refused = slots(&mut client, BossSlotRequest::Equip {
        slot: 1, boss: "oteryn:creature.rat".into(),
    }, Some("p02-refused.bin")).await?;
    if refused != (BossSlotReply { status: 1, ..empty.clone() })
        || slots(&mut client, BossSlotRequest::Query, None).await? != empty
    {
        return Err(format!("P02 ineligible rat changed the unlocked slot: {empty:?} -> {refused:?}").into());
    }
    let equipped = slots(&mut client, BossSlotRequest::Equip {
        slot: 1, boss: ORDINARY_BOSS.into(),
    }, Some("p02-equipped.bin")).await?;
    if equipped.ready_at_ms <= credit.now || equipped != (BossSlotReply {
        bosses: [ORDINARY_BOSS.into(), String::new()],
        ready_at_ms: equipped.ready_at_ms,
        ..empty
    }) {
        return Err(format!("P02 eligible Rotworm Queen assignment did not take effect: {equipped:?}").into());
    }

    let session = logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0x23).await?;
    let persisted_credit = progress(&mut fresh, None).await?;
    let persisted_slot = slots(&mut fresh, BossSlotRequest::Query, None).await?;
    if fresh.join_snapshot().game_session_id == session || !earned(&persisted_credit)
        || persisted_slot != equipped
    {
        return Err(format!("P02 relog lost kill, points or assignment: {persisted_credit:?} {persisted_slot:?}").into());
    }
    evidence("P02_CLIENT_CHECK kills=1 points=10 slot1=rotworm_queen unlocked=1 bonus=26 refusal=1 durable=confirmed");
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await {
        Ok(Ok(())) => evidence("e2e_all scenario=P02.client status=CAPTURED evidence=confirmed-progress-refusal-assignment-relog"),
        Ok(Err(error)) => evidence(&format!("e2e_all scenario=P02.client status=FAIL evidence={error}")),
        Err(_) => evidence("e2e_all scenario=P02.client status=FAIL evidence=timeout"),
    }
}

#[test]
fn p02_client_preconditions_reject_the_audited_false_positives() {
    let mut credit = boss_encounter::Snapshot {
        progress: vec![(ORDINARY_BOSS.into(), 1)],
        points: 10,
        ..Default::default()
    };
    assert!(earned(&credit));
    credit.points = 11;
    assert!(!earned(&credit));
    credit.points = 10;
    credit.progress.clear();
    assert!(!earned(&credit));

    let mut slot = BossSlotReply {
        details: None,
        status: 0,
        unlocked: 1,
        bonus: 26,
        points: 10,
        remove_fee: 0,
        ready_at_ms: 0,
        bosses: [String::new(), String::new()],
    };
    assert!(unlocked_empty(&slot));
    slot.unlocked = 0;
    assert!(!unlocked_empty(&slot));
    slot.unlocked = 1;
    slot.bonus = 25;
    assert!(!unlocked_empty(&slot));
}
