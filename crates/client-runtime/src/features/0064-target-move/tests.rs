use super::*;

#[test]
fn projects_accepted_no_path_and_refused_receipts() -> Result<(), SessionError> {
    for (status, bytes, expected) in [
        (
            CommandStatus::Accepted,
            vec![8, 1],
            Some(WalkDisposition::Accepted),
        ),
        (
            CommandStatus::DuplicateReplay,
            vec![8, 1],
            Some(WalkDisposition::Accepted),
        ),
        (
            CommandStatus::Accepted,
            vec![8, 2],
            Some(WalkDisposition::NoPath),
        ),
        (
            CommandStatus::Rejected,
            vec![8, 3],
            Some(WalkDisposition::Rejected),
        ),
        (CommandStatus::Rejected, vec![], None),
    ] {
        let reply = GatedCommandResult {
            command_id: 7,
            status,
            server_sequence: 41,
            payload: bytes,
        };
        assert_eq!(
            project(&reply)?,
            WalkResponse {
                status,
                disposition: expected
            }
        );
    }
    Ok(())
}

#[test]
fn malformed_or_inconsistent_accepted_receipts_fail_closed() {
    for payload in [vec![], vec![8, 0], vec![8, 3], vec![8, 4], vec![8, 1, 8, 2]] {
        let reply = GatedCommandResult {
            command_id: 7,
            status: CommandStatus::Accepted,
            server_sequence: 41,
            payload,
        };
        assert!(project(&reply).is_err());
    }
}
