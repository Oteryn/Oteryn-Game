# GUILD-BANK-1 — guild balance, ledger and disband payout (consolidated task card)

```yaml
task_id: GUILD-BANK-1
title: "GUILD-BANK-1: guild balance, ledger and disband payout"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
class: "hard (persistence, economy review)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-BANK-1 (row 113); §7 'GUILD-*' row GUILD-BANK-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md   # §4.1, §4.4, §5, §11
  - docs/architecture/reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md     # guild amendment
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true   # Bank/Guild event payloads, retention, resource limits
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2139, branch skeleton/guild-bank-1

## Goal

Guild balance and ledger, deposit and withdraw, disband payout and its retained custody
claims (GUILD0 §5), as a financial participant composed into the real Guild transactions.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| BANK-1 (Account bank, migrations 0071/0072) | MERGED (#1766) |
| NPC-ECON-1 (shared Bank source, D1099 hunks waited on it) | MERGED (#2197) |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| GUILD-1 (Guild/rank/member tables, guards, `GuildDisbandTransaction`, disband controller) | MERGED (#2136) as unregistered checkpoint; its `domain/guild.rs`, `durability/guild.rs`, `guild_audit.rs` sources were removed by #2280 (d25623a36) as dead code — BLOCKING until the Guild core is re-landed (`docs/agents/tasks/GUILD-1.md`) |
| GUILDHALL-1 (Hall bid source / operation tables for the escrow participant) | BLOCKING for the Hall escrow participant only — re-implementation pending (draft #2153 closing) |
| HOUSE-1a (property/bid release before payout, D1100) | BLOCKING for the disband payout composition — re-implementation pending (draft #2137 closing) |
| Junior/OnIsland source (native source is still `NoStarterIsland`) | OPEN — owning source needed for a genuine OnIsland refusal |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas.

## Owned paths (ADR-0022 layout)

- `crates/gameplay/economy/src/guild_bank.rs` (new, L3 pure rules: conservation plans, payout split, claim arithmetic, ceilings)
- `crates/gameplay/economy/src/guild_bank_tests.rs` (new)
- `crates/gameplay/durability/src/durability/guild_bank.rs` (new, deposit/withdraw/claim transactions and disband payout participant)
- `crates/gameplay/durability/src/durability/guild_bank_audit.rs` (new, closed TRANSFER-only conservation and bounded evidence)
- `crates/gameplay/durability/src/durability/guild_bank_tests.rs` (new)
- `apps/game-server/tests/guild_bank_postgres.rs` (new)
- `docs/agents/tasks/GUILD-BANK-1.md`

Slot-only (final commit at the queue turn): the migration (number: next free number at
allocation time (only one migration PR in the queue)); module registration in
`crates/gameplay/economy/src/lib.rs`, `crates/gameplay/durability/src/durability/core_modules.rs`
/ `lib.rs`; the shared Bank ledger-kind hunks in `durability/bank.rs` / `bank_audit.rs`;
`docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`, `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`.

## Wiring points (verified on origin/main)

- `crates/gameplay/durability/src/durability/bank.rs`, `bank_audit.rs` — add closed guild ledger kinds, cause and reference read arms (BANK0 §3 amendment); `bank_audit.rs` value-line soft ceiling needs an allocated compatible event/audit change before ordinary Bank operations on above-soft retained balances
- `crates/gameplay/economy/src/bank.rs` — Junior port
- `crates/gameplay/durability/src/durability/character_progression.rs::assert_item_only_fence` — reuse for Root deposit/withdraw/claim
- `apps/game-server/migrations/0072_character_gold_fee_bank_debit.sql` — immutable; preserve its complete bank-entry guard body when adding guild arms in the new migration
- `apps/game-server/src/node/guild_jobs.rs` (`run_guild_disband_pass`; unregistered) — the payout participant is invoked from the Guild disband transaction only
- `docs/contracts/game-events/v2/bank_operation.proto`, `docs/contracts/OTERYN_GAME_GUILDS_V1.md`
- Reference only (removed from main): `git show d25623a36^:crates/gameplay/durability/src/durability/guild.rs` (`GuildDisbandTransaction`, table/guard SQL)

## Accepted decisions carried from the draft

- **D1098:** GUILD-1 owns the real terminal controller; GUILDHALL-1 owns its bid release
  participant; HOUSE-1a owns property/auction/Inbox ports. The Bank participant consumes the
  genuine `GuildDisbandTransaction`, never a copied xid, a currentness boolean or a House token.
- **D1099:** symbolic closed ledger kinds/cause/reference arms; no new numeric ledger/event IDs
  invented by this task; old numeric event paths refuse symbolic variants.
- **D1100 (accepted, no longer a question):** V1 refuses additional Account funding after a
  leader-Account change; original-contributor refunds remain fixed. Full House completion
  proof precedes payout; a bid-only release never implies property/Inbox closure.
- CP P1 (2026-10-16): no substitute Guild prerequisite schema. Guild lifecycle tables and
  mutation guards belong only to GUILD-1.
- BANK0 adds no Premium requirement for these transfers; founding/leadership Premium stays
  with GUILD-1.

## Reference behaviour built by the draft (re-implement, do not copy)

- Root deposit/withdraw/claim transactions with immutable occurrence binding and replay.
  Account/World come from the fenced Character root, never request fields.
- Lock order: recovery/admission → occurrence → acting Character/root → guild → member →
  guild balance/claim → Account bank balance. The guild state gate runs before membership and
  balance reads. Refusals roll back with no money, ledger or audit change.
- Deposit/withdraw = Account balance delta + guild balance delta + ledger in one transaction.
- Disband payout: one atomic guild debit = Bank credit that fits the Account/World held-escrow
  headroom + retained claim remainder. One payout per GuildId survives changed occurrences,
  member cleanup and DISBANDED. Claims keep an immutable Account/World binding, never delete,
  only decrease; zero rows stay terminal. Claim commands derive entitlement from the stored
  claim and current Character Account/World without guild membership.
- In-transaction API: `lock_guild_disband_payout_occurrence`,
  `payout_guild_disband_in_transaction`, `read_guild_disband_payout_in_transaction`. It rechecks
  the physical transaction, the current Guild and all seven prelocked Bank identity fields;
  fresh payout stamps must match the current PostgreSQL xid. Retries load the immutable payout,
  current custody, zero Guild balance and absence of every HELD bid row.
- SQL `game_guild_bank_assert_disband_paid(UUID,UUID,UUID)` is a financial integrity check for
  Guild's deferred final guard (payout conservation, ledger/audit rows, claim binding, zero
  balance, no HELD custody), not a House release proof. Runtime gets EXECUTE only.
- `apply_guildhall_escrow_in_transaction`: consumes the GUILDHALL same-xid Hall operation,
  checks per-source escrow conservation/headroom and writes native chains with fixed pre-attempt
  entry IDs. An absent escrow source fails; it is never zero. Multi-bid batches prelock all
  guild balances, then all Account balances, in the combined global order.
- Audit: closed TRANSFER-only before/after conservation and hard ceilings before encoding.
  Draft payload rows were unregistered proposals, not admitted EventEnvelopes.
- Draft evidence (historical): 7/7 then 8/9 PG 17.6 cases; the one failure was a fixture
  missing SELECT on `game_guild_job_checkpoints` (42501 instead of 23514), repaired without
  weakening a guard. The two disband-port bodies were drafted, not executed.

## Findings and open questions

- Production Bank readers, closed ledger enums, audit codecs and balances above the soft
  ceiling need coordinated shared BANK-1 updates in the same slot.
- Shared credit-headroom seam (`HOUSE-1a-BANK-HEADROOM`, SOCIAL-MAP §1.12) must be one helper
  covering Bank + Market held + House held + Guildhall held + incoming.
- Nonzero partial claims near 9e15 were arithmetic-only on the draft; a positive nonzero claim
  delivery is unqualified.

## Acceptance criteria

- [ ] Deposit/withdraw conserve gold across account and guild balances (property test).
- [ ] Disband pays out once; custody claims survive restart.
- [ ] Junior and Premium rules of BANK-0 apply.
- [ ] The payout runs only inside the genuine Guild disband transaction after House/Hall release.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer (pure arithmetic in `economy`, storage in `durability`).
- [ ] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-economy guild_bank
cargo test -p oteryn-gameplay-durability guild_bank
cargo test -p oteryn-game-server --test guild_bank_postgres   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md` §5
- `docs/agents/tasks/archive/OTV2-20261004-bank-1.md`
- `docs/reference/tibia-manual/guilds.md`
- CP coordination: #2131 (D1098, D1099, D1100; CP comment 6193914573)
