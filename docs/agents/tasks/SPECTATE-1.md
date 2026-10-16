# SPECTATE-1 — spectating

```yaml
task_id: SPECTATE-1
title: "SPECTATE-1: spectating (skeleton)"
mode: IMPLEMENT
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/spectate-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 row SPECTATE-1 (row 138); App. 7 LOOP-WORLD-1, SPECTATE-1; §5 Wave 5"
class: "hard (child graph)"
needs_migration: false
needs_registry: yes
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/spectate/ (new) + tests"
  - "crates/protocol-oteryn/src/spectate.rs (wire child only)"
  - "docs/agents/tasks/SPECTATE-1.md"
slot_only_paths:
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json"
  - "crates/protocol-oteryn/src/lib.rs"
  - "apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Spectating (read-only view of a player session).

## Accepted ruling and delivery boundaries (2026-10-16)

The [CP/architect ruling D1035–D1039](https://github.com/Oteryn/Oteryn-Game/pull/2183#issuecomment-6216592075) accepts the brief with amendments and supersedes the conflicting skeleton requirements below. The owner already chose Global parity option (b), an accepted Oteryn deviation; no owner decision is pending. O1–O12 remain unchanged. `SPECTATE_V1` is offered only after the three children merge and CP enables it.

- SPECTATE-GRANT-1 and SPECTATE-SESSION-1 land as commits on one mechanic PR. Audit uses the existing event foundation.
- SPECTATE-WIRE-1 is a separate PR with protocol review; `spectate.rs` belongs only to that child.
- Observation has no durable observer state. Any required migration is a separate PR, not a slot commit in this mechanic PR.
- O2 Gateway admission is a separate child allocated by CP; this task does not allocate or implement that boundary.
- The candidate brief and the completion-plan pointer edit move to an architecture acceptance PR under D1038. Both are outside these owned paths; their transfer is a CP/architecture action.

## Dependencies (D1037; verify merge evidence before implementation)

| Dependency | State | Gate |
|---|---|---|
| Architecture acceptance PR | D1035 accepts with amendments; D1038 requires brief and plan transfer/integration | **BLOCKING** before GRANT-1/SESSION-1 |
| FND-04B | Current merged delivery evidence required | **BLOCKING** before GRANT-1/SESSION-1 |
| MAP-WIRE-2 | MERGED (#1810, accepted completion plan) | Prerequisite; revalidate on implementation base |
| CHAT-1b-2b | Current merged delivery evidence required | **BLOCKING** before GRANT-1/SESSION-1 |
| SCOPE-HANDOFF-1 | MERGED (D1037; #1782 in accepted completion plan) | Prerequisite; revalidate on implementation base |
| O2 Gateway admission child | CP allocation pending | Observer admission/integration boundary |

LOOP-WORLD-1 is not a prerequisite under D1037. Next: architecture acceptance PR, then GRANT-1/SESSION-1 after FND-04B, MAP-WIRE-2 and CHAT-1b-2b are merged. Do not infer accepted schemas or Gateway authority from the candidate brief.

## Owned paths (builder edits freely)

- apps/game-server/src/spectate/ (new) + tests
- crates/protocol-oteryn/src/spectate.rs (wire child only)
- docs/agents/tasks/SPECTATE-1.md

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
- crates/protocol-oteryn/src/lib.rs
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

Shared module registration and wiring require the exact CP slot and its allocated author. Any migration requires a separate PR under D1036.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/world_map.rs, world_spatial.rs (view stream)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Mechanic and wire delivery (D1036)

- GRANT-1 + SESSION-1, including their tests, form one mechanic PR.
- WIRE-1, including its codec, registry, capability, projection privacy filter and tests, is a separate PR with protocol review and CP-allocated shared-path work.
- Any migration is a separate PR; Gateway O2 admission is a separately allocated child.

## Acceptance criteria

- [ ] spectator cannot issue any gameplay command
- [ ] privacy: consent of the spectated player
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] No durable observer state; restart ends observer sessions (O11). Target session-generation change ends its grant and observer streams (O3).
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] All registered gameplay command types are refused before a gameplay handler runs, with no state change.
- [ ] Projection privacy tests exclude inventory, private chat and command outcomes.
- [ ] Audit uses the existing event foundation; activation waits for all three children and CP enablement.

## Validation commands

```sh
cargo test -p oteryn-game-server spectate
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- CP/architect D1035–D1039: https://github.com/Oteryn/Oteryn-Game/pull/2183#issuecomment-6216592075
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §7 LOOP-WORLD-1, SPECTATE-1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Checkpoint

At PR head `e42184633af97cf32494713ab101ed8e7870d5b6`, the delivered scope is the task record plus candidate brief and plan pointer; no spectate runtime or codec is implemented. This card correction does not claim implementation or validation of the mechanic. Remaining own code is GRANT-1/SESSION-1 after the dependency gate; WIRE-1 follows its separate allocation. CP/architecture owns the acceptance PR and outside-owned transfer; CP owns the Gateway allocation, shared-path slots, ready, queue and merge. Resume on architecture acceptance integration plus merged prerequisite evidence and the applicable allocation, not on an owner re-approval. Required implementation checks are listed above; the card-only correction requires documentation validation.
