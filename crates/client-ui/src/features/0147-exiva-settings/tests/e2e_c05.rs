use super::*;

// Replay owner: `ExivaPanel::apply` in
// `crates/client-ui/src/features/0147-exiva-settings/state.rs`, which decodes the
// command-result payload with `PlayerList::decode` and projects it through
// `panel_text` in `crates/client-ui/src/features/0147-exiva-settings/view.rs`.
// The server journey changes the allow modes (empty list), then adds a named player; both List
// payloads are replayed in that order and the expected modes, id and name are the server's.

type Expectation = (u8, [u8; 16], String);

fn parse_expected(text: &str) -> Result<Expectation, Box<dyn std::error::Error>> {
    let mut modes = None;
    let mut player = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("modes=") {
            modes = Some(value.parse::<u8>()?);
        } else if let Some(value) = line.strip_prefix("player=") {
            player = value.split_once(' ');
        }
    }
    let (hex, name) = player.ok_or("expected player absent")?;
    let mut id = [0_u8; 16];
    for (at, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(at * 2..at * 2 + 2).ok_or("short player id")?, 16)?;
    }
    Ok((modes.ok_or("expected modes absent")?, id, name.to_owned()))
}

#[test]
fn e2e_c05_replays_exiva_mode_change_and_player_list() -> Result<(), Box<dyn std::error::Error>> {
    let (modes_payload, list_payload, (modes, id, name)) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("c05-modes.bin"))?,
                std::fs::read(path.join("c05-list.bin"))?,
                parse_expected(&std::fs::read_to_string(path.join("c05-expected.txt"))?)?,
            )
        } else {
            let modes_only = PlayerList {
                modes: 21,
                players: Vec::new(),
            };
            let with_player = PlayerList {
                modes: 21,
                players: vec![([7; 16], "Vaelor Quin".into())],
            };
            (
                modes_only
                    .encode()
                    .ok_or("could not encode C05 modes fixture")?,
                with_player
                    .encode()
                    .ok_or("could not encode C05 list fixture")?,
                (21, [7; 16], "Vaelor Quin".to_owned()),
            )
        };

    let mut panel = ExivaPanel {
        selected: true,
        ..ExivaPanel::default()
    };
    // Step 1: the mode change arrives with an empty allow list.
    assert!(panel.apply(&modes_payload));
    assert_eq!(
        panel.list,
        Some(PlayerList {
            modes,
            players: Vec::new()
        })
    );
    assert_eq!(panel_text(&panel), "Exiva allow list (0/100)\r\n");

    // Step 2: the named player is added; the panel shows exactly the server's name.
    assert!(panel.apply(&list_payload));
    assert_eq!(
        panel.list,
        Some(PlayerList {
            modes,
            players: vec![(id, name.clone())]
        })
    );
    assert_eq!(
        panel_text(&panel),
        format!("Exiva allow list (1/100)\r\n{name}")
    );

    // A malformed payload and a mode byte outside the five Exiva bits are refused and leave the
    // last authoritative list untouched.
    let accepted = panel.list.clone();
    assert!(!panel.apply(&[0]));
    assert_eq!(panel.list, accepted);
    assert_eq!(panel.error, Some("Exiva request refused; refresh the list"));
    assert!(!panel.apply(&[2, 0xe0, 0]));
    assert_eq!(panel.list, accepted);
    Ok(())
}
