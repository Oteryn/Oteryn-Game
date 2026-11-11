use super::*;
use oteryn_client_runtime::{
    CommandStatus,
    raw_feature::GatedCommandResult,
    sanction_record::{SanctionEntry, SanctionKind, SanctionRecord, encode_reply},
};

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_800_000_000_000;

fn entry(
    kind: SanctionKind,
    character_scope: bool,
    starts: i64,
    ends: Option<i64>,
) -> SanctionEntry {
    SanctionEntry {
        kind,
        character_scope,
        revoked: false,
        reason_code: 7,
        starts_at_ms: starts,
        ends_at_ms: ends,
        reason: "Spamming".into(),
    }
}

fn result(status: CommandStatus, payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    }
}

fn panel() -> SanctionsPanel {
    SanctionsPanel {
        selected: true,
        ..Default::default()
    }
}

#[test]
fn c0497_open_sends_the_empty_query_and_the_reply_fills_the_record() -> Result<(), &'static str> {
    let mut p = panel();
    let command = request(&mut p).ok_or("a query")?;
    assert_eq!((command.capability, command.kind), (497, 6240));
    assert!(command.payload.is_empty());
    assert!(p.pending);
    assert!(request(&mut p).is_none(), "one query at a time");
    let record = SanctionRecord {
        more: false,
        entries: vec![
            entry(SanctionKind::Mute, true, NOW - DAY, Some(NOW + DAY)),
            entry(SanctionKind::AccountBan, false, NOW - 9 * DAY, None),
            SanctionEntry {
                revoked: true,
                ..entry(
                    SanctionKind::Jail,
                    true,
                    NOW - 20 * DAY,
                    Some(NOW - 19 * DAY),
                )
            },
            entry(SanctionKind::Notation, true, NOW - 30 * DAY, None),
        ],
    };
    let payload = encode_reply(&record).ok_or("encode")?;
    apply_result(&mut p, &result(CommandStatus::Accepted, payload));
    assert!(!p.pending);
    assert_eq!(p.record.as_ref(), Some(&record));
    let text = panel_text(&p, NOW);
    assert!(text.contains("Mute — This character — Active"), "{text}");
    assert!(text.contains("Account ban — Account — Active"), "{text}");
    assert!(text.contains("Jail — This character — Revoked"), "{text}");
    assert!(text.contains("Reason: Spamming"), "{text}");
    assert!(text.contains("End: Permanent"), "{text}");
    assert!(text.contains("Start: 2027-01-14 "), "{text}");
    Ok(())
}

#[test]
fn c0497_an_elapsed_end_reads_expired_and_an_empty_record_says_so() {
    let mut p = panel();
    p.record = Some(SanctionRecord {
        more: true,
        entries: vec![entry(
            SanctionKind::Mute,
            true,
            NOW - 3 * DAY,
            Some(NOW - DAY),
        )],
    });
    let text = panel_text(&p, NOW);
    assert!(text.contains("Mute — This character — Expired"), "{text}");
    assert!(text.contains("Older records are not shown"), "{text}");
    p.record = Some(SanctionRecord {
        more: false,
        entries: Vec::new(),
    });
    assert!(panel_text(&p, NOW).contains("No sanctions on record"));
}

#[test]
fn c0497_a_refusal_or_bad_bytes_keep_the_record_and_say_so() {
    let mut p = panel();
    p.record = Some(SanctionRecord {
        more: false,
        entries: Vec::new(),
    });
    p.pending = true;
    apply_result(&mut p, &result(CommandStatus::Rejected, Vec::new()));
    assert!(!p.pending && p.record.is_some());
    assert!(panel_text(&p, NOW).contains("The server refused the sanction record"));
    apply_result(&mut p, &result(CommandStatus::Accepted, vec![9, 9, 9]));
    assert!(panel_text(&p, NOW).contains("unreadable sanction record"));
}

#[test]
fn c0497_without_the_capability_nothing_is_sent() {
    let mut p = SanctionsPanel::default();
    assert!(request(&mut p).is_none());
    assert!(!p.pending);
    assert!(panel_text(&p, NOW).contains("unavailable in this session"));
}
