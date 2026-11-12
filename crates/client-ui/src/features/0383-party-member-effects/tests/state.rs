use crate::party::{Intent, PartyPanel, panel_text_at, party, request};
use oteryn_client_runtime::party::{CommandStatus, PartyResponse};

#[test]
fn member_effects_replace_and_count_down() -> Result<(), Box<dyn std::error::Error>> {
    let mut panel = PartyPanel {
        selected: true,
        ..PartyPanel::default()
    };
    let mut state = party::PartyResult::refused(party::PartyDisposition::Ok);
    state.members.push(party::PartyEntry {
        list: Vec::new(),
        handle: 1,
        name: "Friend".into(),
        leader: false,
        expires_at: 0,
        party_spell_remaining_ms: Some([0, 120_000, 0, 0, 5_000]),
    });
    for (remaining, expected) in [
        (Some([0, 120_000, 0, 0, 5_000]), true),
        (Some([0; 5]), false),
        (None, false),
    ] {
        request(&mut panel, Intent::Refresh).ok_or("query")?;
        state.members[0].party_spell_remaining_ms = remaining;
        let bytes = party::encode_party_result(&state).map_err(|error| format!("{error:?}"))?;
        assert!(super::apply_result(
            &mut panel,
            &PartyResponse {
                status: CommandStatus::Accepted,
                operation: bytes.clone(),
                view: Some(bytes)
            }
        ));
        assert_eq!(
            panel_text_at(&panel, 1_000).contains("Protect Party: 119s"),
            expected
        );
        assert!(!panel_text_at(&panel, 120_000).contains("Protect Party:"));
        assert!(!panel_text_at(&panel, 5_000).contains("Enlighten Party:"));
    }
    Ok(())
}
