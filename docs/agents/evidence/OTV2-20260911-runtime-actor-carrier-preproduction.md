# OTV2 runtime actor carrier pre-production evidence

## Scope and classification

- Task: `OTV2-20260911-runtime-actor-carrier-preproduction-530`
- Canonical PR: Draft #573
- Authority: `PRE_PRODUCTION_FOUNDATION_RUNTIME_ONLY`
- Production capacity/readiness: **not claimed**
- Physical production-capacity E2E: `NOT_APPLICABLE`

## Implemented result

The private Foundation module contains one fixed-capacity Channel actor carrier. Construction consumes
a move-only continuity grant supplied from outside the carrier, requires an explicit non-zero finite
capacity, checks identity and byte arithmetic, and uses fallible exact allocation before publishing a
carrier. The module has no default, production constructor, capacity reporter, issuer, reissuer, or
raw-fact continuity constructor.

Each reference binds `WorldId + ChannelId + ScopeOwnershipGeneration + ActorLocalId +
ActorLocalGeneration`. Lookup and removal convert the one-based identity directly to the backing slot;
there is no secondary index, tombstone collection, or retirement history.

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

## Deterministic validation

| Command | Result |
| --- | --- |
| `cargo +1.94.0 fmt --all -- --check` | PASS |
| `cargo +1.94.0 test -p oteryn-game-server runtime_actor_carrier` | PASS — 7 passed, 0 failed |
| `cargo +1.94.0 clippy -p oteryn-game-server --all-targets -- -D warnings` | PASS |
| `python tools/agents/validate_governance.py` | PASS |
| `git diff --check` | PASS |

Focused coverage uses injected capacities 1, 2, and 4 to prove M/M+1 without selecting a product
policy. Negative coverage includes zero and overflowing capacities; wrong World, Channel, and scope
generation; invalid/vacant identities; stale local generations; one-shot continuity consumption;
post-selection rollback; `g_max` exhaustion; unrelated-state preservation; and later avoidance of an
exhausted slot.

Allocation failure uses `Vec::try_reserve_exact`; deterministic arithmetic rejection precedes it and
carrier publication occurs only after the backing allocation and initialization succeed. A physical
allocator failure is environment-dependent, so the fallible operation and its pre-publication ordering
are structurally reviewed rather than forced by unsafe or global allocator substitution.

## Adversarial whole-diff self-review

- **Authority:** no production constructor/default/readiness surface and no continuity issuer exists.
- **Fencing:** all five exact-reference components are checked; a reference is not treated as current
  assignment authority beyond the carrier's consumed continuity boundary.
- **Lifecycle:** the local ID remains slot-bound; generation persists across vacancy; wrap cannot reuse
  a generation; the sole terminal bookkeeping mutation is isolated.
- **Boundedness:** backing storage is fixed after construction. Lookup/removal are direct O(1);
  insertion scans at most the explicit capacity and retains no growing history.
- **Failure atomicity:** checked construction failures publish nothing; representable selection failure
  does not mutate; exhaustion changes only the selected terminal cell.
- **Scope exclusions:** no geometry, gameplay, protocol, admission, persistence, registry, Cargo,
  deployment, Ability #508, Movement #139, or external-repository changes are present.
- **Finding verdict:** zero unresolved material self-review findings.

## Qualification state

Local deterministic qualification is complete. Hosted exact-head CI and genuinely independent
exact-head review remain **OPEN** and are intentionally not requested in this pass. This candidate is
not ready for Merge Queue, merge, production use, or a production-capacity claim.
