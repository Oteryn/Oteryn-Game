//! N07a replay: server CHARACTER_INFO replies (QuerySelf, accepted Select, refused Select of an
//! unearned title, follow-up QuerySelf) applied by the production `command` and `apply` in
//! `protocol.rs`, which populate the Cyclopedia title selector through `apply_title_reply` and
//! `title_rows` in `crates/client-ui/src/features/0097-character-info/state/titles.rs`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::character_info::{OwnTitles, TitleFilter, title_rows};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: Vec<u8>,
    select: Vec<u8>,
    after: Vec<u8>,
    expected: String,
}

/// The documented CHARACTER-INFO-1 layouts, used only when no capture is present.
fn fixture() -> Captured {
    let mut query = vec![0];
    for value in [37i64, 120_500, 4, 6, 12, 3, 1, 0] {
        query.extend(value.to_le_bytes());
    }
    query.extend([0, 0, 0, 0, 2, 1, 2, 0]);
    let mut after = query.clone();
    let last = after.len() - 1;
    after[last] = 1;
    let mut select = vec![3];
    select.extend(7u64.to_le_bytes());
    Captured {
        query,
        select,
        after,
        expected: "earned=1,2\nselected_before=0\nselect_title=1\nselect_revision=7\nrefused_title=3\nrefused_status=Rejected\nselected_after=1\n".into(),
    }
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(fixture());
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        query: std::fs::read(path.join("n07a-query.bin"))?,
        select: std::fs::read(path.join("n07a-select.bin"))?,
        after: std::fs::read(path.join("n07a-query-after.bin"))?,
        expected: std::fs::read_to_string(path.join("n07a-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn title(id: &str) -> Result<Option<wire::CharacterTitle>, Error> {
    let id: u8 = id.parse()?;
    Ok(if id == 0 {
        None
    } else {
        Some(wire::CharacterTitle::from_id(id).ok_or("unknown title id")?)
    })
}

fn earned(expected: &str) -> Result<Vec<wire::CharacterTitle>, Error> {
    value(expected, "earned")?
        .split(',')
        .filter(|id| !id.is_empty())
        .map(|id| title(id)?.ok_or_else(|| "zero earned id".into()))
        .collect()
}

fn pend(panel: &mut Panel, intent: Intent) -> Result<FeatureCommand, Error> {
    let command = command(panel, intent)?;
    panel.pending = true;
    panel.pending_kind = command.kind;
    Ok(command)
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status,
        server_sequence: 1,
        payload: payload.to_vec(),
    }
}

#[test]
fn e2e_n07a_replays_title_ownership_selection_and_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let owned = earned(e)?;
    let selected_before = title(value(e, "selected_before")?)?;
    let chosen = title(value(e, "select_title")?)?;
    let mut panel = Panel {
        selected: true,
        fields: ["0".into(), "1".into(), String::new(), String::new()],
        ..Panel::default()
    };

    // The selector opens with a one-row self query; the reply fills the ownership list.
    let query = pend(&mut panel, Intent::Query)?;
    assert_eq!(query.payload, [0, 0, 0, 1]);
    apply(&mut panel, &query, &result(CommandStatus::Accepted, &captured.query));
    assert_eq!(panel.feedback, "Character information updated");
    assert_eq!(
        panel.own_titles,
        Some(OwnTitles {
            earned: owned.clone(),
            selected: selected_before
        })
    );
    panel.title_filter = TitleFilter::Earned;
    let rows = title_rows(&panel);
    assert_eq!(rows.len(), owned.len());
    assert!(rows.iter().all(|row| row.earned && owned.contains(&row.title)));
    assert_eq!(
        rows.iter().filter(|row| row.selected).map(|row| row.title).next(),
        selected_before
    );
    panel.title_filter = TitleFilter::Locked;
    assert_eq!(
        title_rows(&panel).len(),
        wire::CharacterTitle::ALL.len() - owned.len()
    );

    // Selecting an earned title (or clearing the selection) is accepted and moves the marker.
    panel.fields[3] = value(e, "select_title")?.to_owned();
    let select = pend(&mut panel, Intent::Select)?;
    assert_eq!(select.payload, [3, value(e, "select_title")?.parse()?]);
    apply(&mut panel, &select, &result(CommandStatus::Accepted, &captured.select));
    assert_eq!(panel.feedback, "Character information updated");
    assert_eq!(
        panel.summary,
        format!(
            "Selected title: {}; revision {}",
            chosen.map_or("None", wire::CharacterTitle::name),
            value(e, "select_revision")?
        )
    );
    assert_eq!(
        panel.own_titles,
        Some(OwnTitles {
            earned: owned.clone(),
            selected: chosen
        })
    );
    panel.title_filter = TitleFilter::Earned;
    assert_eq!(
        title_rows(&panel).iter().filter(|row| row.selected).map(|row| row.title).next(),
        chosen
    );

    // The server refuses an unearned title: the ownership list and the summary stay.
    let (own, summary) = (panel.own_titles.clone(), panel.summary.clone());
    panel.fields[3] = value(e, "refused_title")?.to_owned();
    let refused = pend(&mut panel, Intent::Select)?;
    assert_eq!(refused.payload, [3, value(e, "refused_title")?.parse()?]);
    assert_eq!(value(e, "refused_status")?, "Rejected");
    apply(&mut panel, &refused, &result(CommandStatus::Rejected, &[]));
    assert_eq!(panel.feedback, "The server refused the request");
    assert_eq!((panel.own_titles.clone(), panel.summary.clone()), (own.clone(), summary));
    assert!(!panel.pending);

    // The follow-up query reports the selection the server stored.
    let query = pend(&mut panel, Intent::Query)?;
    apply(&mut panel, &query, &result(CommandStatus::Accepted, &captured.after));
    assert_eq!(
        panel.own_titles,
        Some(OwnTitles {
            earned: owned,
            selected: title(value(e, "selected_after")?)?
        })
    );
    let after = title(value(e, "selected_after")?)?;
    assert!(panel.summary.ends_with(&format!(
        "\r\nSelected: {}",
        after.map_or("None", wire::CharacterTitle::name)
    )));

    // A truncated Select reply is refused and neither the ownership list nor the summary change.
    let (own, summary) = (panel.own_titles.clone(), panel.summary.clone());
    panel.fields[3] = value(e, "select_title")?.to_owned();
    let select = pend(&mut panel, Intent::Select)?;
    let cut = &captured.select[..captured.select.len() - 1];
    apply(&mut panel, &select, &result(CommandStatus::Accepted, cut));
    assert_eq!(panel.feedback, "Invalid character information reply");
    assert_eq!((panel.own_titles, panel.summary), (own, summary));
    Ok(())
}
