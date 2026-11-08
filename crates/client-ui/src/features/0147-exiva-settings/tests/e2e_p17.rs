use super::*;

// Replay owner: `ExivaPanel::apply` in
// `crates/client-ui/src/features/0147-exiva-settings/state.rs`, which decodes the command-result
// payload with `PlayerList::decode`, projected through `panel_text` in `view.rs`.
// The server journey denies everyone, adds a named player, refuses a malformed SetModes and
// removes the player; the three List payloads are replayed in that order.

fn expected_value<'a>(text: &'a str, key: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

#[test]
fn e2e_p17_replays_exiva_deny_add_and_remove_lists() -> Result<(), Box<dyn std::error::Error>> {
    let (denied, allowed, removed, expected) = if let Some(path) =
        std::env::var_os("E2E_ALL_CAPTURE_DIR")
    {
        let path = std::path::Path::new(&path);
        (
            std::fs::read(path.join("p17-denied.bin"))?,
            std::fs::read(path.join("p17-allowed.bin"))?,
            std::fs::read(path.join("p17-removed.bin"))?,
            std::fs::read_to_string(path.join("p17-expected.txt"))?,
        )
    } else {
        let list = |modes, players| PlayerList { modes, players }.encode();
        (
            list(0, Vec::new()).ok_or("deny fixture")?,
            list(2, vec![([9; 16], "Aldric Vane".into())]).ok_or("allow fixture")?,
            list(2, Vec::new()).ok_or("remove fixture")?,
            format!(
                "denied_modes=0\nallowed_modes=2\nplayer={} Aldric Vane\nremoved_modes=2\nrefused_status=Rejected\n",
                "09".repeat(16)
            ),
        )
    };
    let (hex, name) = expected_value(&expected, "player")?
        .split_once(' ')
        .ok_or("expected player line malformed")?;
    let mut id = [0_u8; 16];
    for (at, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(at * 2..at * 2 + 2).ok_or("short id")?, 16)?;
    }
    let modes = |key: &str| -> Result<u8, Box<dyn std::error::Error>> {
        Ok(expected_value(&expected, key)?.parse::<u8>()?)
    };

    let mut panel = ExivaPanel {
        selected: true,
        ..ExivaPanel::default()
    };
    // Deny everyone: mode 0 and an empty list.
    assert!(panel.apply(&denied));
    assert_eq!(
        panel.list,
        Some(PlayerList {
            modes: modes("denied_modes")?,
            players: Vec::new()
        })
    );
    assert_eq!(panel_text(&panel), "Exiva allow list (0/100)\r\n");

    // Add the named player: mode 2 and exactly the server's resolved id and name.
    assert!(panel.apply(&allowed));
    assert_eq!(
        panel.list,
        Some(PlayerList {
            modes: modes("allowed_modes")?,
            players: vec![(id, name.to_owned())]
        })
    );
    assert_eq!(
        panel_text(&panel),
        format!("Exiva allow list (1/100)\r\n{name}")
    );

    // A malformed or out-of-range payload leaves the last authoritative list untouched.
    let accepted = panel.list.clone();
    assert!(!panel.apply(&[0, 32]));
    assert_eq!(panel.list, accepted);
    assert_eq!(panel.error, Some("Exiva request refused; refresh the list"));
    let mut truncated = allowed.clone();
    truncated.pop();
    assert!(!panel.apply(&truncated));
    assert_eq!(panel.list, accepted);

    // Removal keeps the mode and empties the list.
    assert!(panel.apply(&removed));
    assert_eq!(
        panel.list,
        Some(PlayerList {
            modes: modes("removed_modes")?,
            players: Vec::new()
        })
    );
    assert_eq!(panel_text(&panel), "Exiva allow list (0/100)\r\n");
    Ok(())
}
