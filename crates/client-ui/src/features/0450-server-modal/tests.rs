use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
#[test]
fn generic_modal_panel_drives_wire_and_clears_only_after_matching_ack()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel::default();
    assert!(query(&mut panel).is_none());
    panel.selected = true;
    let request = query(&mut panel).ok_or("query")?;
    assert_eq!(request.kind, 736);
    assert_eq!(
        request.payload,
        oteryn_client_runtime::server_modal::QUERY_WITH_TERMINAL_REPLY
    );
    let dialog = Dialog {
        id: 42,
        title: "Choose".into(),
        message: "Select one".into(),
        choices: vec![(7, "First".into())],
        buttons: vec![(1, "OK".into()), (2, "Cancel".into())],
        cancel_button: Some(2),
        image: None,
    };
    let result = |payload| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    };
    assert!(apply_result(
        &mut panel,
        &result(Reply::Dialog(dialog.clone()).encode()?)
    ));
    assert!(panel_text(&panel).contains("Select one"));
    let chosen = Answer {
        dialog_id: 42,
        button_id: 1,
        choice_id: Some(7),
    };
    assert!(
        answer(
            &mut panel,
            Answer {
                choice_id: Some(9),
                ..chosen
            }
        )
        .is_none()
    );
    let request = answer(&mut panel, chosen).ok_or("answer")?;
    assert_eq!(request.kind, 737);
    assert_eq!(Answer::decode(&request.payload)?, chosen);
    assert!(panel.active.is_some());
    assert!(!apply_result(
        &mut panel,
        &result(
            Reply::Answered(Answer {
                dialog_id: 43,
                ..chosen
            })
            .encode()?
        )
    ));
    assert_eq!(panel.active, Some(dialog));
    assert!(answer(&mut panel, chosen).is_some());
    assert!(apply_result(
        &mut panel,
        &result(Reply::Answered(chosen).encode()?)
    ));
    assert!(panel.active.is_none());
    assert!(!panel.logged_out);
    assert!(answer(&mut panel, chosen).is_none());
    Ok(())
}

fn result(status: CommandStatus, payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    }
}

fn chosen() -> Answer {
    Answer {
        dialog_id: 42,
        button_id: 1,
        choice_id: Some(7),
    }
}

fn active_panel() -> Result<Panel, Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    query(&mut panel).ok_or("query")?;
    assert!(apply_result(
        &mut panel,
        &result(
            CommandStatus::Accepted,
            Reply::Dialog(Dialog {
                id: 42,
                title: "Choose".into(),
                message: "Select one".into(),
                choices: vec![(7, "First".into())],
                buttons: vec![(1, "OK".into()), (2, "Cancel".into())],
                cancel_button: Some(2),
                image: None,
            })
            .encode()?
        )
    ));
    Ok(panel)
}

#[test]
fn terminal_ack_ends_only_the_negotiated_matching_answer() -> Result<(), Box<dyn std::error::Error>>
{
    let mut panel = active_panel()?;
    assert_eq!(panel.terminal_replies, TerminalReplies::Enabled);
    answer(&mut panel, chosen()).ok_or("answer")?;
    assert!(apply_result(
        &mut panel,
        &result(
            CommandStatus::Accepted,
            Reply::AnsweredAndLoggedOut(chosen()).encode()?
        )
    ));
    assert!(panel.logged_out);
    assert!(panel.active.is_none());
    assert!(query(&mut panel).is_none());
    assert!(answer(&mut panel, chosen()).is_none());
    Ok(())
}

#[test]
fn refused_unnegotiated_stale_or_cancelled_answers_never_log_out()
-> Result<(), Box<dyn std::error::Error>> {
    let selected = chosen();
    let cancel = Answer {
        button_id: 2,
        choice_id: None,
        ..selected
    };
    for (sent, received, status, mode) in [
        (
            selected,
            selected,
            CommandStatus::Rejected,
            TerminalReplies::Enabled,
        ),
        (
            selected,
            selected,
            CommandStatus::DuplicateReplay,
            TerminalReplies::Enabled,
        ),
        (
            selected,
            selected,
            CommandStatus::Accepted,
            TerminalReplies::Probing,
        ),
        (
            selected,
            selected,
            CommandStatus::Accepted,
            TerminalReplies::Unsupported,
        ),
        (
            selected,
            Answer {
                dialog_id: 43,
                ..selected
            },
            CommandStatus::Accepted,
            TerminalReplies::Enabled,
        ),
        (
            selected,
            Answer {
                choice_id: Some(8),
                ..selected
            },
            CommandStatus::Accepted,
            TerminalReplies::Enabled,
        ),
        (
            cancel,
            cancel,
            CommandStatus::Accepted,
            TerminalReplies::Enabled,
        ),
    ] {
        let mut panel = active_panel()?;
        panel.terminal_replies = mode;
        answer(&mut panel, sent).ok_or("answer")?;
        assert!(!apply_result(
            &mut panel,
            &result(status, Reply::AnsweredAndLoggedOut(received).encode()?)
        ));
        assert!(!panel.logged_out);
        assert!(panel.active.is_some());
    }
    for sent in [selected, cancel] {
        let mut panel = active_panel()?;
        answer(&mut panel, sent).ok_or("answer")?;
        assert!(apply_result(
            &mut panel,
            &result(CommandStatus::Accepted, Reply::Answered(sent).encode()?)
        ));
        assert!(!panel.logged_out);
        assert!(query(&mut panel).is_some());
    }
    Ok(())
}

#[test]
fn only_explicit_initial_query_rejection_selects_the_legacy_query()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    let opt_in = oteryn_client_runtime::server_modal::QUERY_WITH_TERMINAL_REPLY;
    assert_eq!(query(&mut panel).ok_or("query")?.payload, opt_in);
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, vec![255])
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Probing);
    assert_eq!(query(&mut panel).ok_or("retry query")?.payload, opt_in);
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::DuplicateReplay, Reply::None.encode()?)
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Probing);
    assert_eq!(query(&mut panel).ok_or("query")?.payload, opt_in);
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Rejected, vec![])
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Unsupported);
    for _ in 0..2 {
        assert!(query(&mut panel).ok_or("legacy query")?.payload.is_empty());
        assert!(apply_result(
            &mut panel,
            &result(CommandStatus::Accepted, Reply::None.encode()?)
        ));
        assert_eq!(panel.terminal_replies, TerminalReplies::Unsupported);
        assert!(!panel.logged_out);
    }
    Ok(())
}

#[test]
fn terminal_payload_requires_an_answer_and_canonical_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    query(&mut panel).ok_or("query")?;
    assert!(!apply_result(
        &mut panel,
        &result(
            CommandStatus::Accepted,
            Reply::AnsweredAndLoggedOut(chosen()).encode()?
        )
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Probing);
    assert!(!panel.logged_out);
    let mut panel = active_panel()?;
    answer(&mut panel, chosen()).ok_or("answer")?;
    let mut trailing = Reply::AnsweredAndLoggedOut(chosen()).encode()?;
    trailing.push(0);
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, trailing)
    ));
    assert!(!panel.logged_out);
    assert!(panel.active.is_some());
    Ok(())
}

#[test]
fn t158_modal_text_input_and_dont_show_again_follow_the_byte_budget_and_explicit_ack() {
    use crate::modal_dialogs::{DismissedNotices, NoticeDialog, TextEditor};
    let mut amount = TextEditor::new("250", 10);
    assert_eq!(amount.submit(), Some("250"));
    amount.text = "12345678901".into();
    assert_eq!(amount.submit(), None);
    let mut edit = TextEditor::new("line one\nline two", 64);
    assert_eq!(edit.submit(), Some("line one\nline two"));
    edit.text.push('\0');
    assert_eq!(edit.submit(), None);
    let mut preferences = DismissedNotices::default();
    let mut notice = NoticeDialog::new("confirm:v1", "Really?", true);
    assert!(!notice.acknowledge(false, true, &mut preferences));
    assert!(!preferences.contains("confirm:v1"));
    assert!(notice.acknowledge(true, true, &mut preferences));
    assert!(preferences.contains("confirm:v1"));
}

#[test]
fn t159_modal_show_image_and_button_variants_validate_and_resolve_answers()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = active_panel()?;
    let dialog = panel.active.clone().ok_or("active")?;
    assert!(dialog.valid());
    assert!(dialog.accepts(chosen()));
    assert!(!dialog.accepts(Answer {
        button_id: 9,
        ..chosen()
    }));
    let image = Image {
        width: 2,
        height: 1,
        rgba: vec![255; 8],
    };
    let with_image = Dialog {
        image: Some(image.clone()),
        ..dialog.clone()
    };
    assert!(with_image.valid());
    let bad = Dialog {
        image: Some(Image {
            rgba: vec![0; 7],
            ..image
        }),
        ..dialog.clone()
    };
    assert!(!bad.valid());
    let single = Dialog {
        choices: Vec::new(),
        buttons: vec![(3, "Close".into())],
        cancel_button: None,
        ..dialog
    };
    assert!(single.accepts(Answer {
        dialog_id: 42,
        button_id: 3,
        choice_id: None
    }));
    panel.active = Some(single);
    assert!(panel_text(&panel).starts_with("Choose"));
    Ok(())
}

#[test]
fn t166_nagging_balancing_and_generic_notices_keep_separate_suppression_keys() {
    use crate::modal_dialogs::{DismissedNotices, NoticeDialog};
    let mut preferences = DismissedNotices::default();
    let mut nagging = NoticeDialog::new("nag:donate", "Support us", true);
    let mut balancing = NoticeDialog::new("balance:v2", "Damage rules changed", true);
    let mut generic = NoticeDialog::new("notice:generic", "Server restarts soon", false);
    assert!(nagging.acknowledge(true, true, &mut preferences));
    assert!(preferences.contains("nag:donate"));
    assert!(balancing.visible(&preferences));
    assert!(balancing.acknowledge(true, false, &mut preferences));
    assert!(!preferences.contains("balance:v2"));
    assert!(generic.acknowledge(true, true, &mut preferences));
    assert!(!preferences.contains("notice:generic"));
    assert!(!NoticeDialog::new("nag:donate", "Support us", true).visible(&preferences));
    assert!(NoticeDialog::new("balance:v2", "Damage rules changed", true).visible(&preferences));
}

fn catalogue(
    language: &str,
) -> Result<std::collections::BTreeMap<String, String>, serde_json::Error> {
    serde_json::from_str(match language {
        "en" => include_str!("../../components/localization/locales/en.json"),
        "de" => include_str!("../../components/localization/locales/de.json"),
        "es" => include_str!("../../components/localization/locales/es.json"),
        "pl" => include_str!("../../components/localization/locales/pl.json"),
        "pt" => include_str!("../../components/localization/locales/pt.json"),
        _ => include_str!("../../components/localization/locales/sv.json"),
    })
}
fn unescape(literal: &str) -> String {
    let mut out = String::new();
    let mut chars = literal.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('u') => {
                let code: String = chars.by_ref().skip(1).take_while(|c| *c != '}').collect();
                out.extend(u32::from_str_radix(&code, 16).ok().and_then(char::from_u32));
            }
            Some('\n') => {
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}
fn used_texts(dir: &std::path::Path, found: &mut Vec<(bool, String)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            used_texts(&path, found)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let source = std::fs::read_to_string(&path)?;
            for (index, _) in source.match_indices('!') {
                let name = source[..index]
                    .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("");
                if !matches!(
                    name,
                    "tr" | "lfmt" | "lwrite" | "lwriteln" | "lprintln" | "leprintln"
                ) || !source[index + 1..].starts_with('(')
                {
                    continue;
                }
                let mut rest = source[index + 2..].trim_start();
                if !rest.starts_with('"') {
                    match rest.find(',') {
                        Some(comma)
                            if name != "tr"
                                && rest[..comma]
                                    .chars()
                                    .all(|c| c.is_alphanumeric() || "_.&*".contains(c)) =>
                        {
                            rest = rest[comma + 1..].trim_start();
                        }
                        _ => continue,
                    }
                }
                let Some(body) = rest.strip_prefix('"') else {
                    continue;
                };
                let mut end = 0;
                let mut escaped = false;
                for (offset, c) in body.char_indices() {
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        end = offset;
                        break;
                    }
                }
                found.push((name != "tr", unescape(&body[..end])));
            }
        }
    }
    Ok(())
}

#[test]
fn o005_every_client_text_has_all_six_languages_with_unchanged_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    let english = catalogue("en")?;
    let formats: std::collections::BTreeSet<String> = serde_json::from_str(include_str!(
        "../../components/localization/locales/formats.json"
    ))?;
    let braces = |text: &str| text.matches('{').count() + text.matches('}').count();
    for language in ["de", "es", "pl", "pt", "sv"] {
        let translated = catalogue(language)?;
        assert!(
            english.keys().eq(translated.keys()),
            "{language} keys differ"
        );
        for (source, target) in &translated {
            assert!(!target.trim().is_empty(), "{language}: empty {source:?}");
            if formats.contains(source) {
                assert_eq!(braces(source), braces(target), "{language}: {source:?}");
            }
        }
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    used_texts(&manifest.join("src"), &mut found)?;
    used_texts(&manifest.join("../../apps/client/src"), &mut found)?;
    assert!(found.len() > 1000, "scan found {} texts", found.len());
    for (format, text) in found {
        assert!(english.contains_key(&text), "missing client text {text:?}");
        assert!(
            !format || formats.contains(&text),
            "missing client format {text:?}"
        );
    }
    Ok(())
}
