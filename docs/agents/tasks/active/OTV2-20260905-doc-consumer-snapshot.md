# OTV2-20260905-doc-consumer-snapshot

```yaml
task_id: OTV2-20260905-doc-consumer-snapshot
title: Restore reviewed document-consumer PR routing
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/doc-consumer-snapshot
issue: 309
pr: null
base_sha: b9b1a4317858bffc25ad6af3cffcf7b5eff93445
head_sha: null
final_head_sha: null
owner: Codex-doc-snapshot
created_at: 2026-09-05T14:44:38Z
updated_at: 2026-09-05T14:44:38Z
execution_budget_minutes: 120
large_budget_reason: Independent review, full hosted qualification, protected integration and actual docs measurement
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260905-doc-consumer-snapshot.md
  - docs/agents/evidence/OTV2-20260905-system-ci-impact/audit.json
  - docs/agents/evidence/OTV2-20260905-system-ci-impact/README.md
public_contracts: []
depends_on:
  - issue: 308
    state: active
external_repositories: []
```

## Outcome and source of truth

Restore the existing protected-base neutral-documentation PR lane after a bounded consumer re-audit. Live Issue #309 owns this task's status and exact final head; programme #308 owns measured history, ranking and overall lifecycle. This packet records the technical contract, not a second mutable status database. The coordinator owns publication, independent review and integration; the worker owns one isolated branch/worktree and the allocated paths.

Static admission measurements and the coordinator's workflow/ownership/ranking audit are preserved in `docs/agents/evidence/OTV2-20260905-system-ci-impact/README.md` and `audit.json`. They are immutable supporting evidence, not mutable programme status or achieved savings.

## Consumer proof

PROVEN: the immutable comparison from #297 integration `9631cbfe718e75d6bc530352fb811e08a444b6b0` to admission main changes only five server test paths: `authority_invariants.rs`, `durability_postgres.rs`, `server_ci_qualification.rs`, `support/authority_matrix.rs`, and `support/authority_recovery.rs`, under `apps/game-server/tests/`. Production, Cargo, build and migration inputs remain unchanged in that range.

The authority matrix constructs sources in memory from fixture values. Recovery queries PostgreSQL resolver rows and restarts the current test executable with explicit scenario environment. Process qualification executes the Cargo-built server with CLI arguments. None introduces a document reader. Existing Rust/SQL includes stay package-local. The architecture-check file reader consumes `workspace-boundaries.toml`, already a protected build input. No new generator or external artifact provenance is introduced.

Protected-tree Git mode/type/blob/path records independently produce non-server SHA256 `9f7aff4dc25c9c6561b77ea73342b675eeccb1d008ab9d1fbdbd504618ec5ab8` (63 entries, unchanged) and all-consumer SHA256 `f8eed774249df64a5a64612b4a169a73bac093a7bcbfb21e59ea0e06dd2ddc26` (124 entries). The prior docs digest was stale, correctly selecting FULL. This task adopts only the reviewed all-consumer digest. Future unreviewed reader/tree changes still select FULL; no automatic snapshot update is authorized.

## Acceptance criteria and exclusions

- Existing neutral-doc PR classification selects no Rust lanes when both reviewed snapshots match; always-required authority, security, governance and fan-in remain required.
- Modified/new server and non-server consumers, unknown/mixed/malformed inputs, rename and special-mode cases retain conservative FULL behavior.
- The implementation candidate changes control-plane code and therefore qualifies FULL using the protected-base classifier.
- Both classifier suites, repository/governance checks, whole-diff self-review, independent exact-head deep review, hosted CI, normal FULL Merge Queue and protected-main readback are required.
- Actual post-deployment docs qualification and comparable hosted measurements are required for savings claims. A normal documentary lifecycle change may qualify; no no-op or product probe.

No workflow, ruleset, required status, PR fan-in, Merge Queue, protected audit, runtime, Cargo or product-test behavior changes. Post-merge docs still use existing FULL policy. No cache/artifact execution-proof reuse.

## High-risk authority/recovery qualification

NOT_APPLICABLE to runtime session/lease/controller/persistence matrices: no runtime authority is changed. The CI trust boundary is material and receives independent review. Only protected-base classifier execution may select PR omissions; candidate diagnostic output cannot supply trusted routing.

## Validation

Fresh baseline PR regression suite passed. TDD RED added an independent reviewed-input fixture and failed with `rust=True`, `windows=True`, `unreviewed-document-consumer-inputs` on the previous digest (exit 1). Minimal GREEN updates one snapshot constant; the same suite passes (exit 0). New negative fixtures exercise real input hashing with changed or added server/non-server consumer tree records and require FULL. Fixtures are independent of current HEAD so later legitimate consumer changes remain valid FULL inputs instead of breaking repository validation.

Existing post-merge real-Git adapter suite, repository policy and governance validators pass. Static audit JSON parses successfully and diff whitespace validation passes. Exact-head hosted/review evidence is recorded on Issue #309 after the final commit. No self-referential SHA/checkpoint commits.

## Review and closeout

Whole-diff author review and one genuinely independent stable-head Codex deep review are required for this classifier authority update. External review remains advisory; current `game-gate` and normal FULL Merge Queue enforce integration. Hosted qualification, actual measured savings, final readback and ownership release remain pending; local GREEN is not programme completion. Archive only after terminal acceptance and coordinator ownership release.

## Context checkpoint

```yaml
last_progress: Focused RED then minimal GREEN and both classifier regression suites passed
status: validating
branch: ci/doc-consumer-snapshot
head_sha: null
pr: null
final_head_sha: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Publish the verified material candidate for independent exact-head review and hosted FULL qualification
```
