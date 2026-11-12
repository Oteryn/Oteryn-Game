//! E12 replay ownership: `apply_result` in `protocol.rs` consumes the hireling-management List
//! and Outfit-refusal replies; `panel_text` in `view.rs` projects them.
use super::*;
use oteryn_client_runtime::{CommandStatus, raw_feature::GatedCommandResult};

type Error = Box<dyn std::error::Error>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Result<[u8; 16], Error> {
    let mut out = [0; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2).ok_or("short hex")?, 16)?;
    }
    Ok(out)
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .ok_or_else(|| format!("expected.txt lacks {key}").into())
}

fn capture() -> Result<(Vec<u8>, Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("e12-list.bin"))?,
            std::fs::read(root.join("e12-refused.bin"))?,
            std::fs::read_to_string(root.join("e12-expected.txt"))?,
        ));
    }
    let mut id = [0x12; 16];
    id[6] = 0x72;
    id[8] = 0x82;
    let entry = wire::Entry {
        hireling: id,
        name: "Lamp Keeper".into(),
        sex: 2,
        look_type: 1107,
        colors: [1, 2, 3, 4],
        skills: 3,
        outfits: 1,
        position: None,
    };
    Ok((
        wire::Reply::State(vec![entry])
            .encode()
            .ok_or("encode list")?,
        wire::Reply::Refused(wire::Refusal::OutfitNotOwned)
            .encode()
            .ok_or("encode refusal")?,
        format!(
            "entries=1\nhireling={}\nname=Lamp Keeper\nsex=2\nlook_type=1107\ncolors=[1, 2, 3, 4]\noutfits=1\nskills=3\nplaced=none\nrefused_look=1125\nrefusal=OutfitNotOwned\n",
            hex(&id)
        ),
    ))
}

fn result(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}

#[test]
fn e12_real_server_payloads_replay_into_hireling_panel() -> Result<(), Error> {
    let (list, refused, expected) = capture()?;
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let command = request(&mut panel, Intent::List).ok_or("list")?;
    assert_eq!(
        (command.capability, command.kind, command.payload),
        (250, 470, vec![0])
    );
    assert!(apply_result(&mut panel, &result(list)));
    assert_eq!(
        panel.entries.len(),
        value(&expected, "entries")?.parse::<usize>()?
    );
    let entry = &panel.entries[0];
    assert_eq!(entry.hireling, unhex(value(&expected, "hireling")?)?);
    assert_eq!(entry.name, value(&expected, "name")?);
    assert_eq!(entry.sex, value(&expected, "sex")?.parse::<u8>()?);
    assert_eq!(
        entry.look_type,
        value(&expected, "look_type")?.parse::<u16>()?
    );
    assert_eq!(format!("{:?}", entry.colors), value(&expected, "colors")?);
    assert_eq!(entry.outfits, value(&expected, "outfits")?.parse::<u16>()?);
    assert_eq!(entry.skills, value(&expected, "skills")?.parse::<u8>()?);
    assert_eq!(
        entry
            .position
            .map_or_else(|| "none".to_owned(), |(x, y, z)| format!("{x},{y},{z}")),
        value(&expected, "placed")?
    );
    let text = panel_text(&panel);
    assert!(text.contains(&format!(
        "{}: {} ({})",
        entry.name,
        hex(&entry.hireling),
        if entry.sex == 2 { "female" } else { "male" }
    )));
    // The refused outfit is not selectable client-side, so the panel never sends it; the
    // server's own refusal reply is applied against that pending request.
    let before = panel.entries.clone();
    let look_type: u16 = value(&expected, "refused_look")?.parse()?;
    assert!(
        !outfits(&panel.entries[0])
            .iter()
            .any(|(_, look)| *look == look_type)
    );
    panel.pending = Some(wire::Request::Outfit {
        hireling: before[0].hireling,
        look_type,
        colors: [1, 2, 3, 4],
    });
    assert!(!apply_result(&mut panel, &result(refused)));
    assert_eq!(panel.feedback, Some("This outfit is not owned"));
    assert_eq!(panel.entries, before);
    assert_eq!(panel.pending, None);

    // A malformed reply is reported and leaves the entries unchanged.
    assert!(request(&mut panel, Intent::List).is_some());
    assert!(!apply_result(&mut panel, &result(vec![0xff])));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid hireling management data")
    );
    assert_eq!(panel.entries, before);
    Ok(())
}
