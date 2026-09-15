# OTV2-20260907-remediation-wp1-semantic-dispatch

```yaml
task_id: OTV2-20260907-remediation-wp1-semantic-dispatch
title: Repair semantic audit affected-scope dispatch
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 364
pr: 371
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: 10446c31b2508ccf6b65cc609a35246bf2666567
head_sha: c1d8639b503c1c519b7774e918ef2978dce0167c
final_head_sha: c1d8639b503c1c519b7774e918ef2978dce0167c
owner: WP1_F03_SEMANTIC_DISPATCH
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Repair audit F03 so an unrelated additive file or partial change cannot suppress an applicable legacy semantic profile. Multiple affected profiles must execute deterministically; truly unaffected changes remain explicitly NOT_APPLICABLE. This is evidence-path correctness, not merge authority.

## Source of truth

PROVEN on admission main: dispatcher uses exact equality against E_PATHS/F_PATHS/R_PATHS. Current workflow invokes the dispatcher on every PR but does not run a dedicated dispatch regression. Exact allocation is #162 comment `5567281231`. #308 frozen merge-group/fan-in/protected-audit paths are excluded.

## Acceptance criteria

- [x] Any changed path intersecting one profile selects that profile, even with unrelated additive files.
- [x] A one-file relevant change selects its profile.
- [x] Multiple affected profiles all run; order/output is deterministic.
- [x] No affected paths returns explicit NOT_APPLICABLE.
- [x] Existing profile semantic bodies are unchanged unless a test proves a necessary compatibility repair.
- [x] Deterministic dispatcher regressions run in the owning architecture semantic workflow.
- [x] Workflow triggers, permissions, exact-head resolution and advisory-only authority remain unchanged.
- [x] Exact-head canonical CI and one independent deep review pass before integration.

## Excluded scope

No merge-group/merge-gate/game-gate fan-in, protected audit/pins, runtime/product/Cargo/schema, ruleset/MQ, production or external mutation.

## Validation, review and closeout

- Exact delivery head `c1d8639b503c1c519b7774e918ef2978dce0167c` (tree `5c9d3b124192cd5fbcd5c7be14391f998d8c2b90`) passed 37 focused semantic-audit tests, including the cfg-shadow and comment-brace scanner repairs.
- Independent Codex deep review reported no major issues: PR #371 comment `5570057820`; root qualification `5570096888`.
- Exact-head Merge gate `34116962490`, Architecture semantic audit `34116962613` and Agent governance `34116962923` passed.
- Full Merge Queue run `34118019006` passed; PR #371 squash-merged as `160dff0eab12df4e0c61d42d3591460d171af44b`.
- All four delivery blobs, including task blob `aa3299c06c5f1f93daae53ec423787f528653e3f`, were read back equal from protected `main@160dff0eab12df4e0c61d42d3591460d171af44b`. Durable proof: PR #371 comment `5570219653`.
- Full changed-file and effective-diff review: PASS; zero open material findings, no unresolved review threads and no scope outside the four allocated files.
- Ownership released after protected-main readback. The merged task branch was deleted; live matching-ref readback returned no branch.
- This closes F03 only. Broader WP1, G0 and G1 remain open.

## Context checkpoint

```yaml
last_progress: PR 371 integrated through successful Merge Queue and protected-main blob readback; F03 task archived and ownership released
status: completed
branch: null
head_sha: c1d8639b503c1c519b7774e918ef2978dce0167c
final_head_sha: c1d8639b503c1c519b7774e918ef2978dce0167c
pr: 371
blocker: null
next_action: null
```
