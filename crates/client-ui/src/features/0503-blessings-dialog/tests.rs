use crate::cyclopedia_character::{CyclopediaCharacterPanel, apply_result, request_blessings};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
#[test]
fn blessings_dialog_replays_real_dispatch_capture() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var_os("OTERYN_BLESSINGS_DIALOG_CAPTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/features/0503-blessings-dialog/fixtures/server-reply.bin"
            ))
        });
    let bytes = std::fs::read(path)?;
    let snapshot = super::decode(&bytes).ok_or("invalid server capture")?;
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    panel.blessings.details_selected = true;
    let request = request_blessings(&mut panel).ok_or("request")?;
    assert_eq!(request.payload, [134]);
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 4,
            server_sequence: 4,
            status: CommandStatus::Accepted,
            payload: bytes.clone()
        }
    ));
    assert_eq!(panel.blessings.details.as_ref(), Some(&snapshot));
    assert_eq!(
        panel.blessings.holdings(),
        Some(snapshot.blessings.as_slice())
    );
    let text = panel.blessings.text();
    assert!(text.contains(&format!(
        "Experience loss protection: {}%",
        snapshot.experience_protection_percent
    )));
    assert!(text.contains("Skill/magic progress protection: 100%"));
    assert!(text.contains("Item chance"));
    assert!(text.contains("UTC"));
    for probability in [
        snapshot.forecast.pve.equipment_per_mille,
        snapshot.forecast.pve.container_per_mille,
    ] {
        assert!(text.contains(&format!("{}.{:01}%", probability / 10, probability % 10)));
    }
    assert_eq!(snapshot.history.len(), 3);
    assert!(text.contains("Acquired:"));
    assert!(!text.contains("oteryn:blessing."));
    assert!(!text.contains("Loss percentages are unavailable"));
    for end in 0..bytes.len() {
        assert!(super::decode(&bytes[..end]).is_none());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(super::decode(&trailing).is_none());
    Ok(())
}
