# OTV2-20260824-content-format-spike

```yaml
task_id: OTV2-20260824-content-format-spike
title: Produce native World Project / World Bundle format evidence
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
issue: 95
allocation_pr: 112
allocation_merge_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
base_branch: main
branch: spike/content-format-evidence-20260824
pr: null
base_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260824-content-format-spike
created_at: 2026-08-24T19:40:43+02:00
updated_at: 2026-08-24T22:12:50+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - tools/content-format-spike/**
  - docs/agents/evidence/OTV2-20260824-content-format-spike.md
  - docs/agents/evidence/OTV2-20260824-content-format-spike-results.json
  - docs/agents/tasks/active/OTV2-20260824-content-format-spike.md
public_contracts: []
depends_on:
  - Oteryn-Game#95
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce a reproducible evidence package comparing bounded physical representations for editable World Project and compiled World Bundle concerns. The result is advisory evidence only and must preserve `SPIKE_RESULT != OWNER_FORMAT_DECISION`.

## Architecture and source of truth

- `PROVEN`: `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md` requires separate editable source and compiled runtime representations; OTBM remains an import input, not canonical Oteryn format.
- `PROVEN`: `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` and accepted `VSL-CONTENT-01` define bounded, deterministic content/runtime requirements.
- `PROVEN`: merged CONTENT evidence seam provides a deterministic non-production artifact parser/compiler and separate server-authoritative/client-safe projections.
- `PROVEN`: `docs/migration/CRYSTAL_WORLD_CONTENT_MIGRATION_DESIGN_CHECKPOINT.md` is non-authoritative migration evidence and may inform representative scale/shape only.
- `UNKNOWN`: permanent World Project / World Bundle physical encodings remain owner-gated.

## Acceptance criteria

- [x] Candidate set is small, explicit, and independently isolated from production runtime interfaces.
- [x] Deterministic synthetic fixtures cover multiple bounded scales and have recorded hashes.
- [x] Reproducible measurements cover serialization/build, load/access, size/compression, diff locality, patch granularity, and bounded failure behavior.
- [x] Malformed/truncated/oversize/ratio-limit negative tests execute fail-closed.
- [x] Exact Python/SQLite/zlib tool versions and benchmark configuration are recorded.
- [x] Decision dossier distinguishes editable-project versus compiled-runtime concerns and records tradeoffs without selecting the permanent format.
- [ ] Governance, focused tests, `git diff --check`, whole-diff self-review, and exact-head repository CI are clean before merge.

## Excluded scope

No production loader adoption, permanent `.omap`/`.owb` decision, ADR/contract/registry mutation, Cargo/workspace changes, protected deployment, proprietary asset inclusion, Platform/external-repository writes, or production activation authority.

## Implementation / findings

Allocation PR #112 merged as `22a3eb866dae19d048969edff1e1fa5012a429b6`, activating this exact evidence-only worker scope. Implemented three isolated stdlib candidates: pretty canonical chunked JSON tree, deterministic SQLite project container, and indexed per-chunk zlib runtime bundle. The tooling records exact-byte determinism, build/load/peak-memory evidence, one-cell diff/patch locality, corruption rejection, client/server projection separation and fail-closed size/depth/count/ratio/path controls. Results SHA-256: `afb871a435dd5d4333087fdb6456568c5ac01784dc25118448618d3b16da464e`; dossier SHA-256: `68aedf28ee8425b36969829239e0893fb65e589a3b14f4bc01d8def7c8718afd`. Permanent-format selection remains forbidden.

## Validation

### Focused

- command/run: `python tools/content-format-spike/self_test.py`
- result: `PASS` — 12/12 focused tests including determinism, corruption, ratio fence, path containment, reviewable JSON, CLI dossier generation and candidate coverage.
- evidence run: `python tools/content-format-spike/spike.py --work-dir C:\Temp\oteryn-content-format-spike\bench-final-frozen --results docs\agents\evidence\OTV2-20260824-content-format-spike-results.json --dossier docs\agents\evidence\OTV2-20260824-content-format-spike.md --base-sha 22a3eb866dae19d048969edff1e1fa5012a429b6 --iterations 9`
- evidence result: `PASS` — 15 measurement cells; exact-byte determinism and corruption rejection true for all; 7/7 negative classes true; client projection server-only leakage false for all scales.

### Component/integration

- command/run: `NOT_APPLICABLE` — tooling is deliberately isolated from production runtime/Cargo/workspace composition; baseline full workspace tests passed before implementation and no product path is modified.
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — evidence tooling only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: candidate tree to be frozen by the next commit; immutable commit SHA will be bound in PR metadata because a commit cannot contain its own SHA
- method/reviewer: implementing/coordinating agent; whole owned-path diff plus security/authority/claim audit
- material findings: repaired before freeze — JSON manifest path traversal was reproducible and fenced with root containment; editable JSON was changed to pretty canonical form for measurable reviewability; malformed JSON chunk-index shapes are now normalized to deterministic `SpikeError`; SQLite crash-recovery and binary manifest-signing claims were narrowed to `NOT_PROVEN`; final material P0/P1/P2 findings: 0/0/0
- verdict: `PASS` on the candidate tree; exact-head CI and independent review remain external pre-merge gates

## Independent review

- required: `YES` — the final spike includes bounded parsers/container indexing/decompression and a path-based project loader, so a fresh non-authoring exact-head review is required before merge.
- exact head: pending final freeze
- method/auditor: pending independent local reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: candidate tree contains exactly the six coordinator-allocated paths; no runtime/Cargo/contract/registry/workflow path
- unresolved review threads: pending
- related/superseded PRs: allocation PR #112 merged as `22a3eb866dae19d048969edff1e1fa5012a429b6`
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: allocation PR #112 merged as 22a3eb866dae19d048969edff1e1fa5012a429b6; three bounded candidates, 15-cell benchmark, decision dossier and adversarial evidence are implemented; focused/governance/ruff checks are locally green
status: validating
branch: spike/content-format-evidence-20260824
head_sha: candidate_tree_pending_commit
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
blocker: final whole-diff self-review, independent exact-head review and exact-head repository CI are still required before merge
next_action: commit and push the verified six-path candidate tree, then open the Issue #95 delivery PR and bind fresh independent exact-head review plus protected CI without moving the head
```
