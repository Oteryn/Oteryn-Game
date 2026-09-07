# OTV2-20260907-remediation-wp1-semantic-dispatch

```yaml
task_id: OTV2-20260907-remediation-wp1-semantic-dispatch
title: Repair semantic audit affected-scope dispatch
mode: REPAIR
status: review_pending
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/remediation-wp1-semantic-dispatch-364
issue: 364
pr: 371
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: 15164c38a2775e45eaff4001fddddbabf4b63ab6
head_sha: null
final_head_sha: null
owner: WP1_F03_SEMANTIC_DISPATCH
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths:
  - tools/architecture/semantic_contract_audit.py
  - tools/architecture/tests/test_semantic_contract_audit_dispatch.py
  - .github/workflows/architecture-semantic-audit.yml
  - docs/agents/tasks/active/OTV2-20260907-remediation-wp1-semantic-dispatch.md
public_contracts: []
depends_on: []
blocks: [WP1_verification_credibility]
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
- [ ] Exact-head canonical CI and one independent deep review pass before integration.

## Excluded scope

No merge-group/merge-gate/game-gate fan-in, protected audit/pins, runtime/product/Cargo/schema, ruleset/MQ, production or external mutation.

## Context checkpoint

```yaml
last_progress: repaired the second exact-head review findings with balanced-brace helper/method extraction, exact comparison checks and independent V1/V2 mutation regressions; reconciled protected main at 15164c38a2775e45eaff4001fddddbabf4b63ab6
status: review_pending
branch: fix/remediation-wp1-semantic-dispatch-364
pr: 371
blocker: null
next_action: publish the validated stable candidate, obtain one fresh independent exact-head deep review and require canonical exact-head CI before integration
```
