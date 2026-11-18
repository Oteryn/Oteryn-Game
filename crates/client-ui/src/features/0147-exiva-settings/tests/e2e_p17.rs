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
    // Deny everyone without discarding unrelated stored entries.
    assert!(panel.apply(&denied));
    let denied_list = panel.list.clone().ok_or("denied list absent")?;
    assert_eq!(denied_list.modes, modes("denied_modes")?);
    assert!(!denied_list.players.iter().any(|(member, _)| *member == id));
    let display = |list: &PlayerList| {
        let names = list
            .players
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>();
        format!(
            "Exiva allow list ({}/100)\r\n{}",
            names.len(),
            names.join("\r\n")
        )
    };
    assert_eq!(panel_text(&panel), display(&denied_list));

    // Add exactly the named player and preserve all other entries.
    assert!(panel.apply(&allowed));
    let allowed_list = panel.list.as_ref().ok_or("allowed list absent")?;
    assert_eq!(allowed_list.modes, modes("allowed_modes")?);
    assert!(allowed_list.players.contains(&(id, name.to_owned())));
    assert_eq!(allowed_list.players.len(), denied_list.players.len() + 1);
    assert!(
        allowed_list
            .players
            .iter()
            .filter(|(member, _)| *member != id)
            .eq(denied_list.players.iter())
    );
    assert_eq!(panel_text(&panel), display(allowed_list));

    // A malformed or out-of-range payload leaves the last authoritative list untouched.
    let accepted = panel.list.clone();
    assert!(!panel.apply(&[0, 32]));
    assert_eq!(panel.list, accepted);
    assert_eq!(panel.error, Some("Exiva request refused; refresh the list"));
    let mut truncated = allowed.clone();
    truncated.pop();
    assert!(!panel.apply(&truncated));
    assert_eq!(panel.list, accepted);

    // Removal keeps the mode and restores the other entries.
    assert!(panel.apply(&removed));
    let removed_list = panel.list.as_ref().ok_or("removed list absent")?;
    assert_eq!(removed_list.modes, modes("removed_modes")?);
    assert_eq!(removed_list.players, denied_list.players);
    assert_eq!(panel_text(&panel), display(removed_list));
    Ok(())
}
