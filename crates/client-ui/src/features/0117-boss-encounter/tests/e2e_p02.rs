//! P02 replay feeds live replies through the production reducer after matching the requests.
use super::*;
use oteryn_client_runtime::{
    boss_encounter::bosstiary_slots::BossSlotReply,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};

const BOSS: &str = "oteryn:creature.rotworm_queen";
const RAT: &str = "oteryn:creature.rat";
type Error = Box<dyn std::error::Error>;

struct Exchange {
    session: [u8; 16],
    request: FeatureCommand,
    reply: GatedCommandResult,
}

fn read_exchange(directory: &std::path::Path, name: &str) -> Result<Exchange, Error> {
    let bytes = std::fs::read(directory.join(name))?;
    let (session, bytes) = bytes
        .split_first_chunk::<16>()
        .ok_or("P02 session missing")?;
    let (command, bytes) = bytes
        .split_first_chunk::<8>()
        .ok_or("P02 command missing")?;
    let (sequence, bytes) = bytes
        .split_first_chunk::<8>()
        .ok_or("P02 sequence missing")?;
    let (status, bytes) = bytes.split_first_chunk::<4>().ok_or("P02 status missing")?;
    let (capability, bytes) = bytes
        .split_first_chunk::<4>()
        .ok_or("P02 capability missing")?;
    let (kind, bytes) = bytes.split_first_chunk::<4>().ok_or("P02 kind missing")?;
    let (length, bytes) = bytes
        .split_first_chunk::<4>()
        .ok_or("P02 request length missing")?;
    let (request, payload) = bytes
        .split_at_checked(usize::try_from(u32::from_be_bytes(*length))?)
        .ok_or("P02 truncated request")?;
    if u32::from_be_bytes(*status) != CommandStatus::Accepted as u32 {
        return Err(format!("P02 capture {name} has non-accepted envelope").into());
    }
    Ok(Exchange {
        session: *session,
        request: FeatureCommand {
            capability: u32::from_be_bytes(*capability),
            kind: u32::from_be_bytes(*kind),
            payload: request.to_vec(),
        },
        reply: GatedCommandResult {
            command_id: u64::from_be_bytes(*command),
            server_sequence: u64::from_be_bytes(*sequence),
            status: CommandStatus::Accepted,
            payload: payload.to_vec(),
        },
    })
}

fn apply_exchange(
    panel: &mut BossEncounterPanel,
    intent: Intent,
    exchange: &Exchange,
) -> Result<(), Error> {
    let actual = request(panel, intent).ok_or("P02 UI command absent")?;
    if actual != exchange.request {
        return Err(format!(
            "P02 UI request mismatch: sent={:?} generated={actual:?}",
            exchange.request
        )
        .into());
    }
    if !apply_result(panel, &exchange.reply) {
        return Err(format!("P02 UI did not apply live reply for {intent:?}").into());
    }
    Ok(())
}

fn assert_credit(panel: &BossEncounterPanel) {
    let snapshot = panel.snapshot.as_ref();
    assert_eq!(snapshot.map(|value| value.points), Some(10));
    assert_eq!(
        snapshot.map(|value| value.progress.as_slice()),
        Some([(BOSS.to_owned(), 1)].as_slice())
    );
    let text = panel_text(panel);
    assert!(text.lines().any(|line| line == "Boss points: 10"));
    assert!(
        text.lines()
            .any(|line| line == format!("  {BOSS}: 1 kills"))
    );
}

fn assert_slots(panel: &BossEncounterPanel, status: u8, boss: &str, ready: u64) {
    let slots = panel.slots.as_ref();
    assert_eq!(
        slots.map(|value| (value.status, value.unlocked, value.bonus, value.points)),
        Some((status, 1, 26, 10))
    );
    assert_eq!(
        slots.map(|value| value.bosses.as_slice()),
        Some([boss.to_owned(), String::new()].as_slice())
    );
    assert_eq!(slots.map(|value| value.ready_at_ms), Some(ready));
    let label = if status == 1 {
        "Locked or ineligible"
    } else {
        "Updated"
    };
    let fee = slots.map_or(0, |value| value.remove_fee);
    assert_eq!(
        panel.summary,
        format!(
            "Boss slots: {label}\r\n1: {boss} | 2: \r\nUnlocked mask: 1 | Bonus: 26 | Points: 10\r\nRemoval fee: {fee} | Ready at: {ready}"
        )
    );
}

fn assert_malformed_preserves_state(
    panel: &mut BossEncounterPanel,
    slots: &Exchange,
    cooldowns: &Exchange,
) -> Result<(), Error> {
    let (summary, confirmed_slots, snapshot) = (
        panel.summary.clone(),
        panel.slots.clone(),
        panel.snapshot.clone(),
    );
    request(panel, Intent::SlotsQuery).ok_or("malformed slot request")?;
    let invalid_slots = GatedCommandResult {
        payload: vec![9, 9],
        ..slots.reply.clone()
    };
    assert!(!apply_result(panel, &invalid_slots));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss data")
    );
    assert_eq!((&panel.summary, &panel.slots), (&summary, &confirmed_slots));
    request(panel, Intent::Refresh).ok_or("malformed cooldown request")?;
    let invalid_cooldowns = GatedCommandResult {
        payload: vec![8],
        ..cooldowns.reply.clone()
    };
    assert!(!apply_result(panel, &invalid_cooldowns));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss status")
    );
    assert_eq!(panel.snapshot, snapshot);
    Ok(())
}

#[test]
fn e2e_p02_replays_bosstiary_progress_slots_and_refusal() -> Result<(), Error> {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        // The required node-boot replay supplies the new capture directory and checks its marker.
        return Ok(());
    };
    let directory = std::path::Path::new(&directory);
    let cooldowns = read_exchange(directory, "p02-cooldowns.bin")?;
    let slots = read_exchange(directory, "p02-slots.bin")?;
    let refused = read_exchange(directory, "p02-refused.bin")?;
    let invested = read_exchange(directory, "p02-equipped.bin")?;
    let exchanges = [&cooldowns, &slots, &refused, &invested];
    if cooldowns.session == [0; 16]
        || exchanges.iter().any(|item| {
            item.session != cooldowns.session
                || item.reply.command_id == 0
                || item.reply.server_sequence == 0
        })
        || exchanges.windows(2).any(|pair| {
            pair[0].reply.command_id >= pair[1].reply.command_id
                || pair[0].reply.server_sequence >= pair[1].reply.server_sequence
        })
    {
        return Err("P02 captures do not share an ordered live session".into());
    }

    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    assert!(panel.snapshot.is_none());
    apply_exchange(&mut panel, Intent::Refresh, &cooldowns)?;
    assert_credit(&panel);
    assert!(
        panel
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.taints)
            .is_some_and(|taints| (
                taints.soul_war,
                taints.rotten_blood,
                taints.rotten_blood_flags,
                taints.rotten_blood_final
            ) == (0, 0, 0, false))
    );
    assert!(panel.slots.is_none());
    apply_exchange(&mut panel, Intent::SlotsQuery, &slots)?;
    assert_slots(&panel, 0, "", 0);
    let empty = panel.slots.clone().ok_or("P02 UI slot state missing")?;
    panel.fields = [RAT.into(), "1".into(), String::new()];
    apply_exchange(&mut panel, Intent::SlotEquip, &refused)?;
    assert_slots(&panel, 1, "", 0);
    assert_eq!(
        panel.slots,
        Some(BossSlotReply {
            status: 1,
            ..empty.clone()
        })
    );

    panel.fields[0] = BOSS.into();
    apply_exchange(&mut panel, Intent::SlotEquip, &invested)?;
    let equipped =
        BossSlotReply::decode(&invested.reply.payload).ok_or("P02 assigned slot reply")?;
    assert!(equipped.ready_at_ms > 0);
    assert_slots(&panel, 0, BOSS, equipped.ready_at_ms);
    assert_eq!(
        panel.slots,
        Some(BossSlotReply {
            bosses: [BOSS.into(), String::new()],
            ready_at_ms: equipped.ready_at_ms,
            ..empty
        })
    );
    assert_malformed_preserves_state(&mut panel, &slots, &cooldowns)?;
    println!(
        "SEAM_EVIDENCE P02_REPLAY progress=exact requests=4 refusal=unlocked assignment=exact malformed=2"
    );
    Ok(())
}

#[test]
fn p02_replay_catches_wrong_slot_and_boss_requests() {
    for payload in [
        [&[1, 2][..], RAT.as_bytes()].concat(),
        [&[1, 1][..], BOSS.as_bytes()].concat(),
    ] {
        let mut panel = BossEncounterPanel {
            selected: true,
            fields: [RAT.into(), "1".into(), String::new()],
            ..BossEncounterPanel::default()
        };
        let exchange = Exchange {
            session: [0; 16],
            request: FeatureCommand {
                capability: 117,
                kind: 491,
                payload,
            },
            reply: GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: Vec::new(),
            },
        };
        assert!(
            apply_exchange(&mut panel, Intent::SlotEquip, &exchange)
                .is_err_and(|error| error.to_string().contains("request mismatch"))
        );
    }
}
