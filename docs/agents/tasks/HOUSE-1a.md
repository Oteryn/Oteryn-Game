# HOUSE-1a — house ownership, auction and first rent (consolidated task card)

```yaml
task_id: HOUSE-1a
title: "HOUSE-1a: house ownership, auction (bid escrow) and first rent"
mode: IMPLEMENT
status: waiting
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

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. House custody/interior foundations exist, but no HOUSE-1a auction/rent ownership writer is retained. Evidence: `crates/gameplay/durability/src/durability/house_interior.rs`.

Remaining: Auction escrow/refunds, eligibility, winner settlement, rent and ownership lifecycle remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

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

- `crates/gameplay/housing/src/house_auction.rs` (new, L3 pure rules: proxy price, deadlines, value plans)
- `crates/gameplay/housing/src/house_auction_tests.rs` (new)
- `crates/gameplay/durability/src/durability/house_ownership.rs` (new, PostgreSQL property/tiles/slot storage)
- `crates/gameplay/durability/src/durability/house_ownership_tests.rs` (new)
- `crates/gameplay/durability/src/durability/house_guildhall_release.rs` (new, Guildhall terminal-release participant; only after the Guild core is back on main)
- `crates/gameplay/durability/src/durability/house_guildhall_release_tests.rs` (new)
- `apps/game-server/tests/house_ownership_postgres.rs` (new)
- `docs/agents/tasks/HOUSE-1a.md`

Slot-only (final commit at the queue turn): the migration (number: next free number at
allocation time (only one migration PR in the queue)); module registration in
`crates/gameplay/housing/src/lib.rs`, `crates/gameplay/durability/src/durability/core_modules.rs`
/ `lib.rs`; `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`,
`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`.

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

## Findings and open questions

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
- [ ] Structure: one mechanic per module file in its ADR-0022 layer (pure auction rules in `housing`, storage in `durability`).
- [ ] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-housing
cargo test -p oteryn-gameplay-durability house_ownership
cargo test -p oteryn-game-server --test house_ownership_postgres   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/reference/tibia-manual/houses.md`
- `docs/agents/tasks/archive/OTV2-20261010-house-1-contract.md`
- `docs/agents/tasks/archive/OTV2-20261004-bank-1.md`
- CP coordination: #2131 (D1098, D1122, D1162, D1330, D1345, D1349, D1366, D1375, D1383, D1403, D1409)
