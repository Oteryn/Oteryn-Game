# OTV2-20260908-wp3-aws-lc-kx-resource-accounting-439

```yaml
task_id: OTV2-20260908-wp3-aws-lc-kx-resource-accounting-439
title: Resolve WP3 AWS-LC KX native resource accounting boundary
mode: CONTRACT
status: NOT_ACTIVE
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/wp3-aws-lc-kx-resource-439
issue: 439
pr: null
base_sha: 54f19765c07e3b33ce2d9c10ad57df4818434a52
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex architecture author for Issue 439
created_at: 2026-09-08T00:00:00Z
updated_at: 2026-09-08T00:00:00Z
execution_policy: continuous_progress
classification: ARCHITECTURE_BLOCKED_EVIDENCE_REQUIRED
architecture_disposition: D_BLOCKED_MISSING_PROVIDER_BOUNDARY
activation: architecture-only; NOT_ACTIVE until Work protects the decision and separately allocates follow-up
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_KX_RESOURCE_ACCOUNTING_DECISION_2026-09-08.md
  - docs/agents/tasks/active/OTV2-20260908-wp3-aws-lc-kx-resource-accounting-439.md
public_contracts:
  - DUR-FRESH-RESOURCE-ENVELOPE-V1
depends_on:
  - Issue 439
  - agent/sqlx-driver-budget-351@8a97ed5e0bdfd42b934295ecf3f98584bb49a00f
  - KX source af54111b3f0125e130e0bac66057f2afa2bcf07c
blocks:
  - WP3 KX continuation in Issues 351/PR 356
  - terminal WP3 evidence required by WP4 Issues 329/335
  - dependent Server Seam Issue 247 readiness
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Publish exactly two documentation files that answer Issue #439 without changing executable authority. The architecture decision selects D because the pinned provider evidence proves operation-scaled opaque native KX state but proves neither a supported per-operation owner hook nor a complete finite native reservation. WP3 KX remains blocked.

## Architecture and source of truth

- **PROVEN:** protected admission is `main@54f19765c07e3b33ce2d9c10ad57df4818434a52`; the canonical material head is `agent/sqlx-driver-budget-351@8a97ed5e0bdfd42b934295ecf3f98584bb49a00f`.
- **PROVEN:** the seven specified rustls KX sources and its manifest are byte-identical from `af54111b3f0125e130e0bac66057f2afa2bcf07c` to the material head. Exact digests are recorded in the architecture decision.
- **PROVEN:** the production lock resolves `aws-lc-rs 1.18.0` (`ce2b2dcc879c3bae0d371e77c99f2238400ef24ec001394befa67b6e543add9e`) and `aws-lc-sys 0.44.0` (`f09fae7be8bb3174e05c6afdb34199e6dc0c7c04ba9fa237b1967adfbde27483`), with package VCS revision `f464440d1fd3983ce9fb023e9eaf1698530919a2`.
- **PROVEN:** initial and HRR starts use ordinary `SupportedKxGroup::start()` with no owner. Classical and ML-KEM starts retain native `EVP_PKEY` owners plus visible Rust storage; hybrid retains both children and a combined vector.
- **PROVEN:** `CRYPTO_set_mem_functions` is process-global, one-time, debug-recommended, untagged, and constrained; it is not a supported per-operation KX owner.
- **DERIVED:** safe HRR accounting must cover conservative old+replacement overlap, and hybrid accounting must cover both children plus combined/temporary state.
- **UNKNOWN:** complete native peak/retained bytes, a supported scoped owner/preallocation interface, and a rigorously separable finite provider-global class.
- **CONFLICT:** `DUR-FRESH-RESOURCE-ENVELOPE-V1` requires charging or finite reservation before acceptance, while the pinned KX/provider API supplies neither proven mechanism.

The durable analysis is `docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_KX_RESOURCE_ACCOUNTING_DECISION_2026-09-08.md`. This unmerged record and decision are candidates only and grant no runtime/provider/Cargo/registry authority.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation-only architecture evidence; no production mutation, PREPARE/COMMIT, controller, session, recovery, credential, or external-repository action.
```

## Acceptance criteria

- [x] Choose exactly one disposition: D — `BLOCKED_MISSING_PROVIDER_BOUNDARY`.
- [x] Record exact admission, material/KX revisions, file digests, locked crate versions/checksums, and provider package revision.
- [x] Classify material statements as FACT/PROVEN, DERIVED, UNKNOWN, or CONFLICT without inventing a provider number.
- [x] Define initial, hybrid, HRR overlap, completion/error, and final-drop custody.
- [x] Preserve ordinary `start()` and require dedicated owner-aware custom-provider failure before allocation.
- [x] Preserve `X25519MLKEM768`, X25519, P-256, and P-384 ordering/security semantics without blanket `ALL_KX_GROUPS` authority.
- [x] State RED/GREEN and negative proof obligations and all prohibited evasions.
- [x] Leave future mutation paths empty because D is not implementation-ready.
- [x] State programme effects and make provider/evidence work the sole next Work action.
- [ ] Work protects the independently reviewed decision. Until then this record remains `NOT_ACTIVE`.

## Excluded scope

No edit or authority for PR #356's branch, rustls/aws-lc/sqlx/Tokio/Game source, Cargo/lock, `RESOURCE_LIMITS_REGISTRY.json`, workflows, AGENTS, production, external repositories, protection, Issue body, readiness, enqueue, or merge. No PR is part of this architecture publication lane. Issue #439 remains open.

## Implementation / findings

### Disposition

`D_BLOCKED_MISSING_PROVIDER_BOUNDARY`. A is not proven because the available allocator replacement is process-global debug machinery rather than a supported KX owner. B is not proven because no authoritative complete native bound exists. C is unsafe because the key objects are created per start and can scale with admitted operation/HRR activity. Therefore there is no material implementation path set.

### Required follow-up

After protected integration, Work may allocate only evidence/provider investigation that returns either a supported scoped allocation owner/preallocation/release interface or provider-authored complete finite native bounds for the exact pinned reachable KX paths. It may not allocate a material KX lease from this result.

## Validation

### Focused

- command/run: governance/policy validation selected for documentation paths
- result: pending publication validation

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation-only blocked decision changes no executable component
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime surface changed
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending after commit
- trigger source: branch push only
- workflow/run/job: repository CI, if automatically created
- runner assignment: pending
- classification: documentation-only candidate; not Ready
- result: pending remote readback

## Self-review

- exact head: pending after commit
- method/reviewer: implementing architecture author; complete whole diff, changed-path inventory, whitespace, and policy/governance checks
- material findings: pending
- verdict: pending

## Independent review

- required: YES — provider/native resource boundary blocks accepted same-ledger security work
- exact head: pending after commit
- method/auditor: Work-selected independent reviewer after publication
- material findings: pending
- verdict: pending; no acceptance claimed

## PR and closeout

- changed-file review: pending exact two-path proof
- unresolved review threads: `NOT_APPLICABLE` — no PR opened by this lane
- related/superseded PRs: PR #356 remains Draft/stopped at KX; not modified or superseded
- protected auto-merge: prohibited/not requested
- merge commit/result: none; Work retains integration authority
- ownership release: after exact remote branch head/tree readback

## Context checkpoint

```yaml
last_progress: Architecture evidence establishes disposition D; publication validation/readback pending.
status: NOT_ACTIVE
branch: arch/wp3-aws-lc-kx-resource-439
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: branch_push_only
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
owner_action_required: independent review and protected integration by Work
blocker: missing supported provider owner interface or authoritative complete finite native KX bound
next_action: Validate, commit, push, and remote-read back this two-document architecture candidate; then stop.
```
