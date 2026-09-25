# OTV2-20260925 ChannelActorCarrier O(1) admission evidence

Status: READY_FOR_INTEGRATION / PHYSICAL_RELEASE_MEASUREMENT_PASS

## Admission state

- Protected base: `d092fe979e0ff1bff50aab4aa589c3e06bfd21d8`.
- Allocation: #162 comment 5832603591.
- Governing resource issue: #530.
- Canonical PR: #899.
- Runtime implementation candidate measured directly: `4dc024fb422be5956f2ad22fe69e4de45192181c`.
- Exact owned runtime path: `apps/game-server/src/foundation/runtime_actor_carrier.rs`.
- Open-PR overlap census at allocation: 30 open PRs, zero touching the runtime path.
- Jira workstream: KAN-24, active High priority.

## Problem and RED proof

Protected source selected a reusable slot with `slots.iter().position(...VacantReusable...)` and rejected a second creature by scanning all slots with `.iter().any(...CreatureOccupied...)`.

The behavior-first test was committed before implementation at exact historical head `92a4eff554f94816985b336cd36405deb050260f`. Validation-only PR #901 executed that immutable RED head and is now closed without merge. Merge-gate run `36140552435` failed in the Rust Linux workspace exactly at:

`foundation::runtime_actor_carrier::tests::multiple_removed_holes_reuse_in_lifo_removal_order`

Observed mismatch:
- actual first recycled slot: `ActorLocalId(2)`;
- required free-list LIFO slot after removing IDs 2 then 4: `ActorLocalId(4)`.

The RED run reported 583 passed / 1 failed / 2 ignored in the affected library test stage. This proves the test distinguished the old lowest-index linear scan from the intended free-list reuse policy.

## GREEN implementation

The current implementation:
- adds `next_free: Option<u32>` only to `Slot::VacantReusable`;
- adds private `free_head: Option<u32>` and `has_creature: bool` to `ChannelActorCarrier`;
- links the fixed slot array once in deterministic 0..M-1 order during bootstrap;
- consumes the current free head in O(1) on admission;
- returns a removed slot to the free-list head in O(1), giving explicit LIFO hole reuse;
- removes only a generation-exhausted selected head from the free list and preserves the #541 `ActorGenerationExhausted` exception;
- keeps representable post-selection failure before slot/free-head mutation;
- replaces the second-creature full-carrier scan with one bounded carrier-local marker;
- preserves direct-index lookup, exact actor/local-generation fencing, stale-ref rejection, owner continuity, no eviction and no alternate identity index.

Fresh exact-source inspection at the current branch found:
- no `.iter().position` in `runtime_actor_carrier.rs`;
- no admission-time `.iter().any(...CreatureOccupied...)` scan;
- free-list state remains carrier-local and fixed-capacity.

## Exact-head qualification already completed

At exact PR #899 head `d8409b56fc3303978bdfc1a25192ba4af0519fb2`:

- Agent governance run `36138754039`: SUCCESS.
- Architecture semantic audit run `36138753724`: SUCCESS.
- Merge gate run `36138753871`: SUCCESS.
- Aggregate `game-gate`: SUCCESS.
- Rust Linux workspace job `108083241053`: SUCCESS.
- Strict Clippy step: SUCCESS.
- Workspace tests: SUCCESS.
- Affected carrier/Ability/Movement tests all passed, including:
  - multi-capacity M/M+1;
  - multi-hole LIFO reuse;
  - generation exhaustion;
  - rollback/free-head preservation;
  - one-creature marker admission/removal;
  - stale/recycled/cross-scope owner checks.
- One test stage reported 447 passed / 0 failed; another reported 434 passed / 0 failed.

No external AI review is required by the bound META review policy for this ordinary one-file non-control-plane code change. Whole-diff coordinator inspection found no scope widening or authority change.

## Physical measurement status

The allocation's remaining physical acceptance item is now satisfied by PR #899 comment 5834840597 against exact source head `4dc024fb422be5956f2ad22fe69e4de45192181c`.

Named host/profile: `ASSUMED-REF-A` — x86_64, 4 shared Intel Xeon vCPU @ 2.80 GHz, 15 GiB RAM, Linux 6.18, Rust 1.94.0, release build, single-thread harness. The temporary measurement harness was not committed.

Measured layout:
- `size_of::<Slot>() = 192 B` before and after the free-list change;
- `size_of::<ActorRef>() = 56 B`;
- the intrusive `next_free: Option<u32>` adds no measured bytes per slot.

Release-mode ranges:

| M | source | fill from empty | M+1 reject | worst-hole churn | random churn | exact lookup |
|---:|---|---:|---:|---:|---:|---:|
| 4,096 | base `d092fe97` | 6.6–6.7 ms | 3.1 µs | 3.2 µs | 1.6–2.3 µs | 9–10 ns |
| 4,096 | #899 `4dc024fb` | 0.19–0.34 ms | 3–6 ns | 29–45 ns | 28–67 ns | 9–15 ns |
| 131,072 | base `d092fe97` | 26.4–26.5 s | 384–418 µs | 370–463 µs | 203–238 µs | 42–64 ns |
| 131,072 | #899 `4dc024fb` | 9.2–9.9 ms | 3 ns | 21 ns | 178–266 ns | 61–68 ns |

Additional #899 points: M=16,384 fill 0.96–1.34 ms; M=65,536 fill 4.5–5.1 ms. The result demonstrates that admission/rejection/churn no longer scale with M while exact lookup remains the same direct-index code path.

The measurement closes only the O(1)-admission acceptance item. It does **not** select production M, authorize >1 creature, register a runtime-actor capacity row, activate a Channel runtime, or widen Movement/Ability/Content/protocol scope.

## Integration disposition

- semantic/algorithmic implementation qualification: PASS;
- physical release-mode measurement: PASS;
- source scope: unchanged one-file bounded implementation;
- external AI review: not required by current bound policy for this slice;
- production M / registry row / runtime activation: OPEN and explicitly outside PR #899;
- next requirement: fresh exact-head CI on the final metadata successor head, then coordinator-owned governed Merge Queue and protected-main readback.

The physical measurement is bound to unchanged runtime source bytes at `4dc024fb`; subsequent task/evidence-only metadata authoring does not invalidate the source measurement. Any runtime-source change requires a fresh physical qualification.
