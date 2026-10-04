# OTV2-20261004-spawn-1a-packet-1

```yaml
task_id: OTV2-20261004-spawn-1a-packet-1
title: "SPAWN-1A-PACKET-1: the D116 fixture spawn and entry-room revision 2"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spawn-1a-packet-1-20261004
issue: 162
pr: 1745
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SPAWN1A_FIXTURE_SPAWN_ROOM_R2_DECISION_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_NATIVE_ENTRY_ROOM_PRODUCT_BINDINGS_DECISION_2026-09-26.md
  - docs/agents/tasks/archive/OTV2-20261004-spawn-1a-packet-1.md
public_contracts: []
depends_on: []
blocks: [SPAWN-1a]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane D492 (owner 7a): SPAWN-1a moves out of #1735 (ARCH-CORE-LOOP-PACKETS-2 §1.4 room
  revision 2 and spawn inputs, §2.7 packet) into this decision. The bindings "Amendment
  2026-10-04" comes with it. #1735 keeps the SPAWN-1a/1b split ruling and SPAWN-1b.
- #1735 P1 4177035356: room r2 is a new immutable package and lock identity
  (`oteryn:package-rev/entry-r2`, `lock:oteryn:package-rev/entry-r2`); definition revisions follow
  their bytes, so the unchanged definitions keep `oteryn:rev/entry-r1`; mixed r1/r2 tuples are
  refused (§1.2).
- #1735 P1 4177035359: `gameplay_transport/qualification.rs` is owned as the second
  `into_channel_parts` consumer; the spawn source enters through an additive constructor, so the
  other call sites compile unchanged (§1.3). The file is serialized with SESSION-PUSH-1 (#1736, §0).
- #1735 P1 4177035362: `tools/monster-lab/test_arena_map.py` is owned and expects five cells and
  the rat's slot at (2, 0, 0) (§2.1).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
