//! P08 replay: server weapon-proficiency replies (catalogue, browse, perk selection and a stale
//! RankUp refusal) applied through the production `request` and `apply_result`; the visible text
//! comes from `panel_text`.
use super::*;
use oteryn_client_runtime::{
    CommandStatus,
    raw_feature::GatedCommandResult,
    weapon_proficiency::{
        ModificationOperation, ModificationReply, ModificationResult, ProficiencyCatalogueEntry,
        ProficiencyCatalogueLevel, ProficiencyCataloguePage, ProficiencyPage, ProficiencyRow,
        SelectPerkReply, SelectPerkResult,
    },
};
use std::num::NonZeroU32;

type Error = Box<dyn std::error::Error>;

struct Captured {
    catalogue: Vec<u8>,
    browse: Vec<u8>,
    select: Vec<u8>,
    stale: Vec<u8>,
    expected: String,
}

fn fixture() -> Result<Captured, Error> {
    let none = || Error::from("fixture encoder refused");
    let catalogue = ProficiencyCataloguePage {
        offset: 0,
        total: 1,
        entries: vec![ProficiencyCatalogueEntry {
            definition: 258,
            weapon: "oteryn:item.wand".into(),
            name: "Wand".into(),
            experience: 120,
            thresholds: [10, 20, 30, 40, 50, 60, 70, 80, 90],
            levels: vec![ProficiencyCatalogueLevel {
                selected: None,
                perks: vec!["+5% damage".into(), "+2 mana".into()],
            }],
        }],
    };
    let browse = ProficiencyPage {
        offset: 0,
        total: 1,
        rows: vec![ProficiencyRow {
            definition: NonZeroU32::new(258).ok_or("definition")?,
            name: "Wand".into(),
            experience: 120,
            level: 1,
            selections: vec![None],
        }],
    };
    let select = SelectPerkReply {
        result: SelectPerkResult::Accepted,
        selections: vec![Some(0)],
    };
    let stale = ModificationReply {
        result: ModificationResult::StaleRevision,
        slot: None,
    };
    Ok(Captured {
        catalogue: catalogue.encode().ok_or_else(none)?,
        browse: browse.encode().ok_or_else(none)?,
        select: select.encode().ok_or_else(none)?,
        stale: stale.encode(),
        expected: "catalogue_total=1\ncatalogue_first=258|Wand|120\nbrowse_total=1\nbrowse_rows=1\nselect_result=Accepted\nselect_levels=[Some(0)]\nstale_result=StaleRevision\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        catalogue: std::fs::read(path.join("p08-catalogue.bin"))?,
        browse: std::fs::read(path.join("p08-browse.bin"))?,
        select: std::fs::read(path.join("p08-select.bin"))?,
        stale: std::fs::read(path.join("p08-stale.bin"))?,
        expected: std::fs::read_to_string(path.join("p08-expected.txt"))?,
    })
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
        status,
        server_sequence: 1,
        payload: payload.to_vec(),
    }
}

#[test]
fn e2e_p08_replays_catalogue_browse_select_and_stale_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let mut p = WeaponProficiencyPanel {
        selected: true,
        ..WeaponProficiencyPanel::default()
    };

    let catalogue = request(&mut p, Intent::Catalogue).ok_or("catalogue request")?;
    assert_eq!(
        (catalogue.kind, catalogue.payload.as_slice()),
        (660, &[0, 0, 0, 0][..])
    );
    assert!(apply_result(
        &mut p,
        &result(CommandStatus::Accepted, &captured.catalogue)
    ));
    let [definition, name, experience]: [&str; 3] = value(e, "catalogue_first")?
        .splitn(3, '|')
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "catalogue_first fields")?;
    let text = panel_text(&p);
    let entry = p
        .catalogue
        .as_ref()
        .and_then(|page| page.entries.first())
        .ok_or("catalogue entry")?;
    assert_eq!(entry.definition, definition.parse::<u32>()?);
    let weapon = format!("{name}: {experience} XP");
    assert!(text.lines().any(|line| line == weapon));
    assert_eq!(
        p.catalogue.as_ref().map(|c| c.total.to_string()).as_deref(),
        Some(value(e, "catalogue_total")?)
    );

    let browse = request(&mut p, Intent::Browse).ok_or("browse request")?;
    assert_eq!(
        (browse.kind, browse.payload.as_slice()),
        (641, &[0, 0, 0, 0][..])
    );
    assert!(apply_result(
        &mut p,
        &result(CommandStatus::Accepted, &captured.browse)
    ));
    let rows: u32 = value(e, "browse_rows")?.parse()?;
    let total = value(e, "browse_total")?;
    let header = format!("Weapons {}–{rows} / {total}", u32::from(rows > 0));
    assert!(panel_text(&p).lines().any(|line| line == header));
    let confirmed = p.page.clone();

    // SelectPerk sends the exact weapon/level/perk bytes and shows the server's typed reply.
    p.weapon = definition.parse()?;
    p.level = 0;
    p.perk = 0;
    let select = request(&mut p, Intent::SelectPerk).ok_or("select request")?;
    let mut expected_payload = p.weapon.to_be_bytes().to_vec();
    expected_payload.extend([0, 0]);
    assert_eq!((select.kind, select.payload), (378, expected_payload));
    assert!(apply_result(
        &mut p,
        &result(CommandStatus::Accepted, &captured.select)
    ));
    assert_eq!(
        p.result,
        Some(format!(
            "Selection: {}; levels: {}",
            value(e, "select_result")?,
            value(e, "select_levels")?
        ))
    );

    // A RankUp bound to a stale revision is refused with the typed result and no slot.
    p.slot = 1;
    p.revision = u64::MAX;
    let stale = request(&mut p, Intent::RankUp).ok_or("rank-up request")?;
    assert_eq!(stale.kind, ModificationOperation::RankUp.command_type());
    assert!(apply_result(
        &mut p,
        &result(CommandStatus::Accepted, &captured.stale)
    ));
    assert_eq!(
        p.result,
        Some(format!(
            "Modification: {}; slot: None",
            value(e, "stale_result")?
        ))
    );
    assert_eq!(p.page, confirmed);

    // A rejected status and a malformed or mismatched page leave the confirmed page alone.
    request(&mut p, Intent::Browse).ok_or("browse request")?;
    assert!(!apply_result(&mut p, &result(CommandStatus::Rejected, &[])));
    assert_eq!(
        p.feedback,
        Some("The server refused the proficiency action")
    );
    assert_eq!(p.page, confirmed);
    request(&mut p, Intent::Browse).ok_or("browse request")?;
    assert!(!apply_result(
        &mut p,
        &result(CommandStatus::Accepted, &[0xff, 0xff, 0xff])
    ));
    assert_eq!(
        p.feedback,
        Some("The server returned an invalid proficiency page")
    );
    assert_eq!(p.page, confirmed);
    Ok(())
}
