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
