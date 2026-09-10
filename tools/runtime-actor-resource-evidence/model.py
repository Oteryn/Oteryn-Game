#!/usr/bin/env python3
"""Deterministic non-production model for #530 runtime actor carrier resource evidence.

This module intentionally models only the protected #530/#508 semantics needed to
classify resource dimensions. It does not select a production container, actor ID
allocator, ECS, wire identity, capacity maximum, or runtime module layout.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, replace
from enum import IntEnum
from typing import Iterator, Optional

U64_MAX = (1 << 64) - 1

# Explicit network-order fixed-width logical layouts used only for deterministic
# structural accounting. They are synthetic evidence formats, not Python object
# size, process RSS, Rust ABI, wire protocol, or a production storage decision.
#
# ActorRef: WorldId, ChannelId, scope generation, local id, actor generation.
ACTOR_REF_STRUCT = struct.Struct(">QQQQQ")
# Fused bucket: five u64 identity/generation fields; x/y/z i32; kind/actionable
# u8; ten reserved padding bytes so every candidate bucket is exactly 64 bytes.
FUSED_BUCKET_STRUCT = struct.Struct(">QQQQQiiiBB10x")
# Split record carries the same fields with two reserved bytes to 56 bytes.
SPLIT_RECORD_STRUCT = struct.Struct(">QQQQQiiiBB2x")
# Split index entry: local id, actor generation, fixed slot locator.
SPLIT_INDEX_ENTRY_STRUCT = struct.Struct(">QQQ")
# Separate generation-history candidate: local id + retired generation.
SPLIT_GENERATION_ENTRY_STRUCT = struct.Struct(">QQ")

ACTOR_REF_LOGICAL_BYTES = ACTOR_REF_STRUCT.size
FUSED_BUCKET_LOGICAL_BYTES = FUSED_BUCKET_STRUCT.size
SPLIT_RECORD_LOGICAL_BYTES = SPLIT_RECORD_STRUCT.size
SPLIT_INDEX_ENTRY_LOGICAL_BYTES = SPLIT_INDEX_ENTRY_STRUCT.size
SPLIT_GENERATION_ENTRY_LOGICAL_BYTES = SPLIT_GENERATION_ENTRY_STRUCT.size

EMPTY = 0
ACTIVE = 1
RETIRED = 2


class ActorKind(IntEnum):
    PLAYER = 1
    CREATURE = 2
    NPC_SYSTEM = 3


class CapacityExceeded(RuntimeError):
    """Active actor ceiling would be exceeded."""


class HistoryCapacityExceeded(RuntimeError):
    """Retired opaque-identity history exhausted the fixed candidate table."""


class InvalidReference(RuntimeError):
    """Reference does not name the current authoritative actor generation."""


class GenerationOverflow(RuntimeError):
    """Actor-local generation cannot be advanced without integer overflow."""


@dataclass(frozen=True)
class ActorRef:
    world_id: int
    channel_id: int
    scope_generation: int
    local_id: int
    actor_generation: int


@dataclass(frozen=True)
class Bucket:
    state: int = EMPTY
    world_id: int = 0
    channel_id: int = 0
    scope_generation: int = 0
    local_id: int = 0
    actor_generation: int = 0
    x: int = 0
    y: int = 0
    z: int = 0
    kind: int = 0
    actionable: bool = False


def checked_mul_u64(left: int, right: int) -> int:
    if left < 0 or right < 0 or left > U64_MAX or right > U64_MAX:
        raise OverflowError("value outside u64")
    if left != 0 and right > U64_MAX // left:
        raise OverflowError("u64 multiplication overflow")
    return left * right


def checked_add_u64(left: int, right: int) -> int:
    if left < 0 or right < 0 or left > U64_MAX or right > U64_MAX:
        raise OverflowError("value outside u64")
    if right > U64_MAX - left:
        raise OverflowError("u64 addition overflow")
    return left + right


def next_power_of_two(value: int) -> int:
    if value < 1:
        raise ValueError("value must be positive")
    result = 1 << (value - 1).bit_length()
    if result > U64_MAX:
        raise OverflowError("power-of-two result exceeds u64")
    return result


def candidate_bucket_count(active_ceiling: int) -> int:
    if active_ceiling < 1:
        raise ValueError("active_ceiling must be positive")
    doubled = checked_mul_u64(active_ceiling, 2)
    return next_power_of_two(doubled)


def fused_layout(active_ceiling: int) -> dict[str, int]:
    buckets = candidate_bucket_count(active_ceiling)
    retained = checked_mul_u64(buckets, FUSED_BUCKET_LOGICAL_BYTES)
    return {
        "active_ceiling": active_ceiling,
        "bucket_count": buckets,
        "bucket_logical_bytes": FUSED_BUCKET_LOGICAL_BYTES,
        "retained_logical_bytes": retained,
    }


def split_layout(active_ceiling: int) -> dict[str, int]:
    buckets = candidate_bucket_count(active_ceiling)
    record_bytes = checked_mul_u64(active_ceiling, SPLIT_RECORD_LOGICAL_BYTES)
    index_bytes = checked_mul_u64(buckets, SPLIT_INDEX_ENTRY_LOGICAL_BYTES)
    without_generation_history = checked_add_u64(record_bytes, index_bytes)
    return {
        "active_ceiling": active_ceiling,
        "record_count": active_ceiling,
        "record_logical_bytes_each": SPLIT_RECORD_LOGICAL_BYTES,
        "record_logical_bytes": record_bytes,
        "index_bucket_count": buckets,
        "index_entry_logical_bytes_each": SPLIT_INDEX_ENTRY_LOGICAL_BYTES,
        "index_logical_bytes": index_bytes,
        "retained_logical_bytes_without_generation_history": without_generation_history,
        "generation_history_entry_logical_bytes": SPLIT_GENERATION_ENTRY_LOGICAL_BYTES,
    }


def _mix64(value: int) -> int:
    """Stable 64-bit mixer; never uses Python's randomized hash()."""
    value &= U64_MAX
    value ^= value >> 30
    value = (value * 0xBF58476D1CE4E5B9) & U64_MAX
    value ^= value >> 27
    value = (value * 0x94D049BB133111EB) & U64_MAX
    value ^= value >> 31
    return value & U64_MAX


class FusedBucketCarrier:
    """Synthetic fixed-table candidate.

    A bucket is both lookup location and retained actor/generation state. Retired
    opaque identities remain in-place so stale same-local-id generations can be
    rejected if that identity is reused. This intentionally demonstrates why
    retirement/generation state can exhaust independently of active actor count.

    It is an evidence model only; it is not a production container choice.
    """

    def __init__(
        self,
        *,
        world_id: int,
        channel_id: int,
        scope_generation: int,
        active_ceiling: int,
    ) -> None:
        self.world_id = world_id
        self.channel_id = channel_id
        self.scope_generation = scope_generation
        self.active_ceiling = active_ceiling
        self.bucket_count = candidate_bucket_count(active_ceiling)
        self._buckets = [Bucket() for _ in range(self.bucket_count)]
        self.active_count = 0

    def snapshot(self) -> tuple[object, ...]:
        return (
            self.world_id,
            self.channel_id,
            self.scope_generation,
            self.active_ceiling,
            self.bucket_count,
            self.active_count,
            tuple(self._buckets),
        )

    @property
    def retired_count(self) -> int:
        return sum(1 for bucket in self._buckets if bucket.state == RETIRED)

    @property
    def retained_logical_bytes(self) -> int:
        return checked_mul_u64(self.bucket_count, FUSED_BUCKET_LOGICAL_BYTES)

    def _probe_indices(self, local_id: int) -> Iterator[int]:
        start = _mix64(local_id) & (self.bucket_count - 1)
        for offset in range(self.bucket_count):
            yield (start + offset) & (self.bucket_count - 1)

    def insert(
        self,
        *,
        local_id: int,
        kind: ActorKind,
        position: tuple[int, int, int] = (0, 0, 0),
    ) -> ActorRef:
        # Active-capacity rejection occurs before touching any bucket.
        if self.active_count >= self.active_ceiling:
            raise CapacityExceeded("tested active ceiling exceeded")

        empty_index: Optional[int] = None
        retired_match: Optional[int] = None

        for index in self._probe_indices(local_id):
            bucket = self._buckets[index]
            if bucket.state == EMPTY:
                empty_index = index
                break
            if bucket.local_id == local_id:
                if bucket.state == ACTIVE:
                    raise ValueError("local_id already active")
                retired_match = index
                break

        selected = retired_match if retired_match is not None else empty_index
        if selected is None:
            # Active count can be below the active ceiling here. That is the
            # evidence that retained identity/generation history is a distinct
            # exhaustion dimension.
            raise HistoryCapacityExceeded("retired identity history exhausted table")

        previous = self._buckets[selected]
        actor_generation = previous.actor_generation if previous.state == RETIRED else 1
        x, y, z = position
        self._buckets[selected] = Bucket(
            state=ACTIVE,
            world_id=self.world_id,
            channel_id=self.channel_id,
            scope_generation=self.scope_generation,
            local_id=local_id,
            actor_generation=actor_generation,
            x=x,
            y=y,
            z=z,
            kind=int(kind),
            actionable=True,
        )
        self.active_count += 1

        return ActorRef(
            world_id=self.world_id,
            channel_id=self.channel_id,
            scope_generation=self.scope_generation,
            local_id=local_id,
            actor_generation=actor_generation,
        )

    def resolve_with_probes(self, actor_ref: ActorRef) -> tuple[Optional[Bucket], int]:
        if not isinstance(actor_ref, ActorRef):
            raise TypeError("resolver requires ActorRef")

        # Scope checks precede table access. WorldId and ChannelId are distinct.
        if (
            actor_ref.world_id != self.world_id
            or actor_ref.channel_id != self.channel_id
            or actor_ref.scope_generation != self.scope_generation
        ):
            return None, 0

        probes = 0
        for index in self._probe_indices(actor_ref.local_id):
            probes += 1
            bucket = self._buckets[index]
            if bucket.state == EMPTY:
                return None, probes
            if bucket.local_id == actor_ref.local_id:
                if bucket.state != ACTIVE:
                    return None, probes
                if bucket.actor_generation != actor_ref.actor_generation:
                    return None, probes
                return bucket, probes
        return None, probes

    def resolve(self, actor_ref: ActorRef) -> Optional[Bucket]:
        return self.resolve_with_probes(actor_ref)[0]

    def remove(self, actor_ref: ActorRef) -> None:
        if not isinstance(actor_ref, ActorRef):
            raise TypeError("remove requires ActorRef")

        if (
            actor_ref.world_id != self.world_id
            or actor_ref.channel_id != self.channel_id
            or actor_ref.scope_generation != self.scope_generation
        ):
            raise InvalidReference("scope mismatch")

        for index in self._probe_indices(actor_ref.local_id):
            bucket = self._buckets[index]
            if bucket.state == EMPTY:
                raise InvalidReference("missing actor")
            if bucket.local_id != actor_ref.local_id:
                continue
            if bucket.state != ACTIVE or bucket.actor_generation != actor_ref.actor_generation:
                raise InvalidReference("stale actor generation")
            if bucket.actor_generation == U64_MAX:
                raise GenerationOverflow("actor generation overflow")

            # Advance generation while retiring the current identity. No live
            # actor is evicted to make space.
            self._buckets[index] = replace(
                bucket,
                state=RETIRED,
                actor_generation=bucket.actor_generation + 1,
                actionable=False,
            )
            self.active_count -= 1
            return

        raise InvalidReference("missing actor")


def stable_actor_ids(count: int) -> list[int]:
    """Opaque-looking deterministic IDs; none are storage indexes."""
    base = 0x0100_0000_0000_0001
    stride = 0x0000_0000_0001_1D7B
    return [base + stride * index for index in range(count)]


def populate_mixed(carrier: FusedBucketCarrier, count: int) -> list[ActorRef]:
    refs: list[ActorRef] = []
    kinds = [ActorKind.PLAYER, ActorKind.CREATURE, ActorKind.NPC_SYSTEM]
    for index, local_id in enumerate(stable_actor_ids(count)):
        refs.append(
            carrier.insert(
                local_id=local_id,
                kind=kinds[index % len(kinds)],
                position=(100 + index, 200 + index, 7),
            )
        )
    return refs


def measured_probe_stats(active_ceiling: int) -> dict[str, object]:
    carrier = FusedBucketCarrier(
        world_id=100,
        channel_id=7,
        scope_generation=3,
        active_ceiling=active_ceiling,
    )
    refs = populate_mixed(carrier, active_ceiling)
    probes = [carrier.resolve_with_probes(actor_ref)[1] for actor_ref in refs]
    return {
        "active_ceiling": active_ceiling,
        "bucket_count": carrier.bucket_count,
        "lookup_count": len(probes),
        "observed_min_probes": min(probes),
        "observed_max_probes": max(probes),
        "observed_total_probes": sum(probes),
        "structural_probe_upper_bound": carrier.bucket_count,
        "variable_candidate_collection_materialized": False,
    }


def history_exhaustion_probe(active_ceiling: int) -> dict[str, int]:
    carrier = FusedBucketCarrier(
        world_id=100,
        channel_id=7,
        scope_generation=3,
        active_ceiling=active_ceiling,
    )
    unique_retirements = 0
    for local_id in stable_actor_ids(carrier.bucket_count + 1):
        try:
            actor_ref = carrier.insert(local_id=local_id, kind=ActorKind.CREATURE)
        except HistoryCapacityExceeded:
            return {
                "active_ceiling": active_ceiling,
                "bucket_count": carrier.bucket_count,
                "unique_retirements_before_history_exhaustion": unique_retirements,
                "active_count_at_history_exhaustion": carrier.active_count,
                "retired_count_at_history_exhaustion": carrier.retired_count,
            }
        carrier.remove(actor_ref)
        unique_retirements += 1
    raise AssertionError("history exhaustion was not observed")


def capacity_boundary_probe(active_ceiling: int) -> dict[str, object]:
    carrier = FusedBucketCarrier(
        world_id=100,
        channel_id=7,
        scope_generation=3,
        active_ceiling=active_ceiling,
    )
    populate_mixed(carrier, active_ceiling)
    before = carrier.snapshot()
    try:
        carrier.insert(
            local_id=0x7FFF_FFFF_FFFF_F001,
            kind=ActorKind.CREATURE,
            position=(0, 0, 7),
        )
    except CapacityExceeded:
        pass
    else:
        raise AssertionError("M+1 admission was not rejected")
    return {
        "active_ceiling": active_ceiling,
        "m_plus_one_rejected": True,
        "state_unchanged_after_rejection": before == carrier.snapshot(),
    }
