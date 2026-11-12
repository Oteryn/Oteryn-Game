//! S08 replay: the victim's raw PARTY SKULL query result, taken after an unjustified player attack,
//! applied by the production client reducer `apply` (`protocol.rs`). The request is built by the
//! production `command`; the reply is `s08-skull.bin` (the payload the server sent), and
//! `s08-expected.txt` names the attacker's entity and the white skull. Without captures an inline
//! payload built with the production `encode_party_icons` is used.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;
use oteryn_session::EntityRef;

const WHITE: u8 = 3;
const IDENTITY: usize = 16;

struct Capture {
    payload: Vec<u8>,
    identity: [u8; IDENTITY],
    generation: u64,
    skull: u8,
}

fn hex(text: &str) -> Result<[u8; IDENTITY], String> {
    let bytes = (0..text.len())
        .step_by(2)
        .map(|at| {
            text.get(at..at + 2)
                .ok_or_else(|| "odd hex".to_owned())
                .and_then(|pair| u8::from_str_radix(pair, 16).map_err(|e| e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    <[u8; IDENTITY]>::try_from(bytes.as_slice()).map_err(|_| "identity length".to_owned())
}

fn capture() -> Result<Capture, String> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("s08-expected.txt"))
            .map_err(|e| format!("s08-expected.txt: {e}"))?;
        let value = |key: &str| {
            expected
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or_else(|| format!("{key}= missing"))
        };
        return Ok(Capture {
            payload: std::fs::read(root.join("s08-skull.bin"))
                .map_err(|e| format!("s08-skull.bin: {e}"))?,
            identity: hex(value("identity")?)?,
            generation: value("generation")?.parse().map_err(|e| format!("{e}"))?,
            skull: value("skull")?.parse().map_err(|e| format!("{e}"))?,
        });
    }
    let identity = [0x5a; IDENTITY];
    let icon = |identity, generation, skull| wire::PartyIcon {
        entity: EntityRef {
            identity,
            generation,
        },
        shield: 0,
        skull,
    };
    Ok(Capture {
        payload: wire::encode_party_icons(&[
            icon([0x33; IDENTITY], 4, 0),
            icon(identity, 9, WHITE),
        ])
        .map_err(|e| format!("{e:?}"))?,
        identity,
        generation: 9,
        skull: WHITE,
    })
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn pending(selected: &Panel) -> Result<(Panel, FeatureCommand), String> {
    let command = command(selected, Intent::Query)?;
    let mut next = selected.clone();
    next.selected = true;
    assert!(next.begin(command.kind));
    Ok((next, command))
}

#[test]
fn e2e_s08_white_skull_after_unjustified_attack_replay() -> Result<(), String> {
    let capture = capture()?;
    assert_eq!(capture.skull, WHITE);
    let (mut panel, query) = pending(&Panel::default())?;
    assert_eq!(
        (query.capability, query.kind),
        (wire::CAPABILITY_PARTY_SKULL_V1, wire::COMMAND_TYPE_PARTY_SKULL_QUERY)
    );
    apply(&mut panel, &query, &result(CommandStatus::Accepted, &capture.payload));
    assert!(!panel.pending);
    assert_eq!(panel.feedback, "Party icons updated");
    let attacker = format!(
        "Entity {:02x?} / {} | Shield: None | Skull: White",
        capture.identity, capture.generation
    );
    assert_eq!(
        panel.summary.split("\r\n").filter(|line| **line == *attacker).count(),
        1,
        "{:?}",
        panel.summary
    );
    let shown = panel
        .icons
        .iter()
        .find(|icon| icon.entity.identity == capture.identity)
        .ok_or("attacker icon absent")?;
    assert_eq!((shown.entity.generation, shown.skull), (capture.generation, WHITE));

    // A refused status and an undecodable payload leave the shown icons unchanged.
    for (status, payload, message) in [
        (CommandStatus::Rejected, capture.payload.clone(), "The server refused the request"),
        (CommandStatus::Accepted, vec![0xff, 0x00], "Invalid party icons reply"),
    ] {
        let (mut bad, query) = pending(&panel)?;
        apply(&mut bad, &query, &result(status, &payload));
        assert!(!bad.pending);
        assert_eq!(bad.feedback, message);
        assert_eq!(bad.summary, panel.summary);
        assert_eq!(bad.icons, panel.icons);
    }
    Ok(())
}
