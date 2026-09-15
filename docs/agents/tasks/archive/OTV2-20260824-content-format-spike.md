# OTV2-20260824-content-format-spike

```yaml
task_id: OTV2-20260824-content-format-spike
title: Produce native World Project / World Bundle format evidence
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 95
issue_state: completed
allocation_pr: 112
allocation_merge_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
base_branch: main
branch: null
pr: 125
base_sha: ef82d74a0020cec9c6306336b15d5d85006bcef3
head_sha: 8c2f957f972b2dafa4bf22f239ab6a446c06b23a
final_head_sha: 8c2f957f972b2dafa4bf22f239ab6a446c06b23a
final_head_frozen_at: 2026-08-24T22:34:58+02:00
delivery_merge_sha: a909f432cfa887c7e99191f18bd9cbb5ca58fc7a
delivery_merged_at: 2026-08-24T22:45:14+02:00
owner: released
created_at: 2026-08-24T19:40:43+02:00
updated_at: 2026-08-24T22:50:20+02:00
execution_budget_minutes: 60
owned_paths: []
public_contracts: []
depends_on:
  - Oteryn-Game#95
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Delivered a reproducible, bounded evidence package comparing editable World Project and compiled World Bundle physical-representation candidates. The result is advisory only; `SPIKE_RESULT != OWNER_FORMAT_DECISION` remains binding.

## Delivered evidence

- candidate `chunked-json-tree`: pretty canonical per-chunk editable-project prototype with per-chunk integrity and project-root path containment;
- candidate `sqlite-project`: deterministic single-file project-container prototype with indexed chunk access and per-row integrity; crash-recovery/WAL behavior remains explicitly not proven;
- candidate `indexed-zlib-bundle`: read-only indexed per-chunk compressed runtime-artifact prototype with bounded decompression and per-chunk integrity;
- 15 benchmark cells across 32/64/128 synthetic world scales and 32/64 chunk candidates;
- all measured artifacts exact-byte deterministic and corruption-rejected;
- all seven adversarial evidence classes rejected fail-closed;
- every measured client projection excludes `server_only` data;
- decision dossier retains explicit not-proven boundaries for signing/CDN/production patching, real Crystal/OTBM parity, Studio concurrency/autosave and permanent compatibility policy.

Evidence files:
- `docs/agents/evidence/OTV2-20260824-content-format-spike-results.json` — SHA-256 `afb871a435dd5d4333087fdb6456568c5ac01784dc25118448618d3b16da464e`;
- `docs/agents/evidence/OTV2-20260824-content-format-spike.md` — SHA-256 `68aedf28ee8425b36969829239e0893fb65e589a3b14f4bc01d8def7c8718afd`.

## Verified delivery

- delivery PR: #125
- exact delivery head: `8c2f957f972b2dafa4bf22f239ab6a446c06b23a`
- delivery squash merge: `a909f432cfa887c7e99191f18bd9cbb5ca58fc7a`
- merge time: `2026-08-24T22:45:14+02:00`
- changed delivery paths: exactly six coordinator-allocated spike/evidence/task paths
- exact-head Agent governance run `32774705978`: PASS
- exact-head Architecture semantic audit run `32774705997`: PASS
- exact-head Merge authority audit run `32774706008`: PASS
- exact-head Merge gate run `32774706140`: PASS
- aggregate `Merge gate / validate` job `97585439345`: PASS
- canonical `game-gate` job `97585462248`: PASS
- unresolved review threads before merge: 0
- delivery source branch `spike/content-format-evidence-20260824`: absent after merge
- stale parallel allocation PR #113: closed superseded; source branch absent
- Issue #95: closed with state reason `completed`

## Validation

### Focused

- `ruff check tools/content-format-spike` — PASS
- `ruff format tools/content-format-spike --check` — PASS
- Python compile — PASS
- `python tools/content-format-spike/self_test.py` — PASS, 12/12
- `python tools/agents/validate_governance.py` — PASS, 25 required policy documents / 9 lanes
- architecture check — `workspace-boundaries: PASS`
- `cargo +1.94.0 test --workspace --locked` — PASS
- `git diff origin/main...HEAD --check` — PASS before delivery publication

### E2E

- `NOT_APPLICABLE` — this task is evidence tooling only and grants no production runtime activation.

### Exact-head CI

- final head: `8c2f957f972b2dafa4bf22f239ab6a446c06b23a`
- trigger source: pull request #125
- runs: `32774705978`, `32774705997`, `32774706008`, `32774706140`
- result: PASS, including canonical `game-gate`

## Self-review

- exact head: `8c2f957f972b2dafa4bf22f239ab6a446c06b23a`
- method/reviewer: implementing agent whole-diff security/authority/claim review
- repaired before freeze: JSON project-root path containment, pretty canonical editable diffability, deterministic malformed-manifest rejection, bounded decompression finalization and overbroad crash/signing claims
- final material findings: P0=0 / P1=0 / P2=0
- verdict: PASS

## Independent review

- required: YES — delivery contained bounded parser/container/decompression and path-based loader trust surfaces
- exact head: `8c2f957f972b2dafa4bf22f239ab6a446c06b23a`
- method/auditor: local non-authoring `qwen2.5-coder:14b` via Ollama 0.32.14, packet built only from exact Git blobs plus binding constraints
- packet SHA-256: `18786ac8ca57b215a9ada3e6c8d512a92dc7fd1526072c3b8be8d40940f38f4c`
- raw response SHA-256: `e2c63281f316bb829c07263234b08dfe27965f5a2d0153136ed1fcea0a0b0f53`
- material findings: P0=0 / P1=0 / P2=0; `NONE`
- verdict: PASS

## PR and closeout

- changed-file review: PASS — exactly six declared delivery paths
- unresolved review threads: 0
- related/superseded PRs: allocation PR #112 merged; stale parallel allocation PR #113 closed superseded
- squash merge: PASS — `a909f432cfa887c7e99191f18bd9cbb5ca58fc7a`
- Issue #95: closed completed
- delivery source branch: deleted/absent
- ownership release: PASS — `owned_paths: []`

## Ownership release

All spike tooling/evidence/task write ownership is released. This closeout grants no runtime, Cargo/workspace, contract/ADR/registry, production activation, Platform or external-repository authority. Permanent physical format selection remains a separate owner decision.

## Context checkpoint

```yaml
last_progress: PR #125 exact head 8c2f957f972b2dafa4bf22f239ab6a446c06b23a passed local verification, independent exact-head review and all protected gates, then squash-merged as a909f432cfa887c7e99191f18bd9cbb5ca58fc7a; Issue #95 closed completed; delivery and stale allocation branches are absent
status: completed
branch: null
head_sha: 8c2f957f972b2dafa4bf22f239ab6a446c06b23a
pr: 125
final_head_sha: 8c2f957f972b2dafa4bf22f239ab6a446c06b23a
final_head_frozen_at: 2026-08-24T22:34:58+02:00
ci_trigger_source: pull_request
ci_check_generation: exact_head_8c2f957f972b2dafa4bf22f239ab6a446c06b23a
ci_checks_for_current_head: 4
ci_run_ids:
  - 32774705978
  - 32774705997
  - 32774706008
  - 32774706140
ci_job_ids:
  - 97585439345
  - 97585462248
runner_assignment_state: completed
terminal_ci_checks_for_current_generation: 4
repair_cycles_for_current_gate: 0
stall_warnings: 0
owner_action_required: select, rework, or defer the permanent World Project and World Bundle physical formats using this dossier and any additional evidence
blocker: none for the completed spike lifecycle
next_action: none — spike task lifecycle closed; permanent-format owner decision is a separate future action
```
