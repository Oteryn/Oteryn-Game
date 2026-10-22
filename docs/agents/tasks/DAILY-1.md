# DAILY-1 — daily reward (consolidated task card)

```yaml
task_id: DAILY-1
title: "DAILY-1: daily reward"
mode: IMPLEMENT
status: NOT_STARTED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row DAILY-1 (row 47); §7 'DAILY-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2151, branch skeleton/daily-1

Status note: the draft qualified a bounded component (claim arithmetic + an uninstalled
storage schema candidate with read/lock APIs, 11/11 tests incl. 8 PG on PG 17.6) on the
pre-ADR-0022 `apps/game-server/src/...` layout. No claim path was enabled. It is closing;
the mechanic is re-implemented fresh on the current layout.

## Goal

Reward state, joker, claim and line tables; claim transaction; item binding table; line
delivery to Inbox and wildcard balance (DAILY-1) with content, bind refusals and
`DAILY_REWARD_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):
DAILY-CONTENT-1, DAILY-1 (hard), DAILY-BIND-1, DAILY-WIRE-1 (Reg).

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| DAILY_REWARD0 acceptance | ACCEPTED (#2209) |
| NPC-ECON-1 | MERGED (#2197) |
| Reward Inbox bridge (MAIL-1, D1318 same-TX seam) | MERGED (#2133) — `prepare/commit_reward_claim_inbox_mint` |
| PREMIUM-1 | MERGED (#2110) |
| WO-1 | MERGED (#1748) |
| Committed World-reset month history (MAP-OVERLAY-1c, D1317) | BLOCKING (draft #2167 open, not on main) |
| HUNT-1 / PREY wildcard sink (D1319) | BLOCKING — re-implementation pending (draft #2149 closing) |
| PREM-3 (Free lane until then) | BLOCKING for Premium lane (no record on main) |
| EXERCISE-CONTENT-1, RUNE-USE content | verify at allocation (RUNE-USE-0 accepted) |
| WORLDINT-USE-1 (DAILY-WIRE-1) | verify at allocation (WORLDINT-WIRE-1 archived) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/durability/src/durability/character_inbox/reward_claim/mint.rs` —
  `prepare_reward_claim_inbox_mint` / `commit_reward_claim_inbox_mint` (Inbox line delivery).
- `crates/gameplay/durability/src/durability/item_transfer.rs` —
  `character_item_fence_is_current` (current session/lease/scope; composition rule 2).
- `crates/gameplay/durability/src/durability/` mod registration (wiring only).
- `crates/protocol-oteryn/src/lib.rs` — `pub mod` + registry constants (wiring only).
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`, `capabilities.rs`
  (`GATED`: `DAILY_REWARD_V1`) — wiring only.
- `apps/game-server/tests/durability_postgres.rs` — registered PG target that executes
  durability-mounted PG cases once in CI.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/progression/src/daily_reward.rs` (new) + `daily_reward_tests.rs` (new)
  — L3 lane/streak/joker arithmetic (draft: `domain/daily_reward.rs`).
- `crates/gameplay/durability/src/durability/daily_reward.rs` (new) — L4 storage, readers,
  line locks.
- `crates/gameplay/durability/src/durability/daily_reward_claim.rs` (new) — L4 claim writer.
- `crates/gameplay/durability/src/durability/daily_reward_tests.rs` (new) — PG cases.
- `crates/protocol-oteryn/src/daily_reward.rs` (new) + `daily_reward_tests.rs` (new) — wire.
- `crates/gameplay/dispatch/src/gameplay_transport/daily_reward.rs` (new) — handler only.
- `apps/game-server/tests/daily_reward_postgres.rs` (new) — e2e target.
- Content inputs: shrine bindings, rune/potion lists, training weapons, scrolls.
- `apps/game-server/migrations/<next>_daily_reward.sql` (new) — migration number: next free
  number at allocation time (only one migration PR in the queue).

## Accepted design (DAILY_REWARD0 §§3–7 and CP decisions, as built in the draft)

Arithmetic (pure data; neither claims an epoch nor grants/spends a joker):

- Next-day lane wrap 7→1; missed epochs do not advance the lane.
- All-or-nothing joker coverage of a gap; first streak 1; streak ceiling 100,000.
- Equal/older epochs refused; full u64 gap arithmetic without overflow.
- D1317: Berlin 10:00 reset (D1257) supersedes historical UTC wording; epoch 0 does not count;
  a joker grant month requires a committed reset newer than the previous grant.

Storage schema (uninstalled candidate in the draft):

- Character+World reward state; Account+World jokers with immutable joker entries;
  immutable epoch-keyed claims; ordered reward lines. No Channel scope.
- Reuses native Character-root RESTRICT references and `TypedDefinitionRef`. No replacement
  epoch table, wildcard balance, reward definitions or content facts.
- Joker ledger (BANK-0 §3 pattern): deferred commit guard checks stored balance/cursor ==
  latest entry, exact Account+World predecessor (or zero opening), GRANT/SPEND arithmetic;
  re-reads final transaction state; O(1) rows per check; UPDATE/DELETE/TRUNCATE rejected;
  schema-qualified to defeat same-named temp tables.
- Receipts: stored transaction identity survives reload/replay; differing SHA-256 request
  binding → `BindingMismatch`, never a new receipt.
- Pending lines lock in line order `FOR UPDATE` (no `SKIP LOCKED`); held through sink+status
  commit. Runtime claims/ledger SELECT+INSERT only; lines UPDATE only status/delivering
  transaction/excess; terminal lines reject downgrade and receipt rewrite.
- Requested quantity positive; a fully capped wildcard credit is DELIVERED with
  `uncredited_excess == quantity`; pending lines carry no transaction/excess.

Integration obligations (mandatory):

1. Current epoch from the World-reset owner; restart/session-generation changes cannot invent
   a window. Lazy joker grants from committed reset-month history (#2167 port).
2. Claim writer: native admission/recovery locks, current session/lease/scope, then Character
   root and Account+World joker locks. Shrine reach, Account/World ownership, admitted content
   and picks checked independently. Freeze canonical picks/planned-output SHA-256 binding before
   the first attempt.
3. One transaction: state, jokers, entries, claim, complete line set and every available sink,
   including refusal rollback. Create every line with its claim; never append on replay.
   Per-claim item output bound 20; definition/vocation/count rules; atomic item binding rows.
4. Typed seams: Inbox delivery, closed `DailyRewardCause`, canonical mint/ledger/outbox, item
   binding lifecycle, PREY Account wildcard writer. XP stays pending until its accepted sink.
   PREY_WILDCARD lines remain PENDING until the HUNT sink exists. Joker UTC/lane parity:
   PARITY_PENDING.
5. PG cases follow the repository convention (no `#[sqlx::test]`): without
   `OTERYN_TEST_POSTGRES_ADMIN_URL` report PRE-ROUTING/NONCANONICAL and return; routed runs
   create an isolated DB, apply native migrations, run, drop.

## Review dispositions (from the draft)

- Gap-review P2: PG cases never ran in CI → mounted via durability so `durability_postgres`
  runs them once.
- SQLSTATE test defect: second native transaction checks raw 55P03 before error mapping.

## Remaining work

Schema migration; claim writer on the MAIL-1 Inbox bridge; lazy joker grants; item binding
table and §6 refusals (DAILY-BIND-1); PREY wildcard sink; content (DAILY-CONTENT-1);
wire/protocol (DAILY-WIRE-1); `tests/daily_reward_postgres.rs`.

## Acceptance criteria

- [ ] One claim per reset epoch; streak and joker rules per §3.
- [ ] Bound items refused at every §6 site.
- [ ] Claim idempotent across reconnect.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- content + regeneration + pins by one author
- migration + claim writer + durability
- claim = line MINT into Inbox + wildcard credit + ledger in one transaction
- wire + registry + capability + handler + tests

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability daily
cargo test --locked -p oteryn-game-server --test durability_postgres daily -- --test-threads=1  # PG 17.6
cargo test --locked -p protocol-oteryn daily_reward
python3 tools/content-migration/regenerate_content.py --check   # content slot only
python3 tools/agents/validate_governance.py
```

## Reference evidence

- DAILY_REWARD0 §3–§7; BANK-0 §3 ledger pattern
- Completion plan §4, §5 (Wave 4), §7; coordination #2131 (D1257, D1317, D1318, D1319, D1329)
