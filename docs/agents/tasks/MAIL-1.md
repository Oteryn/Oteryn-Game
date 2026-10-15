# MAIL-1 — mail and parcels (skeleton task record)

```yaml
task_id: MAIL-1
title: "MAIL-1: mail and parcels"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row MAIL-1 (row 88); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (MAIL-CONTENT-1, MAIL-TEXT-1, MAIL-1, MAIL-WIRE-1, MAIL-PARCEL-1, MAIL-SYSTEM-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/mail-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Letters and parcels between characters of one World, delivered to the CharacterInbox: mail operations, letter posting, stamp transform, mail event, Inbox mail ceiling and posting rate (MAIL-1), with its content, text store, wire, parcel and system-letter children.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

MAIL-CONTENT-1 (content) -> MAIL-TEXT-1 (hard) -> MAIL-1 (hard) -> MAIL-WIRE-1 (Reg) / MAIL-PARCEL-1 / MAIL-SYSTEM-1 (after HOUSE-1b, Wave 5). WRITE-WIRE-1 completes with this family.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a | BLOCKING |
| MAIL-CONTENT-1, MAIL-TEXT-1 (own children) | BLOCKING (not started) |
| INBOX-1a | MERGED (#1785) |
| ITEM-MOVE-1 | MERGED |
| MAP-LOAD-1 | MERGED (archive MAP-LOAD-1) |
| MAP-WIRE-2 | MERGED |
| BAGS-1, BAGS-DEPOT-1 (MAIL-PARCEL-1) | BLOCKING |
| HOUSE-1b (MAIL-SYSTEM-1) | BLOCKING (Wave 5) |
| acceptance of MAIL0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/mail.rs`
- `apps/game-server/src/durability/mail.rs`
- `apps/game-server/src/durability/mail_tests.rs`
- `apps/game-server/src/durability/item_text.rs`
- `apps/game-server/src/gameplay_transport/mail.rs`
- `apps/game-server/src/gameplay_transport/mail_tests.rs`
- `crates/protocol-oteryn/src/mail.rs`
- `crates/protocol-oteryn/src/mail_tests.rs`
- `apps/game-server/tests/mail_postgres.rs`
- content authoring inputs for parcel/letter/label/stamped definitions and mailbox bindings (source files only, not regenerated outputs)
- `docs/agents/tasks/MAIL-1.md` (this record: status and notes only)

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

- durability/character_inbox.rs (INBOX-1a) is the delivery target; extend via new functions, no schema change outside the MAIL migration
- connection.rs command dispatch chain (l.2735-2766 pattern)
- capabilities.rs `GATED` l.165 (`MAIL_V1`)
- crates/protocol-oteryn/src/lib.rs `pub mod` list
- GAME_EVENT_FOUNDATION_REGISTRY.json (mail event type)

## Inseparable bundles (never split across PRs)

- content (MAIL-CONTENT-1) + regenerate_content.py + pins by one author
- migration + mail writer + durability tests (MAIL-1)
- wire + registry + capability + handler + tests (MAIL-WIRE-1)
- stamp transform = BURN of label/parcel + MINT into Inbox + ledger in one transaction

## Acceptance criteria

- [ ] Posting is idempotent; a crash between debit and Inbox write leaves neither.
- [ ] Unknown recipient -> `UNKNOWN_RECIPIENT`; junior parcel -> `RECIPIENT_CANNOT_RECEIVE_PARCELS`; refused item stays with sender.
- [ ] Inbox mail ceiling and posting rate enforced from RESOURCE_LIMITS_REGISTRY rows.
- [ ] Text store rejects oversize/unsafe text (MAIL-TEXT-1 security review).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server mail
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md §3-§10, §14
- docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md §5 (CharacterInbox)
- apps/game-server/src/durability/character_inbox.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
