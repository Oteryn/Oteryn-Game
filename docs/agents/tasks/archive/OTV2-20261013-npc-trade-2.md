# OTV2-20261013-npc-trade-2

```yaml
task_id: NPC-TRADE-2
title: "NPC-TRADE-2: command 8 NPC_TRADE_INTENT wired to the NPC trade writer"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-trade-1-20261013
issue: 1622
pr: 2077
head_sha: "the PR head; the control plane records the frozen SHA"
created_at: 2026-10-13
updated_at: 2026-10-13
migration_lease: none
decisions:
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md   # §4, §5.1
owned_paths:
  - apps/game-server/src/gameplay_transport/npc_command.rs
  - apps/game-server/src/gameplay_transport/npc_command_tests.rs
  - apps/game-server/src/gameplay_transport/npc_trade_command.rs
  - apps/game-server/src/gameplay_transport/npc_trade_command_tests.rs
  - apps/game-server/src/gameplay_transport/connection.rs    # the command 8 branch and its trait method
  - apps/game-server/src/gameplay_transport/mod.rs           # module line and the authority method
  - apps/game-server/src/durability/npc_trade.rs             # read_npc_trade (replay first)
  - apps/game-server/tests/support/npc_trade_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261013-npc-trade-2.md
public_contracts: []
depends_on: ["NPC-TRADE-1 (#2040)", "NPC-CMD-1b-read"]
external_repositories: []
next_action: "NPC-TRADE-3 (charged and fluid offers) is opened by the control plane"
```

## Design

- The occurrence, TransactionId, EventId and the three output identities are SHA-256 derivations
  of the command's CommandRef (the occurrence also of the intent's NPC actor) with distinct tags
  (UUIDv7-shaped), so every retry of one command reuses them and the occurrence is bound 1:1 to
  the CommandRef (NPC-0 §5.1).
- Replay first: the occurrence is derived from the CommandRef and the intent's NPC actor
  (identity and generation), so the stored occurrence is the immutable record of the actor.
  `read_npc_trade` reads the trade of the command under the occurrence's advisory lock, never the
  live NPC: a trade committed as this occurrence answers `OK` when the intent names the same
  revision, offer, side, quantity and price, before any conversation or window check (a reconnect
  closes both); a trade of the command committed as another occurrence (another actor) or for
  another offer is `REJECTED` (Codex review: replay is bound to the NPC without a live lookup).
- `NpcConversations::trade_in` under the runtime lock: a vanished NPC or another generation is
  `STALE`; no conversation with it `NO_CONVERSATION`; beyond the talk range `NOT_IN_RANGE`; no
  window open on it or a quantity outside 1..=100 `REJECTED`; a catalogue revision, offer index,
  side or unit price that is not the server's is `STALE` and re-projects domain 8 (NPC-0 §5.1, the
  STALE disposition of `npc_service_v1.proto`). Charged, fluid and non-gold offers are `REJECTED`
  (NPC-TRADE-3).
- The commit uses the pinned generation's item and backpack facts and the coins at the pinned Item
  index's revisions. Any writer refusal is `REJECTED` and writes nothing; a storage failure ends the
  connection before the CommandId is sequenced, so the resend resolves by replay. A committed trade
  sends the domain 9 inventory delta after the result when the session shows items.
- No migration; the trade touches only the backpack, so it takes no Spell lane (not a key-33
  writer).

## Validation

- `cargo fmt --check`: pass
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-game-server --lib`: pass (2923 passed)
- `cargo test -p oteryn-game-server --test character_authority_postgres npc_trade`: pass (12
  passed) on local PostgreSQL 17.11 only, with the repository's 17.6 version pin relaxed locally
  and not committed; PostgreSQL 17.6 is not available here, so no local result on the pinned
  version is claimed and 17.6 qualification rests on CI
- `cargo run --locked -p oteryn-architecture-check -- workspace .`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Deviations

- A mismatched catalogue revision, offer, side or price is `STALE` with the re-projected window,
  not `REJECTED`: NPC-0 §5.1 makes the client's revision and price stale checks only, and the
  protocol's STALE disposition names exactly these mismatches.
- Charged, fluid and foreign-currency offers are `REJECTED` until NPC-TRADE-3.
