# ITEM-IMBUE-1 — imbuing (consolidated task card)

```yaml
task_id: ITEM-IMBUE-1
title: "ITEM-IMBUE-1: imbuing"
mode: IMPLEMENT
status: NOT_STARTED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row ITEM-IMBUE-1 (row 72); §7 'ITEM-TIMED-1' IMBUE-* rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
base_branch: main
branch: null
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

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | MERGED (#2190) |
| ITEM-MOVE-2a | MERGED (equip slice EQUIP-WIRE-1 #2009; remainder in ITEM-LOOP-1 #2190) |
| NPC-ECON-1 (per child: Bank confirmation, bank-backed BUY) | MERGED (#2197) |
| ITEM-TIMED-1 timed repair planner | MERGED (#2155) — does NOT provide Imbue elapsed provenance |
| GOLD-FEE-2 | MERGED (#1781) |
| ATTACK-1b | MERGED (#1798) |
| SPEED-1 | MERGED (per draft card; re-check at allocation) |
| PREMIUM-1 | MERGED (#2110) |
| NPC-TRADE-1 | MERGED (#2040) |
| ITEM-SEM-USE / USE-WIRE-V1 / ITEM-MOVE-WIRE-1 | MERGED (archive records) |
| IMBUE_FORGE0 acceptance | ACCEPTED (IMBUE-FORGE-0 #1415 + protected ACCEPT-BATCH) |
| Imbue-eligible elapsed / actor-tick producer | BLOCKING for IMBUE-RT-1 (no owner on main) |
| Item-only fee causes Imbue/ImbueClear/ImbueScroll in shared fee API | BLOCKING for Apply/Clear (see below) |

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
- `apps/game-server/tests/character_authority_postgres.rs` + `apps/game-server/tests/support/`
  — canonical PG case mount.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/items/src/imbuement.rs` (new) — L3 slot/tier model, `SlotRecord`,
  `check_target_slot`, access subchecks (draft: `domain/imbuement.rs`).
- `crates/gameplay/items/src/imbuement_checkpoint.rs` (new) — L3 pure `CheckpointBatch` planner.
- `crates/gameplay/items/src/imbuement_tests.rs` (new) — pure cases.
- `crates/gameplay/durability/src/durability/imbuement.rs` (new) — L4 checkpoint load/store.
- `crates/gameplay/durability/src/durability/imbuement_expiry.rs` (new) — L4 Expire compositor.
- `crates/gameplay/durability/src/durability/imbuement_expiry_audit.rs` (new) +
  `imbuement_expiry_audit_tests.rs` (new) — type-2 audit message.
- `apps/game-server/tests/support/imbuement_postgres_cases.rs` (new),
  `imbuement_expiry_postgres_cases.rs` (new) — PG cases, mounted wiring-only.
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
- Numbered migration, production grants, registration and runtime activation.

## Acceptance criteria

- [ ] Apply/Clear/Expire each commit the item-state delta, fee and audit in one transaction or none.
- [ ] Remaining time only decreases, driven by an admitted elapsed producer; restart resumes exactly.
- [ ] All three tiers allowed on every slot-bearing item; Premium subcheck enforced; structural misuse refused typed.
- [ ] Checkpoint store respects bounds (≤9 equipped items / 27 rows) and refuses stale snapshots, including omitted items.
- [ ] Expire deletes only a witnessed zero row; replay read-only; forged/bare deletion rejected.
- [ ] Wire gated by `IMBUE_V1`; older clients receive no imbuement domain.
- [ ] Canonical PG cases show real database-start markers (server_version_num 170006).
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
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability imbuement
cargo test --locked -p oteryn-game-server --test character_authority_postgres imbuement -- --test-threads=1  # PG 17.6 admin URL
python3 tools/agents/validate_governance.py
```

## Reference evidence

- IMBUE_FORGE0 brief §§3,4,6,7,15; DUR-03 one-item transaction contract
- Completion plan §4, §5 (Wave 4), §7; coordination #1622 (D1122, D1160, D1163, D1329)
- Draft #2144 also edited the DUR-03 contract doc and resource limits registry (+114 lines):
  re-derive those rows from the brief, do not copy.
