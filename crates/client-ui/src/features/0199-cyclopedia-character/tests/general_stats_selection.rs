//! Required per-payload capability-487 evidence for live GeneralStats captures.
//! Capability 488 does not add fields to this tab; no history-extension proof is claimed.
use crate::cyclopedia_character::{CyclopediaCharacterPanel, TabExtensions};
use std::path::Path;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
pub(super) struct Selection {
    pub extensions: TabExtensions,
    pub vocation: Option<u8>,
}

pub(super) fn legacy() -> Selection {
    Selection {
        extensions: TabExtensions {
            vocation: false,
            death_details: false,
        },
        vocation: None,
    }
}

pub(super) fn read(directory: &Path, name: &str) -> TestResult<Selection> {
    parse(&std::fs::read_to_string(
        directory.join(format!("{name}.selection")),
    )?)
}

fn parse(text: &str) -> TestResult<Selection> {
    let lines = text.lines().collect::<Vec<_>>();
    match lines.as_slice() {
        ["vocation_selected=0", "vocation_id=none"] => Ok(legacy()),
        ["vocation_selected=1", id] => {
            let id: u8 = id
                .strip_prefix("vocation_id=")
                .ok_or("missing vocation ID")?
                .parse()?;
            crate::cyclopedia_character_vocation::vocation_title(id)
                .ok_or("invalid vocation ID")?;
            Ok(Selection {
                extensions: TabExtensions {
                    vocation: true,
                    death_details: false,
                },
                vocation: Some(id),
            })
        }
        _ => Err("missing or invalid GeneralStats selection metadata".into()),
    }
}

pub(super) fn check(panel: &CyclopediaCharacterPanel, selected: Selection) {
    if let Some(id) = selected.vocation {
        assert_eq!(panel.vocation_id, Some(id));
    }
}

#[test]
fn live_selection_metadata_is_required_and_validated() -> TestResult<()> {
    assert!(
        !parse("vocation_selected=0\nvocation_id=none\n")?
            .extensions
            .vocation
    );
    assert_eq!(
        parse("vocation_selected=1\nvocation_id=1\n")?.vocation,
        Some(1)
    );
    for invalid in [
        "",
        "vocation_selected=0\n",
        "vocation_id=1\n",
        "vocation_selected=false\nvocation_id=none\n",
        "vocation_selected=1\nvocation_id=0\n",
        "vocation_selected=1\nvocation_id=12\n",
        "vocation_selected=0\nvocation_id=6\n",
        "vocation_selected=1\nvocation_id=6\nextra=1\n",
    ] {
        assert!(
            parse(invalid).is_err(),
            "accepted invalid metadata: {invalid:?}"
        );
    }
    Ok(())
}
