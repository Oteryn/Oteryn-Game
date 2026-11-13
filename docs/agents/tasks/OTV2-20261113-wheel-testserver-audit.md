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
updated_at: 2026-11-13T21:07:02Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/OTV2-20261113-wheel-testserver-audit.md
  - docs/reference/wheel-of-destiny-testserver-20261113.md
  - docs/reference/wheel-of-destiny-testserver-20261113/
  - crates/gameplay/progression/src/wheel_effects.rs
  - crates/gameplay/progression/src/wheel_effects/
  - crates/gameplay/dispatch/src/gameplay_transport/gem_presentation/tests.rs
  - apps/game-server/tests/support/character_wheel_cast_postgres_cases.rs
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
remain open; this PR is a draft. Six further current-menu value/type mismatches are
qualified in W07 and remain open pending combat/stat qualification.

## Architecture and source of truth

- Baseline: `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`.
- Bound META: `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, version 3.1.0.
- Existing Wheel/Gem producers, consumers and fenced writers remain their owners.
- Root is the only branch publisher. Workers did not commit, push, merge or trigger review.
- Initial documentation candidate: `b8239a172cc2cb4c906fe914c7653d807f7a3c21`.
- Numeric candidate: `bff551df2c10c27857da4c4665092c53bc633917`; all local gates were
  rerun on that published head.
- Source-qualified and independently reviewed candidate:
  `cec30251a3592eaf6b69c1bd28ae7c8c738b821a`, tree
  `b61a5bda21d1cb9f6c78c99ac93d098a6fde259e`; production Rust blobs are identical to
  the numeric candidate. The subsequent delta updates evidence and stale consumer test
  expectations, without changing production behavior.
- Exact final candidate/check SHAs are recorded in the PR; no self-referential commit
  hash is invented in this file. Root returned to authoring for the bounded follow-up;
  the final candidate is separately frozen and checked before its qualification claim.

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
- [x] Stable published implementation candidate has independent review with exact-head readback.
- [ ] Final candidate has completed consumer checks and exact-head CI readback.
- [ ] Native combat/paid operations/save/relog acceptance is completed; host disconnections
      prevented controlled operations in this session.

## Implementation and reference parity

| Finding | Reference | Original behavior | Result |
| --- | --- | --- | --- |
| Workshop stats | +5 HP/Mana/Capacity per upgrade, without gems | No shared stat contribution | Added once under current grade revision; all five vocations tested |
| Gem Revelation Mastery | 150/165/180/225, official composed preset has Outburst I | Values doubled; preset had Outburst II | Single contribution; all 16 IDs/all grades and exact preset tested |
| Monk mixed Mana | 100/110/120/150; official mod 33 Grade IV shows150 | Zero for IDs33–36 | Catalogue values restored with resistance retained |
| Vessel layout | Eight, L/R/G/L/G/R/G/G, four +125 Convictions | Twelve, L/R/G repeated; old Convictions | Confirmed open implementation/migration gap |
| New supreme mods | IDs94–113, all five vocations | Catalogue ends at93 | Qualified derived table retained; not admitted to runtime |
| Sorcerer elemental state | Required by Lord of Destruction-related behavior | Elemental spells CONTRACT_GAP; access producer absent | Open integration gap, no claim of observed live refusal |
| Other retained overrides | Six current Grade IV menu values/types | Capacity, Flurry, Sweeping Takedown, Divine Caldera and Nature's Embrace differ | Confirmed reference mismatches retained in W07; not accepted differences |
| Removal permission | Owner decision2026-11-05 | Combat-lock/PZ rule | Preserved intentional difference |

## Validation

Executed for the implementation batch, and repeated on the published source-qualified
candidate `cec30251a3592eaf6b69c1bd28ae7c8c738b821a`:

- Workshop RED: actual `(0,0,0)`, expected `(5,5,5)` after a paid grade upgrade.
- Gem RED: Grade I Mastery300000 vs150000; Monk Mana0 vs100000.
- Composed preset RED: Spiritual Outburst stageII vs official stageI when only the original
  Mastery multiplier was temporarily restored; the repair was restored before all gates.
- Consumer RED on published `cec30251a3592eaf6b69c1bd28ae7c8c738b821a`:
  [CI job132142331795](https://github.com/Oteryn/Oteryn-Game/actions/runs/43499498221/job/132142331795)
  failed `gem_presentation_all_canonical_vocations_fit_and_use_executed_corrections`
  at the stale Grade I Mastery `>=300000` assertion. The shard completed3989 passes/1 failure.
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
- The follow-up found active dispatch presentation tests still expecting zero Monk Mana
  and doubled Mastery, plus the same doubled Mastery expectation in the PostgreSQL cast
  fixture. Their source-proven expected values are corrected without changing the fixture's
  unresolved Grade IV automatic-domain-point phase. Their executed results and the final
  exact-head checks are recorded in the PR; a database guard that returns early is never
  counted as PostgreSQL qualification.
- The cloud dispatch lib-test build was killed by SIGKILL at `-j1` before executing any
  test. Its cause is UNKNOWN; the surface did not expose cgroup memory events. A prior
  missing tracked fixture was hydrated from the exact existing HEAD and is unchanged.
  Neither compilation failure is presented as regression RED. The prepared native PC
  qualification uses the actual dispatch test target, not a substitute implementation.
- No complete game-server/client build or deployed gameplay acceptance is claimed.

The official public test planner accepted the exported preset and independently confirmed
the compared values. Planner grades are explicit Grade IV and are not the native character's
full Workshop state. A fresh native session later showed Workshop +75 HP/Mana/Capacity,
actual HP8400/Mana8305/total Capacity20595 and +3 promotion points. It disconnected again
before any upgrade, gem removal or temporary allocation. All four controlled native checks
remain NOT_EXECUTED, with zero game-value mutations. Evidence and limits are retained in
`native-checks.json`. The literal level757 and42/742 budget were rechecked in retained
captures. The derived32 points beyond level and Workshop remain unattributed; no specific
reward is invented. Existing Task Board/pilgrimage budget producers are recorded separately.

## Self-review / independent review

Root inspected the numeric diff and current vitals/spell/capacity callers. Source research
and native evidence were independently collected. The cloud Codex CLI exited during
app-server initialization with a read-only filesystem error, before any model review began.
After fresh PR comments/reviews/provider-status deduplication, one native WSL Codex review
ran on an isolated clean checkout of `cec30251a3592eaf6b69c1bd28ae7c8c738b821a`:
single-agent `gpt-6.1-sol`, high effort, read-only sandbox, exit0 and worktree unchanged.
Its verdict was CLEAN with no P0–P3 findings. It inspected the full eight-file diff and
callers, but did not independently rerun Rust, PostgreSQL or native combat checks.
The later evidence and consumer-assertion delta changes no production logic; no optional
second paid review was triggered. Review is advisory and is not substituted for tests.
The new lower-grade evidence directly qualifies all Mastery values. Monk row naming
remains an explicit high-confidence inference from the unique official Grade IV match.

## Context checkpoint

Publish the coherent evidence/consumer-test follow-up, verify its exact tree/head, complete
the bounded consumer checks and update PR validation. Full current Winter parity remains
blocked by the documented implementation gaps and native-session limitation.
