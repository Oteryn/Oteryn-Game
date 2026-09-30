# OTV2-20260930-house-runtime0

```yaml
task_id: OTV2-20260930-house-runtime0
title: "HOUSE-RUNTIME-0 house interior runtime"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-house-runtime-0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: a6a054e6
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_CUSTODY0_HOUSE_ITEM_CUSTODY_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-house-runtime0.md
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

HOUSE-RUNTIME-0 decides the house interior runtime (owner direction 2026-09-30, item 1 of the
owner's order).

- **Topology:** one house scope (runtime scope kind 2, keyed by `HouseId`) per active house,
  activated on first entry, unloaded after 5 idle minutes.
- **Entry and exit:** a door handoff through the ADR-0001 §10 session transition, refused in
  combat or while busy; exit to the origin Channel's `entrance`.
- **Access:** owner, subowners, guests and guild entries; door lists; guests take only their own
  items; kick and leave in the panel; revalidation on each ACL revision.
- **Items:** the three HOUSE-CUSTODY-0 shapes with provenance under the house fence.
- **Login** back into the house when access still holds; disposition quiesce; read-only outside
  projection of furniture.
- **Children:** SCOPE-HANDOFF-1, HOUSE-RUNTIME-1, HOUSE-VIEW-1.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: EXP-HOUSES-01 §4-§5, §14, §16, §17, §19; ADR-0001 §10-§12; HOUSE-CUSTODY-0;
  HOUSE-OWN-0; CHAR-POSITION-0; migrations `0001`, `0003`, `0006`.
- `DERIVED`: the Tibia manual `houses.md`; GUILD-0 (#1395).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. SCOPE-HANDOFF-1 and HOUSE-RUNTIME-1 need security, durability and
persistence review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (security, persistence, protocol).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations and content; beds, Rested, offline training, Residence, containers in houses.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.
- Codex round 1 (4 P1) repaired in one authoring write: recoverable durable handoff (§4.1, §4.3),
  admission-time ACL and disposition revalidation (§4.1), outside position committed on every exit
  (§4.2, §6.3), session fence on house item transfers (§6.1); validators re-run PASS.
- Codex round 2 (2 P1, 3 P2) repaired in one authoring write: guild and membership row locks in
  the admission commit (§4.1, §5.4), login through the serialized admission commit, saved-tile
  revalidation with fallback and atomic clearing of a rejected house position (§6.3), content
  revision advanced by every `HouseInterior` mutation (§7, §9); validators re-run PASS.
- Codex round 3 (#1399, 1 P1, 1 P2): item transactions lock and revalidate the ACL and guild
  revisions like the admission commit (§5.4); the failed-scope recovery admission clears the house
  columns and writes the entrance position atomically (§4.3); validators re-run PASS.

This record was archived in the final authoring commit of its PR.
