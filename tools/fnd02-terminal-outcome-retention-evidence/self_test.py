#!/usr/bin/env python3
"""Focused deterministic tests for FND-02 terminal-outcome retention evidence."""

from __future__ import annotations

from model import (
    RECORD_FIXED_LOGICAL_BYTES,
    STORE_FIXED_LOGICAL_BYTES,
    U64_MAX,
    OutOfOrderTerminal,
    RetentionAdmissionFailed,
    SequenceGap,
    SessionRetentionModel,
    checked_add_u64,
    checked_mul_u64,
    first_playable_two_session_probe,
    semantic,
    synthetic_candidate_envelopes,
    synthetic_record_charge,
)


def store(*, count: int, record_bytes: int, records_fit: int | None = None) -> SessionRetentionModel:
    if records_fit is None:
        records_fit = count
    return SessionRetentionModel(
        max_records=count,
        max_charged_bytes=STORE_FIXED_LOGICAL_BYTES + records_fit * record_bytes,
    )


def test_first_playable_two_session_witness_needs_one_record_per_session() -> None:
    result = first_playable_two_session_probe()
    assert result["session_a_retained_count"] == 1
    assert result["session_b_retained_count"] == 1
    assert result["per_gamesession_count_lower_bound_demonstrated"] == 1
    assert result["cross_session_total_records_in_witness"] == 2
    assert result["replay_kind"] == "REPLAY_RETAINED_OUTCOME"
    assert result["replay_outcome"] == "OPENED"
    assert result["production_byte_claim"] is False


def test_zero_retention_cannot_satisfy_replayability_preflight() -> None:
    sem = semantic("OPEN", "binding=A")
    record_bytes = synthetic_record_charge(len(sem.normalized_intent), len(sem.binding), 2)
    s = store(count=0, record_bytes=record_bytes, records_fit=1)
    assert s.reserve(1, sem) == "RESERVED"
    before = s.snapshot()
    try:
        s.terminalize(1, b"OK")
    except RetentionAdmissionFailed:
        pass
    else:
        raise AssertionError("zero-retention envelope accepted replayable terminalization")
    assert s.snapshot() == before
    assert s.gameplay_mutations == 0


def test_same_input_replays_original_outcome_without_second_mutation() -> None:
    sem = semantic("OPEN", "binding=A")
    record_bytes = synthetic_record_charge(len(sem.normalized_intent), len(sem.binding), 6)
    s = store(count=1, record_bytes=record_bytes)
    s.reserve(1, sem)
    s.terminalize(1, b"OPENED")
    before = s.gameplay_mutations
    replay = s.classify_duplicate(1, sem)
    assert replay["kind"] == "REPLAY_RETAINED_OUTCOME"
    assert replay["outcome"] == b"OPENED"
    assert s.gameplay_mutations == before


def test_changed_normalized_intent_or_binding_conflicts() -> None:
    original = semantic("OPEN", "binding=A")
    changed_intent = semantic("CLOSE", "binding=A")
    changed_binding = semantic("OPEN", "binding=B")
    record_bytes = synthetic_record_charge(
        len(original.normalized_intent), len(original.binding), 6
    )
    s = store(count=1, record_bytes=record_bytes)
    s.reserve(1, original)
    s.terminalize(1, b"OPENED")
    assert s.classify_duplicate(1, changed_intent)["kind"] == "CONFLICT_CHANGED_INPUT"
    assert s.classify_duplicate(1, changed_binding)["kind"] == "CONFLICT_CHANGED_INPUT"


def test_count_boundary_and_one_over_evict_oldest_terminal_only() -> None:
    sem1 = semantic("ONE", "binding=1")
    sem2 = semantic("TWO", "binding=2")
    sem3 = semantic("THREE", "binding=3")
    record_bytes = max(
        synthetic_record_charge(len(x.normalized_intent), len(x.binding), 2)
        for x in (sem1, sem2, sem3)
    )
    s = store(count=2, record_bytes=record_bytes, records_fit=2)
    for command_id, sem in ((1, sem1), (2, sem2)):
        s.reserve(command_id, sem)
        s.terminalize(command_id, b"OK")
    assert tuple(s.retained) == (1, 2)
    s.reserve(3, sem3)
    plan = s.terminalize(3, b"OK")
    assert plan.evict_command_ids == (1,)
    assert tuple(s.retained) == (2, 3)
    assert s.classify_duplicate(1, sem1)["kind"] == "OUTCOME_EXPIRED"


def test_byte_boundary_and_one_over_evict_deterministically() -> None:
    sem = semantic("OPEN", "binding=A")
    record_bytes = synthetic_record_charge(len(sem.normalized_intent), len(sem.binding), 2)
    s = store(count=8, record_bytes=record_bytes, records_fit=2)
    for command_id in (1, 2):
        current = semantic("OPEN", f"binding={command_id}")
        # Keep equal-length bindings so exact byte arithmetic is stable.
        assert len(current.binding) == len(sem.binding)
        s.reserve(command_id, current)
        s.terminalize(command_id, b"OK")
    assert s.retained_charged_bytes == STORE_FIXED_LOGICAL_BYTES + 2 * record_bytes
    third = semantic("OPEN", "binding=3")
    s.reserve(3, third)
    plan = s.terminalize(3, b"OK")
    assert plan.evict_command_ids == (1,)
    assert tuple(s.retained) == (2, 3)


def test_single_record_byte_overflow_fails_before_gameplay_mutation() -> None:
    sem = semantic("OPEN", "binding=A")
    record_bytes = synthetic_record_charge(len(sem.normalized_intent), len(sem.binding), 7)
    s = SessionRetentionModel(
        max_records=1,
        max_charged_bytes=STORE_FIXED_LOGICAL_BYTES + record_bytes - 1,
    )
    s.reserve(1, sem)
    called = False

    def mutate() -> None:
        nonlocal called
        called = True

    before = s.snapshot()
    try:
        s.terminalize(1, b"TOOLONG", gameplay_mutation=mutate)
    except RetentionAdmissionFailed:
        pass
    else:
        raise AssertionError("oversized retained record was admitted")
    assert called is False
    assert s.snapshot() == before
    assert 1 in s.pending


def test_pending_original_is_never_evicted_by_terminal_pressure() -> None:
    sem1 = semantic("ONE", "binding=1")
    sem2 = semantic("TWO", "binding=2")
    r = synthetic_record_charge(len(sem1.normalized_intent), len(sem1.binding), 2)
    s = store(count=1, record_bytes=r)
    s.reserve(1, sem1)
    s.reserve(2, sem2)
    assert tuple(s.pending) == (1, 2)
    s.terminalize(1, b"OK")
    assert tuple(s.pending) == (2,)
    assert tuple(s.retained) == (1,)
    s.terminalize(2, b"OK")
    assert tuple(s.pending) == ()
    assert tuple(s.retained) == (2,)


def test_later_terminalization_cannot_pass_earlier_pending() -> None:
    sem1 = semantic("ONE", "binding=1")
    sem2 = semantic("TWO", "binding=2")
    r = synthetic_record_charge(len(sem1.normalized_intent), len(sem1.binding), 2)
    s = store(count=2, record_bytes=r, records_fit=2)
    s.reserve(1, sem1)
    s.reserve(2, sem2)
    before = s.snapshot()
    try:
        s.terminalize(2, b"OK")
    except OutOfOrderTerminal:
        pass
    else:
        raise AssertionError("later terminalization passed earlier pending command")
    assert s.snapshot() == before


def test_evicted_terminal_command_remains_non_reservable() -> None:
    sem1 = semantic("ONE", "binding=1")
    sem2 = semantic("TWO", "binding=2")
    r = synthetic_record_charge(len(sem1.normalized_intent), len(sem1.binding), 2)
    s = store(count=1, record_bytes=r)
    s.reserve(1, sem1)
    s.terminalize(1, b"OK")
    s.reserve(2, sem2)
    s.terminalize(2, b"OK")
    assert s.classify_duplicate(1, sem1)["kind"] == "OUTCOME_EXPIRED"
    assert s.reserve(1, sem1) == "OUTCOME_EXPIRED"
    assert s.next_command_id == 3


def test_sequence_gap_rejects_without_state_change() -> None:
    sem = semantic("OPEN", "binding=A")
    r = synthetic_record_charge(len(sem.normalized_intent), len(sem.binding), 2)
    s = store(count=1, record_bytes=r)
    before = s.snapshot()
    try:
        s.reserve(2, sem)
    except SequenceGap:
        pass
    else:
        raise AssertionError("sequence gap was accepted")
    assert s.snapshot() == before


def test_checked_u64_arithmetic_rejects_overflow() -> None:
    assert checked_add_u64(2, 3) == 5
    assert checked_mul_u64(2, 3) == 6
    try:
        checked_add_u64(U64_MAX, 1)
    except OverflowError:
        pass
    else:
        raise AssertionError("checked addition accepted overflow")
    try:
        checked_mul_u64(U64_MAX, 2)
    except OverflowError:
        pass
    else:
        raise AssertionError("checked multiplication accepted overflow")


def test_fixed_and_variable_charge_equation_is_explicit() -> None:
    charge = synthetic_record_charge(11, 13, 17)
    assert charge == RECORD_FIXED_LOGICAL_BYTES + 11 + 13 + 17


def test_candidate_measurements_are_repeat_deterministic_and_non_authoritative() -> None:
    first = synthetic_candidate_envelopes()
    second = synthetic_candidate_envelopes()
    assert first == second
    assert first
    assert all(row["production_candidate"] is False for row in first)


TESTS = (
    test_first_playable_two_session_witness_needs_one_record_per_session,
    test_zero_retention_cannot_satisfy_replayability_preflight,
    test_same_input_replays_original_outcome_without_second_mutation,
    test_changed_normalized_intent_or_binding_conflicts,
    test_count_boundary_and_one_over_evict_oldest_terminal_only,
    test_byte_boundary_and_one_over_evict_deterministically,
    test_single_record_byte_overflow_fails_before_gameplay_mutation,
    test_pending_original_is_never_evicted_by_terminal_pressure,
    test_later_terminalization_cannot_pass_earlier_pending,
    test_evicted_terminal_command_remains_non_reservable,
    test_sequence_gap_rejects_without_state_change,
    test_checked_u64_arithmetic_rejects_overflow,
    test_fixed_and_variable_charge_equation_is_explicit,
    test_candidate_measurements_are_repeat_deterministic_and_non_authoritative,
)


def main() -> int:
    for test in TESTS:
        test()
    print(f"PASS {len(TESTS)} tests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
