#!/usr/bin/env python3
"""Deterministic evidence-only model for FND-02 retained terminal outcomes.

This is not a production Foundation implementation. It models only the accepted
#663 semantics needed to measure count/byte boundaries and failure behavior.
Normalized intent/binding bytes in this model are already server-normalized
semantic evidence; they are deliberately not raw protobuf byte identity.
"""

from __future__ import annotations

from collections import OrderedDict
from dataclasses import dataclass
from typing import Callable

U64_MAX = (1 << 64) - 1

# Synthetic logical accounting only. These widths make model arithmetic explicit;
# they are not Python/Rust ABI size, RSS, allocator overhead, wire schema, or an
# accepted production charge. The missing accepted production accounting rule is
# one of the evidence findings.
STORE_FIXED_LOGICAL_BYTES = 32
RECORD_FIXED_LOGICAL_BYTES = 40


class SequenceGap(RuntimeError):
    pass


class OutOfOrderTerminal(RuntimeError):
    pass


class RetentionAdmissionFailed(RuntimeError):
    pass


class InvalidCommand(RuntimeError):
    pass


@dataclass(frozen=True)
class SemanticInput:
    normalized_intent: bytes
    binding: bytes


@dataclass(frozen=True)
class RetainedRecord:
    command_id: int
    semantic: SemanticInput
    outcome: bytes

    @property
    def charged_bytes(self) -> int:
        return checked_sum_u64(
            RECORD_FIXED_LOGICAL_BYTES,
            len(self.semantic.normalized_intent),
            len(self.semantic.binding),
            len(self.outcome),
        )


@dataclass(frozen=True)
class TerminalPlan:
    command_id: int
    record: RetainedRecord
    evict_command_ids: tuple[int, ...]
    resulting_charged_bytes: int


def checked_add_u64(left: int, right: int) -> int:
    if left < 0 or right < 0 or left > U64_MAX or right > U64_MAX:
        raise OverflowError("value outside u64")
    if right > U64_MAX - left:
        raise OverflowError("u64 addition overflow")
    return left + right


def checked_mul_u64(left: int, right: int) -> int:
    if left < 0 or right < 0 or left > U64_MAX or right > U64_MAX:
        raise OverflowError("value outside u64")
    if left and right > U64_MAX // left:
        raise OverflowError("u64 multiplication overflow")
    return left * right


def checked_sum_u64(*values: int) -> int:
    total = 0
    for value in values:
        total = checked_add_u64(total, value)
    return total


def synthetic_record_charge(intent_bytes: int, binding_bytes: int, outcome_bytes: int) -> int:
    return checked_sum_u64(
        RECORD_FIXED_LOGICAL_BYTES,
        intent_bytes,
        binding_bytes,
        outcome_bytes,
    )


def synthetic_session_charge(
    retained_records: int,
    *,
    intent_bytes: int,
    binding_bytes: int,
    outcome_bytes: int,
) -> int:
    per_record = synthetic_record_charge(intent_bytes, binding_bytes, outcome_bytes)
    return checked_add_u64(
        STORE_FIXED_LOGICAL_BYTES,
        checked_mul_u64(retained_records, per_record),
    )


class SessionRetentionModel:
    """One synthetic Foundation-owned GameSession retention lifecycle."""

    def __init__(self, *, max_records: int, max_charged_bytes: int) -> None:
        if max_records < 0:
            raise ValueError("max_records must be non-negative")
        if max_charged_bytes < STORE_FIXED_LOGICAL_BYTES:
            raise ValueError("max_charged_bytes cannot omit fixed store charge")
        self.max_records = max_records
        self.max_charged_bytes = max_charged_bytes
        self.next_command_id = 1
        self.pending: "OrderedDict[int, SemanticInput]" = OrderedDict()
        self.retained: "OrderedDict[int, RetainedRecord]" = OrderedDict()
        self.gameplay_mutations = 0

    @property
    def retained_charged_bytes(self) -> int:
        total = STORE_FIXED_LOGICAL_BYTES
        for record in self.retained.values():
            total = checked_add_u64(total, record.charged_bytes)
        return total

    def snapshot(self) -> tuple[object, ...]:
        return (
            self.next_command_id,
            tuple(self.pending.items()),
            tuple(self.retained.items()),
            self.retained_charged_bytes,
            self.gameplay_mutations,
        )

    def reserve(self, command_id: int, semantic: SemanticInput) -> str:
        if command_id < 1 or command_id > U64_MAX:
            raise InvalidCommand("CommandId outside accepted uint64/nonzero range")
        if command_id < self.next_command_id:
            return self.classify_duplicate(command_id, semantic)["kind"]
        if command_id > self.next_command_id:
            raise SequenceGap(f"expected {self.next_command_id}, got {command_id}")
        self.pending[command_id] = semantic
        self.next_command_id = checked_add_u64(self.next_command_id, 1)
        return "RESERVED"

    def classify_duplicate(self, command_id: int, semantic: SemanticInput) -> dict[str, object]:
        if command_id >= self.next_command_id:
            return {"kind": "NOT_DUPLICATE"}
        pending = self.pending.get(command_id)
        if pending is not None:
            if pending != semantic:
                return {"kind": "CONFLICT_CHANGED_INPUT"}
            return {"kind": "PENDING_ORIGINAL"}
        retained = self.retained.get(command_id)
        if retained is not None:
            if retained.semantic != semantic:
                return {"kind": "CONFLICT_CHANGED_INPUT"}
            return {
                "kind": "REPLAY_RETAINED_OUTCOME",
                "outcome": retained.outcome,
            }
        return {"kind": "OUTCOME_EXPIRED"}

    def preflight_terminal(self, command_id: int, outcome: bytes) -> TerminalPlan:
        semantic = self.pending.get(command_id)
        if semantic is None:
            raise InvalidCommand("terminalization requires one current pending original")
        if command_id != next(iter(self.pending)):
            raise OutOfOrderTerminal("later terminalization cannot pass earlier pending CommandId")

        record = RetainedRecord(command_id, semantic, outcome)
        minimum = checked_add_u64(STORE_FIXED_LOGICAL_BYTES, record.charged_bytes)
        if self.max_records == 0 or minimum > self.max_charged_bytes:
            raise RetentionAdmissionFailed("one retained result cannot fit configured envelope")

        candidate: "OrderedDict[int, RetainedRecord]" = OrderedDict(self.retained)
        evictions: list[int] = []

        while True:
            count_after = checked_add_u64(len(candidate), 1)
            bytes_after = STORE_FIXED_LOGICAL_BYTES
            for existing in candidate.values():
                bytes_after = checked_add_u64(bytes_after, existing.charged_bytes)
            bytes_after = checked_add_u64(bytes_after, record.charged_bytes)

            if count_after <= self.max_records and bytes_after <= self.max_charged_bytes:
                return TerminalPlan(
                    command_id=command_id,
                    record=record,
                    evict_command_ids=tuple(evictions),
                    resulting_charged_bytes=bytes_after,
                )

            if not candidate:
                raise RetentionAdmissionFailed("retention envelope cannot admit terminal record")
            oldest_command_id, _ = candidate.popitem(last=False)
            evictions.append(oldest_command_id)

    def terminalize(
        self,
        command_id: int,
        outcome: bytes,
        *,
        gameplay_mutation: Callable[[], None] | None = None,
    ) -> TerminalPlan:
        # The entire plan is proven to fit before gameplay mutation is invoked.
        plan = self.preflight_terminal(command_id, outcome)
        if gameplay_mutation is not None:
            gameplay_mutation()
        self.gameplay_mutations += 1
        for evicted in plan.evict_command_ids:
            del self.retained[evicted]
        del self.pending[command_id]
        self.retained[command_id] = plan.record
        assert self.retained_charged_bytes == plan.resulting_charged_bytes
        return plan


def semantic(intent: str, binding: str) -> SemanticInput:
    return SemanticInput(intent.encode("ascii"), binding.encode("ascii"))


def first_playable_two_session_probe() -> dict[str, object]:
    a_semantic = semantic("OPEN", "placement=door-A;edge=open")
    b_semantic = semantic("CLOSE", "placement=door-A;edge=close")
    a_record_bytes = synthetic_record_charge(
        len(a_semantic.normalized_intent), len(a_semantic.binding), len(b"OPENED")
    )
    b_record_bytes = synthetic_record_charge(
        len(b_semantic.normalized_intent), len(b_semantic.binding), len(b"CLOSED")
    )
    a = SessionRetentionModel(
        max_records=1,
        max_charged_bytes=STORE_FIXED_LOGICAL_BYTES + a_record_bytes,
    )
    b = SessionRetentionModel(
        max_records=1,
        max_charged_bytes=STORE_FIXED_LOGICAL_BYTES + b_record_bytes,
    )
    assert a.reserve(1, a_semantic) == "RESERVED"
    a.terminalize(1, b"OPENED")
    assert b.reserve(1, b_semantic) == "RESERVED"
    b.terminalize(1, b"CLOSED")
    replay = a.classify_duplicate(1, a_semantic)
    return {
        "session_a_retained_count": len(a.retained),
        "session_b_retained_count": len(b.retained),
        "per_gamesession_count_lower_bound_demonstrated": 1,
        "cross_session_total_records_in_witness": 2,
        "replay_kind": replay["kind"],
        "replay_outcome": replay.get("outcome", b"").decode("ascii"),
        "session_a_synthetic_charged_bytes": a.retained_charged_bytes,
        "session_b_synthetic_charged_bytes": b.retained_charged_bytes,
        "production_byte_claim": False,
    }


SYNTHETIC_SHAPES = (
    ("tiny", 8, 16, 8),
    ("cw4_like", 16, 128, 32),
    ("stress", 1024, 2048, 1024),
)
SYNTHETIC_COUNTS = (1, 2, 4, 8)


def synthetic_candidate_envelopes() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for name, intent_bytes, binding_bytes, outcome_bytes in SYNTHETIC_SHAPES:
        per_record = synthetic_record_charge(intent_bytes, binding_bytes, outcome_bytes)
        for count in SYNTHETIC_COUNTS:
            rows.append(
                {
                    "shape": name,
                    "count": count,
                    "intent_bytes": intent_bytes,
                    "binding_bytes": binding_bytes,
                    "outcome_bytes": outcome_bytes,
                    "modeled_record_charged_bytes": per_record,
                    "modeled_session_charged_bytes": synthetic_session_charge(
                        count,
                        intent_bytes=intent_bytes,
                        binding_bytes=binding_bytes,
                        outcome_bytes=outcome_bytes,
                    ),
                    "production_candidate": False,
                }
            )
    return rows
