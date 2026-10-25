# HIRE-1 — hirelings (skeleton task record)

```yaml
task_id: HIRE-1
title: "HIRE-1: hirelings (skeleton)"
mode: IMPLEMENT
status: waiting
pr: null
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/hire-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 row HIRE-1 (row 90); §5 Wave 5"
class: "impl"
needs_migration: yes
needs_registry: yes  # HIRE1-RL-01..04 (RESOURCE_LIMITS_REGISTRY) and the HIRE-WIRE-1 capability/wire limits
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/hireling/ (new) + tests"
  - "content/npcs hireling inputs (source only)"
  - "docs/agents/tasks/HIRE-1.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_hirelings.sql"
  - "regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in crates/gameplay/dispatch/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. The merged #2171 packet is design/tracking only; no hireling delivery/placement implementation is retained. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: Platform Store delivery contract, hireling storage, house placement and services remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Server wiring gap list

Audit 2026-10-23, re-checked 2026-10-25 at `8abd5a621ada20032e97f7409f5c2004669f0692`.

`status: waiting` is retained. This source audit supersedes older wiring/dependency
observations below, without asserting live Platform PR state. Negative findings
were checked across `apps/`, `crates/`, `tools/`, migrations and contract fragments.

| Layer | Current evidence | Remaining server work / gate |
|---|---|---|
| Rules / records | HIRE1 defines Character-owned records, a maximum of ten and shared apprenticeship grants ([`HIRE1 decision:38`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L38)). No hireling Rust implementation or SQL migration was found in the source/migration inventories ([`core_modules.rs:55`](../../../crates/gameplay/durability/src/durability/core_modules.rs#L55), [`durability/src/lib.rs:7`](../../../crates/gameplay/durability/src/lib.rs#L7)). | Record/entitlement rules, durable receipts and guards, placement/removal writers and service grants are still absent. Player writes need session-generation fencing; Character state changes need CharacterRevision receipts ([`HIRE1 decision:71`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L71)). |
| House runtime | `current_house_presence` still ends in `HouseInteriorOwnerUnavailable` ([`house_execution.rs:119`](../../../crates/gameplay/spell/src/spell/house_execution.rs#L119)); `exit_into_channel` returns `ExitNotAdmitted` ([`house_scope/mod.rs:16`](../../../crates/gameplay/world/src/house_scope/mod.rs#L16)). Admission still checks a Channel runtime scope ([`gameplay_transport/mod.rs:1248`](../../../crates/gameplay/dispatch/src/gameplay_transport/mod.rs#L1248)). | A real House actor/session owner and admitted entry/exit remain prerequisites for placement and NPC services. HOUSE-RUNTIME-1b records the transfer-contract gate ([`HOUSE-RUNTIME-1b.md:51`](HOUSE-RUNTIME-1b.md#L51)); a physical Channel tile cannot supply that authority. |
| House ACL / returns | Character guest/subowner ACLs and move-out disposition now exist ([`0168_house_management.sql:71`](../../../apps/game-server/migrations/0168_house_management.sql#L71), [`0178_house_disposition.sql:5`](../../../apps/game-server/migrations/0178_house_disposition.sql#L5)). | No hireling return participant is composed into these owners. H4 requires returns on disposition, owner change and catalogue changes, not only move-out ([`HIRE1 decision:47`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L47)). |
| Content / services | `QualifiedHouseTiles::from_source_world` still yields an explicitly empty catalogue; the separate qualifier can validate authored placement documents ([`native_house_tiles.rs:63`](../../../crates/gameplay/content/src/content/project/native_house_tiles.rs#L63), [`native_house_tiles.rs:73`](../../../crates/gameplay/content/src/content/project/native_house_tiles.rs#L73)). NPC trade has its own route ([`0190-npc_trade.mechanic:1`](../../../crates/gameplay/dispatch/mechanics/0190-npc_trade.mechanic#L1)). | No hireling catalogue consumer, House NPC spawn/service composition or owner/guest grant enforcement exists. Ordinary NPC trade does not establish hireling identity or House authority. |
| Acquisition | H1 requires a delivery-id-keyed Platform Store grant, with no lamp/item or Game gold sale; §4 calls for signed delivery and a status callback ([`HIRE1 decision:38`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L38), [`HIRE1 decision:86`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L86)). No accepted Store-specific delivery contract/runtime was found in this tree. | Production acquisition awaits the owning Store contract. §4 permits test entitlements for non-delivery packets only ([`HIRE1 decision:93`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L93)); they cannot become production grants. External acceptance state is UNKNOWN here. |
| Wire / dispatch / E2E | Protocol modules and mechanic fragments contain no hireling codec, route or capability ([`protocol-oteryn/src/lib.rs:38`](../../../crates/protocol-oteryn/src/lib.rs#L38), [`mechanics/README.md:3`](../../../crates/gameplay/dispatch/mechanics/README.md#L3)); existing dispatch PG cases use directory auto-mounts ([`postgres_tests/README.md:3`](../../../crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/README.md#L3)). | Bounded codecs, admitted House handler, owner-only domain/results and real-dispatch restart/replay/guest/stale-session cases remain missing. No hireling PG case was found. |

**Allocation check:** capability 52, commands 62–63, migration 0171 and resource
fragment ordinals 0820–0826 from the old audit have **no corresponding entries/files**.
They are historical proposals, not allocations held by this HEAD. The authoritative
split protocol/resource layouts and migration loader are
[`registry/README.md:8`](../../../docs/contracts/protocol-oteryn/v1/registry/README.md#L8),
[`build.rs:72`](../../../crates/protocol-oteryn/build.rs#L72) and
[`durability/src/lib.rs:7`](../../../crates/gameplay/durability/src/lib.rs#L7).

Parity requirements remain owner-only placement/removal, door/occupied-tile
refusal, shared services, owner-only Banker/Steward and guest Apprentice/Trader/Cook,
and durable unplacing on House loss
([`HIRE1 decision:42`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L42),
[`HIRE1 decision:53`](../../architecture/reviews/OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md#L53)).
None has a hireling runtime caller at this HEAD. Resolve the House runtime/transfer
prerequisites and Store acquisition boundary, then implement those layers together;
no hireling runtime or PG qualification is claimed by this documentation audit.

## Goal

Hirelings (house NPC servants: purchase, placement in the house, services).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| HOUSE-1b | NOT MERGED | **BLOCKING** (HIRE-DATA-1 extends the HOUSE-1b rent, move-out, eviction and disposition transaction; HOUSE-1a excludes it) |
| NPC-ECON-1 | NOT MERGED (Wave 2) | **BLOCKING** |
| PREMIUM-1 | MERGED (#2110) | ok |
| acceptance of the HIRE1 brief (`OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md`) | ACCEPTED by the owner 2026-10-15 ("akceptuje hire i craft"), recorded in ACCEPT-BATCH-2026-10-15 §7; effective when this PR is integrated. Delivery still waits for the Platform Store contract (#1625) | ok |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/hireling/ (new) + tests
- content/npcs hireling inputs (source only)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_hirelings.sql
- regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- house ownership and disposition (HOUSE-1b)
- NPC trade/bank (NPC-ECON-1)
- premium gate
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- content + regeneration + pins by one author

## Acceptance criteria

- [ ] only house owner may place a hireling
- [ ] delivery consumed from the Platform Store: a signed, idempotent Store delivery is applied once per delivery id and reported by status callback; no price, currency or payment data exists in Game, and Game never sells a hireling for gold
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server hireling
python3 tools/content-migration/regenerate_content.py && git diff --exit-code
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 row HIRE-1
- `docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
