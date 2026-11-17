//! Qualification-only GeneralStats boundary; the production panel owns decoding.
//! GeneralStats uses capability 487; capability 488 extends history tabs only.
use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime, client_ui};

pub(super) fn selected(client: &DevClientSession) -> bool {
    client.selected_capabilities().contains(
        &oteryn_protocol_oteryn::cyclopedia_character::CAPABILITY_CYCLOPEDIA_CHARACTER_VOCATION_V1,
    )
}

pub(super) fn validate(payload: &[u8], vocation_selected: bool) -> TestResult<Option<u8>> {
    use client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent, TabExtensions};
    if payload.len() != 42 + usize::from(vocation_selected) {
        return Err("GeneralStats length does not match negotiated capability 487".into());
    }
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        extensions: TabExtensions {
            vocation: vocation_selected,
            death_details: false,
        },
        ..Default::default()
    };
    client_ui::cyclopedia_character::request(&mut panel, Intent::Tab)
        .ok_or("GeneralStats validation request unavailable")?;
    // This local envelope only invokes the production decoder. Its placeholder command/
    // sequence numbers are never sent or captured as live command/provenance evidence.
    let reply = client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: payload.to_vec(),
    };
    if !client_ui::cyclopedia_character::apply_result(&mut panel, &reply) {
        return Err("GeneralStats rejected by negotiated production decoder".into());
    }
    Ok(panel.vocation_id)
}

/// Each raw capture retains its own actual negotiated mode and decoded vocation.
pub(super) fn record_selection(name: &str, payload: &[u8], vocation_selected: bool) -> TestResult {
    let vocation = validate(payload, vocation_selected)?;
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::write(
            Path::new(&directory).join(format!("{name}.selection")),
            format!(
                "vocation_selected={}\nvocation_id={}\n",
                u8::from(vocation_selected),
                vocation.map_or_else(|| "none".to_owned(), |id| id.to_string()),
            ),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::cyclopedia_character::{GeneralStats, TabExtension, TabView};

    #[test]
    fn selected_general_stats_are_strict_and_keep_the_real_vocation() -> TestResult {
        let view = TabView::GeneralStats(GeneralStats::default());
        let legacy = view.encode().map_err(|_| "legacy stats encode")?;
        let none = oteryn_protocol_oteryn::cyclopedia_character::vocation_id("none")
            .ok_or("canonical none vocation")?;
        assert_eq!(none, 1);
        let selected = view
            .encode_extended(&TabExtension {
                vocation_id: Some(none),
                ..Default::default()
            })
            .map_err(|_| "selected stats encode")?;
        assert_eq!(validate(&legacy, false)?, None);
        assert_eq!(validate(&selected, true)?, Some(none));
        assert!(validate(&legacy, true).is_err());
        assert!(validate(&selected, false).is_err());
        for invalid in [0, 12, u8::MAX] {
            let mut bytes = selected.clone();
            bytes[42] = invalid;
            assert!(validate(&bytes, true).is_err());
        }
        let mut trailing = selected;
        trailing.push(1);
        assert!(validate(&trailing, true).is_err());
        Ok(())
    }
}
