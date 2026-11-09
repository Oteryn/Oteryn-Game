use super::*;

#[test]
fn m51_busy_reservation_projects_only_the_committed_pit_and_victory_phase() {
    let before = MatchState {
        rank: 1,
        pit: 3,
        phase: 1,
        deadline_ms: 600_000,
        run: [7; 16],
        ..MatchState::default()
    };
    let mut proposal = before.clone();
    proposal.pit = 4;
    proposal.deadline_ms = 650_000;
    let mut committed = before.wire(0, [0; 3]);
    committed.phase = 2;
    proposal.reconcile_arena_refusal(&before, ArenaDisposition::Busy, &committed);
    assert_eq!(proposal.pit, 3);
    assert_eq!(proposal.phase, 2);
    assert_eq!(proposal.deadline_ms, 600_000);
    assert_eq!(proposal.run, before.run);
}

#[test]
fn m51_busy_or_unfunded_entry_cannot_project_an_unpaid_opponent() {
    let before = MatchState::default();
    for disposition in [
        ArenaDisposition::Busy,
        ArenaDisposition::InsufficientGold,
        ArenaDisposition::InventoryFull,
    ] {
        let mut proposal = MatchState {
            rank: 1,
            pit: 1,
            phase: 1,
            deadline_ms: 600_000,
            ..before.clone()
        };
        proposal.reconcile_arena_refusal(&before, disposition, &before.wire(0, [0; 3]));
        assert_eq!(proposal, before);
    }
}

#[test]
fn m51_restored_victory_keeps_the_original_kick_deadline() {
    let mut state = MatchState {
        rank: 1,
        pit: 3,
        phase: 2,
        deadline_ms: 600_000,
        ..MatchState::default()
    };
    assert_eq!(state.projection_remaining_ms(599_999), Some(1));
    assert_eq!(state.projection_remaining_ms(600_000), None);
    assert_eq!(state.projection_remaining_ms(600_001), None);
    state.phase = 1;
    assert_eq!(state.projection_remaining_ms(599_000), Some(1_000));
    state.phase = 3;
    assert_eq!(state.projection_remaining_ms(599_000), None);
    state.phase = 0;
    assert_eq!(state.projection_remaining_ms(599_000), None);
}

#[test]
fn m51_commit_clock_refuses_late_victory_and_portal_progress()
-> crate::bestiary_postgres_harness::TestResult {
    use crate::bestiary_postgres_harness as h;
    use oteryn_protocol_oteryn::arena::{ArenaAction as A, ArenaDisposition as D};
    let fence = h::fence(1)?;
    let command = CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(92).map_err(h::debug)?,
    );
    let before = MatchState {
        rank: 1,
        pit: 3,
        phase: 1,
        deadline_ms: 100_000,
        session: *fence.game_session_id.as_bytes(),
        scope_generation: fence.scope_ownership_generation.get(),
        run: h::id(230),
        ..MatchState::default()
    };
    let mut ready = before.clone();
    ready.phase = 2;
    let status =
        oteryn_protocol_oteryn::arena::encode_match_request(A::Status).map_err(h::debug)?;
    assert!(valid_transition(
        &before,
        &ready,
        &ready,
        D::Applied,
        &status,
        fence,
        command,
        99_999
    ));
    for now in [100_000, 100_001] {
        assert!(
            !valid_transition(
                &before,
                &ready,
                &ready,
                D::Applied,
                &status,
                fence,
                command,
                now
            ),
            "read-before-expiry cannot credit a death after the commit deadline"
        );
    }
    let advance =
        oteryn_protocol_oteryn::arena::encode_match_request(A::Advance).map_err(h::debug)?;
    for pit in [3, 10] {
        let mut before = ready.clone();
        before.pit = pit;
        let mut after = before.clone();
        if pit == 10 {
            after.completed = 1;
            after.pit = 0;
            after.phase = 3;
            after.deadline_ms = 0;
        } else {
            after.pit += 1;
            after.phase = 1;
            after.deadline_ms = 500_000;
        }
        assert!(valid_transition(
            &before,
            &before,
            &after,
            D::Applied,
            &advance,
            fence,
            command,
            99_999
        ));
        for now in [100_000, 100_001] {
            assert!(
                !valid_transition(
                    &before,
                    &before,
                    &after,
                    D::Applied,
                    &advance,
                    fence,
                    command,
                    now
                ),
                "an expired portal cannot renew or complete the run"
            );
            assert!(valid_transition(
                &before,
                &before,
                &before.idle(),
                D::TimedOut,
                &status,
                fence,
                command,
                now
            ));
        }
    }
    Ok(())
}
