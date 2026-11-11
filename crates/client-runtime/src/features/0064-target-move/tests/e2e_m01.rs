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
            vec![vec![8, 2], vec![8, 1], vec![8, 1], vec![8, 2]],
            "autowalk=path-unavailable\nstop=accepted\nturn=accepted\nfollow=path-unavailable\n"
                .into(),
        )
    };
    let expected_rows = expected.lines().collect::<Vec<_>>();
    assert_eq!(expected_rows.len(), 4);
    assert_eq!(payloads.len(), 4);
    let mut visible = String::new();
    for ((name, payload), expected_row) in ["autowalk", "stop", "turn", "follow"]
        .into_iter()
        .zip(payloads)
        .zip(expected_rows)
    {
        let response = project(&GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        })?;
        let expected_value = expected_row
            .strip_prefix(&format!("{name}="))
            .ok_or("M01 golden row name mismatch")?;
        let expected_disposition = match expected_value {
            "accepted" => WalkDisposition::Accepted,
            "path-unavailable" => WalkDisposition::NoPath,
            _ => return Err("M01 invalid golden disposition".into()),
        };
        assert_eq!(
            response,
            WalkResponse {
                status: CommandStatus::Accepted,
                disposition: Some(expected_disposition)
            }
        );
        let observed = match response.disposition {
            Some(WalkDisposition::Accepted) => "accepted",
            Some(WalkDisposition::NoPath) => "path-unavailable",
            _ => return Err("M01 unexpected decoded disposition".into()),
        };
        visible.push_str(&format!("{name}={observed}\n"));
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
