#!/usr/bin/env python3
"""Focused deterministic tests for the #530 evidence-only carrier model."""

from __future__ import annotations

from dataclasses import replace

from model import (
    ACTOR_REF_LOGICAL_BYTES,
    U64_MAX,
    ActorKind,
    ActorRef,
    CapacityExceeded,
    FusedBucketCarrier,
    GenerationOverflow,
    HistoryCapacityExceeded,
    capacity_boundary_probe,
    checked_add_u64,
    checked_mul_u64,
    history_exhaustion_probe,
    measured_probe_stats,
    populate_mixed,
    stable_actor_ids,
)


def carrier(*, world: int = 100, channel: int = 7, scope_generation: int = 3, ceiling: int = 4) -> FusedBucketCarrier:
    return FusedBucketCarrier(
        world_id=world,
        channel_id=channel,
        scope_generation=scope_generation,
        active_ceiling=ceiling,
    )


def test_exact_current_reference_resolves() -> None:
    c = carrier()
    ref = c.insert(local_id=stable_actor_ids(1)[0], kind=ActorKind.PLAYER, position=(10, 20, 7))
    resolved = c.resolve(ref)
    assert resolved is not None
    assert (resolved.world_id, resolved.channel_id, resolved.scope_generation) == (100, 7, 3)
    assert resolved.actionable is True


def test_missing_actor_rejects() -> None:
    c = carrier()
    missing = ActorRef(100, 7, 3, stable_actor_ids(1)[0], 1)
    assert c.resolve(missing) is None


def test_stale_actor_generation_rejects_after_recycle() -> None:
    c = carrier()
    local_id = stable_actor_ids(1)[0]
    old = c.insert(local_id=local_id, kind=ActorKind.CREATURE)
    c.remove(old)
    new = c.insert(local_id=local_id, kind=ActorKind.CREATURE)
    assert new.actor_generation == old.actor_generation + 1
    assert c.resolve(old) is None
    assert c.resolve(new) is not None


def test_same_local_identity_new_generation_cannot_be_targeted_by_old_ref() -> None:
    c = carrier()
    local_id = stable_actor_ids(1)[0]
    first = c.insert(local_id=local_id, kind=ActorKind.NPC_SYSTEM)
    c.remove(first)
    second = c.insert(local_id=local_id, kind=ActorKind.NPC_SYSTEM)
    assert first.local_id == second.local_id
    assert first.actor_generation != second.actor_generation
    assert c.resolve(first) is None


def test_cross_channel_reference_rejects() -> None:
    c = carrier()
    ref = c.insert(local_id=stable_actor_ids(1)[0], kind=ActorKind.PLAYER)
    assert c.resolve(replace(ref, channel_id=8)) is None


def test_same_channel_id_different_world_rejects() -> None:
    c = carrier(world=100, channel=7)
    ref = c.insert(local_id=stable_actor_ids(1)[0], kind=ActorKind.PLAYER)
    assert c.resolve(replace(ref, world_id=101)) is None


def test_stale_scope_generation_rejects_without_mutation() -> None:
    c = carrier(scope_generation=3)
    ref = c.insert(local_id=stable_actor_ids(1)[0], kind=ActorKind.CREATURE)
    before = c.snapshot()
    assert c.resolve(replace(ref, scope_generation=2)) is None
    assert c.snapshot() == before


def test_m_plus_one_capacity_rejects_before_partial_state() -> None:
    result = capacity_boundary_probe(3)
    assert result["m_plus_one_rejected"] is True
    assert result["state_unchanged_after_rejection"] is True


def test_capacity_rejection_preserves_existing_resolutions() -> None:
    c = carrier(ceiling=2)
    refs = populate_mixed(c, 2)
    before = c.snapshot()
    try:
        c.insert(local_id=0x7FFF_FFFF_FFFF_F001, kind=ActorKind.CREATURE)
    except CapacityExceeded:
        pass
    else:
        raise AssertionError("expected CapacityExceeded")
    assert c.snapshot() == before
    assert all(c.resolve(ref) is not None for ref in refs)


def test_checked_arithmetic_rejects_overflow_before_allocation() -> None:
    assert checked_add_u64(3, 4) == 7
    assert checked_mul_u64(3, 4) == 12
    try:
        checked_add_u64(U64_MAX, 1)
    except OverflowError:
        pass
    else:
        raise AssertionError("checked_add_u64 accepted overflow")
    try:
        checked_mul_u64(U64_MAX, 2)
    except OverflowError:
        pass
    else:
        raise AssertionError("checked_mul_u64 accepted overflow")


def test_lookup_result_independent_of_insertion_order() -> None:
    ids = stable_actor_ids(8)
    forward = carrier(ceiling=8)
    reverse = carrier(ceiling=8)
    forward_refs = {}
    reverse_refs = {}
    kinds = [ActorKind.PLAYER, ActorKind.CREATURE, ActorKind.NPC_SYSTEM]
    for index, local_id in enumerate(ids):
        forward_refs[local_id] = forward.insert(local_id=local_id, kind=kinds[index % 3])
    for index, local_id in enumerate(reversed(ids)):
        reverse_refs[local_id] = reverse.insert(local_id=local_id, kind=kinds[index % 3])
    for local_id in ids:
        assert forward.resolve(forward_refs[local_id]) is not None
        assert reverse.resolve(reverse_refs[local_id]) is not None


def test_same_identity_churn_does_not_grow_retired_history() -> None:
    c = carrier(ceiling=1)
    local_id = stable_actor_ids(1)[0]
    retained = c.retained_logical_bytes
    generation = 0
    for _ in range(1000):
        ref = c.insert(local_id=local_id, kind=ActorKind.CREATURE)
        generation = ref.actor_generation
        c.remove(ref)
    assert c.active_count == 0
    assert c.retired_count == 1
    assert c.retained_logical_bytes == retained
    assert generation == 1000


def test_unique_identity_churn_proves_independent_history_exhaustion() -> None:
    result = history_exhaustion_probe(3)
    assert result["bucket_count"] == 8
    assert result["unique_retirements_before_history_exhaustion"] == 8
    assert result["active_count_at_history_exhaustion"] == 0
    assert result["retired_count_at_history_exhaustion"] == 8


def test_only_typed_exact_reference_and_no_variable_candidate_collection() -> None:
    c = carrier(ceiling=8)
    ref = c.insert(local_id=stable_actor_ids(1)[0], kind=ActorKind.PLAYER)
    try:
        c.resolve(ref.local_id)  # type: ignore[arg-type]
    except TypeError:
        pass
    else:
        raise AssertionError("untyped local/client/AI-like handle was accepted")
    stats = measured_probe_stats(8)
    assert stats["variable_candidate_collection_materialized"] is False
    assert ACTOR_REF_LOGICAL_BYTES == 40


TESTS = [
    test_exact_current_reference_resolves,
    test_missing_actor_rejects,
    test_stale_actor_generation_rejects_after_recycle,
    test_same_local_identity_new_generation_cannot_be_targeted_by_old_ref,
    test_cross_channel_reference_rejects,
    test_same_channel_id_different_world_rejects,
    test_stale_scope_generation_rejects_without_mutation,
    test_m_plus_one_capacity_rejects_before_partial_state,
    test_capacity_rejection_preserves_existing_resolutions,
    test_checked_arithmetic_rejects_overflow_before_allocation,
    test_lookup_result_independent_of_insertion_order,
    test_same_identity_churn_does_not_grow_retired_history,
    test_unique_identity_churn_proves_independent_history_exhaustion,
    test_only_typed_exact_reference_and_no_variable_candidate_collection,
]


def main() -> int:
    for test in TESTS:
        test()
    print(f"PASS {len(TESTS)} tests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
