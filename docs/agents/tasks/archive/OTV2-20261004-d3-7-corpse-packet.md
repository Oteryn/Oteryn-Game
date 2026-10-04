# OTV2-20261004-d3-7-corpse-packet

```yaml
task_id: OTV2-20261004-d3-7-corpse-packet
title: "D3-7-PACKET-1: packet the rat corpse Item admission"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/d3-7-corpse-packet-20261004
issue: 162
pr: 1770
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_D3_7_CORPSE_ITEM_ADMISSION_PACKET_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-d3-7-corpse-packet.md
public_contracts: []
depends_on: [D3, STARTER-CONTENT-1]
blocks: [D3-7]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- D3's `i00005801` is an alias of `oteryn:item.tibia.i5964`, the dead rat. The rat's corpse
  binding does not change (packet §1.1).
- D3-7 adds a v2 Item admission packet next to the STARTER-CONTENT-1 v1 packet, with the D137
  shape: materializable, non-stackable, capacity 16, no equipment, a 60 s durable deadline and no
  decay target (§1.2).
- Canary's capacity 10, 10 s and decay to 3994 are recorded as evidence that diverges by decision
  (§1.3).
- The packet starts after #1753 and regenerates `content/world/**` only with the tools (§0, §2).
- No code, contract or wire change.
- Owned paths list every output of the regeneration (#1770 P1 4177992199):
  `content/content.lock.json` and the seven Ability, Behavior, Creature, Loot and Presentation
  indexes that embed the reference blob SHA. All are tool outputs only.
- #1770 P1 4178022290: the evidence is Canary only, pinned with repository, path, revision,
  sha256, lines and attributes from `imports/canary/items-xml/`. CrystalServer and TibiaWiki are
  dropped: neither has a pinned record of item 5964.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
