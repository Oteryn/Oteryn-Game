# HUNT-1 — prey and hunting tasks (consolidated task card)

```yaml
task_id: HUNT-1
title: "HUNT-1: prey and hunting tasks"
mode: IMPLEMENT
status: NOT_STARTED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row HUNT-1 (rows 45, 46); §7 'HUNT-1, RANK-1' PREY0 rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2149, branch skeleton/hunt-1

Status note: the draft qualified three bounded pieces on the pre-ADR-0022
`apps/game-server/src/...` layout: (1) pure nonrandom PREY slot/option/debit rules
(11 tests); (2) the PREY-CONTENT-1 loot-budget compiler leaf (21 Python tests);
(3) the pending DAILY wildcard sink (`deliver_pending_daily_wildcard`) with its audit
event codec — 5 PG + 4 prost cases passed in one combined HUNT+DAILY assembly (D1406),
never on installed main. No migration, registry, wiring or runtime caller was installed.
It is closing; the mechanic is re-implemented fresh on the current layout.

## Goal

Prey slots, resource and wildcard balances, ledger, draws, hunting-time checkpoints
(PREY-1); gold reroll fee; prey effects; Task Board, delivery, shop; `PREY_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):
PREY-CONTENT-1, PREY-1 (hard), PREY-FEE-1, PREY-EFFECT-1 (combat), TASKBOARD-1 (hard),
TASKBOARD-DELIVERY-1, TASKSHOP-1, PREY-WIRE-1 (Reg).

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| PREY0 acceptance | ACCEPTED (#2209) |
| PREMIUM-1 | MERGED (#2110) |
| ITEM-USE-1c (TASKBOARD-DELIVERY-1) | MERGED (#2082) |
| SPELL-TARGET-1 / ATTACK-0 | MERGED (#1787) / ACCEPTED |
| CHAR-REV-SEQ-1, GOLD-FEE-1b | MERGED (verify exact surface at allocation) |
| LOOP-MONSTER-1 (family entry gate; kill caller) | BLOCKING — re-implementation pending (draft #2200 closing) |
| DAILY-1 claim/line schema (wildcard sink source) | BLOCKING — re-implementation pending (draft #2151 closing) |
| MAP-OVERLAY-1c reset history (weekly reset) | BLOCKING (draft #2167 closing) |
| PREY-CONTENT-1 admitted tables (direct PREY-1 dependency) | BLOCKING: `rulesets/progression/prey/index.json` is `READY_UNPOPULATED`; no pool, level bands, grade/draw tables |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned-path code may be
drafted earlier but must not assume unmerged schemas.

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/domain/src/domain/premium.rs` — native `PremiumGate` (no boolean override).
- `crates/gameplay/durability/src/durability/premium_activation_latch.rs`,
  `crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator.rs` —
  existing `with_premium_gate` seam; production Prey calls stay inside it.
- `crates/gameplay/durability/src/durability/character_revision_sequencer.rs` — Character
  sequencer / one CharacterRevision advance per write.
- `crates/gameplay/durability/src/durability/character_authority.rs` — add only the PREY
  receipt UNION arm (inseparable from the Prey migration).
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs` — death-chain kill receipt
  for task kills (wiring point).
- `crates/gameplay/durability/src/durability/bestiary_progress.rs`,
  `crates/gameplay/progression/src/bestiary.rs` — sibling kill-progress owner pattern.
- `tools/content-schema/native-gameplay/build_full_spell_artifact.py` — native loot producer
  (accepts the PREY provider; ordinary compilation unchanged).
- `rulesets/progression/prey/index.json` — authoring directory (`READY_UNPOPULATED`).
- `apps/game-server/migrations/0093_character_appearance.sql` — current effective progression
  guard body; a new guard must extend it with every prior arm retained.
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` — event types 1–3 on main.
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`, `capabilities.rs`
  (`GATED`: `PREY_V1`), `crates/protocol-oteryn/src/lib.rs` — wiring only.
- `apps/game-server/tests/durability_postgres.rs` — registered PG target for
  durability-mounted cases.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/progression/src/prey.rs` (new) + `prey_tests.rs` (new) — L3 slot/option/
  debit rules (draft: `domain/prey.rs`).
- `crates/gameplay/progression/src/task_board.rs` (new) + `task_board_tests.rs` (new) — L3.
- `crates/gameplay/durability/src/durability/prey.rs` (new) — L4 balances/ledger/receipts and
  the pending DAILY wildcard sink.
- `crates/gameplay/durability/src/durability/prey_audit.rs` (new) — wildcard event codec.
- `crates/gameplay/durability/src/durability/prey_tests.rs` (new) — PG cases.
- `crates/gameplay/durability/src/durability/task_board.rs` (new).
- `crates/protocol-oteryn/src/prey.rs` (new) + `prey_tests.rs` (new) — wire.
- `crates/gameplay/dispatch/src/gameplay_transport/prey.rs` (new) — handler only.
- `docs/contracts/game-events/v1/prey_wildcard.proto` (new) — closed payload.
- `tools/content-schema/native-gameplay/prey_loot_budget.py` (new),
  `test_prey_loot_budget.py` (new), `prey-loot-provider.schema.json` (new).
- `rulesets/progression/task-board/` (new) authoring inputs.
- `apps/game-server/tests/prey_postgres.rs` (new) — focused e2e target.
- `apps/game-server/migrations/<next>_prey.sql` (new) — migration number: next free number at
  allocation time (only one migration PR in the queue). Event type number likewise next free
  at allocation (draft's D1341 "6" is void).

## Accepted design (PREY0 §§3–12 and CP decisions, as built in the draft)

Nonrandom slot rules (pure; no draws, no debit side effects):

- Validate slot 1–3 first. A valid current build needs a vocation other than `none`.
  Slot 1 needs no Premium; slot 2 needs the current native Premium gate or its permanent
  Account+World fact; slot 3 always needs its permanent fact. No Store fact is minted.
- An Account+World snapshot never lends balances/facts across Account or World; an
  initialized slot projection binds the exact Character and slot. An absent slot stays
  `SLOT_LOCKED`; commands never initialize a first list; locked access never clears state.
- `set_option`: ≥1 wildcard for Auto Reroll, ≥5 for Lock, none to clear; returns a
  before/after plan, never a debit or reservation.
- Debit helper: closed spend kinds, cost 1 or 5, checked subtraction, balance 0–52
  (§5.2/§12). No arbitrary amount, credit, Store delivery, paid fee or test-grant API.

Content (PREY-CONTENT-1, PREY0 §3, §6.1, `PREY0-RL-16`):

- Closed authored PREY provider with the exact Creature pool and SHA-256 pins of emitted
  Creature-profile and Loot-section bytes; refuse unknown eligibility, missing canonical
  definitions, stale pins, incomplete pools, unsupported shapes, pool < 27 Creatures and any
  preyable table > 7 entries. Canonical Loot discriminator `kind: "Loot"` accepted.
  Budget: 29 RNG draws, 14 output entries. Source flags alone never admit content.
- Draws, grades, level bands, RNG purpose/binding and activation remain to be authored and
  admitted; no inferred pool, grade, fee or random-unit conversion.

Pending DAILY wildcard sink (D1319/D1336/D1338/D1341):

- `deliver_pending_daily_wildcard` consumes a retained DAILY claim + `PREY_WILDCARD` line;
  request carries claim/line and planned delivery/event identities only — never quantity,
  Account, World or credit. Validates current recovery/node/session/lease/scope/
  CharacterRevision in the writing TX, then locks source and Account+World balance.
- Credits `min(quantity, 52 - balance)`, records exact excess. Receipt, ledger entry,
  balance head, audit event, terminal DAILY line and one revision advance commit together.
  Zero credit at cap still terminalizes with receipt/revision but no ledger entry/event.
- Once key `(claim transaction, line)`; replay returns the winner's identities/result and
  never re-acquires authority. Binding excludes retry cursor and newly planned identities.
- Event `PREY_WILDCARD_CHANGED` schema 1, positive `DAILY_REWARD` cause only; retention
  `PREY_WILDCARD_LEDGER_RETENTION_V1` rev 1, restricted player-linked class, 2,592,000 s from
  the trusted event timestamp. Only envelope/payload expire; balances, ledger, unlock facts,
  receipts never do. Holds cannot rewrite the retention clock. `ECONOMY_LEDGER` unchanged.
- Resource bounds (CANDIDATE until natively tested): 191 payload bytes, 500 envelope bytes,
  64 events per expiry call.

Integration obligations (mandatory):

1. Load current live Character→Account+World binding, current build and initialized
   content-compatible slot under the owning locks; read Premium at use; lapsed/unavailable/
   conflicting status stays closed. Missing balance row = zero; failed read ≠ zero.
2. PREY0 §10 lock order: occurrence; Character root/revision; prey/task rows; Account
   wildcard/unlock rows; item/bank rows. Recompute requirements on the locked snapshot.
3. Real debit composes one SPEND ledger entry, event/outbox row and Character receipt.
4. World and Channel remain distinct; Account facts are never copied to Character balances.
5. PG cases follow repository convention (no `#[sqlx::test]`; PRE-ROUTING without admin URL).

## Review dispositions (from the draft)

- CP minor (6216371834): cross-layer `#[path]` test include removed; pure tests belong with
  the rules module (here: own child test file in progression).
- Canonical Loot shape check rejected valid `kind: "Loot"` → fixed with regression.
- Clippy failures in retained attempts: duplicate canonical harness load, unnecessary `vec!`,
  harness module absent from other aggregate roots → PG cases must live in one support
  location consumed once per aggregate.
- Inherited content drift: checked-in Loot section differs in 86 Item `materializable` flags
  (pre-existing; not this task's regeneration).

## Remaining work

Migration + Prey writers + authority UNION; sequencer adapter + retry/replay-cursor case;
PREY-CONTENT-1 production tables; draws/selection/active bonuses/hunting checkpoints/expiry;
PREY-FEE-1; PREY-EFFECT-1; Task Board, delivery, shop; `PREY_V1` wire and registries;
runtime activation; expired-deletion, expired-active-hold and operator access/export
qualification.

## Acceptance criteria

- [ ] Weekly reset settles the finished week under one barrier (TASKBOARD obligation a).
- [ ] Replay-stable overflow policy at `PREY0-RL-14`.
- [ ] Hunting-time checkpoints survive restart.
- [ ] Wildcard balance never exceeds 52; excess recorded exactly; DAILY wildcard delivery once per `(claim, line)`.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- content + regeneration + pins by one author
- migration + prey/task writers + durability + character_authority UNION
- reroll fee burn + ledger; task shop debit + item MINT + ledger
- wildcard event payload + registry row + retention profile
- wire + registry + capability + handler + tests

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability prey
cargo test --locked -p oteryn-game-server --test durability_postgres prey -- --test-threads=1  # PG 17.6
cargo test --locked -p protocol-oteryn prey
(cd tools/content-schema/native-gameplay && python -m unittest test_prey_loot_budget)
python3 tools/content-migration/regenerate_content.py --check   # content slot only
python3 tools/agents/validate_governance.py
```

## Reference evidence

- PREY0 §3–§12; DAILY_REWARD0 §4.5; BANK-0 §3 ledger pattern
- Completion plan §4, §5 (Wave 4), §7; coordination #2131 (D1151, D1319, D1336, D1338,
  D1341, D1392, D1398, D1406)
