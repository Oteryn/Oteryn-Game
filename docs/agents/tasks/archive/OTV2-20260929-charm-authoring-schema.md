# OTV2-20260929-charm-authoring-schema

```yaml
task_id: OTV2-20260929-charm-authoring-schema
title: Charm source facts and authoring schema candidate v1 (static catalogue only)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-cannon-4gzw0y
issue: 162
lane_id: content population (reference source staging)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 50d75c65
head_sha: null   # a commit cannot hold its own SHA
final_head_sha: null
final_head_frozen_at: null
owner: "owner-launched Claude Code session (session_012nzPTz29NThWJG45F2m5fP)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/charm-authoring/**
  - .github/workflows/charm-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260929-charm-authoring-schema.md
public_contracts: []
depends_on:
  - docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner option (a), agreed in session: collect the Charm data and build a schema. The 25 Charms are captured as
source facts from TibiaWiki (primary) and Canary `47dfd51f` (cross-check). A closed JSON Schema candidate and
validator describe the static catalogue. A candidate catalogue is built from the facts and validates. Nothing is
written to `content/`, `rulesets/` or runtime code, and no identity is minted.

## Source verification

- PROVEN: 25 wiki pages, `Category:Charms`, each pinned by page id, revision id and wikitext SHA-256. Canary
  `bestiary_charms.lua` is pinned by SHA-256 and holds 25 charms. The names match one to one.
- PROVEN: wiki and Canary agree for all 25 on category, stage costs and stage values.
- CONFLICT, recorded in `samples/charm-source-comparison.json`. After the owner-requested cross-check of Canary,
  Crystal `00ce02a`, the wiki notes and web sources (2026-09-29), 2 points remain:
  - Bless has an unused Canary `percent` of 10; Canary's code uses only the 6/9/12% stage values, like the wiki.
  - Carnage is physical on the wiki and neutral in Canary. The catalogue follows the wiki until verified in game.
  - Overpower, Overflux and Parry are resolved: they are displayed as physical but ignore resistances, and Parry is
    reduced by armor. The wiki notes, the Canary C++ handlers and Crystal agree.
- DERIVED: the per-charm effect parameters are hand-written. Each is tied to phrases the captured page must contain.
- UNKNOWN: whether the Canary charm id equals the client protocol id.

## Excluded scope

- Option (b): Bestiary charm-point facts per creature (next task).
- Option (c): Charm rules and durable Character state.
- Populating `content/charms/` and minting keys.

## Validation (local)

- `charm_authoring.py build --check`: ok (25 charms).
- `validate samples/charms-candidate.json`: ok.
- `test_charm_authoring.py`: 5 tests pass.
- `ruff check`, `ruff format --check`, `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`:
  see PR.
- Review: none required (reference-only authoring candidate; no protocol, persistence or authority change).
