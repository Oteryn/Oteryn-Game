//! Client owner exercised here:
//! - `crates/client-ui/src/features/0132-inspection/protocol.rs::apply_result`
//!   decodes and applies the server command-result payload to `Panel`.
//! - `crates/client-ui/src/features/0132-inspection/view.rs::panel_text`
//!   projects the applied item, imbuement attributes, and weight for the player.
use super::*;
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};

#[test]
fn e2e_i16_inspection_replay() -> Result<(), Box<dyn std::error::Error>> {
    let (payload, expected) = if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        (
            std::fs::read(directory.join("i16-inspection.bin"))?,
            std::fs::read_to_string(directory.join("i16-expected.txt"))?,
        )
    } else {
        let reply = Reply {
            catalogue: None,
            status: 1,
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 42,
                count: 1,
                tier: 0,
                name: "Fixture blade".into(),
                attributes: vec![
                    (
                        "Imbuement Slot 1".into(),
                        "Basic Strike, lasts 20:00h while fighting.".into(),
                    ),
                    ("Weight".into(), "35.00 oz".into()),
                ],
            }],
            requests: vec![],
        };
        let payload = encode_reply(&reply).map_err(|error| format!("{error:?}"))?;
        let mut fixture = Panel {
            selected: true,
            pending: true,
            ..Panel::default()
        };
        apply_result(&mut fixture, &response(payload.clone()));
        (payload, panel_text(&fixture))
    };

    let mut panel = Panel {
        selected: true,
        pending: true,
        ..Panel::default()
    };
    apply_result(&mut panel, &response(payload));
    assert_eq!(panel_text(&panel), expected);
    let applied = panel.reply.clone();

    panel.pending = true;
    apply_result(&mut panel, &response(vec![0xff]));
    assert_eq!(panel.reply, applied);
    assert_eq!(
        panel.feedback,
        "Inspection refused or invalid reply; try again"
    );
    Ok(())
}

fn response(payload: Vec<u8>) -> ServerMechanicResponse {
    ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}
