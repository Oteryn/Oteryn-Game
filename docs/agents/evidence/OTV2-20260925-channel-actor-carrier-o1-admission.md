# OTV2-20260925 ChannelActorCarrier O(1) admission evidence

Status: VALIDATING / PHYSICAL_RELEASE_MEASUREMENT_PENDING

## Admission state

- Protected base: `d092fe979e0ff1bff50aab4aa589c3e06bfd21d8`.
- Allocation: #162 comment 5832603591.
- Governing resource issue: #530.
- Canonical PR: #899.
- Exact current candidate before this evidence update: `d8409b56fc3303978bdfc1a25192ba4af0519fb2`.
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

Existing #530 evidence remains historical input only:
- protected pre-task `Slot` size was measured as 192 B on the named ASSUMED-REF-A x86_64 profile;
- M=131,072 linear admission/reuse scans were measured at hundreds of microseconds and full fill around 26.4 s;
- production M remains OPEN.

The allocation requires fresh release-mode timings at M=4,096 and M=131,072 plus an exact post-change `size_of::<Slot>()` recapture before this lane can claim full physical qualification.

Current execution surfaces can run the repository's canonical PR gate, but expose no authorized arbitrary release-benchmark dispatch for this exact branch. Local container network access cannot resolve github.com, and adding a permanent workflow or recursive benchmark harness would exceed the allocated three-path/minimum-sufficient scope.

Therefore:
- semantic/algorithmic implementation qualification: PASS;
- canonical exact-head CI: PASS;
- physical release-mode measurement: PENDING;
- production M / registry row / activation: still OPEN and explicitly not claimed.

Do not mark this task `READY_FOR_INTEGRATION` until the required physical measurement is obtained on an authorized named execution surface, or #162/#530 explicitly revises that acceptance requirement.
