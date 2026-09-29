# OTV2-20260928-move-vis1-interest-set

```yaml
task_id: OTV2-20260928-move-vis1-interest-set
title: "VIS-1 pure interest-set logic (MOVE-RL-08/-09/-10, MOVE-VIEW-*)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/move-vis1-interest-set
pr: null
base_sha: 7d1134f090ac249f964fede017efabba91e22b90
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl movement"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/movement/interest.rs
  - apps/game-server/src/movement.rs
  - docs/agents/tasks/active/OTV2-20260928-move-vis1-interest-set.md
  - docs/agents/tasks/archive/OTV2-20260928-move-rl11-visibility-decision.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # six inserted rows after MOVE-RL-11 (#162 5876395221)
public_contracts: []
depends_on:
  - "decision docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md (#1141, 7d1134f)"
  - "#162 comments 5875929512 (packet), 5875958040 (sequencing accepted), 5876031379 (claim)"
blocks:
  - "VIS-2 wire schema and entity payload"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Adds the pure, deterministic server logic of the accepted visibility decision, with no production
caller: a validated per-Channel `VisibilitySettings` (width 15..=36, height 11..=28, Reference
18 x 14), the Global interest area and floor rule, the canonical order (floor distance, Chebyshev
distance on the observer plane, identity bytes), an interest index whose query stops at 256 results
or 1,024 examined candidates, and an interest diff that becomes a full resync above 256 entries.
The task record for the merged decision (#1141) is archived in the same change.

## Architecture and source of truth

- `PROVEN`: decision §4.1-§4.4 and §8 at `7d1134f`; the Global area formula (Canary
  `ProtocolGame::canSee`, OTClient `isAwareOfPosition`) as stated in the #162 packet.
- `DERIVED`: floor offset `d = observerZ - z` added to both axes of both bounds; Chebyshev distance
  measured after removing `d`.
- `UNKNOWN`: dense-scene cost; players per Channel.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: pure projection arithmetic over caller-supplied positions. It performs no
mutation, consumes no authority or fence evidence and adds no persistence, identity or wire
semantics.

## Acceptance criteria

- [ ] Settings reject values outside 15..=36 x 11..=28; bounds accepted; no setters.
- [ ] Offsets at 18 x 14, 15 x 11 and 36 x 28; floor rule above and below ground; offset sign.
- [ ] 256 results accepted and the farthest beyond it excluded; own actor always present.
- [ ] Query result identical under different insertion orders and equal to a brute-force canonical
      sort; 1,024 examined-candidate ceiling at max and max+1.
- [ ] Diff: exactly 256 entries is a delta, more is a resync.
- [ ] Exact-head CI and independent review per #162.

## Excluded scope

`lib.rs`, `crates/protocol-oteryn/**`, wire/codec/capability, config loader, Channel wiring,
entity payload fields (VIS-2), and `MOVE-RL-11` (stays 1; its change moves to VIS-2). The only
registry change is six inserted rows after `MOVE-RL-11`.

## Implementation / findings

- `movement/interest.rs`: local `VisibilityPosition` (floor validated 0..=15), immutable
  `VisibilitySettings`, `InterestIndex` (identity map plus a per-cell identity set),
  `query`/`query_with`, `diff_interest`.
- Enumeration is floor distance, then ring, then per (floor distance, ring) group the smallest
  identities, so insertion order cannot matter. The own actor is emitted first and counts as one
  result and one examined candidate.
- The optional `accept` filter of `query_with` is the only way a candidate can be examined and
  rejected; it makes the 1,024 ceiling reachable and testable ahead of VIS-2.
- No module-level `dead_code` allow: `movement` is already allowed in `lib.rs`.
- Codex P1 (unbounded dense-cell scan) accepted and fixed: each cell contributes at most the remaining
  candidate budget of its smallest identities (`extend_bounded`); result order and semantics unchanged.

## Registry rows

Inserted immediately after `MOVE-RL-11` under the registry lease granted in freeze order
(#162 5876395221): `MOVE-RL-08`, `-09`, `-10`, `MOVE-VIEW-WIDTH`, `MOVE-VIEW-HEIGHT`,
`MOVE-VIEW-FLOORS`. Insert-only (135 lines); no existing row and no `updated_at` changed. Consumers
name only `movement::interest::` items; the wire consumer of `-08`/`-10` arrives with VIS-2.

## Validation

### Focused

- command/run: `cargo test --locked -p oteryn-game-server --lib movement::interest`
- result: 18 passed (re-run after merging main `840072a` and inserting the registry rows)

### Component/integration

- command/run: `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`; `cargo test --locked -p oteryn-protocol-oteryn`; `validate_governance.py`; `validate_repository_policy.py`; `python3 -m json.tool` on the registry
- result: pass

### E2E

- scenario: `NOT_APPLICABLE`: no production caller or wire path.

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending (lane coordinator decides per #162)
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: pending
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: main merged, registry rows written, local validation complete; awaiting freeze
status: implementing
branch: claude/move-vis1-interest-set
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: lane coordinator opens the PR and freezes the pushed head
```
