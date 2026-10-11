# BOOT-RAT-POLICY-1

```yaml
task_id: OTV2-20261011-boot-rat-policy-1
title: "BOOT-RAT-POLICY-1: entry-room rat combat policy"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
pr: null
owner: unassigned
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths: []
public_contracts: []
depends_on: [OTV2-20261011-entry-spell-tiles-1]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Rats spawned in the entry room can fight a player: the boot-time creature policy and the entry-room facts agree, with pins and goldens rotated together. Sequenced with ENTRY-SPELL-TILES-1.

## Architecture and source of truth

- PROVEN: the entry room is qualified by exact tile flags in `apps/game-server/src/content/project/native_entry.rs` (four cells, `protection_zone: false`, one obstacle cell). Source: `native_spell_entry_room.json`.
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
status: waiting
branch: null
pr: null
blocker: needs allocation
next_action: control plane allocates the task
```
