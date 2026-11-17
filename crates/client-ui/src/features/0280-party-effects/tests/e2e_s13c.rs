//! S13c client replay: the party spell's condition shows on both members' PARTY EFFECTS panel.
//! The raw `PARTY_EFFECTS_QUERY` result payloads the server sent to the caster and the member
//! (`s13c-caster-effects.bin`, `s13c-member-effects.bin`) are fed to `PartyEffectsPanel::finish`
//! (`state.rs`), read through `panel_text` (`view.rs`); `s13c-expected.txt` names the spell slot.
//! Without captures an inline payload built with the production `PartyEffects::encode` is used.
use super::*;

const LABELS: [&str; 5] = [
    "Train Party",
    "Protect Party",
    "Enchant Party",
    "Heal Party",
    "Enlighten Party",
];

fn inline(slot: usize, remaining_ms: u32) -> Result<Vec<u8>, &'static str> {
    let mut party_spell_remaining_ms = [0; 5];
    party_spell_remaining_ms[slot] = remaining_ms;
    PartyEffects {
        in_party: true,
        shared_enabled: true,
        shared_active: false,
        eligible: false,
        bonuses: [0, 0, 0, 0, 0],
        party_spell_remaining_ms,
    }
    .encode()
    .ok_or("inline party effects encode")
}

fn payloads() -> Result<(usize, Vec<u8>, Vec<u8>), String> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok((
            0,
            inline(0, 120_000).map_err(str::to_owned)?,
            inline(0, 119_000).map_err(str::to_owned)?,
        ));
    };
    let root = std::path::Path::new(&root);
    let read = |name: &str| std::fs::read(root.join(name)).map_err(|e| format!("{name}: {e}"));
    let expected = std::fs::read_to_string(root.join("s13c-expected.txt"))
        .map_err(|e| format!("s13c-expected.txt: {e}"))?;
    let slot = expected
        .lines()
        .find_map(|line| line.strip_prefix("slot="))
        .ok_or("slot= missing")?
        .parse::<usize>()
        .map_err(|e| format!("slot: {e}"))?;
    Ok((
        slot,
        read("s13c-caster-effects.bin")?,
        read("s13c-member-effects.bin")?,
    ))
}

#[test]
fn e2e_s13c_party_spell_condition_shows_on_both_members() -> Result<(), String> {
    let (slot, caster, member) = payloads()?;
    let label = *LABELS.get(slot).ok_or("slot out of range")?;
    for (who, payload) in [("caster", caster), ("member", member)] {
        let mut panel = PartyEffectsPanel {
            selected: true,
            ..PartyEffectsPanel::default()
        };
        assert_eq!(request(&mut panel), Some(Vec::new()), "{who}");
        assert!(panel.finish(true, &payload), "{who}");
        let value = panel.snapshot.ok_or("snapshot absent")?;
        assert!(value.in_party, "{who} is in the party");
        assert!(
            value.party_spell_remaining_ms[slot] > 0,
            "{who} carries the condition"
        );
        for (other, remaining) in value.party_spell_remaining_ms.iter().enumerate() {
            if other != slot {
                assert_eq!(*remaining, 0, "{who} slot {other}");
            }
        }
        let text = panel_text(&panel);
        let shown = text
            .split("\r\n")
            .filter(|line| line.contains(" Party: "))
            .collect::<Vec<_>>();
        let seconds = value.party_spell_remaining_ms[slot].div_ceil(1000);
        assert_eq!(shown, vec![format!("{label}: {seconds}s")], "{who}");

        let retained = panel.snapshot;
        assert!(!panel.finish(false, &[]));
        assert_eq!(panel.snapshot, retained);
        assert!(!panel.finish(true, &[0]));
        assert_eq!(panel.snapshot, retained);
        assert_eq!(
            panel_text(&panel),
            "Party effects could not be refreshed. Try Refresh."
        );
    }
    Ok(())
}
