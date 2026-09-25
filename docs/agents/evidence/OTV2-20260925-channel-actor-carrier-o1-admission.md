# OTV2-20260925 ChannelActorCarrier O(1) admission evidence

Status: AUTHORING / RED_PENDING

## Admission state

- Protected base: `d092fe979e0ff1bff50aab4aa589c3e06bfd21d8`.
- Allocation: #162 comment 5832603591.
- Governing resource issue: #530.
- Exact owned runtime path: `apps/game-server/src/foundation/runtime_actor_carrier.rs`.
- Open-PR overlap census at allocation: 30 open PRs, zero touching the runtime path.
- Jira workstream: KAN-24, active High priority.

## Problem reproduced from protected source

Protected source selects a reusable slot with `slots.iter().position(...VacantReusable...)` and rejects a second creature by scanning all slots with `.iter().any(...CreatureOccupied...)`. Exact lookup/removal are direct-index operations.

Existing measured evidence on #530 records:
- current protected Slot size before this task: 192 B on x86_64 in the cited ASSUMED-REF-A measurement;
- M=131,072 admission/reuse scans at hundreds of microseconds and a full fill around 26.4 s single-thread;
- production M remains OPEN and is not selected by this task.

These historical measurements are inputs only. This task must recapture Slot size and post-change release-mode timings before qualification.

## Intended invariant-preserving change

- fixed-capacity intrusive free list, no heap-growing secondary index;
- deterministic initial allocation order 0..M-1;
- recycled slots return to the free-list head, producing explicit LIFO multi-hole reuse;
- generation remains stored in the slot;
- selected generation `u64::MAX` becomes only `Exhausted` and is removed from the free list;
- representable post-selection failure changes neither slot bytes nor free-list head;
- one bounded carrier-local creature count replaces the admission full scan while preserving the existing at-most-one-creature fixture.

## Qualification ledger

- RED behavioral test: pending.
- GREEN focused tests: pending.
- Structural no-scan proof: pending.
- Slot size before/after: before historical=192 B; exact task recapture pending.
- Release M=4,096 timing: pending.
- Release M=131,072 timing: pending.
- fmt: pending.
- strict Clippy: pending.
- affected Foundation/library regression: pending.
- governance: pending.
- full game-gate: pending.
- whole-diff self-review: pending.
