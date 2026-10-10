# OTV2-20261010-game-docs-0b

```yaml
task_id: OTV2-20261010-game-docs-0b
title: "GAME-CHAR-CMD-0b and GAME-PUBLIC-PROJ-0b deferred P2 contract fixes"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: contracts
base_branch: main
branch: claude/game-docs-0b-20261010
pr: 1959
base_sha: origin/main at branch creation
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-10
updated_at: 2026-10-10
packet: "deferred P2 review threads of #1936 and #1937 (D245)"
owned_paths:
  - docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md
  - docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md
  - docs/agents/tasks/archive/OTV2-20261010-game-docs-0b.md
depends_on:
  - "#1936 merged"
  - "#1937 merged"
public_contracts:
  - docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md
  - docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md
external_repositories: []
```

## Outcome

Docs only. Applies the P2 findings deferred under D245 from the #1936 and #1937 review threads.

Character authority commands:

- (a) The intent read request bound is 512 bytes (was 256): the request now carries
  `source_authority` and `issuer_authority` of up to 128 bytes each and a `command` of up to 64 (496 bytes worst case).
- (b) Intent reads are keyed by `(issuer_authority, operation_id)`; a pair held for another
  command returns `409`, and Game stores `CHAR_CMD_OPERATION_CONFLICT`.
- (c) Bounded rescan of the pending list (every 8 full pages or 30 s) with a 1,024-entry PENDING backlog bound (worst case 126 s) so a lower
  `source_revision` committed behind the cursor is read before its 300 s expiry.

Public projections:

- Name handoff: the publisher orders the vacating publication before the new holder; Platform
  resolves a name only with exactly one holder, otherwise it answers as unknown; houses are
  routed by `house_key`.
- `world_online` publishes changed Worlds concurrently (one in flight per World, up to 16,
  `PUBPROJ-ONLINE-WORLDS`), so the 5 s scan deadline covers the fanout.

## Validation

- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
- `git diff --check`: pass.

## Review

Independent review on the frozen head; the control plane requests it.
