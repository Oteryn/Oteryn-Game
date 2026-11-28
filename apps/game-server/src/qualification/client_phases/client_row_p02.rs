//! P02 live request/reply capture for the production Bosstiary panel replay.
use super::*;
use full_row_p02::{ORDINARY_BOSS, credited_death, logout, prepare_combat};
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

const BELOW_PROWESS: &str = "oteryn:creature.ferumbras";
const ASSIGN_COOLDOWN_MS: u64 = 490;

fn empty_slots(reply: &BossSlotReply, unlocked: u8, points: u32, bonus: u16) -> bool {
    reply.status == 0
        && reply.unlocked == unlocked
        && reply.points == points
        && reply.bonus == bonus
        && reply.remove_fee == 0
        && reply.ready_at_ms == 0
        && reply.bosses == ["", ""]
}

fn assignment_ready_at(before: u64, after: u64, ready: u64) -> bool {
    before != 0
        && after.checked_sub(before).is_some_and(|span| span <= 2_000)
        && ready
            .checked_sub(ASSIGN_COOLDOWN_MS)
            .is_some_and(|operation_time| (before..=after).contains(&operation_time))
}

async fn catalogue_entry(
    client: &mut DevClientSession,
    search: &str,
    key: &str,
    capture: &str,
) -> TestResult<boss_encounter::catalogue::Entry> {
    let mut query = boss_encounter::catalogue::Query {
        search: search.into(),
        ..Default::default()
    };
    for index in 0..16 {
        let command = FeatureCommand {
            capability: boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            kind: boss_encounter::COMMAND_TYPE_BOSS_COOLDOWNS,
            payload: query
                .encode()
                .ok_or("P02 catalogue query encoding failed")?,
        };
        let name = format!("{capture}-{index}.bin");
        let page =
            boss_encounter::catalogue::Page::decode(&exchange(client, command, Some(&name)).await?)
                .ok_or("P02 malformed catalogue page")?;
        if let Some(entry) = page.entries.into_iter().find(|entry| entry.key == key) {
            return Ok(entry);
        }
        let Some(next) = page.next.filter(|next| *next > query.after) else {
            break;
        };
        query.after = next;
    }
    Err(format!("P02 missing real boss {key} in bounded catalogue search").into())
}

async fn gold(client: &mut DevClientSession) -> TestResult<(u64, u64)> {
    let balance = oteryn_dev_client::client_runtime::npc_shop::balances(client.session_mut())
        .await?
        .ok_or("P02 gold balances missing")?;
    Ok((balance.carried_gold, balance.bank_gold))
}

async fn refuse_assignment(
    client: &mut DevClientSession,
    key: &str,
    empty: &BossSlotReply,
    capture: &str,
) -> TestResult {
    let refused = slots(
        client,
        BossSlotRequest::Equip {
            slot: 1,
            boss: key.into(),
        },
        Some(capture),
    )
    .await?;
    let mut expected = empty.clone();
    expected.details = None;
    expected.status = 1;
    if refused != expected {
        return Err(format!("P02 expected unchanged refusal for {key}: {refused:?}").into());
    }
    expected.status = 0;
    if slots(client, BossSlotRequest::Query, None).await? != expected {
        return Err(format!("P02 refusal changed the slot for {key}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = prepare_combat(clients, "P02.client", 0x22).await?;
    let before = progress(&mut client, None).await?;
    let locked = slots(&mut client, BossSlotRequest::Query, Some("p02-locked.bin")).await?;
    if !before.progress.is_empty()
        || before.progress_next.is_some()
        || before.points != 0
        || !empty_slots(&locked, 0, 0, 25)
        || locked.details.is_some()
    {
        return Err(format!("P02 expected fresh locked pre-state: {before:?} {locked:?}").into());
    }

    credited_death(&mut client, clients, ORDINARY_BOSS, 1).await?;
    let credit = progress(&mut client, Some("p02-cooldowns.bin")).await?;
    if !earned(&credit) || credit.taints != before.taints {
        return Err(
            format!("P02 expected one Rotworm Queen kill and 10 points: {credit:?}").into(),
        );
    }
    let empty = slots(
        &mut client,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
        Some("p02-slots.bin"),
    )
    .await?;
    let details = empty.details.as_ref().ok_or("P02 slot details missing")?;
    if !empty_slots(&empty, 1, 10, 26)
        || details.base_bonus != 26
        || details.bonuses != [0, 0]
        || details.boosted == BELOW_PROWESS
    {
        return Err(format!("P02 expected eligible, empty and ready slot one: {empty:?}").into());
    }

    refuse_assignment(
        &mut client,
        "oteryn:creature.rat",
        &empty,
        "p02-refused.bin",
    )
    .await?;
    let ineligible = catalogue_entry(
        &mut client,
        "Ferumbras",
        BELOW_PROWESS,
        "p02-ineligible-catalogue",
    )
    .await?;
    if ineligible.category != 3 || ineligible.kills != 0 || ineligible.thresholds[0] != 1 {
        return Err(format!("P02 expected an unearned Nemesis: {ineligible:?}").into());
    }
    refuse_assignment(&mut client, BELOW_PROWESS, &empty, "p02-ineligible.bin").await?;
    let eligible =
        catalogue_entry(&mut client, "Rotworm Queen", ORDINARY_BOSS, "p02-catalogue").await?;
    if eligible.category != 3 || eligible.kills != 1 || eligible.thresholds[0] != 1 {
        return Err(format!("P02 expected earned Nemesis Prowess: {eligible:?}").into());
    }

    let balance = gold(&mut client).await?;
    if balance.0 < 20 {
        return Err(
            format!("P02 free assignment needs the funded gold precondition: {balance:?}").into(),
        );
    }
    let ready = slots(
        &mut client,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
        Some("p02-ready.bin"),
    )
    .await?;
    let before_assign = ready
        .details
        .as_ref()
        .ok_or("P02 ready timestamp missing")?;
    if !empty_slots(&ready, 1, 10, 26)
        || before_assign.base_bonus != 26
        || before_assign.bonuses != [0, 0]
        || before_assign.boosted != details.boosted
        || before_assign.now < ready.ready_at_ms
    {
        return Err(format!("P02 invalid slot refresh before assignment: {ready:?}").into());
    }
    let equipped = slots(
        &mut client,
        BossSlotRequest::Equip {
            slot: 1,
            boss: ORDINARY_BOSS.into(),
        },
        Some("p02-equipped.bin"),
    )
    .await?;
    let mut confirmed = slots(
        &mut client,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
        Some("p02-assigned-clock.bin"),
    )
    .await?;
    let after_assign = confirmed
        .details
        .take()
        .ok_or("P02 post-assignment timestamp missing")?;
    let expected = BossSlotReply {
        bosses: [ORDINARY_BOSS.into(), String::new()],
        ready_at_ms: equipped.ready_at_ms,
        details: None,
        ..empty.clone()
    };
    if !assignment_ready_at(before_assign.now, after_assign.now, equipped.ready_at_ms)
        || equipped != expected
        || confirmed != equipped
        || after_assign.base_bonus != 26
        || after_assign.bonuses != [26, 0]
        || after_assign.boosted != details.boosted
    {
        return Err(format!("P02 exact assignment or cooldown mismatch: {before_assign:?} {equipped:?} {after_assign:?}").into());
    }
    evidence(&format!(
        "P02_ASSIGN_CLOCK before={} after={} ready_at_ms={}",
        before_assign.now, after_assign.now, equipped.ready_at_ms
    ));
    if gold(&mut client).await? != balance {
        return Err(
            format!("P02 first assignment changed carried or bank gold from {balance:?}").into(),
        );
    }

    let session = logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0x23).await?;
    let persisted_credit = progress(&mut fresh, None).await?;
    let persisted_slot = slots(&mut fresh, BossSlotRequest::Query, None).await?;
    if fresh.join_snapshot().game_session_id == session
        || !earned(&persisted_credit)
        || persisted_slot != equipped
        || gold(&mut fresh).await? != balance
    {
        return Err(format!("P02 relog lost kill, points, assignment or balance: {persisted_credit:?} {persisted_slot:?}").into());
    }
    evidence(&format!(
        "P02_CLIENT_CHECK kills=1 points=10 slot1=rotworm_queen unlocked=1 bonus=26 rat_refusal=1 below_prowess_refusal=1 cooldown_ms={ASSIGN_COOLDOWN_MS} gold_before_after={balance:?} durable=confirmed"
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=P02.client status=CAPTURED evidence=confirmed-progress-refusal-assignment-relog",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P02.client status=FAIL evidence={error}"
        )),
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
    assert!(empty_slots(&slot, 1, 10, 26));
    slot.unlocked = 0;
    assert!(!empty_slots(&slot, 1, 10, 26));
    slot.unlocked = 1;
    slot.bonus = 25;
    assert!(!empty_slots(&slot, 1, 10, 26));
    slot.bonus = 26;
    slot.remove_fee = 1;
    assert!(!empty_slots(&slot, 1, 10, 26));
    slot.unlocked = 0;
    slot.bonus = 25;
    slot.remove_fee = 0;
    assert!(!empty_slots(&slot, 0, 0, 25));
    assert!(assignment_ready_at(1000, 1010, 1490));
    assert!(!assignment_ready_at(1000, 1010, 1501));
    assert!(!assignment_ready_at(1000, 1010, 1000));
    assert!(!assignment_ready_at(1000, 3010, 1490));
    assert!(!assignment_ready_at(1000, 999, 1490));
}
