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
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_CUSTODY0_HOUSE_ITEM_CUSTODY_DECISION_2026-09-30.md
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
- Self-review: `oteryn-hard-worker`, read-only, on the complete draft (`196e40ae`), and again on
  `51eabf93`:
  - the first pass found 8 material findings;
  - the second found 4 new material problems;
  - all are fixed before freeze (ADR §8).
- HOUSE-CUSTODY-0 was added to this PR at the owner's direct request (2026-09-30): the
  revision-free `HouseInterior` family, reclaim provenance, one item-level exclusivity guard, the
  reset preflight and recheck, and the fence and shapes fixed for the house interior runtime child.
  HOUSE-CUSTODY-1 implements the storage slice only.
- HOUSE-CUSTODY-0 self-review (`oteryn-hard-worker`, read-only) on `1f6f8cba`: 5 material, 4
  evidence gaps and 5 hardening findings, all fixed before freeze (revision-free `HouseId`, no
  Ground source, reset preflight, one exclusivity guard, storage-only slice, no runtime grants).
- Independent review on `36e9223d` (#1315, FIX, 7 items) is answered in one push: verbatim owner
  quotes and D188-D196 in ADR §2 with a separate architect note; HOUSE-CUSTODY-0 §3.4 marked
  non-binding direction; both `0015` tables named with discriminator CHECKs; the `build_class`
  release gate; the overlay re-hide rule and its boundary test; unrepresentable map-item
  attributes excluded; migration `0025` for HOUSE-CUSTODY-1.
- Owner answer on house tiles: a. Ground items on the tiles of a house without an owner are
  retired at a reset.
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
