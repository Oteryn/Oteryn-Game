//! P08 replay: server weapon-proficiency replies (catalogue, browse, perk selection and a stale
//! RankUp refusal) applied through the production `request` and `apply_result`; the visible text
//! comes from `panel_text`.
use super::*;
use oteryn_client_runtime::{
    CommandStatus,
    raw_feature::GatedCommandResult,
    weapon_proficiency::{
        ModificationOperation, ModificationReply, ModificationResult,
        PROFICIENCY_CATALOGUE_PAGE_SIZE, ProficiencyCataloguePage, ProficiencyPage,
    },
};
use std::path::Path;

type Error = Box<dyn std::error::Error>;

struct Captured {
    catalogue: Vec<GatedCommandResult>,
    state: GatedCommandResult,
    browse: GatedCommandResult,
    select: GatedCommandResult,
    stale: GatedCommandResult,
    expected: String,
}

fn captured() -> Result<Captured, Error> {
    let path =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P08 live capture directory required")?;
    let session =
        std::env::var_os("P08_CAPTURE_SESSION").ok_or("P08 live capture session required")?;
    read_capture(Path::new(&path), &session)
}

fn read_capture(path: &Path, session: &std::ffi::OsStr) -> Result<Captured, Error> {
    let expected = std::fs::read_to_string(path.join("p08-expected.txt"))?;
    let captured_session = value(&expected, "session")?;
    if session != captured_session {
        return Err("P08 capture session does not match the live verdict".into());
    }
    let mut previous = (0, 0);
    let mut read = |name: &str| -> Result<GatedCommandResult, Error> {
        let metadata = std::fs::read_to_string(path.join(format!("p08-{name}.meta")))?;
        let mut fields = metadata.split_ascii_whitespace();
        let session = fields.next().ok_or("P08 missing frame session")?;
        let command_id = fields.next().ok_or("P08 missing command id")?.parse()?;
        let server_sequence = fields
            .next()
            .ok_or("P08 missing server sequence")?
            .parse()?;
        let status = fields.next().ok_or("P08 missing status")?.parse::<u32>()?;
        if session != captured_session
            || status != CommandStatus::Accepted as u32
            || command_id <= previous.0
            || server_sequence <= previous.1
            || fields.next().is_some()
        {
            return Err(format!("P08 invalid frame metadata for {name}").into());
        }
        previous = (command_id, server_sequence);
        Ok(GatedCommandResult {
            command_id,
            server_sequence,
            status: CommandStatus::Accepted,
            payload: std::fs::read(path.join(format!("p08-{name}.bin")))?,
        })
    };
    let offset = value(&expected, "wand_offset")?.parse::<usize>()?;
    let total = value(&expected, "catalogue_total")?.parse::<usize>()?;
    if offset >= total || offset % PROFICIENCY_CATALOGUE_PAGE_SIZE != 0 {
        return Err("P08 invalid Wand page offset".into());
    }
    let catalogue = (0..=offset)
        .step_by(PROFICIENCY_CATALOGUE_PAGE_SIZE)
        .map(|offset| read(&format!("catalogue-{offset}")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Captured {
        catalogue,
        browse: read("browse")?,
        state: read("state")?,
        select: read("select")?,
        stale: read("stale")?,
        expected,
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
#[ignore = "requires Node Boot live capture and matching session"]
fn e2e_p08_replays_catalogue_browse_select_and_stale_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let mut p = WeaponProficiencyPanel {
        selected: true,
        state_selected: true,
        ..WeaponProficiencyPanel::default()
    };

    let catalogue = request(&mut p, Intent::Catalogue).ok_or("catalogue request")?;
    assert_eq!(
        (catalogue.kind, catalogue.payload.as_slice()),
        (660, &[0, 0, 0, 0][..])
    );
    assert!(apply_result(&mut p, &captured.catalogue[0]));
    for (index, reply) in captured.catalogue.iter().enumerate().skip(1) {
        let command =
            tick(&mut p, std::time::Instant::now()).ok_or("P08 next catalogue request")?;
        let offset = u32::try_from(index * PROFICIENCY_CATALOGUE_PAGE_SIZE)?;
        assert_eq!(
            (command.kind, command.payload),
            (660, offset.to_be_bytes().to_vec())
        );
        assert!(apply_result(&mut p, reply));
    }
    let expected_entries = captured
        .catalogue
        .iter()
        .map(|reply| {
            ProficiencyCataloguePage::decode(&reply.payload).ok_or("P08 invalid captured catalogue")
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|page| page.entries)
        .collect::<Vec<_>>();
    assert_eq!(
        p.catalogue
            .as_ref()
            .ok_or("P08 client catalogue missing")?
            .entries,
        expected_entries
    );
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
    assert!(apply_result(&mut p, &captured.browse));
    let rows: u32 = value(e, "browse_rows")?.parse()?;
    let total = value(e, "browse_total")?;
    let header = format!("Weapons {}–{rows} / {total}", u32::from(rows > 0));
    assert!(panel_text(&p).lines().any(|line| line == header));
    assert_eq!(p.page, ProficiencyPage::decode(&captured.browse.payload));
    let confirmed = p.page.clone();
    let text = panel_text(&p);
    for row in &confirmed.as_ref().ok_or("P08 client Browse missing")?.rows {
        let line = format!("{}: {} XP, level {}", row.name, row.experience, row.level);
        assert!(
            text.lines().any(|visible| visible == line),
            "missing Browse row {line}"
        );
    }

    // SelectPerk sends the exact weapon/level/perk bytes and shows the server's typed reply.
    let mutation_definition: u32 = value(e, "mutation_definition")?.parse()?;
    let row = p
        .catalogue
        .as_ref()
        .ok_or("P08 missing catalogue")?
        .entries
        .iter()
        .position(|entry| {
            entry.definition == mutation_definition && entry.weapon == "oteryn:item.tibia.i3074"
        })
        .ok_or("P08 Wand missing from client catalogue")?;
    assert!(p.select_weapon(row));
    assert_eq!(
        p.weapon_entry().ok_or("P08 no selected Wand")?.levels[0].selected,
        None
    );
    assert!(
        request(&mut p, Intent::RankUp).is_none(),
        "RankUp must require committed shaping state"
    );
    let state = request(&mut p, Intent::State).ok_or("P08 shaping refresh")?;
    assert_eq!(
        (state.capability, state.kind, state.payload),
        (448, 732, mutation_definition.to_be_bytes().to_vec())
    );
    assert!(apply_result(&mut p, &captured.state));
    let revision = p
        .shaping
        .as_ref()
        .ok_or("P08 shaping state was not applied")?
        .revision;
    assert_ne!(revision, 0);
    let mut rank_panel = p.clone();
    p.level = 0;
    p.perk = 0;
    let select = request(&mut p, Intent::SelectPerk).ok_or("select request")?;
    let mut expected_payload = mutation_definition.to_be_bytes().to_vec();
    expected_payload.extend([0, 0]);
    assert_eq!((select.kind, select.payload), (378, expected_payload));
    assert!(apply_result(&mut p, &captured.select));
    assert_eq!(
        p.result,
        Some(format!(
            "Selection: {}; levels: {}",
            value(e, "select_result")?,
            value(e, "select_levels")?
        ))
    );

    assert_eq!(
        p.weapon_entry()
            .ok_or("P08 selected Wand disappeared")?
            .levels[0]
            .selected,
        Some(0)
    );

    // The prior panel snapshot loses the race to the committed selection.
    let p = &mut rank_panel;
    p.slot = 1;
    let stale = request(p, Intent::RankUp).ok_or("rank-up request")?;
    assert_eq!(stale.kind, ModificationOperation::RankUp.command_type());
    let mut expected_payload = mutation_definition.to_be_bytes().to_vec();
    expected_payload.push(1);
    expected_payload.extend(revision.to_be_bytes());
    assert_eq!(stale.payload, expected_payload);
    assert_eq!(
        ModificationReply::decode(&captured.stale.payload),
        Some(ModificationReply {
            result: ModificationResult::StaleRevision,
            slot: None,
        })
    );
    assert!(apply_result(p, &captured.stale));
    assert_eq!(
        p.result,
        Some(format!(
            "Modification: {}; slot: None",
            value(e, "stale_result")?
        ))
    );
    assert_eq!(p.page, confirmed);

    Ok(())
}

#[test]
fn browse_refusal_and_decode_failure_keep_the_confirmed_page() -> Result<(), Error> {
    let confirmed = Some(ProficiencyPage {
        offset: 0,
        total: 1,
        rows: vec![oteryn_client_runtime::weapon_proficiency::ProficiencyRow {
            definition: std::num::NonZeroU32::MIN,
            name: "Wand".into(),
            experience: 1750,
            level: 1,
            selections: vec![Some(0)],
        }],
    });
    let mut p = WeaponProficiencyPanel {
        selected: true,
        page: confirmed.clone(),
        ..Default::default()
    };
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

#[test]
fn rank_up_rejects_missing_or_mismatched_shaping_observation() -> Result<(), Error> {
    use oteryn_client_runtime::weapon_proficiency_state::State;
    let mut panel = WeaponProficiencyPanel {
        selected: true,
        state_selected: true,
        weapon: 17,
        slot: 1,
        revision: 99,
        ..WeaponProficiencyPanel::default()
    };
    assert!(request(&mut panel, Intent::RankUp).is_none());
    panel.shaping = Some(State {
        definition: 18,
        revision: 41,
        slots: [None, None],
        modify_dust: [0; 2],
        rank_dust: [0; 10],
        reshape_dust: 0,
        clear_dust: 0,
        orbs: 0,
    });
    assert!(request(&mut panel, Intent::RankUp).is_none());
    let state = panel.shaping.as_mut().ok_or("missing fixture state")?;
    state.definition = 17;
    state.revision = 0;
    assert!(request(&mut panel, Intent::RankUp).is_none());
    panel
        .shaping
        .as_mut()
        .ok_or("missing fixture state")?
        .revision = 41;
    let command = request(&mut panel, Intent::RankUp).ok_or("matching committed shaping state")?;
    assert_eq!(&command.payload[5..], &41_u64.to_be_bytes());
    Ok(())
}
