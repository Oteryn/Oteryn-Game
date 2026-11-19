#[cfg(test)]
mod refusal_retention_tests {
    use super::*;

    fn session(tag: u8) -> Result<GameSessionId, String> {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        GameSessionId::decode(&bytes).map_err(|error| format!("fixture session: {error:?}"))
    }

    #[test]
    fn later_command_preserves_replay_current_future_and_server_occurrences() -> Result<(), String>
    {
        let current = session(1)?;
        let policy = ActivityRefusalPrune::BeforeCommand {
            session: current,
            connection_generation: 2,
            command: 7,
        };
        assert!(policy.includes(current, 2, Some(6)));
        assert!(policy.includes(current, 1, Some(6)));
        assert!(
            !policy.includes(current, 2, Some(7)),
            "current command still needs its retained result"
        );
        assert!(!policy.includes(current, 2, Some(8)));
        assert!(
            !policy.includes(current, 3, Some(6)),
            "obsolete connections cannot prune a successor"
        );
        assert!(
            !policy.includes(current, 2, None),
            "autonomous occurrences have no command high-water"
        );
        assert!(
            !policy.includes(session(2)?, 2, Some(6)),
            "another participant keeps its own cache"
        );
        Ok(())
    }

    #[test]
    fn proven_terminal_session_can_forget_its_own_server_refusals() -> Result<(), String> {
        let current = session(1)?;
        let policy = ActivityRefusalPrune::Terminal(current);
        assert!(policy.includes(current, 1, Some(7)));
        assert!(policy.includes(current, 2, None));
        assert!(!policy.includes(session(2)?, 2, None));
        Ok(())
    }
}
