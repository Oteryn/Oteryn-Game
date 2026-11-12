# OTV2-20261112-overnight-read-family-repair

```yaml
task_id: OTV2-20261112-overnight-read-family-repair
title: Repair collected trade read contention and party fixture liveness
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/overnight-trade-read-family-20261112
pr: 3128
base_sha: 096e0afdcbba45c531854ef23dc5defad7f598af
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex overnight integration lead
created_at: 2026-11-12T02:08:00Z
updated_at: 2026-11-12T02:39:00Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/player_trade/preflight.rs
  - crates/gameplay/dispatch/src/gameplay_transport/player_trade/command.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/player_trade.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/party_membership.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/party_effects.rs
  - docs/agents/tasks/archive/OTV2-20261112-overnight-read-family-repair.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and collected evidence

Preserve ordinary foreground trade reads under background-holder contention,
and make the existing real party fixture peers honor live protocol probes.

- PROVEN source gap: shared trade read retry uses a budget without foreground
  demand; Offer/counter, Accept and inspection share it. Wire Look bypasses the
  classified retry, and final Accept reads progression after that budget ends.
- PROVEN runtime failure: #3102 head4d1bd69a, run43135769749/job130915738145,
  party_effects membership/shared eligibility times out after liveness_missed.
  The complete PostgreSQL inventory is23 passing partitions and this one failure.
- PROVEN source gap: party membership/effects snapshot and result readers
  discard live probes instead of acknowledging them.
- UNKNOWN: the exact cause of Node's generic trade-counter Rejected result.
  These source gaps are not represented as its proven cause or a complete Node fix.
- Coordination: #3124. No writes to another session's canonical branch.

## Implementation and invariants

The shared read budget retains foreground demand. Look retries classified
unavailability while preserving the existing true-refusal, stale and terminal
fence outcomes. Potential final Accept reads progression inside that same pure
preflight; missing/invalid progression retains its terminal None path. No
lifecycle mutation, frozen exchange, storage write, unknown commit or recovery
operation gains retries. Authority, item, pair and session-generation fences stay
inside the actual reads and writer.

The new real two-admission test uses command reservations and terminalization,
then Counter/Look/final Accept through the actual owner. It requires pending
while held, denies16 immediate background retakes before repoll, checks exact
results, retains the READY dialog and requires exactly one durable exchange.
It is not a new TCP barrier and does not isolate contention at the final
progression query. Existing TCP/replay/cancellation/inspection cases remain.

Both party fixtures share a typed live-frame reader. It acknowledges actual
probe ids with the admitted connection generation, without consuming command
ids or claiming applied reducer state. All four snapshot/result loops use it.
Existing30s deadlines, expected replies, eligibility and stale-fence assertions
are retained. No workflow, migration, protocol or gameplay rule changes.

## Acceptance

- [ ] Independent whole-batch Codex review has no unresolved confirmed P0/P1.
- [ ] Exact-head native/Clippy and all24 PostgreSQL partitions pass.
- [ ] New actual command regression and existing party/TCP cases execute and pass.
- [ ] Required checks and review threads are resolved before protected squash.
- [ ] Full post-merge main qualification succeeds.

## Reference parity

NOT_APPLICABLE: availability repair and fixture protocol compliance within
accepted mechanics; no new gameplay behavior or balance rule is introduced.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo metadata --locked --no-deps --format-version 1`: pass.
- Scoped Rustfmt1.99.1 on all five Rust files: pass.
- `git diff --check`: pass.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass,129 tests.
- No local client/server compilation or database suite was run. The owner
  explicitly routes heavy verification to GitHub; exact-head results are pending.
- First independent whole-diff Codex review is pending on the frozen candidate.

## Closeout

Implementation authoring is complete. This task is archived in its own PR's
final authoring commit, before freeze, with review and qualification pending.
Completion here does not claim successful runtime validation or integration.
The exact head is the subsequent FREEZE_SHA comment. Intended merge/result:
squash merge of #3128, resolved only by actual protected-main readback. Rollback
is an ordinary revert; no schema or persisted format changes.
