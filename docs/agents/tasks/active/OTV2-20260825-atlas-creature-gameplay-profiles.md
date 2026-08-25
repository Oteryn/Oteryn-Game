# OTV2-20260825-atlas-creature-gameplay-profiles

```yaml
task_id: OTV2-20260825-atlas-creature-gameplay-profiles
title: Game-owned Atlas creature gameplay profiles v1
mode: IMPLEMENT
status: verifying
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/creature-gameplay-profiles-v1
issue: 136
pr: 138
base_sha: 91b73a7566a59991ebf7d471eacb3a858b755c9c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT autonomous implementation session
created_at: 2026-08-25T07:08:00+02:00
updated_at: 2026-08-25T07:55:00+02:00
execution_budget_minutes: 240
large_budget_reason: cross-repository producer/consumer programme with exact-head verification
owned_paths:
  - docs/contracts/OTERYN_GAME_ATLAS_CREATURE_GAMEPLAY_PROFILES_V1.md
  - tools/game-atlas-creatures/identity.py
  - tools/game-atlas-creatures/export.py
  - tools/game-atlas-creatures/self_test.py
  - tools/game-atlas-creature-gameplay/**
  - .github/workflows/game-atlas-creature-gameplay-profiles.yml
  - docs/agents/evidence/OTV2-20260825-atlas-creature-gameplay-profiles-readiness.md
public_contracts:
  - creature-gameplay-profiles-v1
  - oteryn-game-atlas-export-v1
depends_on: []
blocks:
  - Oteryn/Oteryn-Atlas#165
cross_repository_coordination_id: ATLAS-CREATURE-GAMEPLAY-PROFILES
external_repositories:
  - Oteryn/Oteryn-Atlas#159
  - Oteryn/Oteryn-Atlas#165
```

## Outcome

Publish a deterministic, bounded, public-safe Game-owned `creature-gameplay-profiles-v1` projection for NPC shop/services/travel and monster loot/stats/resistances, without changing gameplay runtime behavior.

## Architecture and source of truth

- PROVEN: Game `main` at allocation is `91b73a7566a59991ebf7d471eacb3a858b755c9c`.
- PROVEN: root `AGENTS.md` makes Game the only current product write authority and requires GitHub-first lifecycle.
- PROVEN: `OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md` assigns Game producer semantics/provenance and Atlas consumer validation/presentation.
- PROVEN: Oteryn/Oteryn-Atlas#159 / merged PR #161 is the owner-approved programme design; Atlas implementation is #165.
- PROVEN: Game implementation lifecycle is Oteryn/Oteryn-Game#136.

## Acceptance criteria

- [ ] Shared stable creature entity identity is factored with existing placement IDs unchanged.
- [ ] Capability contract freezes schema, completeness, item identity, integer probability and public-safety semantics.
- [ ] Static fail-closed NPC and monster extraction passes complete/partial/empty/no-script TDD.
- [ ] Product manifest/shards/digests are deterministic and hard-bounded.
- [ ] Real evidence census records truthful supported/unsupported coverage and frozen limits.
- [ ] Focused, governance, exact-head CI and independent audit are green on the final PR head.
- [ ] One PR is squash-merged and the exact merged Game SHA/product digest are handed to Atlas #165.

## Excluded scope

No gameplay runtime changes, Lua execution/eval, live server introspection, Atlas runtime mutation before Game merge, Platform runtime transit, fuzzy/name canonical identity repair, task-branch production deployment, credentials or protected-environment mutation.

## Implementation / findings

Initial GitHub preflight found no overlapping Game feature PR for this producer; open Game PRs were Dependabot-only. Atlas runtime overlap exists in #162/#163/#143, so Atlas mutation is deferred until this producer merges.

Implementation now provides a shared creature identity seam, fail-closed static-only NPC/monster gameplay extraction, honest completeness/ambiguity semantics, no client-ID-derived item authority, deterministic two-hex entity shards, exact digest verification, frozen real-corpus bounds, and a dedicated exact-head workflow. Real evidence census is recorded in `docs/agents/evidence/OTV2-20260825-atlas-creature-gameplay-profiles-readiness.md`.

## Validation

### Focused
- command/run: `python tools/game-atlas-creatures/self_test.py`
- result: baseline PASS before mutation; post-change PASS

### Component/integration
- command/run: `python tools/game-atlas-creature-gameplay/self_test.py`; exact pinned corpus built twice and all 509 files SHA-256 compared
- result: PASS; 1049 NPCs, 1800 monsters, 508 shards, max shard 174660 bytes, max 15 records/shard

### E2E
- scenario: NOT_APPLICABLE for Game producer; Atlas #165 owns browser E2E after producer merge
- result: NOT_APPLICABLE

### Exact-head CI
- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review
- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review
- required: YES — cross-repository public contract + untrusted static parser/resource-boundary surface
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout
- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none identified at allocation
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: GitHub lifecycle allocated as Game #136 and Atlas #165; dedicated Game branch pushed from exact main.
status: verifying
branch: feat/creature-gameplay-profiles-v1
head_sha: null
pr: 138
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
next_action: freeze the post-rebase PR #138 head in immutable PR evidence, run exact-head self-review/audit/CI, then squash merge
```
