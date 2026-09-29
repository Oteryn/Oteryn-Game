> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #690 is merged on protected main as `54869eb9db46d83beaaadfc557270d58a39cc6dd`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-content-world-cw2-b4-project-adapter-504

```yaml
task_id: OTV2-20260919-content-world-cw2-b4-project-adapter-504
title: CW2-B4 protected evidence to canonical project adapter
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-b4-project-adapter-refresh-504
issue: 162
pr: null
base_sha: c22eb6cb5e953b7fa59dea7bd5d50114a10e8e7d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world import
created_at: 2026-09-19T23:59:47Z
updated_at: 2026-09-20T00:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b4_import.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/tests/content_world_cw2_b4_import.rs
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw2-b4-project-adapter-504.md
public_contracts: []
depends_on:
  - protected PR #684 CW2-B4 evidence batch
  - protected PR #685 canonical project snapshot
  - protected PR #687 Linux canonical project capture
  - protected PR #689 trusted routing snapshot refresh
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Provide the smallest reusable API that converts the exact protected CW2-B4
Ability/Effect/Formula evidence bytes into the existing `ImportBatch` type. A
caller appends that candidate-only batch to its own `ProjectDraft`; the existing
canonical writer and parser remain the sole project serialization authority.

The adapter creates no project, package, world or native content identity. It
does not promote the protected `UNKNOWN / PENDING / BLOCKED` evidence to
gameplay behavior.

## Architecture and source of truth

- `PROVEN` — protected B4 evidence blob
  `56b8e4b1d143cc9a68aa691aa30a292d4befe2c3` has raw SHA-256
  `97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491`
  and product digest
  `56cef2d78442a37c00daa4cb737007e8a069e10ae3d4a298c0d38c38976f8289`.
- `PROVEN` — protected B4 source is `Oteryn/Oteryn-Game` revision
  `03a821edd828e24ccff6e2cb7fc819a776cbd238`; the evidence bytes bind all six
  exact protected source inputs.
- `PROVEN` — mapper profile
  `OTERYN_CW2_ABILITY_EFFECT_FORMULA_EVIDENCE_MAPPER/v1` is blob
  `e6d98aadd352ad36b466970e1f7182e1bf93643b`, with LF-canonical SHA-256
  `bc68f0f63a5dd6ea5ee7a3c20b708d6ea78c6033a2f5dddb518c49d6f45f8666`.
- `PROVEN` — the protected project API already owns `ImportBatch`,
  `ProjectDraft`, `CanonicalProjectDocuments`, `ProjectSnapshot` and
  `WorldProject`; this child needs no second schema or serializer.
- `PROVEN` — PR #688 head
  `b125a5225f28d465105a4216be23950d1cf8fd33` received independent review
  PASS with zero P0/P1/P2 findings. This successor preserves the exact reviewed
  adapter, module export and focused-test blobs on protected main after #689.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` — this bounded evidence adapter returns candidate-only data and
performs no production mutation, authority grant, PREPARE/COMMIT, controller,
session, fence or persisted recovery interpretation.

## Acceptance criteria

- [x] Only the four allocated paths change.
- [x] Input bytes are bounded before JSON decoding allocation; the exact bound
  passes and one extra byte fails closed.
- [x] Exact evidence, product, source revision and mapper bindings are retained.
- [x] Both candidates retain qualitative family only, explicit formula losses,
  `UNKNOWN` target/formula, `PENDING` source/legal state, unresolved null native
  identity and `BLOCKED` executable promotion.
- [x] No project/package/world/native identity is created by the adapter.
- [x] A caller-owned draft round-trips through the existing canonical writer and
  parser with byte-stable output.
- [x] Repeated imports are deterministic and create no linked native definition.
- [x] Digest, provenance, extra/missing record, native key, formula and promotion
  drift fail closed.
- [ ] Exact published head passes repository CI and aggregate `game-gate`.

## Excluded scope

No edits to project models, filesystem capture, Cargo, schemas, compiler,
runtime or client; no filesystem writer or CLI; no B5/B6, native/vase mapping,
formula invention, Reference parity claim or production authority.

## Implementation / findings

The public adapter checks its finite byte ceiling before decoding, validates the
protected provenance and unresolved candidate boundary, and finally requires
the exact protected raw digest. It returns one existing `ImportBatch` whose
normalized fields retain the protected product digest and explicit quantitative
losses. Reimport states remain `Unchanged`; none is a conflict or promotion.

## Validation

### Focused

- `cargo test --locked -p oteryn-game-server --test content_world_cw2_b4_import`
  — PASS, 4 tests.
- `cargo test --locked -p oteryn-game-server --test content_world_project`
  — PASS, 19 tests.
- `cargo test --locked -p oteryn-game-server --test content_world_project_fs`
  — PASS, 9 tests.
- `cargo test --locked -p oteryn-game-server --test content_reference_playable`
  — PASS, 31 tests.

### Component/integration

- `cargo fmt --all -- --check` — PASS.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings` —
  PASS.
- `cargo metadata --locked --format-version 1` — PASS.
- `cargo run --locked -p oteryn-architecture-check -- workspace .` — PASS.
- `cargo test --locked --workspace` — PASS.
- `python -B tools/agents/validate_governance.py` — PASS.
- `python -B tools/repository/validate_repository_policy.py` — PASS.
- `git diff --check` — PASS.

### E2E

`NOT_APPLICABLE` — this child ends at canonical project parsing and has no
runtime activation or publication behavior.

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, whole-diff adversarial review
- material findings: none
- verdict: PASS — the byte bound precedes allocation; exact raw digest closes
  unmodelled evidence drift; mapped data remains candidate-only and callers own
  all project identity and canonical serialization

## Independent review

- required: NO — bounded candidate-only adapter using protected types and data;
  no shared model, runtime, identity or authority change
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #684, #685, #687 and #689 are protected
  predecessors; #688 is superseded unmerged by this fresh-base successor
- protected auto-merge: parent control plane owns integration
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Four-path adapter passed focused, regression and workspace gates
status: ready
branch: agent/content-world-cw2-b4-project-adapter-refresh-504
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
next_action: Commit and publish the exact four-path candidate
```
