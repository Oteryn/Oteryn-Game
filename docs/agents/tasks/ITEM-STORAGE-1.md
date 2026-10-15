# ITEM-STORAGE-1 — item storage: depot, supply stash, writable items (skeleton task record)

```yaml
task_id: ITEM-STORAGE-1
title: "ITEM-STORAGE-1: item storage: depot, supply stash, writable items"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
pr: 2156
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ITEM-STORAGE-1; §7 'ITEM-STORAGE-1' (DEPOT0, SUPPLY_STASH0, WRITE0 child graphs)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-storage-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Characters store items durably in a World-wide per-Character depot and a supply stash, and read/write books, scrolls and blackboards, all through the ITEM-MOVE-2 transaction family.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

DEPOT-CONTENT-1 -> DEPOT-1 -> DEPOT-WIRE-1; STASH-CONTENT-1 -> STASH-1 -> STASH-WIRE-1; WRITE-CONTENT-1 -> WRITE-1 -> WRITE-MAP-1 -> WRITE-MOD-1. WRITE-WIRE-1 is excluded from this completion set.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| acceptance of DEPOT0 (plan §7 gate) | ACCEPTED on protected main 3ef900c; stash/write are outside this allocation |
| HOUSE-CUSTODY-1 exclusivity guard (DEPOT-1) | VERIFIED migration 0025 exists; depot join remains CP integration work |
| ITEM-MOVE-2a / 2b (DEPOT-1, STASH-1, WRITE-1) | BLOCKING (ITEM-LOOP-1 children) |
| ITEM-VIEW-1b (DEPOT-WIRE-1) | MERGED (archive OTV2-20261003-item-view-1b) |
| MAP-LOAD-1, MAP-WIRE-1 (MAP-WIRE-2) | MERGED (archive MAP-LOAD-1, #1810) |
| WO-1, BANK-RET-0 | MERGED (#1748) |
| MARKET-CONTENT-1 (STASH-CONTENT-1) | BLOCKING (MARKET-1, Wave 4) |
| MAIL-TEXT-1, BAGS-1 (WRITE-1) | BLOCKING (no record on main) |
| HOUSE-RUNTIME-1a/1b/1c (WRITE-1 house items) | BLOCKING (Wave 3, not merged) |
| LEVER-1 (WRITE-MAP-1) | BLOCKING (no record on main) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (exclusive bounded root allocation)

- `apps/game-server/src/durability/character_depot.rs`
- `apps/game-server/src/durability/character_depot_tests.rs`
- `apps/game-server/src/gameplay_transport/depot.rs`
- `apps/game-server/tests/item_storage_postgres.rs`
- `docs/agents/tasks/ITEM-STORAGE-1.md`

No item_text, stash, protocol, content or shared registration path is allocated.
All module registration, migrations, native transfer/audit hooks and runtime wiring
remain CP slot work. Worker publication/commit authority is absent.

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

- connection.rs dispatch chain (DEPOT/STASH intents near NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` l.165 and offered sets l.76/104/132
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568
- durability/mod.rs registration; HOUSE-CUSTODY-1 exclusivity guard call

## Inseparable bundles (never split across PRs)

- DEPOT-1 / STASH-1: migration + container writer + move transaction + audit
- WRITE-1: text payload limit (RESOURCE_LIMITS) + writer + tests
- each *-WIRE-1: wire + registry + capability + tests

## Acceptance criteria

- [ ] Depot and stash contents survive restart and are per Character per town as the briefs require.
- [ ] Every store/withdraw is one ITEM-MOVE-2 transaction; no item duplication under concurrent moves.
- [ ] Depot access excluded while the same item is under house custody.
- [ ] Written text respects the RESOURCE_LIMITS cap and is sanitized.
- [ ] Older clients receive no depot/stash domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Family integration validation commands (not claimed for inactive preparation)

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server depot
cargo test -p oteryn-game-server stash
cargo test -p oteryn-game-server item_text
cargo test -p oteryn-game-server item_transfer
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
- docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md
- PRs #1810, #1748
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded DEPOT-1 packet, before code (2026-10-15)

Root supplied META 3.1.0 and the exact five-path lease on existing draft #2156,
`skeleton/item-storage-1`, base 89f5598db8db824e5793ae3e349dc4197bf2bed0.
DEPOT0 is ACCEPTED on protected main 3ef900c160c9713092a86bfcd1a8377c7587c898;
this supersedes only the historical acceptance blocker above. Family/child
integration gates and all acceptance boxes remain open. Scope is the accepted
first slice, not BAGS trees, Inbox, stash, writing or mail.

### Source-grounded component

DEPOT0 §§3,5,6,8: one World+Character custody, no Channel/town/runtime scope;
boxes 1..17, positive never-renumbered ordinal, empty items only, one row per item
regardless of quantity; 15,000 hard entries, 2,000 Free / 15,000 current Premium;
32 entries in one page; each of two TRANSFER shapes is 1 item / 2 locations /
3 work units. Taking out is allowed after Premium lapses, with no stack merges.
Native PremiumGate is a raw planning input, not a new current-authority proof.

Prepare a private uninstalled table/guard proposal, validated custody/read models,
bounded summary/page reads and capacity planning. Same-World integrity must bind
both native item FK and Character root; capacity guard locks the root and counts
rows. PostgreSQL tests apply the exact proposed SQL in an isolated 17.6 database;
no production migration/schema/grant is installed. Reads do not open views or
prove ownership. They refuse inconsistent/corrupt counts and limit page reads.

Prepare only raw shape planning over actual native item/definition/backpack
models: direct main-backpack entry into a box, or depot entry out to the empty
container slot when eligible, otherwise one new direct entry. Reuse the native
stack and container-slot planner checks; no fabricated definition facts, existing
merge/top-up path, guessed shape number or independent item relocation.

### cross_domain_findings / withheld value writer

- HOUSE-CUSTODY migration 0025 exists: its item exclusivity function counts the
  native location families, not a depot table. Join depot (and reject contents)
  in that function, including later native amendments, in the CP migration.
- Native item_transfer source/destination/TransferShape and audit locations are
  closed to Ground/Corpse -> container slot/backpack and merge shapes. Add the
  accepted two depot shapes, depot source/destination_box and nullable custody
  channel to reservation/receipt/audit proof composition; preserve every prior
  body and guard. No native enum is changed in this lease.
- Backpack deletion already has non-depot proof clauses (latest replacement 0083:
  equipment, spell/temporal, timed, bank, trade/use and fee cases), but no depot
  TRANSFER clause or callable native depot primitive. CP must preserve that complete
  union while composing entry removal and depot
  insertion/removal evidence, game_item_placement_proven and consistency/exclusivity
  in the same receipt/audit transaction. This component supplies no dead writer.
- Current character_item_fence_is_current is explicitly Channel scoped. Depot
  custody is World scoped; accepted composition rule 2 still requires current
  session/lease/root checks, but not depot runtime ownership. The owner must reuse
  the appropriate current fence boundary without manufacturing scope authority.
- No typed current depot locker/open-view/item-handle owner exists. Recheck current
  reach (Chebyshev distance 1, same floor) and generation/session-bound live view
  at command execution, including after reconnect, before any future value commit.
- Root -> items in ID order -> container-slot lock, replay/ambiguity reconciliation,
  item-only CharacterRevision behavior, capacity at commit and all atomic proof
  obligations remain part of the future shared writer. Raw planner success is not
  permission, reservation, item lock, receipt or committed transfer.

Durable evidence: `/workspace/oteryn-prep/item-storage-1-evidence/`.

### Ordinal representation and validation setup

The proposed schema/read/planner use native B3 u64 ordinal representation (0011
placement_ordinal and destination_ordinal range 1..18446744073709551615; native
ContainerEntryPosition is u64), including checked overflow refusal. DEPOT0 names
NUMERIC(20), without a separate decimal-ordinal API. This is explicit private
native-adapter preparation, not a new resource registry bound or an architecture
claim about values beyond native u64. Preserve the owning ordinal representation
at CP integration; a wider decimal domain would require its actual adapter.

Seven literal owned rule/planner tests passed in a cfg(test)=false production-native
rlib bridge using unchanged actual native types/private planner; no global unit ELF
was linked. The first separate PostgreSQL-target compile failed SQLx0.9 E0521/E0597:
controlled dynamic fixture Execute strings require AssertSqlSafe. Only that target
was corrected; no PostgreSQL behavior ran in the failed setup and no behavioral
RED is claimed. Original target source/argv/diagnostics and the seven-case source
binding are retained. The unstarted-waiter cancellation check raced with completion;
no process was signaled. No unchanged seven-case rerun is needed for this fixture fix.

### Actual reader regression and REPORT_ONLY sibling finding

Canonical PG17.6 exposed a real first-page failure: the projection
ordinal::text AS ordinal made unqualified ORDER BY ordinal sort text, e.g.
9999,...,9990,999,9989. The preserved behavioral RED is 3 pure passes / 1 PG
failure (InvalidStoredState), distinct from the earlier SQLx setup compile error.
The minimal repair qualifies the numeric table column in the ORDER BY. Final
by-path PG qualification, not the old native read symbol, executes that reader.
qualification-source-matrix.json binds unchanged native 4 planner/test file and
unchanged called storage helper/type bytes; no final-whole-file native 7 run is
claimed. The final PG target repeats the 3 storage pure cases against final source.

REPORT_ONLY P2, outside lease: character_inbox.rs read_character_inbox lines 129/132
and read_character_inbox_deliveries lines 178/181 use the same text projection /
unqualified numeric ORDER BY alias. Native floor 89f5598 and protected main ac9f172
are byte-identical; exact full SHA256 is recorded in
inbox-report-only-finding.json. The read-only 170006 SQL probe returns 1,10,2; an
explicit numeric table qualifier returns 1,2,10. Delivery-order readers misorder
cross-digit ordinals, and any future consumer paginating those vectors inherits
wrong page membership/order; no shipped Inbox pagination or value mutation is
asserted. Minimal owner patch: ORDER BY game_item_character_inbox_locations.ordinal
and ORDER BY game_character_inbox_deliveries.ordinal respectively. Root carries
this finding to CP/architect; no unowned file was edited.

### Boundary classification

Identity/binding checks cover Character+World and item+World integrity, box/ordinal
bounds and uniqueness, item definition equality and raw source/contents rules.
Storage capacity consumes independent database counts under a Character root lock;
it does not infer current ownership from a prepared count or row. Current session,
lease/generation, locker/view liveness, admission and provenance are not granted by
any planner/read return and remain future owner boundaries. Expired/replaced fences,
reconnect substitution, reservation replay and native audit causation are therefore
NOT_APPLICABLE to this component as mutation operators; they are explicitly not
qualified for a future writer. Ordinal overflow, immutable-row mutation and database
count/World substitutions are applicable and covered. There is no new timer or
CharacterRevision mutation. Withdraw ignores raw PlanInput.destination and selects
the accepted slot/direct-entry branch from the raw slot state; no new command choice
or public wire semantics is assigned.

Final source sweep corrects the historical fee-only statement in the original
before-code packet: migration 0083 is the latest backpack-removal proof union;
0076 is the latest physical exclusivity union (including equipment and Inbox).
The actual missing delta is a depot TRANSFER clause/family, not the absence of
all non-fee inventory deletion. Existing equipment-specific removal proof cannot
serve as depot placement/audit proof. CP must extend these latest complete bodies,
plus placement/reservation/receipt/audit source/destination unions, preserving all
prior admitted shapes. The immutable before-code packet is historical evidence;
this final task record carries the corrected source-grounded delta.


### Final bounded qualification (2026-10-15)

Eight unique owned scenarios are qualified: seven raw/native-planner cases before
an SQL-only reader repair, and one final proposed-schema PostgreSQL scenario. The
final standalone real-public-import PG target passed 4/0 on PostgreSQL 17.6
(server_version_num 170006), including three repeated storage cases. The source
matrix binds unchanged planner and called helper bodies; final whole-file native
seven execution is not claimed. The actual ordering regression RED 3/1 and both
setup failures are preserved separately.

Final original-unit-context native Clippy metadata passed with warnings denied,
including the exact PG target source in a validation-only self-alias root. The
standalone PG compilation/run independently proves real external public imports.
A coherent Cargo graph refresh intentionally stopped the root compiler at sentinel
86 (Cargo exit 101), before global unit codegen: this capture is not a test/check
pass. Immediate metadata Clippy under the same lock returned 0. Two duplicate_mod
allowances exist only at duplicated snapshot import boundaries; no owned production
bytes or lint settings were weakened. Original registrations remain unchanged.
Final source bindings distinguish the refreshed lint dependency graph from earlier
runtime artifacts rather than claiming a single unchanged graph throughout.

Direct formatting of all four unregistered Rust files and workspace format checks
passed. Governance validated 22 documents/9 lanes and the unchanged validator suite
passed 124/0. Whole owned diff self-review and exact five-path/empty-index/base checks
are retained. Durable final source copies, original roots, restoration inputs,
all RED/GREEN/setup/lint logs, hashes and local.patch are in the evidence folder
above. No native value writer, installed schema/grants, runtime integration or family
acceptance is claimed; CP composition and independent architecture review remain due.
