# OTV2 runtime actor carrier pre-production evidence

## Scope and classification

- Task: `OTV2-20260911-runtime-actor-carrier-preproduction-530`
- Canonical PR: Draft #573
- Authority: `PRE_PRODUCTION_FOUNDATION_RUNTIME_ONLY`
- Production capacity/readiness: **not claimed**
- Physical production-capacity E2E: `NOT_APPLICABLE`

## Implemented result

The private Foundation module contains one fixed-capacity Channel actor carrier and one deliberately
non-Clone/non-Copy namespace continuity guard outside carrier backing. The guard consumes a move-only
continuity grant supplied from outside the module's authority boundary. Construction claims the
guard's current World + Channel + scope generation exactly once, requires an explicit non-zero finite
capacity, checks identity and byte arithmetic, and uses fallible exact allocation before publishing a
carrier. Carrier loss does not reset the surviving claim. The module has no default, production
constructor, capacity reporter, issuer, reissuer, or raw-fact continuity constructor.

Each reference binds `WorldId + ChannelId + ScopeOwnershipGeneration + ActorLocalId +
ActorLocalGeneration`. Lookup and removal require the live guard and validate its current scope against
both carrier and reference before converting the one-based identity directly to the backing slot;
there is no secondary index, tombstone collection, or retirement history. A move-only externally
supplied grant can advance the guard only for the same World + Channel and a strictly newer generation;
that advance immediately fences retained older carriers and permits exactly one fresh namespace claim.

Vacancy retains the local generation and successful reuse advances it. A representable failure after
selection leaves all slots unchanged. The protected generation-exhaustion exception changes only the
selected `VACANT_REUSABLE(g_max)` cell to `EXHAUSTED(g_max)`, publishes no replacement reference or
actor, preserves unrelated slots, and makes that slot ineligible for subsequent selection.

## RED to GREEN

The completed implementation pass used focused module tests as the executable specification. The RED
set covered absent carrier behavior for explicit capacity, exact scope/reference fencing, retained
generation reuse, rollback, and terminal generation exhaustion. The minimal GREEN implementation is
confined to the new private module plus one private module declaration.

The publish-only recovery found that the previously reported commit object had not entered either the
local checkout or GitHub. It reconstructed only the same four-path result on the unchanged canonical
head and reran the deterministic qualification commands before publication.

Coordinator self-review of published head `341e1f56c597c9cc1d3a33056c54f55c709a6ad7` reported a P1:
the consumed grant had been copied into carrier backing, so an old carrier self-proved generation
currentness and carrier loss did not preserve a same-generation namespace claim. **Disposition:
ACCEPTED AND REPAIRED.** That head is superseded and is not described as having zero self-review
findings. Focused RED tests first captured carrier-loss replay, live generation advancement fencing,
non-monotonic/cross-scope advance rejection, and legitimate newer-generation reconstruction.

## Deterministic validation

| Command | Result |
| --- | --- |
| `cargo +1.94.0 fmt --all -- --check` | PASS |
| `cargo +1.94.0 test -p oteryn-game-server runtime_actor_carrier` | PASS — 10 passed, 0 failed |
| `cargo +1.94.0 clippy -p oteryn-game-server --all-targets -- -D warnings` | PASS |
| `python tools/agents/validate_governance.py` | PASS |
| `git diff --check` | PASS |

Focused coverage uses injected capacities 1, 2, and 4 to prove M/M+1 without selecting a product
policy. Negative coverage includes zero and overflowing capacities; wrong World, Channel, and scope
generation; invalid/vacant identities; stale local generations; surviving one-shot namespace claims;
immediate old-carrier lookup/removal fencing after a live generation advance; rejected equal,
backward, and cross-scope advances without guard mutation; fresh strictly newer namespace claims;
post-selection rollback; `g_max` exhaustion; unrelated-state preservation; and later avoidance of an
exhausted slot.

Allocation failure uses `Vec::try_reserve_exact`; deterministic arithmetic rejection precedes it and
carrier publication occurs only after the backing allocation and initialization succeed. A physical
allocator failure is environment-dependent, so the fallible operation and its pre-publication ordering
are structurally reviewed rather than forced by unsafe or global allocator substitution.

## Adversarial whole-diff self-review

- **Authority:** no production constructor/default/readiness surface and no continuity issuer exists.
- **Fencing:** all five exact-reference components are checked against a live continuity guard; neither
  a reference nor carrier-local snapshots self-prove current outer-generation authority.
- **Lifecycle:** the local ID remains slot-bound; generation persists across vacancy; wrap cannot reuse
  a generation; the sole terminal bookkeeping mutation is isolated.
- **Boundedness:** backing storage is fixed after construction. Lookup/removal are direct O(1);
  insertion scans at most the explicit capacity and retains no growing history.
- **Failure atomicity:** checked construction failures publish nothing; representable selection failure
  does not mutate; exhaustion changes only the selected terminal cell.
- **Scope exclusions:** no geometry, gameplay, protocol, admission, persistence, registry, Cargo,
  deployment, Ability #508, Movement #139, or external-repository changes are present.
- **Finding verdict:** the P1 on superseded head `341e1f56c597c9cc1d3a33056c54f55c709a6ad7`
  is accepted and repaired; the repaired successor has zero unresolved material self-review findings.

## Qualification state

Local deterministic qualification is complete. Hosted exact-head CI and genuinely independent
exact-head review remain **OPEN** and are intentionally not requested in this pass. This candidate is
not ready for Merge Queue, merge, production use, or a production-capacity claim.
