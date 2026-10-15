# LOOP-MAP-1 — map loop: Cyclopedia discovery, donations and respawn (skeleton task record)

```yaml
task_id: LOOP-MAP-1
title: "LOOP-MAP-1: map loop: Cyclopedia discovery, donations and respawn"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row LOOP-MAP-1; §7 'LOOP-MAP-1' (CYCLOPEDIA0 child graph)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md
  - docs/architecture/reviews/OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/loop-map-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Close the map loop: the client-visible map runs within the viewport budget, Characters discover map areas durably (Cyclopedia), donate to areas through the bank fee path and see respawn facts, with parity against the reference.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

CYC-CONTENT-1 -> CYC-DISCOVERY-1 (CHAR-REV-SEQ-1) -> CYC-DONATE-1 (BANK-FEE-0) -> CYC-RESPAWN-1 (SPAWN-1b) -> CYC-WIRE-1 (Reg) -> CYC-PARITY-1. MAP-VIEWPORT-OPT-1 / MAP-VIEWPORT-LIMIT-1 are handled separately and are NOT part of this draft.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #1942 client proof | BLOCKING (draft PR open) |
| MAP-VIEWPORT-OPT-1 | BLOCKING (in hand, W3a2) |
| MAP-VIEWPORT-LIMIT-1 | BLOCKING (not merged; in hand) |
| acceptance of CYCLOPEDIA0 (CYC-* children) | BLOCKING |
| MAP-LOAD-1 | MERGED (archive MAP-LOAD-1) |
| MAP-OVERLAY-1a / 1b | MERGED (#1800, #1809) |
| MAP-OVERLAY-1c | BLOCKING (Wave 3, not merged) |
| VIS-2 | MERGED (archive OTV2-20260930-vis-2) |
| CHAR-REV-SEQ-1 (CYC-DISCOVERY-1) | MERGED (archive OTV2-20261003-char-rev-seq1) |
| BANK-FEE-0 (CYC-DONATE-1) | BLOCKING (CANDIDATE decision) |
| SPAWN-1a / SPAWN-1b (CYC-RESPAWN-1) | MERGED (archive OTV2-20261011-spawn-1b) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/cyclopedia/** (sources only; generated outputs are slot-only)`
- `apps/game-server/src/domain/cyclopedia.rs`
- `apps/game-server/src/durability/character_cyclopedia.rs`
- `apps/game-server/src/durability/character_cyclopedia_tests.rs`
- `apps/game-server/src/gameplay_transport/cyclopedia.rs`
- `apps/game-server/src/gameplay_transport/cyclopedia_tests.rs`
- `crates/protocol-oteryn/src/cyclopedia.rs`
- `apps/game-server/tests/cyclopedia_postgres.rs`
- `docs/agents/evidence/LOOP-MAP-1-*.md`
- `docs/agents/tasks/LOOP-MAP-1.md` (this record: status and notes only)

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

- connection.rs dispatch chain (CYCLOPEDIA intent next to CHAT_INTENT l.2735)
- capabilities.rs `GATED` l.165 (CYCLOPEDIA capability not offered until CP decides)
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568
- durability/mod.rs registration; map/view.rs read-only for discovery hook

## Inseparable bundles (never split across PRs)

- CYC-DISCOVERY-1: migration + discovery writer + character revision sequence bump
- CYC-DONATE-1: donation debit + bank fee audit in one transaction
- CYC-WIRE-1: wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Discovered areas persist per Character and survive restart; duplicate discovery is idempotent.
- [ ] Donation debits the bank once and records the area total atomically; concurrent donations serialize.
- [ ] Respawn facts come from SPAWN-1b data, not client input.
- [ ] Map stays within the MAP-VIEWPORT budget (evidence MAP-VIEWPORT-MEASURE-1).
- [ ] Older clients receive no Cyclopedia domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server cyclopedia
cargo test -p oteryn-game-server map
cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md
- docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md §1.2
- docs/agents/evidence/MAP-VIEWPORT-MEASURE-1-viewport.md
- PR #1942 (client proof, open draft)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
