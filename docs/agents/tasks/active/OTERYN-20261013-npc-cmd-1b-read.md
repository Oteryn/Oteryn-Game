# OTERYN-20261013-npc-cmd-1b-read

```yaml
task_id: NPC-CMD-1b-read
title: "NPC-CMD-1b-read: read-only domain 8 NPC_TRADE_WINDOW on bundle Worlds"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-cmd-1b-20261013
pr: null
base_sha: null
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01Q916tixivQCNQY9UrHfBLR
control_plane: "#1622 (CP session_01XYC239givrMonrYoBwwECV)"
governing_issue: 1622
issue: 1622
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/npc_command.rs
  - apps/game-server/src/gameplay_transport/npc_command_tests.rs
  - apps/game-server/src/gameplay_transport/npc_talk.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/content/item_ref.rs
  - docs/agents/tasks/active/OTERYN-20261013-npc-cmd-1b-read.md
public_contracts: []
external_repositories: []
```

## Outcome

On a bundle World, a `NPC_TALK_INTENT` that says `trade` in an open conversation with an NPC that
has projectable offers sends the speaker a whole domain 8 `NPC_TRADE_WINDOW` after the command
result. It closes (an empty window) when the conversation ends: a farewell, walking out of range,
or a vanished NPC. Command 8 stays `REJECTED`. No wire, registry, persistence or value change.

## Scope split (CP decision)

NPC-CMD-1b was split by the control plane: this record is the read-only window. The BUY/SELL commit
(migration, DUR-03 cause record, `NpcTradeCause`, `FeeBurnCause::NpcTrade`) is NPC-TRADE-1, a
separate hard task (NPC-0 section 3 and 5).

## Decisions taken (derived, reversible)

- Offers: the admitted offers of every Service of the NPC in source order; the wire side is the
  player's (`SellToPlayer` is BUY, `BuyFromPlayer` is SELL); price and count are the offer row's.
- `item_definition_ref` comes from the pinned Item index of the generation
  (`item_ref_admission::pinned_index`). An offer the index does not name at the offer's own revision,
  or whose price exceeds `u32`, is left out; with no pinned index there is no window. The offer
  index a trade intent will name is the index in this projection, so NPC-TRADE-1 must read the same
  `project_offers`.
- `catalogue_revision` is non-zero and derived from the catalogue's source tree digest, so it
  changes exactly when the offers' content changes.
- The window opens only from command 7 (the chat `say` path opens no window).

## Excluded scope

Command 8 effects, quantity and price validation, stock, migration, causes (NPC-TRADE-1).

## Acceptance criteria

- [x] Offer projection order, sides, drops (`npc_command_tests`).
- [x] Window opens on `trade`, closes on farewell and on walking away; none without offers or index.

## Validation

See the PR.

## Self-review

- method: implementing agent read the whole diff against owned paths.

## Independent review

- required: per the bound review policy; full Codex review on the PR.
