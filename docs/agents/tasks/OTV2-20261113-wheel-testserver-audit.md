# OTV2-20261113-wheel-testserver-audit

```yaml
task_id: OTV2-20261113-wheel-testserver-audit
title: Wheel and Gem Atelier audit against the Winter Update test server
type: mechanic
mode: AUDIT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/wheel-testserver-audit-20261113
pr: 3181
base_sha: ce45063507c22a11e0db7e9c0ff86dacbe6ff834
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-wheel-audit-root
created_at: 2026-11-13T19:31:16Z
updated_at: 2026-11-13T20:12:21Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/OTV2-20261113-wheel-testserver-audit.md
  - docs/reference/wheel-of-destiny-testserver-20261113.md
  - docs/reference/wheel-of-destiny-testserver-20261113/
  - crates/gameplay/progression/src/wheel_effects.rs
  - crates/gameplay/progression/src/wheel_effects/
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Compare current Wheel/Gem execution with the Winter Update test server, preserve accepted
Oteryn differences, repair source-confirmed defects and retain exact gaps. The owner
explicitly authorized native client actions and branch/PR/push. This task does not declare
full Global parity, protected integration or production deployment complete.

The [audit](../../reference/wheel-of-destiny-testserver-20261113.md) records current sources,
producer/consumer coverage, remaining blockers and a bounded durable/protocol repair plan.
Three numeric defects are repaired in the common progression projector. Eight-slot
migration, new Mastery Convictions, 20 new mods and Sorcerer elemental-state integration
remain open; this PR is a draft.

## Architecture and source of truth

- Baseline: `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`.
- Bound META: `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, version 3.1.0.
- Existing Wheel/Gem producers, consumers and fenced writers remain their owners.
- Root is the only branch publisher. Workers did not commit, push, merge or trigger review.
- Initial documentation candidate: `b8239a172cc2cb4c906fe914c7653d807f7a3c21`.
- Numeric candidate: `bff551df2c10c27857da4c4665092c53bc633917`; all local gates were
  rerun on that published head. The subsequent source-qualification delta changes no Rust blobs.
  Exact subsequent candidate/check/review SHAs are recorded in the PR; no self-referential
  commit hash is invented in this file.

## High-risk authority/recovery qualification

No durable mutation, migration, session/lease gate, receipt or wire boundary is changed.
The production delta changes only shared stat/gem arithmetic. Existing caller eligibility,
current-revision checks and paid Workshop transitions are preserved. The three arithmetic
changes have negative regressions and composed projection coverage.

The unimplemented twelve-to-eight transition is high risk and has explicit migration,
historical replay, paid-inventory and negotiated-client gates in the audit. It is not
represented as a constant-only fix.

## Acceptance criteria

- [x] Dated official/client evidence identifies current Winter Update changes.
- [x] Source-to-consumer map and exact uncovered behaviors are retained.
- [x] Every defect repaired here has a reproduced negative case and passing repair.
- [x] Required local progression checks pass; source provenance is retained.
- [ ] Stable published implementation candidate has independent review and CI readback.
- [ ] Native combat/paid operations/save/relog acceptance is completed after session login.

## Implementation and reference parity

| Finding | Reference | Original behavior | Result |
| --- | --- | --- | --- |
| Workshop stats | +5 HP/Mana/Capacity per upgrade, without gems | No shared stat contribution | Added once under current grade revision; all five vocations tested |
| Gem Revelation Mastery | 150/165/180/225, official composed preset has Outburst I | Values doubled; preset had Outburst II | Single contribution; all 16 IDs/all grades and exact preset tested |
| Monk mixed Mana | 100/110/120/150; official mod 33 Grade IV shows150 | Zero for IDs33–36 | Catalogue values restored with resistance retained |
| Vessel layout | Eight, L/R/G/L/G/R/G/G, four +125 Convictions | Twelve, L/R/G repeated; old Convictions | Confirmed open implementation/migration gap |
| New supreme mods | IDs94–113, all five vocations | Catalogue ends at93 | Qualified derived table retained; not admitted to runtime |
| Sorcerer elemental state | Required by Lord of Destruction-related behavior | Elemental spells CONTRACT_GAP; access producer absent | Open integration gap, no claim of observed live refusal |
| Removal permission | Owner decision2026-11-05 | Combat-lock/PZ rule | Preserved intentional difference |

## Validation

Executed before publication of the implementation batch:

- Workshop RED: actual `(0,0,0)`, expected `(5,5,5)` after a paid grade upgrade.
- Gem RED: Grade I Mastery300000 vs150000; Monk Mana0 vs100000.
- Composed preset RED: Spiritual Outburst stageII vs official stageI when only the original
  Mastery multiplier was temporarily restored; the repair was restored before all gates.
- `/home/agent/.cargo/bin/cargo test --locked -p oteryn-gameplay-progression --quiet`:
  **174 passed, 0 failed**, plus0 doctests.
- `/home/agent/.cargo/bin/cargo clippy --locked -p oteryn-gameplay-progression --all-targets -- -D warnings`:
  pass. Existing vendor Tokio warnings do not fail the command.
- Formatter and `git diff --check`: pass for the numeric patch; full final delta rechecked
  at the publication boundary.
- `python3 tools/agents/validate_governance.py`: pass,22 documents/9 lanes for the
  updated audit/task/evidence batch.
- `python3 -m unittest discover -s tools/agents/tests`:129 tests pass for the updated
  documentation batch.
- No PostgreSQL integration tests, complete game-server/client build or deployed gameplay
  acceptance were run. Existing non-progression consumer tests were inspected only.

The native client disconnected after the first Wheel observation. The official public
test planner accepted the exported preset and independently confirmed the compared
values. Planner grades are explicit Grade IV and are not the native character's full
Workshop state. Screenshot retention and source limitations are recorded in the audit.

## Self-review / independent review

Root inspected the numeric diff and current vitals/spell/capacity callers. Source research
and native evidence were independently collected. The cloud Codex CLI exited during app-server initialization with a read-only filesystem
error, before any model review began. An isolated checkout on the authorized PC was
prepared for the one native WSL Codex review (same model/effort, read-only sandbox).
The review result is recorded in the PR; it is not replaced by tests.
Review dispatch requires a fresh PR comments/reviews/provider-status deduplication check.
The new lower-grade evidence directly qualifies all Mastery values. Monk row naming
remains an explicit high-confidence inference from the unique official Grade IV match.

## Context checkpoint

Publish the coherent numeric/evidence batch, verify its exact tree/head, run the bounded
checks and required review, then update PR validation. Full current Winter parity remains
blocked by the specifically documented implementation gaps and native-session limitation.
