//! E07 replay ownership: `request`, `apply_result` and `carried_intent` in `protocol.rs` consume
//! the Stash open, stow, browse and withdraw replies; `panel_text` and `row_label` in `view.rs`
//! project them.
use super::*;
use oteryn_client_runtime::stash::CommandStatus;

type Error = Box<dyn std::error::Error>;

struct Capture {
    open: Vec<u8>,
    stow: Vec<u8>,
    browse: Vec<u8>,
    withdraw: Vec<u8>,
    after: Vec<u8>,
    expected: String,
}

fn reply(outcome: stash::StashOutcome, stowed: u32, quantity: u32) -> Result<Vec<u8>, Error> {
    let rows = if quantity == 0 {
        vec![]
    } else {
        vec![stash::StashRow {
            handle: [9; 16],
            definition_key: "oteryn:item.tibia.i3277".into(),
            definition_revision: "r1".into(),
            quantity,
            migration_required: false,
        }]
    };
    stash::encode_stash_reply(&stash::StashReply {
        outcome,
        stowed_quantity: stowed,
        transaction: None,
        page: 0,
        has_more: false,
        rows,
    })
    .map_err(|error| format!("{error:?}").into())
}

fn capture() -> Result<Capture, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            open: std::fs::read(root.join("e07-open.bin"))?,
            stow: std::fs::read(root.join("e07-stow.bin"))?,
            browse: std::fs::read(root.join("e07-browse.bin"))?,
            withdraw: std::fs::read(root.join("e07-withdraw.bin"))?,
            after: std::fs::read(root.join("e07-after.bin"))?,
            expected: std::fs::read_to_string(root.join("e07-expected.txt"))?,
        });
    }
    Ok(Capture {
        open: reply(stash::StashOutcome::Opened, 0, 0)?,
        stow: reply(stash::StashOutcome::Committed, 2, 2)?,
        browse: reply(stash::StashOutcome::Opened, 0, 2)?,
        withdraw: reply(stash::StashOutcome::Committed, 0, 1)?,
        after: reply(stash::StashOutcome::Opened, 0, 1)?,
        expected: "key=oteryn:item.tibia.i3277\nrevision=r1\nstowed=2\n".into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .ok_or_else(|| format!("expected.txt lacks {key}").into())
}

fn response(payload: &[u8]) -> Result<StashResponse, Error> {
    Ok(StashResponse {
        status: CommandStatus::Accepted,
        reply: stash::decode_stash_reply(payload).map_err(|error| format!("{error:?}"))?,
    })
}

fn exact_page(panel: &StashPanel, expected: Option<(&str, &str, u32)>) -> bool {
    panel.page == 0
        && !panel.has_more
        && panel
            .rows
            .iter()
            .map(|row| {
                (
                    row.definition_key.as_str(),
                    row.definition_revision.as_str(),
                    row.quantity,
                    row.migration_required,
                )
            })
            .eq(expected
                .into_iter()
                .map(|(key, revision, quantity)| (key, revision, quantity, false)))
}

fn check_withdrawal(
    panel: &StashPanel,
    response: &StashResponse,
    key: &str,
    revision: &str,
) -> bool {
    response.reply.outcome == stash::StashOutcome::Committed
        && response.reply.stowed_quantity == 0
        && !panel.pending
        && panel.feedback.is_none()
        && exact_page(panel, Some((key, revision, 1)))
}

#[test]
fn e07_real_server_payloads_replay_into_stash_panel() -> Result<(), Error> {
    let data = capture()?;
    let live = std::env::var_os("E2E_ALL_CAPTURE_DIR").is_some();
    if let Ok(binding) = std::env::var("NODE_BOOT_E07_CAPTURE_BINDING") {
        let session = value(&data.expected, "session")?;
        let sequences = value(&data.expected, "sequences")?;
        assert_eq!(binding, format!("session={session} sequences={sequences}"));
        let sequences = sequences
            .split(',')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(sequences.len(), 5);
        assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    }
    let key = value(&data.expected, "key")?;
    let revision = value(&data.expected, "revision")?;
    let stowed: u32 = value(&data.expected, "stowed")?.parse()?;
    let mut panel = StashPanel {
        selected: true,
        ..Default::default()
    };

    request(&mut panel, Intent::Refresh).ok_or("open")?;
    apply_result(&mut panel, &response(&data.open)?);
    assert!(exact_page(&panel, None));
    assert_eq!(panel.feedback, None);

    let item = oteryn_client_runtime::depot::ItemEntry {
        appearance_id: 0,
        handle: if live {
            value(&data.expected, "handle")?.parse()?
        } else {
            std::num::NonZeroU64::MIN
        },
        item_definition_ref: if live {
            value(&data.expected, "definition_ref")?.parse()?
        } else {
            std::num::NonZeroU32::MIN
        },
        count: std::num::NonZeroU32::new(stowed).ok_or("stowed zero")?,
        sub_type: 0,
    };
    panel.carried = vec![item];
    assert_eq!(
        carried_intent(&panel, &[item], 0),
        Some(Intent::Stow(item.handle))
    );
    request(&mut panel, Intent::Stow(item.handle)).ok_or("stow")?;
    assert!(panel.pending);
    apply_result(&mut panel, &response(&data.stow)?);
    assert!(!panel.pending);
    assert_eq!(
        panel.feedback,
        Some(format!(
            "Stowed {stowed} object{}",
            if stowed == 1 { "." } else { "s." }
        ))
    );

    assert_eq!(stowed, 2);
    assert!(exact_page(&panel, Some((key, revision, 2))));
    request(&mut panel, Intent::Refresh).ok_or("browse")?;
    apply_result(&mut panel, &response(&data.browse)?);
    assert!(exact_page(&panel, Some((key, revision, 2))));
    assert_eq!(row_label(&panel.rows[0]), format!("{key} ({revision}) × 2"));
    let mapped = visible_withdraw(&panel, 0, 1).ok_or("visible withdrawal")?;
    let command = request(&mut panel, mapped).ok_or("withdraw")?;
    let (intent, filter) =
        stash::decode_stash_query(&command.payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(filter, Some(stash::StashFilter::default()));
    assert_eq!(
        intent.action,
        stash::StashAction::Withdraw {
            row: panel.rows[0].handle,
            amount: 1
        }
    );
    let withdrawn = response(&data.withdraw)?;
    apply_result(&mut panel, &withdrawn);
    assert!(check_withdrawal(&panel, &withdrawn, key, revision));
    request(&mut panel, Intent::Refresh).ok_or("after")?;
    apply_result(&mut panel, &response(&data.after)?);
    assert!(exact_page(&panel, Some((key, revision, 1))));
    let shown = panel
        .rows
        .iter()
        .find(|r| r.definition_key == key)
        .ok_or("definition absent after withdraw")?;
    assert!(panel_text(&panel).contains(&row_label(shown)));

    // A stale handle never produces a command and leaves the rows unchanged.
    let rows = panel.rows.clone();
    assert!(request(&mut panel, Intent::Stow(std::num::NonZeroU64::MAX)).is_none());
    assert_eq!(
        panel.feedback.as_deref(),
        Some("The item changed; select a carried item again")
    );
    assert_eq!(panel.rows, rows);
    Ok(())
}

#[test]
fn e07_rejects_wrong_withdrawal_before_refresh() -> Result<(), Error> {
    let mut panel = StashPanel {
        selected: true,
        ..Default::default()
    };
    let key = "oteryn:item.tibia.i3277";
    let browse = response(&reply(stash::StashOutcome::Opened, 0, 2)?)?;
    apply_result(&mut panel, &browse);
    assert!(visible_withdraw(&panel, 1, 1).is_none());
    let committed = response(&reply(stash::StashOutcome::Committed, 0, 1)?)?;
    apply_result(&mut panel, &committed);
    assert!(check_withdrawal(&panel, &committed, key, "r1"));
    panel.feedback = Some("wrong".into());
    assert!(!check_withdrawal(&panel, &committed, key, "r1"));
    let wrong = response(&reply(stash::StashOutcome::Committed, 0, 2)?)?;
    apply_result(&mut panel, &wrong);
    assert!(!check_withdrawal(&panel, &wrong, key, "r1"));
    Ok(())
}
