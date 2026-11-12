//! M01 replay: `target_move::project` in `features/0064-target-move/protocol.rs` consumes the
//! raw command-result payloads for autowalk, stop, turn, and follow. Movement deltas themselves
//! are applied by `Session::apply_state_delta` in `crates/session/src/domain_dispatch.rs`.

use crate::target_move::{WalkDisposition, WalkResponse, project};
use oteryn_session::{CommandStatus, GatedCommandResult};

#[test]
fn replays_m01_target_move_receipts() -> Result<(), Box<dyn std::error::Error>> {
    let (payloads, expected) = if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        (
            ["autowalk", "stop", "turn", "follow"]
                .map(|name| std::fs::read(root.join(format!("m01-{name}.bin"))))
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            std::fs::read_to_string(root.join("m01-expected.txt"))?,
        )
    } else {
        (
            vec![vec![8, 1]; 4],
            "autowalk=accepted\nstop=accepted\nturn=accepted\nfollow=accepted\n".into(),
        )
    };
    let mut visible = String::new();
    for (name, payload) in ["autowalk", "stop", "turn", "follow"]
        .into_iter()
        .zip(payloads)
    {
        let response = project(&GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        })?;
        assert_eq!(
            response,
            WalkResponse {
                status: CommandStatus::Accepted,
                disposition: Some(WalkDisposition::Accepted)
            }
        );
        visible.push_str(&format!("{name}=accepted\n"));
    }
    assert_eq!(visible, expected);

    let retained = visible.clone();
    assert!(
        project(&GatedCommandResult {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: vec![8, 3],
        })
        .is_err()
    );
    assert_eq!(visible, retained);
    Ok(())
}
