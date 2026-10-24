# PHASE-SPLIT-B — combat preparation phases

```yaml
task_id: PHASE-SPLIT-B
title: Split ordinary and native combat preparation into private phases
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
pr: null
base_sha: 670633ad93893a7d7e82bec64481ed878d8cc1a7
owner: Codex task worker
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat.rs
  - crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat/
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast/
  - docs/agents/tasks/PHASE-SPLIT-B.md
public_contracts: []
```

## Task and scope

Owner-approved structural refactor only. Preserve preparation order, rejection paths,
SQL, transaction and lock boundaries, random draws, public entry paths and test names.
Work stays in the allocated worktree. No Git mutation, publication, credentials,
dependency downloads, CI configuration changes or edits to the connection lane.

The immutable META policy binding resolves to organization policy 3.1.0 at
`Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; read for this operation.
The worktree started clean at the base SHA above, with detached HEAD.

## Mapping: old to new location

All paths below are relative to `crates/gameplay/dispatch/src/gameplay_transport/`.

| Old location | New location and phase |
| --- | --- |
| `ordinary_combat.rs::prepare_inner` | `ordinary_combat/prepare.rs::prepare_inner`: source qualification, owner snapshots and phase composition |
| Same function: positioning, geometry, sight paths and tile reads | `ordinary_combat/prepare.rs::prepare_spatial` |
| Same function: PvP qualification, target selection and world facts | `ordinary_combat/prepare.rs::qualify_targets` |
| Same function: lower-bound preview, training/capacity checks and live preparation | `ordinary_combat/prepare.rs::prepare_payment` |
| `native_combat_cast.rs::prepare_from_owners` | `native_combat_cast/prepare.rs::prepare_from_owners`: source qualification, owner snapshots and phase composition; re-export at the original path |
| Same function: geometry, sight paths, tile reads and operational/native facts | `native_combat_cast/prepare.rs::prepare_spatial` |
| Same function: positioned target census and native bindings | `native_combat_cast/prepare.rs::collect_targets` |
| Same function: lower-bound preview, presentation/timer capacity and training checks | `native_combat_cast/prepare.rs::prepare_preview` |
| Same function: live payment, lowering, presentation, training and timer preparation | `native_combat_cast/prepare.rs::prepare_payment` |

Private phase structs transfer the existing local values explicitly. Preparation
continues using the caller's transaction, runtime and spell-state borrows. The caller
continues to hold all existing locks through preparation. No test modules move.

## Acceptance criteria

- Every resulting preparation function is approximately 250 lines or fewer.
- Every existing test retains its full path and assertions.
- Identical complete dispatch library test inventories before and after extraction.
- Existing module tests and filtered PostgreSQL cases pass through the absolute
  queued cargo wrapper; an absent PostgreSQL route does not qualify.
- Formatting, package clippy with all targets, governance and diff checks pass.

## Authority/recovery qualification and reference parity

NOT_APPLICABLE: this task changes structure only, adding no mechanic, contract,
mutation authority, SQL or recovery behavior. Existing qualification paths and
the real-dispatch PostgreSQL tests remain the behavior evidence.

## Validation

Evidence is retained under `target/PHASE-SPLIT-B/` (ignored build artifacts).
Every cargo command uses `/home/mole/oteryn-cache/codex-bin/cargo`.

- Before: `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib -- --list`:
  PASS, 1,032 tests, zero benchmarks; `tests-before.txt`.
- After: `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib -- --list`:
  PASS, 1,032 tests, zero benchmarks; `tests-after.txt`.
- `diff -u target/PHASE-SPLIT-B/tests-before.txt target/PHASE-SPLIT-B/tests-after.txt`:
  PASS, byte-identical lists, empty `inventory.diff`; SHA-256 for both lists is
  `33d4d7588bf7506c85bcf38176c76b4b12b155b3f8aea8ecb873dbd96ecda924`.
- `/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check`: PASS.
- `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`: PASS.
- `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- ordinary_combat::`: PASS, 11 tests.
- `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- native_combat_cast::`: PASS, 31 tests (including the ordinary subset and one PostgreSQL SQL-read regression).
- `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- rune_use_bridge_postgres_real_dispatch_casts_once_and_reloads_one_debit --nocapture`: PASS, one PostgreSQL test covering ten real-dispatch scenarios, including replay, refusal, stale inputs and commit faults (264.51 seconds).
- `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- pvp_live::pvp_live_wire_melee_spell_death_replay_and_reload --nocapture`: PASS on isolated rerun, one PostgreSQL test (41.94 seconds). Initial concurrent run failed in the unchanged melee setup before spell preparation; see Unfinished.
- `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet -- bundle_gates_tests::`: PASS, five source-inventory/gate checks.
- `python3 tools/agents/validate_governance.py`: PASS, 22 policy documents and nine lanes.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `git diff --check`: PASS.

## Self-review

PASS: re-read the whole change against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`.
No behavior changes or checklist findings. Compared every extracted phase with the
original function, normalizing only whitespace, reborrows and the resolved party
module path; the ordinary protection flag transfers the same immutable tile value.
Every other byte in both original files, including tests/assertions, is unchanged
apart from the replacement module mounts/imports. `structural-review.log` retains
these checks and line counts. Preview temporaries contain value snapshots, with
no resource-owning destructors or lock guards moved across a phase boundary.

Ran `rg -n '\b(SpatialPhase|TargetPhase|PaidPreparation|PreviewPhase)\b' apps crates tools --glob '*.rs'`
and inspected every constructor/destructure site (`workspace-phase-sites.txt`). All
new fields are private and fully populated/consumed. No enum variants or trait
methods were added. All helpers have production callers through the existing
entry points; the native public entry is moved and re-exported. No tests changed.

| Module | Function lengths (including signatures) | New file length |
| --- | --- | --- |
| `ordinary_combat/prepare.rs` | entry 194; spatial 216; targets 187; payment 228 | 865 lines |
| `native_combat_cast/prepare.rs` | entry 230; spatial 185; target census 89; preview 191; payment 153 | 889 lines |

No separate file-size guard was found in the repository's governance tooling;
both new files remain below 900 lines.

## Unfinished

- CI-only shared-PC contention record:
  `gameplay_transport::monk_save::postgres_tests::pvp_live::pvp_live_wire_melee_spell_death_replay_and_reload`.
  The initial run failed at the unchanged `pvp_live.rs:422` melee setup assertion
  (`live melee changes player HP`), before spell preparation. It passed when
  rerun alone without code changes. Per the owner rule, leave full qualification
  to CI and do not investigate this load-sensitive setup further.
- No implementation work remains. Publication and full CI belong to the
  coordinating lane; Git mutation and publication are explicitly prohibited here.
