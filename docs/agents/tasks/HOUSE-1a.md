# HOUSE-1a — house ownership, auction and first rent (consolidated task card)

```yaml
task_id: HOUSE-1a
title: "HOUSE-1a: house ownership, auction (bid escrow) and first rent"
mode: IMPLEMENT
status: in_progress   # backend checkpoint validated; fresh opening and dispatch qualification unfinished
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
class: "hard (persistence, economy, security)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row HOUSE-1a (row 119); App. 7 HOUSE-*; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md   # §3-§9, §12
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md                    # §1.3, §1.4, §1.9, §1.12, §2.3
  - docs/architecture/reviews/OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md # #1771 acceptance
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_TENURE_CONTRACT_CANDIDATE_2026-10-10.md       # §4.2, owner answer Q1a
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true   # House event (event-8 resource accounting), resource limits
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2137, branch skeleton/house-1a

## Goal

House ownership, acquisition by auction (bid escrow) and the first rent reserved with a
bid, per HOUSE-OWN-0 §3-§9 and SOCIAL-MAP-PACKETS HOUSE-1a. Accepted split (SOCIAL-MAP
§1.3): ongoing rent collection, grace, warnings, eviction, move-out and disposition belong
to HOUSE-1b, not here. HOUSE-1a covers acquisition/auction and the first rent only.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| BANK-1 (Account bank) | MERGED (#1766) |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| HOUSE_OWN0 acceptance | ACCEPTED by ACCEPT-SOCIAL-MAP-0 (#1771); names `HOUSE-OWN0-PHYSICAL-HOUSE-OWNERSHIP-V1` |
| HOUSE-RUNTIME-1a | MERGED (#2163) |
| HOUSE-RUNTIME-1b | BLOCKING — re-implementation pending (draft #2165 closing) |
| HOUSE-RUNTIME-1c | BLOCKING — re-implementation pending (draft #2166 closing) |
| MARKET-1 bounded checkpoint (Bank helper/call-site slot per D1375) | MERGED (#2134) as a bounded storage checkpoint; Bank ledger integration still open |
| HOUSE-1b canonical disposition producer (D1403) | BLOCKING — re-implementation pending (draft #2141 closing) |
| GUILD-1 (Guild domain/writer, disband controller) | MERGED (#2136) as unregistered checkpoint; its `domain/guild.rs` / `durability/guild.rs` sources were dropped by the ADR-0022 move (#2280) — Guild core must be re-landed before any Guildhall release participant |
| GUILDHALL-1, GUILD-BANK-1 | BLOCKING for the Guildhall release participant — re-implementation pending (drafts #2153, #2139 closing) |
| BED-1 sleeper storage | BLOCKING for the Guildhall release participant — re-implementation pending (draft #2147 closing) |
| Qualified House catalogue producer (D1383, bundle lane after MAP-OVERLAY-1c) | BLOCKING — MAP-OVERLAY-1c re-implementation pending (draft #2167 closing) |

Integration order decided by CP (D1162/D1349): HOUSE-1b → HOUSE-1a → MAP-OVERLAY-1c → BED-1.

## Owned paths (ADR-0022 layout)

- `crates/gameplay/housing/src/house_auction.rs`, `house_auction_tests.rs`;
  `house_auction/{eligibility,eligibility_tests,value}.rs` (L3 pure rules).
- `crates/gameplay/durability/src/durability/house_ownership.rs`;
  `house_ownership/{bid,eligibility,premium,settlement,value}.rs` (L4 durable owner).
- `crates/gameplay/durability/src/durability/house_ownership_audit{,_tests}.rs`;
  `house_ownership{,_integrity}_tests.rs` (the PG files mount once in game-server `--lib`).
- `crates/gameplay/durability/src/durability/bank_credit_headroom.rs` (prerequisite).
- `apps/game-server/migrations/0143_house_ownership.sql` (new migration only).
- `apps/game-server/tests/support/bestiary_postgres_harness.rs` (reuse and cleanup);
  `house_spell_privacy_cases.sql` (existing fixture's canonical storage data).
- `docs/contracts/game-events/v2/house_operation.proto`; this task record.

Registration/wiring only: housing `lib.rs`, durability `core_modules.rs`,
game-server `src/lib.rs`, the event/resource registries. Bank's existing large
file only routes its two credit sites to the new owner and exposes its existing
balance/UUID helpers to sibling durability modules. No standalone PG target,
Guildhall participant, hot Monk test constructor or in-flight lane module changed.

## Wiring points (verified on origin/main)

- `crates/gameplay/housing/src/house_scope/mod.rs`, `crates/gameplay/housing/src/house_scope/handoff.rs`
- `crates/gameplay/durability/src/durability/world_house_instance.rs` (0047 instance bindings)
- `crates/gameplay/durability/src/durability/house_spell_acl.rs` (0038 legacy ownership anchor + ACL receipts)
- `crates/gameplay/content/src/content/project/native_house_tiles.rs`
- `crates/gameplay/durability/src/durability/bank.rs`, `bank_audit.rs` (`BANK_BALANCE_MAX = 999999999999`)
- `crates/gameplay/economy/src/bank.rs`
- `crates/gameplay/durability/src/durability/character_inbox.rs` (native `game_character_inbox_deliver`)
- `crates/gameplay/durability/src/durability/premium_fence.rs`, `crates/gameplay/domain/src/domain/premium.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` command chain and `capabilities.rs` `GATED` table (new capability not offered until CP decides)

## Accepted decisions carried from the draft

- **D1375 (Bank cap):** House uses the real `bank_audit::BANK_BALANCE_MAX`; no separate
  House ceiling and no change to the Bank audit limit or SQL ceiling.
- **D1375 ledger kinds:** 7 reserve, 8 release, 9 price burn, 10 rent burn, each with
  before/after balance and escrow.
- **D1383:** the full qualified House catalogue producer belongs to the bundle lane after
  MAP-OVERLAY-1c. No VACANT anchor, caller boolean or direct unqualified content read may
  substitute for it.
- **D1403:** the canonical ordinary disposition producer is HOUSE-1b's migration. After it
  lands, HOUSE-1a must not install any private ordinary disposition fragment in fixtures.
- **D1330:** nullable `house_spell_acl` decoding belongs to its later activating owner, not here.
- **D1349:** canonical disposition schema lives in HOUSE-1b's `move_out` module.
- Placement: the old packet put pure rules under `domain/house_auction.rs`; under ADR-0022
  they are L3 `housing` rules (no IO, SQL or carrier).

## Reference behaviour built by the draft (re-implement, do not copy)

Pure auction rules:
- Proxy (second-price) pricing with no reserve; chronological maximum-reached ties; at most
  256 bidders (257 refused); duplicate/ambiguous inputs refused.
- Bid hold = `maximum + snapshotted first rent`; raise reserves only the increase, lowering
  returns only the decrease, repeats produce no entry.
- Seven-day opening and fifteen-minute anti-sniping deadline extension; time overflow refused.
- Value plans with checked widened arithmetic, conservation proven, unfunded/out-of-range refused,
  zero amounts produce neither a ledger plan nor a value line:

| Plan | Value movement |
|---|---|
| `plan_bid_value` | Reserve only the increased `maximum + captured first rent`; return only a decrease |
| `plan_release_value` | Return the full remaining maximum and snapshotted first rent to the captured Account; escrow ends at zero |
| `plan_winner_value` | Return `maximum - price`; burn price and first rent from already-debited escrow (never charges the balance twice) |
| `plan_ongoing_rent_value` | Debit later rent from the actual Account balance, same rent ledger kind (consumed by HOUSE-1b) |

- A winner has at most three ledger entries plus one changed escrow line → the House event
  needs four value lines; the House transaction uses its own event-8 resource accounting, not
  the Bank command's two-entry limit.
- A quote is a calculation over a supplied qualified bid set, not settlement authority.
  Settlement excludes currently ineligible bids before computing the proxy price. The
  maximum-reached order is a serialized durable input, never a client timestamp.

Ordinary-property storage (HOUSE-OWN-0 §3, HOUSE-TENURE §4.2 / Q1a):
- Canonical properties, World-global tiles and one personal housing slot per Account+World.
- Preserves the legacy 0038 ownership anchor (used by 0047 instance bindings and immutable ACL
  receipts); canonical property and legacy row must agree at commit (deferred check).
- Ownership generation is separate from property lifecycle and global ACL revision.
- Acquisition succeeds an auction with a new receipt; release increments ownership generation,
  clears both owners, retains the acquisition receipt and removes the slot atomically. The
  legacy owner trigger is replaced to permit the NULL-owner release.
- Property and legacy anchor are never deleted. A retired property never returns to
  `VACANT`/`AUCTION`. Deferred slot checks cover old and new property keys.
- Legacy ACL change must advance the property ACL revision together with its receipt; a missing
  same-transaction receipt rolls both back. ACL deletion retains receipts and advances only the
  ACL revision. Runtime-role direct property UPDATE is denied.
- Legacy import is fail-closed: 0038 has no first-rent/`paid_until`, so installation refuses
  unreconciled legacy rows; it never infers tenancy dates.
- Guildhall ownership is distinct from Character/Account ownership: a Guildhall occupies the
  unique Guild owner binding, never a personal Account slot.

Guildhall terminal-release participant (caller-transaction, consumes the real
`GuildDisbandTransaction`):
- Next-at-most-100-recipient reservation acquires sorted native deletion pins before the Guild
  spine; protected evidence stamped last; same physical transaction + database marker checked
  on every use; Guild/spine locks reacquired before property/legacy-anchor locks (an in-memory
  xid survives SAVEPOINT rollback); participant errors roll back a protected SAVEPOINT.
- SECURITY DEFINER entrypoints grant runtime EXECUTE only; no runtime DML on protected House,
  ownership, ACL or BED tables.
- Writer: immutable custody census; frees BED rows with `disposition`; transfers flat House
  items through `game_character_inbox_deliver`; immutable delivery/step evidence; deletes live
  ACL preserving receipts/editors; clears canonical and legacy Guild ownership with revision
  successors. A terminal journal requires zero House sources, occupants, admission
  reservations, live ACL, personal slots and open sleepers before the Guild terminal state 5.
- Raw node possession secret is bound as a separate SQL parameter, never serialized; uses the
  existing `NodeIncarnationProof::sql_possession_secret` (now in
  `crates/gameplay/durability/src/durability/runtime_scope_assignment.rs`); no new accessor.
- Trigger names `a_house_disposition_{content,provenance,item,entry}_fence` (renamed by
  HOUSE-1b so the House NOWAIT anchor refuses 55P03 before the generic DUR-03 BEFORE trigger
  refuses 23514); producer and consumer catalogue names must change together.

## Historical draft findings (superseded by the local checkpoint below)

- `cross_domain_finding: HOUSE-1a-BANK-HEADROOM` — SOCIAL-MAP §1.12 requires one shared
  credit-headroom check (`Bank balance + Market held + private House held + Guildhall Account
  held + incoming`). Bank deposit and receiver-transfer paths in `durability/bank.rs` apply
  `checked_add`/`BANK_BALANCE_MAX` directly with no shared helper. CP/architect must allocate this
  seam before live reserve/credit/settlement; it does not block the independent storage unit.
- Draft source-assembly result (not a qualification): ordinary property 6/6 PASS; Guildhall
  release 9 PASS / 4 FAIL. Three fail with SQLSTATE 23514 `house removal lacks actual source
  shape` (the native 0095 guard proves player Take/Move only; a Guild disposition must supply its
  own reservation/census/delivery proof). The fourth
  (`guild_house_runtime_exact_ports_without_direct_table_writes`) returned
  `Unavailable(Database(None))`, cause not established (D1409 diagnostic never reported).
  Guild-specific census/reservation/journal amendment needs a CP allocation.
- Unaccepted proposal: recipient-pin/late-NOWAIT lock-order amendment (required before
  Guildhall release activation).
- Tests must not use `tokio::join!` (macro not enabled); use the existing
  `item_transfer_postgres_cases::join_two` helper for two-connection races.
- PG test runners must clean up their private database after setup errors and assertion panics.

## Acceptance criteria

- [ ] Auction bid escrow and refund exactly-once under the session fence.
- [ ] Winner settlement burns price + first rent from escrow without a second balance debit.
- [ ] Premium gate fails closed; Premium batch gate runs before locks (SOCIAL-MAP §1.4).
- [ ] One bid/slot per Account+World; nomination/eligibility checked at settlement.
- [ ] Shared credit-headroom seam resolved before any credit path is activated.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [x] Structure: one mechanic per module file in its ADR-0022 layer (pure auction rules in `housing`, storage in `durability`).
- [x] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

All Cargo commands below used `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`.
No Cargo manifest/lock changed; package checks replace the historical workspace
Clippy command. The prohibited standalone PG target was not created.

```sh
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 check --locked -p oteryn-gameplay-durability --quiet
cargo +1.95.0 clippy --locked -p oteryn-gameplay-housing -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo +1.95.0 test --locked -p oteryn-gameplay-housing --quiet house_auction
cargo +1.95.0 test --locked -p oteryn-gameplay-durability --lib --quiet house_ownership_audit
cargo +1.95.0 test --locked -p oteryn-game-server --lib --quiet house_ownership -- --nocapture --test-threads=1
cargo +1.95.0 run --locked -p oteryn-architecture-check --quiet -- workspace .
```

## Reference evidence

- `docs/reference/tibia-manual/houses.md`
- `docs/agents/tasks/archive/OTV2-20261010-house-1-contract.md`
- `docs/agents/tasks/archive/OTV2-20261004-bank-1.md`
- CP coordination: #2131 (D1098, D1122, D1162, D1330, D1345, D1349, D1366, D1375, D1383, D1403, D1409)

## Local prerequisite audit — 2026-10-22

Baseline: `98b897f848ccfd75c21e48fb53091d5b5f7cd967` (the allocated lane HEAD).
The worktree was clean before this audit. No Git publication or external mutation
was performed. The bound META organization policy 3.1.0 was read at its immutable
binding revision `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

### Planned module layout

Before implementation: pure auction rules and their child tests in
`housing/src/house_auction{,_tests}.rs`; durable property/auction operations and
their child tests in `durability/src/durability/house_ownership{,_tests}.rs`;
Guildhall release in its separate allocated durability module only after the real
Guild owner exists. Any prerequisite gets its own mechanic module at its ADR-0022
layer. Shared parents get registration/wiring only. Migration allocation is
`0143`; it has not been created. New PostgreSQL cases must mount once through an
existing CI-registered target or library, superseding the new standalone target
named earlier in this historical card.

### Findings on the allocated baseline

- `PROVEN`: searches in `apps/`, `crates/` and `tools/` found no production
  `GuildDisbandTransaction`, canonical House disposition producer, House auction
  owner, or shared credit-headroom helper. Historical lifted Guild tests and
  `node/guild_jobs.rs` references do not supply the absent Guild owner.
- `PROVEN`: `content/project/native_house_tiles.rs` provides qualified placement
  metadata, not the full qualified acquisition catalogue required by D1383.
  `world/src/house_scope/mod.rs::exit_into_channel` still returns
  `ExitNotAdmitted`. HOUSE-RUNTIME-1b/1c are not implemented on this baseline.
- `PROVEN`: HOUSE-1b's required disposition steps lock recipient Character roots
  before property/items and require the native-trigger NOWAIT property pin.
  HOUSE-1a additionally requires sorted native deletion pins before the Guild
  spine for its terminal-release participant.
- `CONFLICT`: accepted GUILD-0 section 4.1 explicitly says only settlement's
  first step takes Character roots; disposition/release/disband jobs take none
  and start at class 3. `docs/contracts/OTERYN_GAME_GUILDS_V1.md` section 3
  requires rollback on an out-of-order class. HOUSE-1b's LO-01 finding and this
  card's recipient-pin/late-NOWAIT finding explicitly leave the amendment open.
  No accepted amendment was found in the relevant contracts/architecture files.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

The owner's continuation supersedes the previous stop: GUILD0-LO-01 remains
accepted, without amendment. Disposition, Guildhall terminal release and their
recipient deletion pins are explicitly **out of scope**, deferred to HOUSE-1b
because their lock-order amendment is not accepted. No such producer or private
fixture fragment was installed here.

Implemented prerequisites on existing owners:

- `bank_credit_headroom.rs`: one native credit-headroom function, called by both
  existing Bank deposit and receiver-transfer paths. Balance plus HELD House
  custody stays within the real Bank ceiling; reserve/return conserves the sum.
  Market/Guildhall value writers are absent on this baseline and must extend the
  same SQL sum when their registered storage lands.
- Migration 0143's canonical property/tile/slot owner supplies the existing
  `game_house_access` / `game_house_access_tile` ports, with separate grant/fence
  seams and a nonzero initial global ACL revision. Runtime only receives bounded
  locking/EXECUTE ports and reads, without protected House table DML.

Backend implementation:

- L3 owns proxy pricing, durable-order ties, 256-bid limit, checked clocks,
  escrow value plans and PhysicalHouseEligibility/v1 with stable audit reasons.
- L4 owns native session/recovery/admission fencing, real Premium single/batch
  gates, transaction-preserving pre-delivery savepoints, request binding/replay,
  named occurrence/Account locks, election, first-rent burn and durable refunds.
- One Account is refunded per transaction, within the accepted at-most-100
  ceiling. Election handles only its winner (or the first excluded refund).
  This preserves one House event per physical transaction; the four-line
  acquisition subshape stays within DUR-03's unchanged 200-line House ceiling.
- Event 8 records the qualified bid calculation's captured eligibility census,
  reasons, maxima/order, price, owner/slot/generation transitions and first paid
  period. Its outbox reuses Bank's publication/expiry and event-keyed legal-hold
  guard, under the existing P30D economy profile. Ledger aggregate and chain
  proofs forbid extra same-transaction entries or fabricated custody returns.
- Legacy installation refuses unreconciled anchors without inferring dates.
  Canonical/legacy/slot consistency is deferred and bidirectional; anchors and
  retired properties cannot be removed/reopened. ACL receipts remain immutable;
  ACL changes/deletion advance only their separate global revision.

### Validation and file-size record — 2026-10-22

- PASS: package check; Clippy `-D warnings` for all targets of all three touched
  packages; workspace rustfmt; architecture `workspace-boundaries`.
- PASS: six `house_auction` rule tests; one `house_ownership_audit` test including
  four value lines, 256/257 census bounds, envelope integrity and transition fields.
- PASS: seven filtered library PG cases, **actually run on PostgreSQL 17.6**.
  Native runtime-role connections and real Premium/recovery owners were exercised;
  cases return `Ok(())` before setup only when the configured admin is absent.
  Positive auction data is explicitly historical recovery DATA, not D1383
  catalogue qualification. The existing ACL SQL fixture is reused, with explicit
  historical receipt transaction boundaries; no assertions were relaxed.
- One earlier House settlement run returned `CommitOutcomeUnknown` during high
  shared-PC load; its isolated rerun and final complete seven-case run passed.
  This own-case failure was not classified as an unrelated CI-only failure.
- PASS: governance validator (22 documents / nine lanes), 124 governance tests,
  full-diff checklist review and `git diff --check`. Workspace-wide constructor,
  enum/match/impl searches and all three protobuf field-tag comparisons completed.
  No trait method was added. Completeness checklist items 13/14 remain open below.
- Reuse: Bank balance locks and UUID allocator (visibility only, no copied body);
  native `assert_item_only_fence`, recovery/admission locks, incarnation proof and
  its existing separate SQL possession-secret binding; real Premium gate owner;
  existing Bank junior rule, foundation envelope and bounded encoder; existing
  Bank immutable/publication/expiry/legal-hold guards; shared Bestiary native PG
  harness and existing ACL fixture; repository governance/architecture validators.
  Harness setup was extracted **in its original file**, retaining the same body,
  with database cleanup on setup failure/panic. Case bodies also clean up on panic.
- Files growing by more than 200 lines (all new): migration 0143, **544 lines**
  (durable schema, privileges, conservation and transition proofs); ownership audit,
  **254 lines** (DUR-owned atomic receipt/evidence encoding); PG owner tests,
  **312 lines** (native owner/session/recovery cases mounted exactly once).
  All remaining new Rust modules are below 200 lines. Existing large source files
  only receive registration, routing or visibility lines; the resource registry grows
  by 123 registration lines and the event registry by 12. No existing migration or CI policy/workflow was edited.

### Unfinished / next step

This is a validated backend checkpoint, **not completion of HOUSE-1a**:

- Fresh first-bid auction opening is not implemented: D1383's qualified catalogue
  producer and the accepted HOUSE-RUNTIME-1b/1c entry/exit prerequisites remain
  absent. A VACANT anchor is explicitly refused; historical auction fixtures do
  not substitute for these producers or their acceptance.
- `DurabilityRoot::house_bid` and `house_settlement` are backend entry points,
  currently called by the native PG qualification, without serve/node scheduling
  or real command-dispatch E2E. SOCIAL-MAP §1.12 assigns runtime wiring to
  HOUSE-WIRE-1; D1462's final production-caller/dispatch qualification is therefore
  still open, and no new capability was offered to bypass that boundary.
- Durable deterministic refusal receipts, multi-bidder/concurrent PG qualification
  and the full activated acquisition path still need completion. Successful value
  operations and job-step replays are durable; rejected fresh requests currently
  return typed errors rather than storing terminal refusal receipts.
- Disposition, ongoing rent, Guildhall release and recipient pins remain the
  explicitly deferred HOUSE-1b scope. All blocking dependency/activation decisions,
  independent exact-candidate review, repository CI and publication remain pending.

Next: complete the qualified opening/runtime prerequisites and refusal journal,
then qualify the full acquisition/dispatch path before claiming the task complete.
Local HEAD remains `98b897f848ccfd75c21e48fb53091d5b5f7cd967`; detached worktree,
no freeze SHA or PR, no publication/merge/review trigger. The record stays active.

### Port onto the DISPATCH-WIRING-1 head — 2026-10-23

The backend checkpoint was re-applied onto `de3fd2190` (DISPATCH-WIRING-1 with
REGISTRY-SPLIT-1, Rust 1.99.1 and PostgreSQL 18.6). Adaptations only, no behaviour change:

- REGISTRY-SPLIT-1: the eleven new resource limits are one file each,
  `docs/contracts/resource-limits/entries/0598-DUR03-RL-03-HOUSE.json` through
  `0608-HOUSEOWN0-RL-07.json` (renumbered from 0592-0602 after SPECTATE-1b took 0592-0597), in the original order; the aggregate file is not recreated.
- PostgreSQL 18.6: the harness setup and the runtime-role check read
  `CANONICAL_POSTGRES_SERVER_VERSION_NUM` instead of the literal 17.6 version.
- Migration `0143` keeps its number (no collision with main's 0130, 0131, 0132, 0136,
  0140, 0144). House event type 8 is still unique in the event registry.
- No dispatch file was touched. Protocol dispatch and scheduling stay with HOUSE-WIRE-1
  (SOCIAL-MAP §1.12 "No runtime wiring"; HOUSE-WIRE-1 owns `HOUSE_V1`/`HOUSE_INTENT`,
  whose numbers CP leases). The production-caller and real-dispatch E2E requirement for
  `house_bid`/`house_settlement` is not met here.
- Owner decision 2026-10-23 (option a): HOUSE-1a lands dormant, like DAILY-1a. The
  production caller and the real-dispatch E2E for `house_bid`/`house_settlement` are a
  HOUSE-WIRE-1 gate. Migration `0143` stays: main already has out-of-order migrations
  and no deployed database exists.

Validation on the port (`cargo +1.99.1 ... --locked`): rustfmt, `git diff --check`,
Clippy `-D warnings --all-targets` for housing, durability and game-server, the
`house_auction` and `house_ownership_audit` unit tests, the seven `house_ownership`
library PG cases on PostgreSQL 18.6, repository policy, governance and architecture checks.
