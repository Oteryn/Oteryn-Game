# ITEM-TIMED-1 — timed items: charges, duration, repair (skeleton task record)

```yaml
task_id: ITEM-TIMED-1
title: "ITEM-TIMED-1: timed items: charges, duration, repair"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
pr: 2155
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ITEM-TIMED-1; §7 'ITEM-TIMED-1' (TIMED_ITEM0/0B child graph)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-timed-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Items with charges and duration behave end to end: runtime consumption on move/equip/use, house placement, condition effects, client wire, reference parity and NPC repair.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

TIMED-RT-1c (ITEM-MOVE-2a/2b) -> TIMED-HOUSE-1 (HOUSE-RUNTIME-1a) -> TIMED-FX-1 (COND-1) -> TIMED-WIRE-1 (Reg) -> TIMED-PARITY-1 -> TIMED-REPAIR-1 (NPC-TRADE-1 fee path). IMBUE/FORGE are Wave 4 and excluded.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| acceptance of TIMED_ITEM0 / TIMED_ITEM0B (plan §7 gate) | ACCEPTED on primary `e2da9805`, ACCEPT-BATCH §1 item 5 |
| TIMED-CONTENT-1, TIMED-RT-1a, TIMED-RT-1b | MERGED (#1699, #1681, archive timed-rt-1b) |
| ITEM-MOVE-2a / ITEM-MOVE-2b (TIMED-RT-1c) | BLOCKING (ITEM-LOOP-1 children; only the #2009 equip slice is on main) |
| ITEM-USE-1c | MERGED (#2082) |
| EQUIP-RT-1 | BLOCKING (no record on main) |
| COND-1 (TIMED-FX-1) | MERGED (MAGIC-COND-1 #2086, #2122) |
| NPC-TRADE-1 gold fee path (TIMED-REPAIR-1) | MERGED (#2040) |
| HOUSE-RUNTIME-1a house placement (TIMED-HOUSE-1) | BLOCKING (Wave 3, not merged) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (exclusive root allocation for this preparation)

- `apps/game-server/src/durability/item_timed_repair.rs` (new)
- `apps/game-server/tests/item_timed_postgres.rs` (new; planning tests only here)
- `docs/agents/tasks/ITEM-TIMED-1.md` (packet/notes)
- `apps/game-server/Cargo.toml` (target registration; added per CP D1280 (https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6227727089))
- `apps/game-server/tests/durability_postgres.rs` (includes the support cases in a CI-run target; added per CP D1280 (https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6227727089))
- `apps/game-server/tests/support/item_timed_postgres_cases.rs` (Postgres cases moved here; added per CP D1280 (https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6227727089))

This narrower root allocation supersedes the skeleton family's broad suggestions.
No module registration is permitted in this local preparation.

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

- durability/item_timed_state.rs (existing, TIMED-RT-1a/1b) extension points
- connection.rs dispatch chain (NPC_TRADE_INTENT l.2766 for repair)
- capabilities.rs `GATED` l.165
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568

## Inseparable bundles (never split across PRs)

- TIMED-RT-1c: charge/duration deltas inside the ITEM-MOVE-2 transaction
- TIMED-REPAIR-1: gold fee debit + item state restore + audit in one transaction
- TIMED-WIRE-1: wire + registry + capability + tests

## Acceptance criteria

- [ ] Charge/duration consumption is exactly-once across move, equip, use and restart.
- [ ] Expired items transition deterministically (tick-independent of client).
- [ ] Repair debits gold and restores state atomically; concurrent repair serializes.
- [ ] Placed house items keep their timed state.
- [ ] Parity evidence against reference values recorded.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server item_timed
cargo test -p oteryn-game-server timed
cargo test -p oteryn-game-server item_transfer
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md
- docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md
- PRs #1699, #1681, #2082, #2086, #2122, #2040
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded TIMED-REPAIR-1 packet (2026-10-15, draft #2155)

Exclusive branch `skeleton/item-timed-1`, initial HEAD
`fa5ae83d3bca7ed927b437cadef622eccf9b2682`. Root supplied META 3.1.0 and the
three-file allocation. Worker does not commit, push, freeze, mark ready, merge,
trigger review or edit shared code/content/migration/registry/pins.

### Accepted source and actual residue before implementation

Primary main `e2da9805d296a27b06b32335bf67394509d4910b` ACCEPT-BATCH §1 item 5
accepts TIMED-ITEM-0/0B and BANK-FEE-0; this branch's CANDIDATE headers are
historical. TIMED-ITEM-0 §7 owns exactly Aldo's worn-soft-boots → soft-boots offer
for 10,000 gold. A future admitted-content adapter must resolve its existing
identities/pinned revision; no offer key, fee source or audit tag is guessed here.
The selected item is the first live exact-definition match in B3 order (highest
placement ordinal first) among direct entries of the equipped main backpack,
never equipped or nested. Preserve its instance and location. Reset the timed
row to the admitted target's full values (paired active full values for an
inactive target), revision +1, or absent row → revision1. Worn rows are spent.

Native domain currency already implements denomination/order/change planning;
reuse `plan_fee_within` and real `CoinStack`/`FeePlan`. Native `TimedValues`,
`TimedDefinitionFacts`, `StoredTimedState`, `FeeChangeFacts`, TypedDefinitionRef
and Type2EventTuple exist and are reused. GOLD-FEE-2 and activation machinery
already exist: V1 refuses short coins; V2 permits the bank shortfall for a
non-junior payer, coins first, all eligible coins burned whole with no change.
Junior stays coins only. Native runtime/persistence code is not duplicated.

### Approved executable subset

Prepare deterministic planning from unqualified snapshots: direct-entry selection,
compatible native coin planning with 19-input maximum, activation/junior/balance
split, same-instance timed reset with checked revision, and derived participant/
work bounds (22 touched/participants, 68 work units). Inputs and output carry no
current authority and mint no occurrence, permission, transaction or audit event.
The target facts must already come from admitted content, including paired-form
resolution; the planner cannot certify that provenance. The private planner is
specific to the fixed 10,000 fee, not a generic new repair framework or offer store.

### cross_domain_findings / required owning hooks

1. `FeeBurnCause` and its audit union still contain only CharmUnassign. The native
   fee writer requires a CharacterRevision advance and emits a fee-only event;
   it cannot be called as an item-only repair or combined repair event today.
   CP must add the accepted NpcRepair cause, item-only branch/guard composition,
   record/binding and one combined audit arm with transform/timed before/after.
2. TimedWriteCause::NpcRepair and migration0054's cause8 already exist. Native
   timed helpers are checkpoint/expiry transactions, not an in-transaction repair
   hook. CP must compose timed record/revision CAS, item transform and fee/bank
   debit under the same native fence/root/item/backpack/balance locks, with exact
   occurrence replay/conflict and rollback. Calling separate commit methods is
   forbidden; this preparation provides no dead writer that cannot commit.
3. Current content/NPC readers expose no admitted repair offer adapter. Resolve
   Aldo/offer/content and inactive→active full facts at the owner boundary; do
   prompt confirmation/current eligibility independently. Do not infer them from
   raw plan/request snapshots or introduce a new offer identity here.
4. Registry rows, exact combined event encoding/payload measurement, numbered
   migration/guards, current session-generation scope fences, runtime sequencing,
   unknown outcomes and PostgreSQL restart/concurrency qualification remain CP
   integration work. No authority, durable mutation or success stub is introduced.

The existing `item_timed_postgres.rs` allocation is a filename only: its new tests
will be pure native planning tests; no PostgreSQL result is claimed without a
committable storage component. Full-task acceptance checkboxes remain open.
Durable evidence/config/native-source hashes/logs are written directly to
`/workspace/oteryn-prep/item-timed-1-evidence/`.

### Source-bound validation design and review limits

The small dependency-free Cargo harness compiles both complete, unchanged native
`domain/currency.rs` and `domain/timed_item.rs`, including their existing unit
tests, and the exact owned root test/planner source. Seven supporting public data
declarations are literal native excerpts with complete-source and excerpt hashes;
no function, validator, writer, authority implementation or fee policy is replaced.
The exact copied declarations are TypedDefinitionRef, ItemStackClass,
ItemDefinitionFacts, FeeChangeFacts, Type2EventTuple, TimedDefinitionFacts and
StoredTimedState. The real library publicly exports their modules/types/fields;
the owned root test reexports actual library domain/durability modules, so its
`crate::domain`/`crate::durability` planner imports resolve in the real auto-
discovered integration-test crate too. Complete original-server compilation is
not established by the harness. The source comparator confirms currency, timed
state, fee, audit and native trade files are byte-identical to accepted main.

Twelve new cases cover B3 ordering and coin-index/instance mapping after reorder,
nested/equipped/retired/wrong-revision exclusion, native full target reset for an
inactive target from both absent and spent rows, invalid/overflowing timed rows,
19/20 consumed coin inputs, the 22 touched/participant and 68 work boundaries,
V1/V2/junior and exact bank shortfall, wholly-bank payment, incompatible/nondefault/
invalid coins, malformed entries/capacity, invalid targets and new-change ordinal
overflow. Fixture keys are explicitly synthetic raw facts, not admitted offers or
current authority. The source and result never certify paired-form provenance.

The private usage checker enforces accepted numeric ceilings on derived native
burn/change work, but combined repair envelope encoding and exact payload byte
measurement are not implemented or qualified. The repair's 1-in/1-out transform
and zero ancestors are inherent to its selected one direct item; no transform
I/O/audit/registry arm is invented. Whole-diff self-review keeps every shared fee,
timed storage, enum, registry, migration and registration root untouched. Only
this planner, its allocated test and this card change. No known incapable writer,
SQL no-op or success placeholder is included.

### Actual execution checkpoint

The exact source-bound harness passed **40 tests, 0 failed**: 12 new repair
planning cases, 8 existing native currency cases and 20 existing native timed
lane cases; 0 ignored or filtered. Harness all-target Clippy passed with the
workspace deny list and `--locked --offline --all-targets --quiet -- -D warnings`.
All Rust jobs use the shared lock/target, two jobs and `CARGO_INCREMENTAL=0`.
Before/after verification confirms exact owned/native source and seven literal
native-data excerpt hashes. No Rust source was changed after these tests; no
repeat tests or behavioral RED are claimed. The no-storage scope has no
production authority-consuming mutation boundary to qualify.

Direct owned-file rustfmt and `cargo fmt --all --check` passed. Governance passed
(22 required policy documents, 9 lanes) and 124/124 agent tests passed.
Unchanged-registration-root checks, source hashes,
restoration instructions and the local patch are preserved in
`/workspace/oteryn-prep/item-timed-1-evidence/`. Full original-server tests/Clippy,
PostgreSQL17.6, combined audit-byte maxima and fenced atomic repair/restart/
replay/concurrency remain unqualified pending CP owning-hook integration.
This preparation is not full ITEM-TIMED completion or an architect acceptance.
