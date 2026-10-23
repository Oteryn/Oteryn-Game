# HUNT-1 — prey and hunting tasks (consolidated task card)

```yaml
task_id: HUNT-1
title: "HUNT-1: prey and hunting tasks"
mode: IMPLEMENT
status: waiting   # HUNT-1 Prey slot slice merged (#2335); remaining slices per card
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row HUNT-1 (rows 45, 46); §7 'HUNT-1, RANK-1' PREY0 rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md
base_branch: main
branch: claude/happy-davinci-ijz3wy-hunt-1
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: #2335 (Prey slot state, migration `0152_prey_slot_state.sql`). Content tables and the PREY_V1 runtime are not on main yet. Evidence: `crates/protocol-oteryn/src/capability_tables.rs`.

Remaining: Prey storage/draws/checkpoints/fees/effects, Task Board/delivery/shop and wire remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

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

## Dependencies (re-verified on origin/main 65e40c57f, 2026-10-23)

Every row was checked with `git log --oneline origin/main --grep <ID>` and a
workspace code/schema search before authoring. Task-card status alone was not used.

| Dependency | Real state and evidence |
|---|---|
| PREY0 acceptance | ACCEPTED (#2209); accepted PREY0 decision on main, Account+World amendment #1426 |
| PREMIUM-1 | MERGED (#2110, `d8b3191b2`); native `domain/premium.rs` gate and durable activation latch |
| ITEM-USE-1c (TASKBOARD-DELIVERY-1) | MERGED (#2082, `385c01b26`); native dispatch/item-use owner exists |
| SPELL-TARGET-1 / ATTACK-0 | MERGED spell target #1787 (`af219641e`), #1808 (`5a9b595e9`), ATTACK-0 #1347 (`2198d18e1`); existing kill/XP dispatch exercised by PARTY-XP-1 |
| CHAR-REV-SEQ-1, GOLD-FEE-1b | MERGED; `character_revision_sequencer.rs` and `item_fee_burn.rs`; multi-denomination fee #1410 (`17a917ed0`) |
| LOOP-MONSTER-1 | MERGED (#2323, `778a4b1bc`); World respawn/monster-think runtime on main |
| DAILY-1 claim/line schema | PARTIAL (#2309, `37bcdf527`): migration 0140 and fenced claim core exist; retained PREY_WILDCARD reward lines/sink remain absent and BLOCKING for wildcard delivery |
| MAP-OVERLAY-1c reset history | PARTIAL: migration 0140 now owns `game_world_reset_history`; reset publisher/barrier and weekly settlement remain BLOCKING (no completed MAP-OVERLAY-1c implementation) |
| PREY-CONTENT-1 admitted tables | BLOCKING: #2293 staged reference evidence only; `rulesets/progression/prey/index.json` remains `READY_UNPOPULATED`, with no admitted native pool/grade/draw tables |

This owner-authorized pass implements pure rules and extends existing storage only.
It does not assume the blocked content or reward-line APIs and does not activate a
slot writer. Task Board depends on PREY-1 and remains unqualified for this pass.

## Existing implementation found before authoring

- Migration `0141_party_xp_hunting.sql` already owns
  `game_character_prey_slots` and `game_account_prey_unlocks`; reuse, no parallel tables.
- `durability/hunting_xp.rs` reads active XP prey, checks the Account+World permanent
  entitlement and its claiming Character, and feeds `party_kill_xp.rs`.
- `dispatch/.../postgres_tests/party_xp.rs` already proves prey then stamina through
  real combat/kill dispatch. `characters/hunting_experience.rs` owns XP arithmetic.
- Content Creature flags (`prey`, `exclusive_prey`) and the gated `PREY_V1` capability
  already exist. No slot command, writer, initialization or draw owner is installed.
- Archived PREY0 and PREY-CONTENT-1 records are acceptance/reference evidence,
  not runtime admission. `MECHANICS_STATUS.md` and a test-support `property` module
  do not exist on this main; neither is created for this pass.

## Delivered in this bounded pass

- L3 `progression/prey.rs`: native Premium/permanent slot access, invalid-slot-first
  refusal, four bonus types and all ten exact grade values, 20-hour free reroll
  planning, level × 200 gold paid quote preserving the free timer, closed wildcard
  costs (1/5), option checks without debit, balance cap 52 and checked subtraction.
  Quotes reject timestamps outside the durable nonnegative BIGINT range and overflow.
- L4 `durability/prey.rs` + `prey/decode.rs`: validate retained prey state on the
  existing real XP consumer; no new public writer or alternate authority.
- Migration **0152** exactly: extend existing Character slots with bounded distinct
  Creature lists, option and Unix-second free deadline; exact grade/bonus CHECK.
  Existing roles retain reads; all slot/unlock writes remain closed pending the
  content + fenced writer + receipt/revision bundle. No partial write grants.
- Auto-mounted `prey_postgres_cases.rs`: schema round trip through a new connection,
  legacy defaults, rollback, role grants, invalid lists/options/times/slots and every
  type/grade combination. The existing real kill-dispatch prey test qualifies the
  production decoder. No duplicate test mount or new PG target.
- Resource entries **0640–0645**, using the accepted PREY0 limit IDs.
- The production decoder is reached through the existing kill/XP dispatch. Pure
  access/cost plans await the content-gated command writer. All new types, enum
  variants, fields and conversion implementations were swept across `apps/`,
  `crates/`, `tools/`; no external construction/match sites needed updates.

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
- `apps/game-server/src/lifted_tests/auto/prey_postgres_cases.rs` — sole mount for new PG cases (owner routing supersedes the historical target proposal).
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

Prey writers + receipts + authority UNION + matching progression guard; sequencer adapter + retry/replay-cursor case;
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

## Reference parity

Reference-only Canary pin: [52550d5e](https://github.com/opentibiabr/canary/tree/52550d5e3c0ffe565f4bd1014f4e5575abf1059b).
Values were read from the pinned sources over read-only HTTP; no source code/assets
are copied. Every table row has a table-driven L3 assertion; all forty bonus values
are asserted explicitly, rather than restating the formula as the test oracle.

| Rule | Value | Source | L3 test |
|---|---|---|---|
| Slot access | 3 slots; 1 free, 2 current Premium or permanent, 3 permanent; vocation required; invalid slot first | PREY0 §5.1, [Prey System](https://tibia.fandom.com/wiki/Prey_System); native Premium supersedes Canary bypasses | `prey_reference_slot_unlocks` |
| Damage grade values | grades 1–10: 7,9,11,13,15,17,19,21,23,25% | Canary `src/io/ioprey.cpp`, `reloadBonusValue` | `prey_reference_bonus_grades` Damage row |
| Reduction grade values | 12,14,16,18,20,22,24,26,28,30% | same | `prey_reference_bonus_grades` Reduction row |
| Experience grade values | 13,16,19,22,25,28,31,34,37,40% | same | `prey_reference_bonus_grades` Experience row |
| Improved loot grade values | 13,16,19,22,25,28,31,34,37,40% | same | `prey_reference_bonus_grades` Loot row |
| Free list reroll | now ≥ deadline; next = now + 72,000 seconds | Canary `config.lua.dist:138–150`; `parsePreyAction` | `prey_reference_reroll_timing_and_gold` free boundary rows |
| Paid list reroll | level × 200 gold, free deadline unchanged | same; PREY0 Y1 | `prey_reference_reroll_timing_and_gold` paid rows |
| Bonus reroll | 1 wildcard | Canary `config.lua.dist:148` | `prey_reference_wildcard_costs_and_option_checks` BonusReroll row |
| Select any | 5 wildcards | Canary `config.lua.dist:147` | `prey_reference_wildcard_costs_and_option_checks` SelectAny row |
| Auto reroll option/expiry | ≥1 to select option, 1 spent at expiry; option selection spends nothing | Canary `parsePreyAction`/`checkPlayerPreys` | `prey_reference_wildcard_costs_and_option_checks` AutoReroll rows |
| Lock option/expiry | ≥5 to select option, 5 spent at expiry; clearing needs 0 | same | `prey_reference_wildcard_costs_and_option_checks` Lock/None rows |
| Storage bounds | 3 slots, ≤9 keys × 256 bytes, ≤7,200 hunting seconds, ≤52 wildcards | PREY0 §§4,12; Canary config; [Prey System](https://tibia.fandom.com/wiki/Prey_System) | `prey_reference_storage_bounds_and_closed_codes` |

Grade 1 is valid retained state; fresh grade/RNG draws remain blocked on admitted
content. Quotes/debit plans are pure; no payment, reservation or unlock is performed.
`crates/test-support` has no property module on main, so that optional property
framework is skipped; wildcard conservation/underflow is exhaustively tested over
all 53 admitted balances, and maximal level/timestamp boundaries are tested.

## Validation

Final worktree validation PASS (2026-10-23; queued cargo wrapper via a
non-login shell, preserving the supplied PostgreSQL route):

- `cargo fmt --all -- --check`: PASS; auto-mounted test also formatted directly.
- Clippy `--all-targets -- -D warnings` on progression, durability and game-server:
  PASS. Existing vendored Tokio/sqlx warnings do not fail these crate checks.
- Progression `--lib`: **66 passed**, including all **5** new Prey parity cases.
- Durability `--lib prey`: compiled successfully, **0 matching tests**; the new
  adapter is exercised by the actual dispatch PG case below, not a fake unit DB.
- Game-server `--lib prey_storage_`: **2 passed**, canonical PostgreSQL **18.6**;
  final rerun covers the bounded SQL CASE and fresh physical-connection reload.
- Dispatch `--lib party_xp_real_dispatch_prey_then_stamina`: **1 passed**, real
  combat/kill dispatch. Its pre-existing diagnostic string says 170006; the shared
  Harness enforces `CANONICAL_POSTGRES_SERVER_VERSION_NUM = 180006`.
- Repository policy validator: PASS (23 policy files, 68 workflows).
- Governance validator: PASS; governance unit tests: **124 passed**.
- Whole-diff checklist review and workspace type/constructor/impl sweep: PASS;
  `git diff --check`: PASS. No Git index/publication operation was performed.

Exact commands:

```bash
cargo fmt --all -- --check
cargo clippy --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-progression --lib --quiet
cargo test --locked -p oteryn-gameplay-durability --lib prey --quiet
cargo test --locked -p oteryn-game-server --lib prey_storage_ --quiet -- --test-threads=1 --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --lib party_xp_real_dispatch_prey_then_stamina --quiet -- --test-threads=1 --nocapture
python3 tools/repository/validate_repository_policy.py
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
```

Only mechanic-filtered PostgreSQL cases run locally; full durability/dispatch PG
suites belong to CI. No cargo manifests/lockfile/content were changed.

## Unfinished for the next bounded pass

- Admitted PREY-CONTENT-1 pool, level bands, grade and draw tables remain the direct
  blocker to slot initialization, selection/reroll/option writers and activation.
- Slot writers require the matching Character receipt, authority UNION,
  progression consistency guard, session-generation fence and sequencer adapter
  as one inseparable bundle. This migration admits no gameplay mutation API.
- Gold burn, Account+World wildcard ledger/events and DAILY reward-line delivery
  are not implemented. Reroll prices and wildcard spends here are pure plans.
- Hunting checkpoints, expiry and remaining bonus effects, Task Board, delivery,
  shop and PREY_V1 command/state wiring remain outside this pass. Task Board is
  not yet qualified; weekly reset publication/barrier also remains blocked.
- No `crates/test-support::property` module exists on main: optional property
  framework tests were skipped, with exhaustive bounded arithmetic tests instead.
- No unrelated load-contention test failures or CI-only deferrals occurred.

## Reference evidence

- PREY0 §3–§12; DAILY_REWARD0 §4.5; BANK-0 §3 ledger pattern
- Completion plan §4, §5 (Wave 4), §7; coordination #2131 (D1151, D1319, D1336, D1338,
  D1341, D1392, D1398, D1406)
