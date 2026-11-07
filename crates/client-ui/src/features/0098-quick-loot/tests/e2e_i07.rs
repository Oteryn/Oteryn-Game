//! I07 server-payload replay through the owning client projection.
//!
//! The payload consumer is `apply_result` in
//! `crates/client-ui/src/features/0098-quick-loot/protocol.rs`; it decodes each
//! `QuickLootResult` and updates `Panel` from
//! `crates/client-ui/src/features/0098-quick-loot/state.rs`.
use super::*;
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};

fn fixture() -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let payload = encode_quick_loot_result(&QuickLootResult { items: Vec::new() })
        .map_err(|error| format!("{error:?}"))?;
    Ok(vec![payload.clone(), payload.clone(), Vec::new(), payload])
}

#[test]
fn replays_i07_quick_loot_results_and_rejects_invalid_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let (payloads, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        (
            (1..=4)
                .map(|index| std::fs::read(path.join(format!("i07-{index}.bin"))))
                .collect::<Result<Vec<_>, _>>()?,
            std::fs::read_to_string(path.join("i07-expected.txt"))?,
        )
    } else {
        (
            fixture()?,
            "skip=3031 accept=3350 confirmed=2 results=0\n".into(),
        )
    };
    let actions = [
        Action::Add {
            mode: QuickLootFilterMode::Skip,
            definition: 3031,
        },
        Action::Add {
            mode: QuickLootFilterMode::Accept,
            definition: 3350,
        },
        Action::Destination {
            category: Some(QuickLootCategory::Gold),
            recipient: QuickLootRecipient::GoldPouch,
            id: Some([7; 16]),
            fallback: true,
        },
        Action::LootAll,
    ];
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    for (index, (action, payload)) in actions.into_iter().zip(payloads).enumerate() {
        let command = request(&mut panel, action);
        assert!(command.is_some());
        apply_result(
            &mut panel,
            &ServerMechanicResponse {
                command_id: 1,
                status: if index == 2 {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                server_sequence: 1,
                payload,
            },
        );
    }
    let result_count = panel.result.as_ref().map_or(0, |result| result.items.len());
    let visible = format!(
        "skip={} accept={} confirmed={} results={}\n",
        panel.skipped.iter().next().copied().unwrap_or_default(),
        panel.accepted.iter().next().copied().unwrap_or_default(),
        panel.confirmed.len(),
        result_count,
    );
    assert_eq!(visible, expected);

    let before = panel.clone();
    assert!(request(&mut panel, Action::LootAll).is_some());
    let stable = panel.clone();
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: vec![0xff],
        },
    );
    assert_eq!(panel.result, stable.result);
    assert_eq!(panel.skipped, stable.skipped);
    assert_eq!(panel.accepted, stable.accepted);
    assert_eq!(panel.confirmed, stable.confirmed);
    assert_eq!(
        panel.feedback,
        "Loot request refused or invalid reply; try again"
    );
    assert_eq!(before.result, panel.result);
    Ok(())
}
