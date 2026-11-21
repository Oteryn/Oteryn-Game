use super::*;
use oteryn_client_runtime::{
    raw_feature::{CommandStatus, GatedCommandResult},
    tutorial_vocation as wire,
};
#[test]
fn tutorial_vocation_replays_real_dispatch_offer_and_submission()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = VocationPanel::default();
    assert!(request(&mut panel, None).is_none());
    panel.selected = true;
    assert!(request(&mut panel, Some(1)).is_none());
    let query = request(&mut panel, None).ok_or("query")?;
    assert!(query.payload.is_empty());
    assert_eq!((query.capability, query.kind), (504, 6243));
    {
        let directory = std::env::var_os("OTERYN_TUTORIAL_VOCATION_CAPTURE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/features/0504-tutorial-vocation/fixtures"
                ))
            });
        let offer = std::fs::read(directory.join("offer.bin"))?;
        apply_result(
            &mut panel,
            &GatedCommandResult {
                command_id: 2,
                server_sequence: 2,
                status: CommandStatus::Accepted,
                payload: offer,
            },
        );
        assert_eq!(
            choices(&panel),
            vec![
                (1, "Knight"),
                (2, "Sorcerer"),
                (3, "Druid"),
                (4, "Paladin"),
                (5, "Monk")
            ]
        );
        let command = request(&mut panel, Some(4)).ok_or("selection")?;
        assert_eq!(command.payload, [4]);
        let selected = std::fs::read(directory.join("selected.bin"))?;
        apply_result(
            &mut panel,
            &GatedCommandResult {
                command_id: 3,
                server_sequence: 3,
                status: CommandStatus::Accepted,
                payload: selected,
            },
        );
        assert_eq!(
            panel.observation,
            Some(wire::Observation {
                offered: false,
                chosen: 4
            })
        );
        assert!(choices(&panel).is_empty());
        assert!(request(&mut panel, Some(1)).is_none());
    }
    Ok(())
}
