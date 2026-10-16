# GUILD-BANK-1 — guild balance, ledger and disband payout (skeleton task record)

```yaml
task_id: GUILD-BANK-1
title: "GUILD-BANK-1: guild balance, ledger and disband payout"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-BANK-1 (row 113); §7 'GUILD-*' row GUILD-BANK-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/guild-bank-1
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Guild balance and ledger, deposit and withdraw, disband payout and its custody claims (GUILD0 §5).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

single child GUILD-BANK-1 (hard, persistence and economy review).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| GUILD-1 | BLOCKING (Wave 4, skeleton only) |
| NPC-ECON-1 (BANK-1) | BANK-1 MERGED (#1766); NPC-ECON-1 family BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/guild_bank.rs`
- `apps/game-server/src/durability/guild_bank_audit.rs`
- `apps/game-server/src/durability/guild_bank_tests.rs`
- `apps/game-server/tests/guild_bank_postgres.rs`
- `docs/agents/tasks/GUILD-BANK-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

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

- durability/bank.rs (BANK-1) ledger kinds: add guild ledger kinds per BANK-0 §3 amendment
- durability/mod.rs registration
- GAME_EVENT_FOUNDATION_REGISTRY.json / RESOURCE_LIMITS_REGISTRY.json rows

## Inseparable bundles (never split across PRs)

- migration + guild bank writer + durability tests
- deposit/withdraw = account balance delta + guild balance delta + ledger in one transaction
- disband payout + custody claims in the disband transaction of GUILD-1

## Acceptance criteria

- [ ] Deposit/withdraw conserve gold across account and guild balances (property test).
- [ ] Disband pays out once; custody claims survive restart.
- [ ] Junior and Premium rules of BANK-0 apply.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server guild
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md §5
- apps/game-server/src/durability/bank.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local native preparation — 2026-10-15 (not full mechanic acceptance)

The original plan, dependency rows, status, open criteria and CP builder rules above remain unchanged. Independent authoring was released by CP comment `6193914573`; the exact five-path allocation is `guild-bank-1-allocation.json`, base `db93304be76a3405a585fb94871d4144e21b6e53`. Accepted policy was read at protected-main object `ca5ea5930edf0d609e91d7a3f5436e7ca2fde610`: GUILD0 §4.1, §4.4, §5 and §11 plus the BANK0 guild amendment. Neither the prepared pure GUILD1 module nor a private fixture row is an installed/current guild owner.

### Prepared owned component

- `guild_bank.rs` adds native Root deposit/withdraw/claim transactions, immutable occurrence binding/replay, guild ledger/balance chains and retained disband custody claims. Account/World come from the actual native fenced Character root, not request authority fields. It reuses `assert_item_only_fence`, recovery/admission checks, current node/session/lease/scope checks and the canonical bank Junior port. BANK0 does not introduce a Premium requirement for these transfers; founding/leadership Premium authority stays with GUILD1. The current native Junior source is still `NoStarterIsland`; genuine current OnIsland refusal qualification needs that owning source, not a supplied boolean.
- Lock order is recovery/admission → occurrence → current acting Character/root → guild → member → guild balance/claim → Account bank balance. The guild state gate precedes membership/balance reads. Refusals roll back without money, ledger or audit changes. The World-job payout participant takes no Character root/session; GUILD1 must invoke it in its actual disband transaction after the real property/bid release step, with recovery/admission and combined locks already held. No independent disband controller or fabricated release flag is created.
- Whole-balance payout is one atomic guild debit = bank credit fitting actual Account/World held-escrow headroom + retained claim remainder. A unique payout per GuildId survives changed occurrences, member cleanup and DISBANDED. Claims retain immutable Account/World binding, never delete and only decrease; zero rows remain terminal. Claim commands derive entitlement from the stored claim and current Character Account/World without requiring guild membership.
- Private **UNINSTALLED** DDL lives only in the owned source constants. `GUILD_PREREQUISITE_SCHEMA_PROPOSAL` is a minimal candidate GUILD1 read/lock dependency, not a lifecycle writer. Runtime row-lock privileges have an actual UPDATE-denial trigger. `GUILD_BANK_SCHEMA_PROPOSAL` preserves native balance chains and the complete latest 0072 bank-entry guard body after its closed candidate arm. It uses symbolic candidate guild causes and an exclusive guild-operation reference, not invented admitted ledger/event numbers.
- `guild_bank_audit.rs` checks closed TRANSFER-only before/after conservation and hard ceilings before encoding bounded private evidence. Deposits/withdrawals/payouts retain separate Bank and Guild proposal rows; claims retain Bank only. These are explicitly **unregistered payload/proposal rows**, not admitted EventEnvelopes, protobuf revisions, retention installation or publication delivery. They cannot justify production activation.
- `apply_guildhall_escrow_in_transaction` consumes the separately owned actual same-xid Hall operation, resolves/locks current balances, checks per-source escrow conservation/headroom and writes real native chains with fixed pre-attempt entry IDs. Its separate uninstalled ledger extension preserves the previous Guild and native guard arms. This is a financial participant, not current House/auction/content authority or a full Hall writer. Future multi-bid batches must prelock all guild balances then all Account balances in the combined global order; single-participant calls do not establish that batch orchestration.

### Exact candidate composition inputs

Owned tests contain literal copies of three unmerged GUILDHALL constants from its owned `durability/guildhall.rs`, with source/hash manifests. They are proposed storage inputs, never main-installed/current authority:

| Constant | Raw UTF8 SHA256 |
|---|---|
| `BID_SOURCE_SCHEMA` | `9fff79763a5218a304c953e7ebdc8e6449a1185dd35ac994228112838483dbbc` |
| `OPERATION_SCHEMA` | `3113f56a8e7310b9cc2ee5780b92f3f97d56b526d905b6b63d245f454695e97e` |
| `HALL_GUARDS_SCHEMA` | `731f76500a5ad629ed329252be31fcbfc372164271daff79329e8c1aa8cfa531` |

Apply only in isolated qualification in this order: candidate Guild prerequisite → exact Hall bid/operation constants → GuildBank proposal → Hall ledger extension → exact Hall guards. An absent escrow source fails; it is never interpreted as zero. Hall current-command/catalogue/auction admission and required House/Guild/Bank events remain its separate integration obligations.

### Actual local qualification and limits

Evidence is retained outside the branch at `/workspace/oteryn-prep/guild-bank-1-evidence/`, with exact commands, source-before/after hashes, minimal bridge transformations and raw logs. `attempt-4/pg-tests.log` reports **7 passed / 0 failed in 8.21s**: six genuine PostgreSQL17.6 (`170006`) cases and one arithmetic/audit boundary case. Native funding is actual MINT → TRANSFER → existing `bank_deposit`; no guard was disabled and no nonzero balance/escrow was seeded. Checks cover deterministic transfer conservation, exact replay/conflict, current-fence/rank/state refusals, accepted amount maximum/max+1, two independently reconciled Roots serializing a typed losing writer, deferred audit rollback and same-identity retry, disband-once/reopen/member-cleanup recovery, actual terminal-claim mutation rejection with typed 23514 and unchanged binding, and real funded dual-source escrow reserve/return chains. The state-before-membership priority control deliberately combines those two business facts; it is not claimed as independent authority-invariant coverage.

Final rustfmt/check, original untransformed native unit-context Clippy metadata, normal cfg(test) native rlib, enabled original standalone-wrapper Clippy metadata and selected-body driver all returned 0. The deliberate Cargo capture 101/exit86 is not a test pass. The tracked standalone target is explicitly inactive until CP registration; its zero-test metadata check is not runtime qualification. Native baseline unused-scaffolding warnings from the no-global-harness rlib are retained separately from clean original-unit Clippy. No full native unit ELF/full Cargo test pass is claimed.

The unchanged native fixture uses its actual admin/ambient test Root constructor; this qualifies native fences/SQL semantics, not production credentials/TLS. Runtime source row-lock/read-only permissions were additionally checked with `SET LOCAL ROLE`. Nonzero partial claims near 9e15 are arithmetic/guard boundaries only: genuine funded PostgreSQL tests qualify full payout and retained zero-claim recovery, not fabricated near-ceiling custody or a positive nonzero claim delivery. Current Junior OnIsland, Premium leadership and the complete guild lifecycle remain owning-source qualification gates.

Historical failures remain explicit: attempt 1 failed only the external bridge's Result alias; attempt 2 was a real candidate DDL CASE syntax failure (1 pure/6 PG fail); rollback-only preflight then found the same defect in the exact Hall guard, repaired by its owner; attempt 3 was 6/1 because a negative UPDATE targeted an empty claims table. Attempt 4 moves that negative to the real retained claim. Completed failed-run binaries are losslessly compressed with verified restore SHA manifests; raw logs and exact owned sources remain. No failed result is reported as green or merely environmental.

### Mandatory CP/controller/production gates

The accepted inseparable bundles remain open. GUILD1 must supply/admit its complete current guild/member/leadership owner, schema and real disband release/progress controller. The financial participant must be composed in that actual transaction, not exposed as an independent valuable grant. Production Bank readers, closed ledger enums/reference handling, audit codecs and balances above the soft ceiling require coordinated shared BANK1 updates; symbolic private rows are deliberately not compatible with unchanged live readers. Normative Bank/Guild (and Hall House) payloads, EventEnvelope/retention/resource admission, migration numbering/installation, module registration, current command/World-job routing and exact-head protected integration must land together at CP. None was edited, installed or qualified here. This packet is independent native component preparation, not architect approval, full GUILD-BANK acceptance or gameplay E2E.


## 2026-10-16 owner-schema compatibility repair (uncommitted authoring candidate)

Author: family D GuildBank worker, sole writer in `/workspace/oteryn-family/guild-bank`,
existing PR #2139 at parent `e5979785342b1f0e3a282aa5324e5d65452ce4b2`.
CP P1 is accepted and repaired in owned source: removed the substitute production
`GUILD_PREREQUISITE_SCHEMA_PROPOSAL`, including its runtime UPDATE-denial triggers.
Guild lifecycle tables and mutation guards belong to the genuine GUILD1 owner.
Earlier references above describe historical preparation and do not prescribe installation.
No migration, native Bank reader/kind, shared module registration or runtime wiring changed.

Owned tests now consume the real GUILD1 table and guard constants directly, in a disclosed
unmerged native validation overlay. The exact stable GUILD source is based on
`92a5a965db23771d12697c0be9fee25f30d05f86`; candidate durability/guild.rs SHA256 is
`57935341f8d2d19ef000c0d7c9294035533102698e980adf0078e38e9079f1f1`.
Literal raw UTF8 table SQL SHA256:
`5ae70c7540915ef2368cab887fcaf0a66b0123f71535bb82bc8e71f750cbb3a1`;
raw guard SQL SHA256:
`1c74f22edc18db2d54b39330038b2c61a5b2cb1d90aab71965c26d08a8dd4a2e`.
Runtime assertions reject mismatching dependencies. These are candidate source bytes,
not a claim of acceptance/installation on main. Future Hall test integration must consume
these same genuine owner inputs; retaining the removed substitute for that caller is unsafe.

Inherited tests remain present. Explicit fixture adaptations: install complete genuine
Guild owner tables; use named full-column guild/rank/member/leadership initial snapshots;
install the genuine owner guards for ordinary transfer, rollback, concurrency and escrow
qualification; grant only fixture SELECT/row-lock privileges. The rank refusal is now an
independent pre-guard rank3 snapshot and the state-before-membership case uses a separate
DISBANDING/no-member snapshot. No installed guard is disabled, removed or weakened.
The runtime role lock test now expects the genuine owner's unbound-mutation SQLSTATE23514,
rather than the deleted substitute's blanket42501 denial.

The inherited `disband_once_survives_member_cleanup_and_native_root_reopen` body is
preserved except its explicitly named historical installation mode: complete real owner
tables are seeded without installing lifecycle guards. Its privileged state/member changes
remain historical financial-component fixture DATA. It does not execute or qualify the
real Guild disband lifecycle. The absent terminal Guild/House closure controller remains a
blocker; the test is neither ignored nor represented as integrated lifecycle coverage.

Two added native PG bodies require an actual configured PG route: a genuine guarded Guild
SetMessage writer followed by Bank deposit/withdraw with the same owner tables and active
mutation guards; and four deterministic generated funded sequences (33 operations per seed),
with independent balance modeling, zero/max/max+1/insufficient boundaries, unchanged refused
footprints, exact replay and conflicting-occurrence rejection after every successful step.
Funding remains actual MINT→TRANSFER→native bank_deposit; no nonzero bank balance seed.
No Cargo dependency or manifest change. The earlier fixed example and other inherited
negative/rollback/concurrency/recovery tests remain in the source.

Owned Rust1.94 rustfmt and `git diff --check` passed. Governance validation passed
(22 documents/9 lanes) and all 124 governance unit tests passed. No heavy/native check ran in the
worker; lead owns the native Cargo/PG slot and independent review. The candidate is not yet
runtime qualified and has no fresh native PASS claim. Actual current OnIsland/Junior source,
shared closed Bank ledger/cause/readers, admitted event envelopes/publication, numbered SQL
installation, real Hall release→Guild disband→Bank payout composition, module registration
and exact-head protected qualification remain pending their owning source/CP slots.


Native compile follow-up: the lead's genuine combined-unit compile rejected two raw-digest
`LowerHex` formatting expressions with locked sha2's crypto-common Array type. Returned to
AUTHORING and replaced only rendering with per-u8 lowercase two-digit hex formatting; raw
SQL hash values and dependency source remain unchanged. This repairs an actual compiler
failure, not an environmental failure. Rustfmt/diff checks rerun; native recompile remains
with the lead. No native pass is claimed from the failed run or this formatting check.


Actual PG17.6 qualification of frozen6108985d executed9 Bank bodies:8 passed and the runtime
role guard case failed because it received42501 instead of23514. Source diagnosis: the
updated genuine Guild UPDATE guard first reads `game_guild_job_checkpoints`, which the
qualification role could not SELECT; the intended unbound-operation guard never ran.
Returned to AUTHORING: added only fixture SELECT on that exact genuine checkpoint table,
retained23514 and added exact guard-message assertion to prove the intended native guard
executes. No production grant or mutation capability changed; no installed guard weakened.
The genuine owner-writer/property tests remain unchanged. Failed candidate/log retained;
focused native retry and independent exact-source review remain with the lead.
