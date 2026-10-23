# ITEM-IMBUE-1 — imbuing (consolidated task card)

```yaml
task_id: ITEM-IMBUE-1
title: "ITEM-IMBUE-1: imbuing"
mode: IMPLEMENT
status: IN_PROGRESS
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row ITEM-IMBUE-1 (row 72); §7 'ITEM-TIMED-1' IMBUE-* rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
base_branch: main
branch: claude/happy-davinci-ijz3wy-item-imbue-1
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2144, branch skeleton/item-imbue-1

Status note: the draft qualified a private checkpoint storage owner and an audited
Expire compositor (native PG 17.6 groups 10/10 and 8/8, full package green on its head)
on the pre-ADR-0022 `apps/game-server/src/...` layout, with an uninstalled schema
proposal. It is closing; the mechanic is re-implemented fresh on the current layout.

## Goal

Imbuements: content (24 types x 3 tiers, slots, shrines), item-state table with
apply/clear/ImbueCause/checkpoints/expiry, ticking runtime and effects, scrolls and
etcher, `IMBUE_V1` wire.

Children (allocate from the IMBUE_FORGE0 child graph; the plan row is only the family
entry gate): IMBUE-CONTENT-1 (content), IMBUE-1 (hard), IMBUE-RT-1 (hard, combat),
IMBUE-SCROLL-1 (hard), IMBUE-WIRE-1 (Reg).

## Dependencies (re-verified on origin/main ac3f7fbc6, 2026-10-23)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | MERGED (#2190) |
| ITEM-MOVE-2a | MERGED (equip slice EQUIP-WIRE-1 #2009; remainder in ITEM-LOOP-1 #2190) |
| NPC-ECON-1 (per child: Bank confirmation, bank-backed BUY) | MERGED (#2197) |
| ITEM-TIMED-1 timed repair planner | MERGED (#2155) — does NOT provide Imbue elapsed provenance |
| GOLD-FEE-2 | MERGED (#1781) |
| ATTACK-1b | MERGED (#1798) |
| SPEED-1 | MERGED (#1715, 7d7d5374e; native paced steps and equipment speed consumer present) |
| PREMIUM-1 | MERGED (#2110) |
| NPC-TRADE-1 | MERGED (#2040) |
| ITEM-SEM-USE / USE-WIRE-V1 / ITEM-MOVE-WIRE-1 | MERGED (archive records) |
| IMBUE_FORGE0 acceptance | ACCEPTED (IMBUE-FORGE-0 #1415 + protected ACCEPT-BATCH) |
| Imbue-eligible elapsed / actor-tick producer | BLOCKING for IMBUE-RT-1 (no owner on main) |
| Item-only fee causes Imbue/ImbueClear/ImbueScroll in shared fee API | BLOCKING for Apply/Clear: main admits CharmUnassign and promotion/blessing purchases, all on the CharacterRevision composition; no item-only Imbue causes |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/durability/src/durability/character_equipment.rs` — `EquipmentAuthority`
  and current-equipment checks (consumed by checkpoint load).
- `crates/gameplay/durability/src/durability/item_mint_audit.rs` — `OneItemOperationV1`
  (tags 2..11 and 13 on main; 12 is a coordinated private Imbue/TRADE proposal slot).
- `crates/gameplay/durability/src/durability/item_fee_burn.rs` (`FeeBurnCause`,
  `burn_fee_in_transaction`) and `item_fee_burn_audit.rs` (`FeeBurnCauseV1`) — today
  `CharmUnassign`-only and requiring CharacterRevision composition.
- `crates/gameplay/durability/src/durability/item_timed_state_audit.rs` — `TimedExpiry`
  is timed-item expiry, NOT Imbue expiry; do not reuse it.
- `crates/gameplay/domain/src/domain/timed_item.rs`, `timed_item_host.rs` and
  `crates/gameplay/content/src/content/timed_item_definition.rs` — timed-item host
  (wiring point for elapsed integration, not an Imbue elapsed authority).
- `crates/gameplay/items/src/item_timed_repair.rs` — merged timed repair planner (#2155).
- `docs/contracts/game-events/v1/native_one_item_transaction.proto` — hand-authored type-2 op.
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, `GAME_EVENT_FOUNDATION_REGISTRY.json`.
- `apps/game-server/src/lifted_tests/auto/` — auto-mounted mechanic PG cases in the
  existing server library target; shared native item-transfer harness in `tests/support/`.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/items/src/imbuement.rs` (new) — L3 slot/tier model, `SlotRecord`,
  `check_target_slot`, access subchecks (draft: `domain/imbuement.rs`).
- `crates/gameplay/items/src/imbuement_checkpoint.rs` (new) — L3 pure `CheckpointBatch` planner.
- `crates/gameplay/items/src/imbuement_tests.rs` (new) — pure cases.
- `crates/gameplay/durability/src/durability/imbuement.rs` (new) — L4 checkpoint load/store.
- `crates/gameplay/durability/src/durability/imbuement_expiry.rs` (new) — L4 Expire compositor.
- `crates/gameplay/durability/src/durability/imbuement_expiry_audit.rs` (new) +
  `imbuement_expiry_audit_tests.rs` (new) — type-2 audit message.
- `apps/game-server/src/lifted_tests/auto/imbuement_postgres_cases.rs` (installed),
  `imbuement_expiry_postgres_cases.rs` (planned) — auto-mounted PG cases.
- `apps/game-server/migrations/<next>_game_item_imbuements.sql` (new) — migration number:
  next free number at allocation time (only one migration PR in the queue).
- Later children: content catalogue in `crates/gameplay/content/`, ticking/effects in the
  L4 runtime (`crates/gameplay/combat/`), `IMBUE_V1` handler in
  `crates/gameplay/dispatch/src/gameplay_transport/imbuement.rs` (new, handler only).

## Accepted design (IMBUE_FORGE0 §§3,4,6,7,15 and CP decisions, as built in the draft)

Bounds:

- 3 slots per item (index 0..2); remaining time 0..72,000,000 ms; ticking granularity 60 s.
- Checkpoint transaction ceiling: 10 items / 10 participants / 40 work units. Native
  equipment has 9 slots, so the real PG maximum is 9 items / 27 rows / 36 work.
- Content: 24 of 72 draft entries existed, `DRAFT_NOT_RUNTIME_READY`; Basic-capable
  content codec and admitted catalogue/access readers are still needed.

Eligibility (D1163, CP comment 6225136278): Basic, Intricate and Powerful are allowed on
every slot-bearing item; no per-item tier maximum. Unknown slot count, zero/out-of-range
slot, category/occupancy and complete-snapshot checks fail closed. Premium is a real
`PremiumGate` subcheck (Free accounts refuse higher tiers). Helpers confer no session,
Content or mutation authority.

Retained-slot repair: refuse the whole apply snapshot with `InvalidSnapshot` when any
retained row's slot index ≥ the item's declared slot count (also for zero-time rows).
`SlotRecord::new` validates UUIDs via `ItemInstanceId::from_bytes`.

Storage (private table `game_item_imbuements`):

- Remaining updates are downward-only; no DELETE except a witnessed expiry of a zero row.
- `load_checkpoint_in_transaction` consumes `EquipmentAuthority`.
- `store_checkpoint_in_transaction` replans the complete batch, does full-binding CAS,
  uses a savepoint, and accepts a canonical sparse changed-item subset of the loaded items
  (sparse repair). It refuses a changed item outside the selection and re-reads/compares
  the entire original equipment, item selection and slot snapshot before the first row
  update; a no-op batch performs no updates but still requires the current before state.
- Supplied remaining values are DATA; equipment authority does not justify an elapsed amount.

Expire (`expire_imbuement_in_transaction`): exactly one already-zero retained slot, in a
native `Type2Transaction` with recovery/root/node/session/lease/World/Channel/Content/
equipment fences; compares every retained binding field; savepoint for receipt, real
DELETE witness and one type-2 audit. ItemInstance, equipment location/revision and
CharacterRevision unchanged. Exact replay is read-only; changed binding/occurrence/identity
conflicts. Bare deletion, forged witness and an old transaction's receipt reject.

Audit tag: next free `OneItemOperationV1` tag at allocation. History: the draft first
proposed 12, collided with TRADE 12 (and Forge 13), then proposed 14 privately; main's
proto now notes 12 as a root-coordinated Imbue slot. The CP assigns the tag.

Fees (IMBUE_FORGE0 §6 I1 admits Imbue/ImbueClear/ImbueScroll): add item-only fee causes
(no CharacterRevision) to the shared `FeeBurnCause`/`FeeBurnCauseV1` owner as a
wiring-sized extension, not a fork.

## Review dispositions (from the draft)

- Sparse checkpoint (2026-10-16): mixed/no-op batches failed `InvalidBatch`; fixed as above.
- Root `RootPassDeadlineExceeded` timeouts in two cloud PG runs were environment-only;
  later focused run passed 22/22 with unchanged deadlines. No deadline was relaxed.
- Retained-slot gap (2026-10-17): fixed with `binding.slot >= slots` refusal.
- Shared fee API and `TimedExpiry` are not reusable unchanged (see design).

## Remaining work

- Genuine Imbue-eligible elapsed/actor-tick producer (IMBUE-RT-1) and ticking runtime.
- Apply / Clear with audited item-only fees; scrolls and etcher (IMBUE-SCROLL-1).
- Content: 72-entry catalogue, shrines, Basic-capable codec, catalogue/Quest/access readers.
- Effects in combat runtime; `IMBUE_V1` wire, registry, capability.
- Native audited writers, production mutation grants, registration and runtime activation
  (read-only storage preparation is installed by migration 0150).

## Acceptance criteria

- [ ] Apply/Clear/Expire each commit the item-state delta, fee and audit in one transaction or none.
- [ ] Remaining time only decreases, driven by an admitted elapsed producer; restart resumes exactly.
- [ ] All three tiers allowed on every slot-bearing item; Premium subcheck enforced; structural misuse refused typed.
- [ ] Checkpoint store respects bounds (≤9 equipped items / 27 rows) and refuses stale snapshots, including omitted items.
- [ ] Expire deletes only a witnessed zero row; replay read-only; forged/bare deletion rejected.
- [ ] Wire gated by `IMBUE_V1`; older clients receive no imbuement domain.
- [x] Canonical PG storage cases show real database-start markers on PostgreSQL 18.6 (`server_version_num=180006`); native audited writer cases remain pending.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`item_mint_audit.rs`,
      `item_fee_burn.rs`, `character_equipment.rs`, `timed_item.rs` are already large).

## Inseparable bundles

- migration + checkpoint/expiry owners + PG cases (IMBUE-1)
- fee cause + audit message + proto tag + registry rows
- wire + registry + capability + handler + tests (IMBUE-WIRE-1)

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-items --lib imbuement --quiet
cargo test --locked -p oteryn-gameplay-items --lib --quiet
cargo test --locked -p oteryn-gameplay-durability --lib npc_trade::tests --quiet
cargo test --locked -p oteryn-game-server --lib imbuement_postgres -- --test-threads=1 --nocapture
python3 tools/repository/validate_repository_policy.py
python3 tools/agents/validate_governance.py
```

## Reference evidence

- IMBUE_FORGE0 brief §§3,4,6,7,15; DUR-03 one-item transaction contract
- Completion plan §4, §5 (Wave 4), §7; coordination #1622 (D1122, D1160, D1163, D1329)
- Draft #2144 also edited the DUR-03 contract doc and resource limits registry (+114 lines):
  re-derive those rows from the brief, do not copy.

## 2026-10-23 local authoring checkpoint

Status remains **IN_PROGRESS**. Local worktree edits only; no commit, publication,
freeze or review was requested. No runtime activation is claimed.

### Step 0: existing owners and dependency evidence

Read the complete card and ran `git log --oneline origin/main --grep <ID>` for
all dependency IDs, including the three combined wire/use IDs separately.
Confirmed the results against `crates/`, `apps/` and archived task records:

- ITEM-LOOP-1 / ITEM-MOVE-2a: a43c37f12 (#2190), existing item transfer/equipment
  and dispatch. NPC-ECON-1: 5e7a3b771 (#2197), Bank confirmation and bank-backed BUY.
- ITEM-TIMED-1: 6db7da236 (#2155), existing pure timed repair and timed-item host;
  these do not provide Imbue elapsed provenance.
- GOLD-FEE-2: 77197a1f8 (#1781), existing bank fee burn. ATTACK-1b: 64c1bb3dd
  (#1798), real auto-attack runtime. SPEED-1: 7d7d5374e (#1715), native pacing.
- PREMIUM-1: d8b3191b2 (#2110), existing fail-closed PremiumGate.
  NPC-TRADE-1: b4774770a (#2040), existing item-only trade transaction.
- ITEM-SEM-USE: aa3749ae8 (#1790), consumption content;
  USE-WIRE-V1: 9ab7bdbb6 (#1703), existing item wire codecs and dispatch;
  ITEM-MOVE-WIRE-1: a65950fd2 (#1711), equipment capability and handler.
- IMBUE_FORGE0 acceptance: the owning brief is ACCEPTED by ACCEPT-BATCH; the
  abbreviated dependency spelling is not a unique merge-title match.
- Existing content has item slot/family/category evidence and a reference-only
  Two/Three/Ten tier enum, not a Basic/Intricate/Powerful runtime codec. Archived
  authoring/import tasks are content evidence, not an installed Imbue writer.
- No `game_item_imbuements` table, runtime Imbue operation, Imbue audit codec,
  IMBUE_V1 dispatch handler or eligible elapsed/actor-tick producer exists on
  this main snapshot. EQUIP-RT-1 (#2307) explicitly leaves IMBUE-RT-1 pending.
- Shared fee causes are now CharmUnassign plus promotion/blessing purchases;
  the Imbue item-only causes remain genuinely unmerged. Apply/Clear stay blocked.

### This pass

- Added the L3 reference rules table for the three actual tiers, prices,
  Premium requirements, deterministic success, twenty-hour duration, maximum
  three slots and clear price. No per-item tier cap was introduced.
- Added migration 0150 (main's highest migration was 0144; owner floor 0150 (0149 is BED-1c))
  for the one item/slot state table: category uniqueness, immutable application
  binding, twenty-hour initialization, downward-only remaining time, no bare
  deletion including at zero, and no truncation. Runtime/control grants are
  SELECT only. No fee, checkpoint, expiry or clear permission is inferred from
  possessing this data. Native audited writer/receipt guards must precede write grants.
- Extended the existing NPC trade default-state predicate to exclude retained
  Imbue rows, including zero-time rows, rather than adding another trade owner.
- Auto-mounted PostgreSQL cases use the normal migrator and native MINT/TRANSFER
  with no disabled constraints. They cover storage rejection, retained occupancy,
  move/reload preservation, native NPC sale rejection even after every slot
  reaches zero, and the runtime's read-only grants.
- `crates/test-support` has no property module on this main snapshot; property
  test integration is skipped under the task's explicit condition.
- `MECHANICS_STATUS.md` does not exist on main; no new status index was created.

### Unfinished

- Apply/Clear: shared item-only fee causes, material burn, exact revision-bound
  receipts, native type-2 audit and real dispatch composition are still required.
- Checkpoint/Expire: equipment-bound complete-snapshot CAS, elapsed provenance,
  native audited deletion witness and retry/replay remain unimplemented. No
  runtime write grants exist and the migration refuses deletion until this lands.
- Executable slot/category/access planners and public L3/L4 APIs require their
  production composition and real-dispatch tests. The continuation explicitly
  allows this dormant storage slice while those dependencies remain blocked;
  no owner clarification is pending and no uncalled public functions were added.
- IMBUE-RT-1 remains blocked on an actual eligible elapsed producer; no decay
  timer, wall-clock substitute or generic actor-tick authority was invented.
- Content catalogue, shrine/quest readers, scrolls, effects and IMBUE_V1 remain
  later work. Full mechanic acceptance remains incomplete; the PostgreSQL storage
  evidence above qualifies only this dormant slice.

## Reference parity

Prices are reference evidence (`PARITY_PENDING`), not
runtime content activation. Each row below is a table-driven row in
`imbuement::tests::imbuement_reference_parity` in `oteryn-gameplay-items`.

| Rule | Value | Source | Asserting test row |
|---|---|---|---|
| Basic | 5,000 gold; no Premium; 100% success | [Canary base 1](https://github.com/opentibiabr/canary/blob/04b83b51/data/XML/imbuements.xml); success per accepted IMBUE_FORGE0 §3/R2 and [onApplyImbuement](https://github.com/opentibiabr/canary/blob/04b83b51/src/creatures/players/player.cpp) | `Rule::Tier(Basic, 5000, false, 100)` |
| Intricate | 30,000 gold; Premium; 100% success | Same Canary base 2 and handler; manual characters §5.1.8; accepted R2 | `Rule::Tier(Intricate, 30000, true, 100)` |
| Powerful | 200,000 gold; Premium; 100% success | Same Canary base 3 and handler; manual characters §5.1.8; accepted R2 | `Rule::Tier(Powerful, 200000, true, 100)` |
| Duration | 20 hours = 72,000,000 ms of eligible time | [Canary duration=72000](https://github.com/opentibiabr/canary/blob/04b83b51/data/XML/imbuements.xml); `docs/reference/tibia-manual/characters.md:124` | `Rule::Duration(20 * 60 * 60 * 1000)` |
| Slots per item | At most three; indices 0..2 | `docs/reference/tibia-manual/characters.md:118`; accepted IMBUE_FORGE0 §3/D1163 | `Rule::Slots(3)` |
| Clear | 15,000 gold | [Canary removecost](https://github.com/opentibiabr/canary/blob/04b83b51/data/XML/imbuements.xml) | `Rule::Clear(15000)` |
| Checkpoint interval (accepted accounting rule) | 60,000 ms of eligible time | Accepted IMBUE_FORGE0 §4.3/R1, not an original-game wall-clock guarantee | `Rule::Checkpoint(60 * 1000)` |

Canary's unused XML success percentages are deliberately not runtime failure
chances: the accepted R2 ruling requires deterministic success without protection.

## 2026-10-23 continuation

The previous run's PostgreSQL qualification was blocked by the cargo wrapper's
disk guard. The repaired queued wrapper now runs the mechanic against real
PostgreSQL. No mechanic failure was found in the initial two-case run.

Strengthened the existing cases to compare every stored field across native
TRANSFER and a fresh connection, reject native NPC sales with both active and
entirely zero-time retained slots, and assert that rejection preserves item state,
CharacterRevision, audit/transfer footprint and the absence of trade receipts.
Both runtime and control roles are checked for read access and denial of
INSERT/UPDATE/DELETE/TRUNCATE. No public API, command or capability was added.

The permitted deliverable remains a **dormant backend/storage preparation** with
reference rule data, not an audited Apply/Clear/Expire implementation. The
unmerged fee/composition and elapsed-producer dependencies above remain real on
`origin/main ac3f7fbc6`; existing owners were reused. `MECHANICS_STATUS.md` is
absent from both this worktree and main, so there is no existing row to update.
The seven-row Reference parity table remains matched by its L3 test.

### Continuation validation evidence

All cargo commands use `/home/mole/oteryn-cache/codex-bin/cargo` and the pinned
1.99.1 toolchain. Only this mechanic's PostgreSQL filter was run.

- `cargo test --locked -p oteryn-gameplay-items --lib --quiet`: **36 passed**, including the seven-row parity test.
- `cargo test --locked -p oteryn-gameplay-items --lib imbuement --quiet`: **1 passed**, 35 filtered out; all seven Reference parity rows asserted.
- `cargo test --locked -p oteryn-gameplay-durability --lib npc_trade::tests --quiet`: **3 passed**, 389 filtered out; pure unit tests.
- `cargo test --locked -p oteryn-game-server --lib imbuement_postgres -- --test-threads=1 --nocapture`: **2 passed**, 415 filtered out, no ignored tests; both database-start markers record **180006 (PostgreSQL 18.6)**. The strengthened cases ran against separate freshly migrated databases.
- `cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings`: **PASS**. Existing vendored Tokio/SQLx dependency warnings remain; touched crates have no clippy errors.
- `cargo fmt --all -- --check`: **PASS**. Auto-mounted source also formatted and checked explicitly with `rustfmt --edition 2024 --check apps/game-server/src/lifted_tests/auto/imbuement_postgres_cases.rs`, since cargo fmt does not discover build-generated mounts.
- `python3 tools/repository/validate_repository_policy.py`: **PASS**.
- `python3 tools/agents/validate_governance.py`: **PASS**; `python3 -m unittest discover -s tools/agents/tests`: **124 passed**.
- Re-read the entire main-to-worktree diff against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and searched all `apps/`, `crates/`, `tools/` sites of the introduced tier/rule types. All constructors/matches are accounted for; the continuation adds no enum variants, fields, trait methods or public functions. `git diff --check`: **PASS**. No tests were weakened or removed. No load-contention failures or CI-only test deferrals occurred.
