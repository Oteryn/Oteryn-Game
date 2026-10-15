# NPC-WORLD-1 — NPC world: travel, walking actors and voices (skeleton task record)

```yaml
task_id: NPC-WORLD-1
title: "NPC-WORLD-1: NPC world: travel, walking actors and voices"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row NPC-WORLD-1; §7 'NPC-WORLD-1' (TRAVEL0 and NPC_BEHAVIOUR0 graphs)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_TRAVEL0_NPC_TRAVEL_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/npc-world-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

NPCs become world actors: travel NPCs move Characters between destinations for a fee (premium-gated where the brief says), NPCs walk within their movement facts and speak admitted voice lines.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

TRAVEL-CONTENT-1 -> NPC-TRAVEL-2 (NPC-TRAVEL-1, PREM-3); NPC-ACTOR-1 (CREATURE-MOVE-1, NPC-CONTENT-2); NPC-VOICE-1 (CHAT-1).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #2040 NPC trade writer | MERGED |
| CHAR-POSITION-1 | MERGED (#2006) |
| PREMIUM-1 (PREM-WIRE-1) | MERGED (#2110) |
| CHAT-1 / CHAT-1b-2b (NPC-VOICE-1) | MERGED (#2093, archive chat-1b-2b) |
| acceptance of TRAVEL0 (TRAVEL-* children) | BLOCKING |
| NPC-CONTENT-2 (movement facts, voices) | BLOCKING (open PR #2129, Wave 2) |
| CREATURE-MOVE-1 residue (NPC-ACTOR-1) | BLOCKING (Wave 2, not merged) |
| NPC-PLACE-1b, NPC-WIRE-1 (NPC-ACTOR-1) | MERGED (archive npc-place-1b, npc-wire-1) |
| NPC-TRAVEL-1, PREM-3 (NPC-TRAVEL-2) | BLOCKING (no record on main) |
| PREM-1, GOLD-FEE-1b, QUEST-PRED-1, SPEED-1, VIS-2 | MERGED (archive records) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/npc/travel/** (sources only)`
- `apps/game-server/src/gameplay_transport/npc_travel.rs`
- `apps/game-server/src/gameplay_transport/npc_travel_tests.rs`
- `apps/game-server/src/gameplay_transport/npc_actor.rs`
- `apps/game-server/src/gameplay_transport/npc_actor_tests.rs`
- `apps/game-server/src/gameplay_transport/npc_voice.rs`
- `apps/game-server/src/durability/npc_travel.rs`
- `apps/game-server/tests/npc_world_postgres.rs`
- `docs/agents/evidence/NPC-WORLD-1-*.md`
- `docs/agents/tasks/NPC-WORLD-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- connection.rs NPC_TRADE_INTENT l.2766 / talk path (travel keyword routing)
- world_runtime.rs actor tick registration
- CHAR-POSITION-1 position writer (travel destination)
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568

## Inseparable bundles (never split across PRs)

- NPC-TRAVEL-2: fee debit + position write + audit in one transaction
- NPC-ACTOR-1: movement facts + actor tick + visibility tests

## Acceptance criteria

- [ ] Travel debits the fee once and moves the Character atomically; premium refusal typed.
- [ ] Walking NPCs stay within movement facts and never block mandatory tiles.
- [ ] Voice lines come only from admitted content and respect chat rate limits.
- [ ] Restart resumes NPC positions from content, not stale runtime.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server npc_travel
cargo test -p oteryn-game-server npc_actor
cargo test -p oteryn-game-server npc
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_TRAVEL0_NPC_TRAVEL_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
- PRs #2040, #2006, #2110, #2093, #2123; open #2129
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
