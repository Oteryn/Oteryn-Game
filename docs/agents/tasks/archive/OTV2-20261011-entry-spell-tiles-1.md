# ENTRY-SPELL-TILES-1

```yaml
task_id: OTV2-20261011-entry-spell-tiles-1
title: "ENTRY-SPELL-TILES-1: entry-room spell tile facts allow player attack"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
pr: 2005
owner: unassigned
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2005](https://github.com/Oteryn/Oteryn-Game/pull/2005). Entry-room spell tile facts, protection-zone flags and qualified pins are merged under the short ENTRY-SPELL-TILES-1 ID. Evidence: `crates/gameplay/content/src/content/project/native_entry.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

## Outcome

Entry-room protection-zone and spell_tiles facts are set so a player attack is not refused there. This rotates the qualified content pin digests and goldens in one reviewed change.

## Architecture and source of truth

- PROVEN: the entry room is qualified by exact tile flags in `crates/gameplay/content/src/content/project/native_entry.rs` (four cells, `protection_zone: false`, one obstacle cell). Source: `native_spell_entry_room.json`.
- DERIVED: any change to those facts rotates the qualified content digests and the goldens that pin them, so it is not a documentation change.
- UNKNOWN: the exact current refusal a player or rat meets in the entry room. The implementing task reproduces it first.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: content and policy facts only until the task is allocated.

## Excluded scope

- No change in the acceptance PR (ACCEPT-CHAT-ATTACK-DEATH-0); that PR is documentation only.
- Needs its own #162 allocation, owned paths and pin rotation review.

## Context checkpoint

```yaml
last_progress: recorded as follow-up of ACCEPT-CHAT-ATTACK-DEATH-0
status: completed
branch: null
pr: 2005
blocker: needs allocation
next_action: control plane allocates the task
```

## Validation

CARDS-AUDIT-1 metadata/archive validation (2026-10-23):

- `python tools/agents/validate_governance.py`: pass (metadata/archive qualification).
- `python -m unittest discover -s tools/agents/tests`: pass (124 tests).
