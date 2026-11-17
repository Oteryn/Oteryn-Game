use super::SanctionsPanel;
use oteryn_client_runtime::sanction_record::{SanctionEntry, SanctionKind};

fn kind_text(kind: SanctionKind) -> &'static str {
    match kind {
        SanctionKind::Mute => tr!("Mute"),
        SanctionKind::Jail => tr!("Jail"),
        SanctionKind::CharacterBan => tr!("Character ban"),
        SanctionKind::AccountBan => tr!("Account ban"),
        SanctionKind::Namelock => tr!("Namelock"),
        SanctionKind::Notation => tr!("Notation"),
    }
}

/// A sanction is active until it is revoked or its end passed.
fn status_text(entry: &SanctionEntry, now_ms: i64) -> &'static str {
    if entry.revoked {
        tr!("Revoked")
    } else if entry.ends_at_ms.is_some_and(|end| end <= now_ms) {
        tr!("Expired")
    } else {
        tr!("Active")
    }
}

fn entry_lines(entry: &SanctionEntry, now_ms: i64) -> Vec<String> {
    let scope = if entry.character_scope {
        tr!("This character")
    } else {
        tr!("Account")
    };
    let mut lines = vec![
        lfmt!(
            "{kind} — {scope} — {status}",
            kind = kind_text(entry.kind),
            scope = scope,
            status = status_text(entry, now_ms)
        ),
        lfmt!("Reason: {reason}", reason = entry.reason),
        lfmt!(
            "Start: {start}",
            start = crate::house::format_ownership_date(entry.starts_at_ms)
        ),
    ];
    // A ban without an end is permanent; a namelock and a notation have no end at all.
    match (entry.kind, entry.ends_at_ms) {
        (_, Some(end)) => lines.push(lfmt!(
            "End: {end}",
            end = crate::house::format_ownership_date(end)
        )),
        (SanctionKind::CharacterBan | SanctionKind::AccountBan, None) => {
            lines.push(lfmt!("End: {end}", end = tr!("Permanent")));
        }
        _ => {}
    }
    lines
}

/// The window text; `now_ms` is the current Unix time in milliseconds.
#[must_use]
pub fn panel_text(panel: &SanctionsPanel, now_ms: i64) -> String {
    let mut lines = vec![tr!("Sanction record").to_owned(), String::new()];
    match &panel.record {
        Some(record) if record.entries.is_empty() => {
            lines.push(tr!("No sanctions on record").to_owned());
        }
        Some(record) => {
            for entry in &record.entries {
                lines.extend(entry_lines(entry, now_ms));
                lines.push(String::new());
            }
            if record.more {
                lines.push(tr!("Older records are not shown").to_owned());
            }
        }
        None if !panel.selected => {
            lines.push(tr!("The sanction record is unavailable in this session").to_owned());
        }
        None => {}
    }
    if panel.pending {
        lines.push(tr!("Waiting for the server").to_owned());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.to_owned());
    }
    lines.join("\r\n")
}
