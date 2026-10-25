# GUILD-BANK-1 — guild balance, ledger and disband payout (consolidated task card)

```yaml
task_id: GUILD-BANK-1
title: "GUILD-BANK-1: guild balance, ledger and disband payout"
mode: IMPLEMENT
status: in_progress
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
branch: claude/happy-davinci-ijz3wy-srv-gbank
pr: null
needs_migration: true
needs_registry: true   # Bank/Guild event payloads, retention, resource limits
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Historical merged audit evidence: none. The 2026-10-23 local authoring result below adds the missing bank owner; it is uncommitted and does not change live Issue/PR status.

`pr: null`: this local work has no publication or merge authority. Full Guildhall escrow/property release composition remains dependent on its owning mechanics.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2139, branch skeleton/guild-bank-1

## Server wiring gap list

Local authoring audit, 2026-10-23; the E2E audit row in
`docs/agents/tasks/MECHANICS_STATUS.md` classifies Guild as BACKEND. This HEAD
already has the basic Guild wire owner (`guild_intent.rs`), beyond that snapshot.
No client work, publication, dependency download or external review is authorized here.
Bound META 3.1.0 resolved read-only at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

| Layer | This HEAD before this task | Missing server work |
|---|---|---|
| L3 rules | Account-bank arithmetic and native Guild rank/state rules exist; no guild-bank module | Transfer conservation, rank/junior refusals, ceiling-safe payout and claims |
| L4 storage | Account bank and Guild core are present; no guild-bank writer/tables | Fenced receipt/replay, guild balance/ledger, paired Account entries, payout and retained claims |
| Migration | Guild core 0132, House owner 0143, roster 0161 exist | Allocated 0174 for the bank participant and guards; preserve existing Bank/House arms |
| Dispatch/wire | Guild commands 40/41 exist; no bank command handler/codec | Mechanic-owned handler and bounded commands 68/69, capability 55 |
| Capability/content | Basic Guild available on every World; bank absent | Offer 55 with durable owner; no extra content required |
| Runtime/disband | Server composes `ComposedFreshAdmission`; real disband controller exists, stops when financial/property participants exist | Register bank wire and integrate payout only after genuine House/Hall release proof |
| PostgreSQL E2E | Real Guild dispatch fixture exists; no bank E2E | Auto-mounted dispatch --lib cases for wire, durable conservation, replay/reload and refusals |

## Local server result (resumed 2026-10-24)

Basic server implementation and focused validation are complete locally; status remains `in_progress` for independent review and
programme publication. No commit, push, protected CI/routing edit, client edit or external review
trigger was made. The owner's current basic-server direction supersedes the historical new-test-target
and workspace-wide validation suggestions below.

- Capability **55** is registered, admitted and offered on every supported World, including the
  shipped bundle. Commands **68/69** reach the existing real `serve_admitted` Guild dispatch route.
  No additional content is needed.
- Members deposit from their Account/World bank; ranks 1/2 withdraw to their own bank. Guild state,
  current native role, shared BANK-0 Junior source, recovery, node assignment, session generation,
  lease, root and runtime ownership are checked. There is no extra Premium condition on money moves.
- Migration **0174** adds immutable request-bound receipts, Guild balance/ledger and symbolic paired
  Account entries, bounded BANK/GUILD native owner evidence and permanently retained disband claims.
  Existing numeric Bank/Fee/NPC and House trigger arms remain installed unchanged. Ordinary Bank
  readers/audit accept above-soft retained balances; ordinary incoming credits retain the soft cap.
- Payout runs only with the genuine `GuildDisbandTransaction`, rechecking its physical xid/current
  recovery/assignment/Guild before writing. The World scheduler commits payout once before bounded
  member cleanup. The shared hard headroom subtracts current House and Market held custody. Excess
  stays with the original leader's immutable Account/World claim, including after `DISBANDED`.
- Command 69 reads a member's latest 30-day ledger with a 100-row descending cursor, or lists the
  authenticated Account/World's open claims with a 100-row ascending GuildId cursor. Command 68
  delivers partial claims without membership. Original successful/refused receipts survive reload;
  changed payload bytes on the same occurrence are rejected.
- Current House 0143 supports only Character property. No Guildhall/property custody source is
  installed. This is proven from the installed schema; any uncomposed Hall/Guild-property owner
  blocks payout and cleanup. The test adds such an owner and proves that funds stay untouched.

### Resume checkpoint (2026-10-24)

- Resumed the existing uncommitted delta on local base `cb5b3de92d2e19df53e05a7e0f5b9d98fa11d299`.
  No frozen candidate or PR exists. Refreshed the bound META 3.1.0 policy read-only at its exact
  bound revision; no external review, publication or credential mutation was performed.
- Whole-diff review found a first insufficient-funds deposit could persist an empty Guild balance.
  Added a real-dispatch regression, observed it fail on that extra row, then isolated lazy balance
  creation and value planning in a native SQL savepoint. Every plan refusal rolls it back before
  storing only the immutable replay receipt. Financial state and owner evidence stay unchanged.
- Extended the same PostgreSQL E2E to replay the original denied member withdrawal after promotion
  to vice and again after Root reconstruction and disband cleanup. Both return the original refusal.
- New mechanic modules remain below 800 lines (largest: the 750-line dispatch E2E). Removed 16
  inherited `.guild-bank*.log` scratch files from the repository root.

### Reference parity

| Visible behavior | Reference evidence | Oteryn now |
|---|---|---|
| Separate Guild bank; every member deposits, leader/vice withdraw | `docs/reference/tibia-manual/guilds.md` §5.8.1; official owner capture 2026-09-28 | Real commands 68/69, current membership/rank checked under the Guild lock; ordinary members refused withdrawals |
| Balance and rolling 30-day activity visible to members | Same official capture §5.8.1; GUILD0 §§4.4/5.1 | Durable Guild balance and paged ledger; outsiders receive no financial projection |
| Transfer from/to Account bank, no coins or new value | Accepted GUILD0 §5.2 and BANK0 §3 amendment | Paired immutable chains and bounded TRANSFER owner records commit atomically; audit failure rolls everything back |
| Soft credit cap and Junior restrictions; no extra Premium check | Accepted GUILD0 §5.2 and BANK0 §4.4 | Soft caps enforced, existing native Junior source reused; pure OnIsland refusal qualified (native island source remains unavailable) |
| Leader receives disband balance after release | GUILD0 §5.3, explicit Oteryn decision; manual silent (`PARITY_PENDING`) | Genuine disband transaction pays once, hard headroom includes House/Market custody, pending unknown release owners fail closed |
| Overflow survives disband/restart and is claimable by the same Account/World | Accepted GUILD0 §5.3 | Retained immutable entitlement, positive partial delivery near 9e15, zero row kept terminal, own claim discovery after membership cleanup |
| Lost acknowledgement/retry never moves money twice | GUILD-BANK-1 accepted occurrence/replay decision | Real dispatch replays successful and denied receipts after owner/Root reconstruction; promotion cannot change an earlier withdrawal refusal; two independent Roots produce one spend and one typed insufficient-balance result |

### Validation evidence

All cargo commands use `/home/mole/oteryn-cache/codex-bin/cargo`; PostgreSQL tests use the wrapper's
private route and create isolated databases. The new E2E confirmed `server_version_num=180006`.
No PostgreSQL suite was run without a mechanic filter.

Fresh resume validation uses `CARGO_NET_OFFLINE=true` and
`CARGO_TARGET_DIR=/home/mole/oteryn-cache/target-codex/lanes/srv-gbank`. Exact mechanic test selectors:

| Package | `cargo test --locked` selector | Resume result |
|---|---|---|
| `oteryn-protocol-oteryn`, `oteryn-gameplay-economy` | `--lib` (pure unit suites) | 394 + 25 passed |
| `oteryn-gameplay-dispatch` | `--lib guild_bank_postgres_real_dispatch -- --nocapture` | 1 passed on PostgreSQL 18.6 after the savepoint repair |
| `oteryn-gameplay-dispatch` | `--lib capabilit` | 94 passed |
| `oteryn-gameplay-durability` | `--lib guild_bank` | 1 passed |
| `oteryn-gameplay-durability` | `--lib bank_audit::tests` | 5 passed |
| `oteryn-game-server` | `--lib guild_core_terminal -- --nocapture` | 3 passed |
| `oteryn-game-server` | `--lib house_ownership_guild_participants_require_a_guild_owner -- --nocapture` | 1 passed on PostgreSQL 18.6 |
| `oteryn-game-server` | `--lib house_ownership_recovered_bid_escrow_replay_conflict_and_session_fence -- --nocapture` | 1 passed on PostgreSQL 18.6 |

Final resume `fmt --all -- --check` and `clippy --locked` on all targets of all five touched
packages with `-D warnings` passed after the repair. Governance and repository-policy validators
and all 129 governance unit tests passed, including a final run after the task-card evidence update.
The whole-diff owner checklist and workspace enum/constructor/caller sweep found no remaining hit.

Historical 2026-10-23 evidence retained below:

- `test --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-economy --quiet`: 394 protocol + 25 economy tests passed (codec/registry bounds and conservation properties included).
- `test --locked -p oteryn-gameplay-dispatch --lib guild_bank_postgres_real_dispatch -- --nocapture`: **1 passed**, real runtime-role PostgreSQL and connection frames; member/vice rights, both ceilings, raw-payload conflict, stale lease, omitted capability, malformed bytes, audit-failure rollback, independent competing writers, unknown-release refusal, payout once, positive partial claims, full delivery, native Root reload, immutable zero custody and negative SQL balance/claim probes.
- `test --locked -p oteryn-game-server --lib guild_core_terminal -- --nocapture`: **3 passed**, including foreign xid/unassigned World, an uncomposed financial owner and two-Root core cleanup. Historical fixtures explicitly remove this later bank owner when reconstructing the old core-only schema; the installed-schema E2E does not do that.
- `test --locked -p oteryn-gameplay-durability --lib bank_audit::tests --quiet`: **5 passed**; high-balance Guild payout audit regression is also mounted under filter `guild_bank`.
- `test --locked -p oteryn-gameplay-dispatch --lib capabilities::tests --quiet`: **61 passed**, including registry offers, accepted capability 55 and bundle availability.
- `test --locked -p oteryn-gameplay-durability --lib guild_bank --quiet`: **1 passed**, ordinary Bank audit can spend retained balances above the soft cap, but refuses above the hard ceiling.
- `test --locked --offline -p oteryn-game-server --lib house_ownership_guild_participants_require_a_guild_owner -- --nocapture`: **1 passed**. The new installed Bank owner is asserted first; the existing core-only snapshot then preserves every original House-only and unknown-owner assertion.
- `test --locked --offline -p oteryn-game-server --lib house_ownership_recovered_bid_escrow_replay_conflict_and_session_fence -- --nocapture`: **1 passed**. Shared hard headroom reserves the same 100 liquid + 500 held House gold; ordinary incoming credits retain the separate soft cap. Both compatibility regressions were observed failing before the fixture/expectation repair and passing afterwards; neither was load contention.
- `fmt --all` and `fmt --all -- --check`: passed. Final `clippy --locked --offline -p oteryn-gameplay-economy -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets --quiet -- -D warnings`: **passed**, including both compatibility-test repairs. Existing warnings come only from vendored Tokio/sqlx dependencies; no dependency was edited or downloaded.
- `python3 tools/agents/validate_governance.py`: passed. `python3 tools/repository/validate_repository_policy.py`: passed. `python3 -m unittest discover -s tools/agents/tests`: **129 passed**.
- Whole-diff D1462 review includes the owner checklist, all new enum/field/constructor/impl sites across `apps/`, `crates/`, `tools/`, production callers and real-dispatch coverage. No production unwrap/expect, new PostgreSQL target, client path or forbidden CI routing file was added.

### Changed files (local, uncommitted)

```text
apps/game-server/migrations/0174_guild_bank.sql
apps/game-server/src/lifted_tests/auto/house_ownership_tests.rs
apps/game-server/src/lifted_tests/auto/house_ownership_tests/guild_participants.rs
crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs
crates/gameplay/dispatch/src/gameplay_transport/capability_quest_log_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/guild_bank.rs
crates/gameplay/dispatch/src/gameplay_transport/guild_intent.rs
crates/gameplay/dispatch/src/gameplay_transport/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/guild_bank.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/guild_bank/seed.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/guild_bank/transport.rs
crates/gameplay/durability/src/durability/bank_audit.rs
crates/gameplay/durability/src/durability/core_modules.rs
crates/gameplay/durability/src/durability/guild/disband.rs
crates/gameplay/durability/src/durability/guild_bank.rs
crates/gameplay/durability/src/durability/guild_bank/payout.rs
crates/gameplay/durability/src/durability/guild_bank/query.rs
crates/gameplay/durability/src/durability/guild_bank/storage.rs
crates/gameplay/durability/src/durability/guild_bank_audit.rs
crates/gameplay/durability/src/durability/guild_bank_bank_audit_tests.rs
crates/gameplay/durability/src/durability/guild_chat_fixture/schema.rs
crates/gameplay/economy/src/guild_bank.rs
crates/gameplay/economy/src/guild_bank_tests.rs
crates/gameplay/economy/src/lib.rs
crates/protocol-oteryn/src/capability_tables.rs
crates/protocol-oteryn/src/guild_bank.rs
crates/protocol-oteryn/src/guild_bank_tests.rs
crates/protocol-oteryn/src/lib.rs
docs/agents/tasks/GUILD-BANK-1.md
docs/contracts/protocol-oteryn/v1/guild_bank_v1.proto
docs/contracts/protocol-oteryn/v1/registry/capabilities/0055-guild-bank-v1.json
docs/contracts/protocol-oteryn/v1/registry/command_types/0068-guild-bank-intent.json
docs/contracts/protocol-oteryn/v1/registry/command_types/0069-guild-bank-query.json
docs/contracts/resource-limits/entries/0850-GUILDBANK-RL-01.json
docs/contracts/resource-limits/entries/0851-GUILDBANK-RL-02.json
docs/contracts/resource-limits/entries/0852-GUILDBANK-RL-03.json
docs/contracts/resource-limits/entries/0853-GUILDBANK-RL-04.json
```

### Unfinished / outside this basic server slice

- Full GUILDHALL-1 escrow reserve/release/settlement and genuine Guild-property/Inbox disposition are
  still absent owners; no substitute proof or table is provided. Installing an unknown owner blocks
  disband until its accepted release composition lands.
- A genuine Junior/OnIsland authority source is still absent, as for BANK-1. No false source is invented.
- Native symbolic owner evidence follows the current Guild core proposal pattern, not a new numeric
  ANL EventEnvelope allocation. Its eventual event admission/export/retention integration remains
  with the accepted registry and retention owners.
- Independent persistence/economy review, repository CI, publication/merge and final task archival
  remain with the coordinator. No request to spend or send repository contents was issued here.

## Goal

Guild balance and ledger, deposit and withdraw, disband payout and its retained custody
claims (GUILD0 §5), as a financial participant composed into the real Guild transactions.

## Dependencies (historical allocation snapshot; current local findings above)

| Dependency | State |
|---|---|
| BANK-1 (Account bank, migrations 0071/0072) | MERGED (#1766) |
| NPC-ECON-1 (shared Bank source, D1099 hunks waited on it) | MERGED (#2197) |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| GUILD-1 (Guild/rank/member tables, guards, `GuildDisbandTransaction`, disband controller) | MERGED (#2136) as unregistered checkpoint; its `domain/guild.rs`, `durability/guild.rs`, `guild_audit.rs` sources were removed by #2280 (d25623a36) as dead code — BLOCKING until the Guild core is re-landed (`docs/agents/tasks/archive/GUILD-1.md`) |
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
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/guild_bank.rs` and `guild_bank/` (auto-mounted existing --lib PG tree)
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

## Historical findings and current disposition

- Production Bank readers, closed ledger enums, audit codecs and balances above the soft
  ceiling needed coordinated shared BANK-1 updates. Addressed locally by migration 0174's
  symbolic arms and the hard-ceiling audit change; numeric paths remain unchanged.
- Shared credit-headroom seam (`HOUSE-1a-BANK-HEADROOM`, SOCIAL-MAP §1.12) must be one helper
  covering Bank + Market held + House held + Guildhall held + incoming. The installed owners
  now share that helper; Guildhall remains absent and blocks disband if introduced uncomposed.
- Nonzero partial claims near 9e15 were arithmetic-only on the draft; a positive nonzero claim
  delivery is now qualified by the real-dispatch PostgreSQL E2E above.

## Acceptance criteria

- [x] Deposit/withdraw conserve gold across account and guild balances (property test).
- [x] Disband pays out once; custody claims survive restart.
- [ ] Junior and Premium rules of BANK-0 apply. Native BANK-0 source and pure OnIsland refusal are reused/qualified; genuine OnIsland source still absent.
- [x] The payout runs only inside the genuine Guild disband transaction after House/Hall release.
- [x] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [x] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [x] Structure: one mechanic per module file in its ADR-0022 layer (pure arithmetic in `economy`, storage in `durability`).
- [x] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-economy -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-protocol-oteryn -p oteryn-gameplay-economy --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib guild_bank --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib bank_audit::tests --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib capabilities::tests --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib guild_bank_postgres_real_dispatch -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --lib guild_core_terminal -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --lib house_ownership_guild_participants_require_a_guild_owner -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --lib house_ownership_recovered_bid_escrow_replay_conflict_and_session_fence -- --nocapture
python3 tools/agents/validate_governance.py
python3 tools/repository/validate_repository_policy.py
git diff --check
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md` §5
- `docs/agents/tasks/archive/OTV2-20261004-bank-1.md`
- `docs/reference/tibia-manual/guilds.md`
- CP coordination: #2131 (D1098, D1099, D1100; CP comment 6193914573)
