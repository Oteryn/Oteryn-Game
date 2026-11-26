//! P02 replay: server BOSS_COOLDOWNS and BOSSTIARY_SLOTS replies applied by the production client
//! reducer `apply_result` (`protocol.rs`), requested through the production `request`.
use super::*;
use oteryn_client_runtime::{
    boss_encounter::{
        BosstiaryProgressEntry, bosstiary_slots::BossSlotReply, encode_bosstiary_list,
    },
    raw_feature::{CommandStatus, GatedCommandResult},
};

struct Captured {
    cooldowns: GatedCommandResult,
    slots: GatedCommandResult,
    refused: GatedCommandResult,
    expected: String,
}

type Error = Box<dyn std::error::Error>;

fn fixture() -> Result<Captured, Error> {
    let progress =
        [("oteryn:creature.boss_a", 3), ("oteryn:creature.boss_b", 1)].map(|(key, kills)| {
            BosstiaryProgressEntry {
                boss_key: key.into(),
                kill_count: kills,
            }
        });
    let mut cooldowns = encode_bosstiary_list(1_000, &[], false, &progress, 25, false)
        .map_err(|error| format!("{error:?}"))?;
    cooldowns.extend_from_slice(&[58, 8, 8, 1, 16, 2, 24, 5, 32, 0]);
    let slot = |status, bosses: [&str; 2]| BossSlotReply {
        details: None,
        status,
        unlocked: 1,
        bonus: 4,
        points: 25,
        remove_fee: 0,
        ready_at_ms: 0,
        bosses: bosses.map(str::to_owned),
    };
    let encode = |reply: BossSlotReply| reply.encode().ok_or("slot encoder");
    Ok(Captured {
        cooldowns: result(CommandStatus::Accepted, &cooldowns),
        slots: result(CommandStatus::Accepted, &encode(slot(0, ["oteryn:creature.boss_a", ""]))?),
        refused: result(CommandStatus::Accepted, &encode(slot(1, ["oteryn:creature.boss_a", ""]))?),
        expected: "points=25\ntaints=1,2,5,false\nprogress=oteryn:creature.boss_a=3,oteryn:creature.boss_b=1\nslots_status=0\nslots_unlocked=1\nslots_bonus=4\nslots_points=25\nslots_fee=0\nslots_ready_at=0\nslot1=oteryn:creature.boss_a\nslot2=\nrefusal_status=1\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    let (session, cooldowns) = read_reply(path, "p02-cooldowns.bin")?;
    let (slot_session, slots) = read_reply(path, "p02-slots.bin")?;
    let (refusal_session, refused) = read_reply(path, "p02-refused.bin")?;
    if session == [0; 16]
        || session != slot_session
        || session != refusal_session
        || !(0 < cooldowns.command_id
            && cooldowns.command_id < slots.command_id
            && slots.command_id < refused.command_id
            && 0 < cooldowns.server_sequence
            && cooldowns.server_sequence < slots.server_sequence
            && slots.server_sequence < refused.server_sequence)
    {
        return Err("P02 captures do not share an ordered live session".into());
    }
    let expected = std::fs::read_to_string(path.join("p02-expected.txt"))?;
    if value(&expected, "session")? != format!("{session:02x?}") {
        return Err("P02 expectation belongs to a different session".into());
    }
    Ok(Captured {
        cooldowns,
        slots,
        refused,
        expected,
    })
}

fn read_reply(path: &std::path::Path, name: &str) -> Result<([u8; 16], GatedCommandResult), Error> {
    let bytes = std::fs::read(path.join(name))?;
    let (session, bytes) = bytes
        .split_first_chunk::<16>()
        .ok_or("P02 session missing")?;
    let (command, bytes) = bytes
        .split_first_chunk::<8>()
        .ok_or("P02 command missing")?;
    let (sequence, bytes) = bytes
        .split_first_chunk::<8>()
        .ok_or("P02 sequence missing")?;
    let (status, payload) = bytes.split_first_chunk::<4>().ok_or("P02 status missing")?;
    if u32::from_be_bytes(*status) != CommandStatus::Accepted as u32 {
        return Err(format!("P02 capture {name} has non-accepted envelope").into());
    }
    Ok((
        *session,
        GatedCommandResult {
            command_id: u64::from_be_bytes(*command),
            server_sequence: u64::from_be_bytes(*sequence),
            status: CommandStatus::Accepted,
            payload: payload.to_vec(),
        },
    ))
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn slot_text(expected: &str, status: &str, slot1: &str, slot2: &str) -> Result<String, Error> {
    let label = match status {
        "0" => "Updated",
        "1" => "Locked or ineligible",
        "2" => "Occupied or duplicate",
        "3" => "Cooldown",
        "4" => "Insufficient money",
        _ => "Change cannot fit",
    };
    Ok(format!(
        "Boss slots: {label}\r\n1: {slot1} | 2: {slot2}\r\nUnlocked mask: {} | Bonus: {} | Points: {}\r\nRemoval fee: {} | Ready at: {}",
        value(expected, "slots_unlocked")?,
        value(expected, "slots_bonus")?,
        value(expected, "slots_points")?,
        value(expected, "slots_fee")?,
        value(expected, "slots_ready_at")?,
    ))
}

#[test]
fn e2e_p02_replays_bosstiary_progress_slots_and_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let expected = captured.expected.as_str();
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    assert!(panel.snapshot.is_none());
    request(&mut panel, Intent::Refresh).ok_or("cooldown request")?;
    assert!(apply_result(&mut panel, &captured.cooldowns));
    let snapshot = panel.snapshot.as_ref().ok_or("snapshot absent")?;
    assert_eq!(snapshot.points.to_string(), value(expected, "points")?);
    let progress = snapshot
        .progress
        .iter()
        .map(|(key, kills)| format!("{key}={kills}"))
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(progress, value(expected, "progress")?);
    let text = panel_text(&panel);
    assert!(text.contains(&format!("Boss points: {}\n", value(expected, "points")?)));
    for entry in value(expected, "progress")?
        .split(',')
        .filter(|e| !e.is_empty())
    {
        let (key, kills) = entry.rsplit_once('=').ok_or("progress entry")?;
        assert!(
            text.lines()
                .any(|line| line == format!("  {key}: {kills} kills")),
            "missing progress line for {key}: {text}"
        );
    }
    match value(expected, "taints")? {
        "none" => assert!(snapshot.taints.is_none()),
        taints => {
            let mut fields = taints.split(',');
            let taint = snapshot.taints.ok_or("taints absent")?;
            assert_eq!(taint.soul_war.to_string(), fields.next().ok_or("soul")?);
            assert_eq!(
                taint.rotten_blood.to_string(),
                fields.next().ok_or("rotten")?
            );
            assert_eq!(
                taint.rotten_blood_flags.to_string(),
                fields.next().ok_or("flags")?
            );
            assert_eq!(
                taint.rotten_blood_final.to_string(),
                fields.next().ok_or("final")?
            );
        }
    }

    // The slot query shows the server's slot state; the refused Equip changes only the status line.
    let (slot1, slot2) = (value(expected, "slot1")?, value(expected, "slot2")?);
    assert!(panel.slots.is_none());
    request(&mut panel, Intent::SlotsQuery).ok_or("slot query")?;
    assert!(apply_result(&mut panel, &captured.slots));
    assert_eq!(
        panel.summary,
        slot_text(expected, value(expected, "slots_status")?, slot1, slot2)?
    );
    let slots_before = panel.slots.clone().ok_or("slot state missing")?;
    let summary_before = panel.summary.clone();
    panel.fields = ["oteryn:creature.rat".into(), "1".into(), String::new()];
    request(&mut panel, Intent::SlotEquip).ok_or("slot equip")?;
    assert!(apply_result(&mut panel, &captured.refused));
    let refusal = value(expected, "refusal_status")?;
    assert_eq!(refusal, "1");
    assert_eq!(
        panel.slots,
        Some(BossSlotReply {
            status: 1,
            ..slots_before
        })
    );
    assert_ne!(panel.summary, summary_before);
    assert_eq!(panel.summary, slot_text(expected, refusal, slot1, slot2)?);

    Ok(())
}

#[test]
fn p02_malformed_replies_preserve_confirmed_state() -> Result<(), Error> {
    let captured = fixture()?;
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    request(&mut panel, Intent::Refresh).ok_or("cooldown request")?;
    assert!(apply_result(&mut panel, &captured.cooldowns));
    request(&mut panel, Intent::SlotsQuery).ok_or("slot query")?;
    assert!(apply_result(&mut panel, &captured.slots));

    // An undecodable slot reply and a truncated snapshot leave the confirmed state unchanged.
    let (summary, snapshot) = (panel.summary.clone(), panel.snapshot.clone());
    request(&mut panel, Intent::SlotsQuery).ok_or("slot query")?;
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &[9, 9])
    ));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss data")
    );
    assert_eq!(panel.summary, summary);
    request(&mut panel, Intent::Refresh).ok_or("cooldown request")?;
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &[8])
    ));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss status")
    );
    assert_eq!(panel.snapshot, snapshot);
    Ok(())
}
