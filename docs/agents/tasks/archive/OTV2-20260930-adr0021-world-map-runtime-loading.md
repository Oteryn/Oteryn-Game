# OTV2-20260930-adr0021-world-map-runtime-loading

```yaml
task_id: OTV2-20260930-adr0021-world-map-runtime-loading
title: "ADR-0021 World map runtime loading"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/laughing-goldberg-4gwjfq
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: b92b4d61
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/ADR-0021-world-map-runtime-loading.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20260930-adr0021-world-map-runtime-loading.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ADR-0021 answers the owner's architecture request on runtime loading of the world map. The owner
answered 1a-8a on 2026-09-30.

- **Load and read.** The full base is loaded eagerly from a compiled server World Bundle (B3 stays
  the source), and all channels of a World share it.
- **Coordinates.** The compiler maps the project frame to native positions (`floor = -z`).
- **Overlay.** Each channel has a volatile overlay with Tibia-parity resets:
  - DUR-03 Ground items survive a crash and are retired at the planned reset;
  - picking up a map item is a MINT with a typed cause.
- **Keys.** Unknown keys fail. Provisional keys are skipped outside production.
- **Drafts.** Draft areas are flag-gated.
- **Updates.** A new base activates only at a planned reset.
- **Budgets.** Four registry rows.
- **DUR-03 amendment.** §39.3 is amended for map-item materialization and world-reset
  retirement.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: ADR-0001 §7; ADR-0005 §1, §3 and §6; DUR-04 §9, §11, §20 and §24 (proposed); the
  CrystalServer import profile; coordinate profile §6; the B3 codec on #1170; DUR-03 Ground
  custody (migrations `0010`-`0015`); the owner's benchmark as stated in the request.
- `UNKNOWN`: compact-model RSS; the reset schedule; Tibia server-save behaviour for dropped items
  (parity accepted without a test).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. MAP-OVERLAY-1 needs a persistence review.

## Acceptance criteria

- [x] ADR, DUR-03 amendment and registry rows on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence scope for §4.4 and DUR-03).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations, content, the bundle byte layout, the reset schedule, houses and depots.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review: `oteryn-hard-worker`, read-only, on the complete draft before freeze.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

## Context checkpoint

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/laughing-goldberg-4gwjfq
owner_action_required: null
blocker: null
next_action: null
```
