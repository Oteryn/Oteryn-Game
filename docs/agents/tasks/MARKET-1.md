# MARKET-1 — world market (skeleton task record)

```yaml
task_id: MARKET-1
title: "MARKET-1: world market"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row MARKET-1 (row 122); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (MARKET-CONTENT-1, INBOX-1b, MARKET-1, MARKET-WIRE-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/market-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

World Market: operations, offers, escrow, book lock, place/accept/cancel/expiry and matching, market event and every §8 delta (MARKET-1); marketable facts (MARKET-CONTENT-1); MARKET_V1 wire and Inbox view (MARKET-WIRE-1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

MARKET-CONTENT-1 (content) -> MARKET-1 (hard, persistence/economy/security) -> MARKET-WIRE-1 (Reg, after DEPOT-WIRE-1). INBOX-1b after DEPOT-1. MARKET-TIER-1 belongs to ITEM-FORGE-1.

## Dependencies (reconciled with protected main@3ef900c1, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| NPC-ECON-1 (family entry gate) | BLOCKING (Wave 2, not merged) |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| ECON-RET-0 / MARKET-RET-0 | MERGED (#1748) |
| BANK-1 | MERGED (#1766) |
| GOLD-FEE-2 | MERGED |
| INBOX-1a | MERGED (#1785) |
| MARKET-CONTENT-1 (own child) | BLOCKING (not started) |
| DEPOT-1, DEPOT-WIRE-1 (INBOX-1b, MARKET-WIRE-1) | BLOCKING |
| acceptance of MARKET0 | ACCEPTED: effective ACCEPT-BATCH #2209 §1; stale draft heading superseded |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/market.rs`
- `apps/game-server/src/durability/market.rs`
- `apps/game-server/src/durability/market_audit.rs`
- `apps/game-server/src/durability/market_tests.rs`
- `apps/game-server/src/gameplay_transport/market.rs`
- `apps/game-server/src/gameplay_transport/market_tests.rs`
- `crates/protocol-oteryn/src/market.rs`
- `crates/protocol-oteryn/src/market_tests.rs`
- `apps/game-server/tests/market_postgres.rs`
- content authoring inputs for `trade.marketable` / `trade.market_category` (source side only)
- `docs/agents/tasks/MARKET-1.md` (this record: status and notes only)

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

- durability/bank.rs + bank_audit.rs (BANK-1) for escrow debit/credit; reuse, do not fork
- durability/character_inbox.rs for delivery
- apps/game-server/src/content/item_market_true_promotion.rs (existing market flag promotion)
- connection.rs dispatch chain; capabilities.rs `GATED` (`MARKET_V1`)
- GAME_EVENT_FOUNDATION_REGISTRY.json (market event), RESOURCE_LIMITS_REGISTRY.json

## Inseparable bundles (never split across PRs)

- content (MARKET-CONTENT-1) + regeneration + pins by one author
- migration + market writer + durability tests
- escrow = bank debit + offer row + ledger in one transaction (mint/burn + ledger)
- wire + registry + capability + handler + tests (MARKET-WIRE-1)

## Acceptance criteria

- [ ] Accept of an offer moves gold and item atomically; double accept loses deterministically under the book lock.
- [ ] Expiry returns escrow to Inbox/bank exactly once after restart.
- [ ] Market fee burn uses `FeeBurnCause::MarketFee` (D238) and balance hard ceiling holds.
- [ ] Every delta of MARKET0 §8 has a durability test.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server market
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md §3-§10, §13
- apps/game-server/src/durability/bank.rs
- archive OTV2-20261004-gold-fee-2.md
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Owned component preparation (2026-10-15)

Exact allocated draft base: `5eda6caeeed134212e3f71ad4e40b958c6ae21a3`; no merge/rebase.
Owning acceptance is protected main `3ef900c160c9713092a86bfcd1a8377c7587c898`, effective
ACCEPT-BATCH #2209 §1 and MARKET0 §§4/6/7/8. MARKET-COINS-0 remains CANDIDATE and is excluded.
This round's narrow allocation supersedes the broader skeleton lease and module-registration
permission: only domain/market.rs, durability/market.rs, market_audit.rs, market_tests.rs,
tests/market_postgres.rs and this card are writable. Shared wiring/schema/registry/content
files remain unchanged. No audit codec or inaccessible standalone root wrapper is fabricated.

### Concrete storage/selection subset

The new private domain owns accepted ITEM offer states, checked price/amount/fee rules,
escrow/held-credit state validation, maker-price matching terms and noncommitting cancel,
expiry and held-credit-release requirements. Stored typed definition references identify an
offer's historical ware/revision; they do not admit `marketable`, default instance state or
current DUR-03 §46 compatibility. No synthetic ware is admitted and no Coin kind is added.

The native storage component holds a real transaction-bound World+definition book row lock,
reads/validates persisted offers and locks matching pairs in ascending offer-id order. It
selects the oldest pending offer and best crossing counter (price, oldest, id), excluding
same-Account offers and expired/future-created rows. Different definition revisions remain
explicit, awaiting real compatibility resolution. No touched-item limit is confused with
an amount limit: a match's economics needs independent canonical custody selection of at
most 100 items before execution. A busy book is observable, never silently cleared.

Job book acquisition uses existing offer-FK book rows with `FOR UPDATE SKIP LOCKED`; it does
not first block in a conflicting upsert. Expiry discovery is an unlocked, World-scoped,
oldest-first page of at most 100 candidates. Each must be re-read after acquiring its book;
an already-ended candidate cannot produce a new expiry plan. Source escrow reads lock actual
native Item rows in ItemInstanceId order, bound to the re-read offer and exact definition
revision/World, with at most 100 entries and exact OPEN-offer unit totals.

Operation history has globally unique, disjoint CommandRef and (offer, job kind, positive
step) occurrence keys, immutable persisted outcome bytes and TransactionId, and explicit
World/actor/kind/SHA-256 binding conflict checks. A changed World cannot make an already
recorded occurrence appear absent. It does not invent a public outcome codec or reconstruct current
authority from receipts. The eventual owner must run actual native recovery checks before
history lookup, return matching history before fresh session/state checks, and obtain the
real current admission/session/root fences before book acquisition. These requirements are
integration obligations, not implemented authority claims of this read component.

The only write exposed here is a value-neutral book-row upsert. No offer transition, Market
operation, bank entry/balance, escrow placement, Inbox delivery or audit event is committed
by this component. End and match plans cannot be mistaken for an atomic Market operation;
the stored offer remains unchanged after planning. No CharacterRevision is advanced.

### Native validation and concrete CP hooks

The schema in market_tests.rs is an explicit isolated admin-owned proposal: operations,
books, offers and escrow locations plus local state constraints. It is not an installed
migration or production privilege boundary. Administrative historical Item fixture loading
follows existing Inbox tests; native mint/placement guards are disabled only for that load.
Those fixtures do not qualify custody provenance, conservation, gameplay admission or any
current authority. All actual component reads and locks execute against native PostgreSQL
tables/types, not a replica of the server model.

Validation uses an outside-tree native snapshot with only literal module registrations,
then a separately disclosed cfg(test) rlib bridge removing only this task's test attributes
and appending forwarding dispatchers. Exact test bodies, native types/helpers and fixtures
are preserved. The original test-bearing source receives native unit Clippy metadata checks;
the small runner avoids the full server unit ELF memory limit. Results/hashes and every
bridge delta are recorded in `/workspace/oteryn-prep/market-1-evidence`.
The initial bridge preparation case-count error occurred before Rust (setup failure).
The first actual native unit metadata check found new-code import/fixture-SQL/unused-binding
compiler errors; that is preserved as a real new-code compile failure, not mislabelled setup
or a preexisting product regression. Corrected results are recorded separately.
The corrected original-context native unit Clippy metadata check passed (`-D warnings`).
The disclosed native cfg(test) rlib/forwarding runner then executed all 11 unchanged test
bodies: 4 pure and 7 actual PostgreSQL 17.6, zero failed/ignored, 9.50 seconds. These cover
state/price/fee/held-credit bounds, scoped immutable offer reads and noncommitting plans,
oldest/best matching with same-Account filtering and explicit revision differences, real
SKIP LOCKED/release, 100-step expiry discovery/recheck, immutable global operation binding,
native Item escrow reads with substitution/101-item refusal, and proposed schema guards.
Workspace/owned-source formatting, governance plus 124 tests, diff and complete-patch
reverse-apply/whitespace checks passed. This qualifies the stated component in the disclosed
native overlay, not a full server unit ELF, production Market writer, or all-target CI.

**cross_domain_finding / atomic CP slot:** BANK-1's ledger/cause/event are closed to bank
Conversion/Transfer, and GOLD-FEE-2's `insert_fee_debit_entry` references a fee record rather
than a Market operation. `FeeBurnCause`/its audit oneof currently admit only CharmUnassign.
The Market operation reference, five Market ledger kinds, MarketFee bank-only burn, nullable
job actor/counterparty, hard versus voluntary ceiling and one Market event must be added
together. Reusing the separate root-advancing fee writer would break atomic ownership.

Inbox migration 0076 grants runtime no EXECUTE on `game_character_inbox_deliver`; cause kinds
are caller-owned and its current counter is entries only. The Market definer must supply
reserved delivery/return accounting without inventing a runtime public-delivery API. The same
slot must extend custody exclusivity, placement/removal/split evidence, reservation/receipt
shapes and deferred per-operation gold+item conservation, install production migration and
definer/grants, event/retention wiring and admitted content/compatibility/depot/current actor
consumers. Only then may offer state and every bank/item/Inbox effect commit together.
The current native `ReferenceItemTradeRestrictions` exposes typed `marketable` but has no
`market_category`; MARKET-CONTENT-1 must provide/admit both accepted ware facts, not infer a
category from stored offer keys or test fixture definitions.
Full MARKET-1, every acceptance checkbox, wire and architect qualification remain pending.

### ROOT-CAŁA-RODZINA: owned My Offers read (2026-10-16)

Returned only MARKET to AUTHORING from `98be1ad4b879fcae341917572143cac76e753ecb`.
Added `open_character_offers` in the allocated durability leaf: one snapshot scoped by
World and Character, newest-created first with deterministic offer-id ties, at most 100
OPEN offers plus one overflow sentinel. It decodes and validates every returned row, retains
anonymous own offers and expired OPEN rows awaiting the expiry job, and rejects overflow
instead of hiding it. This is private read data; caller-owned recovery/session/depot-view
admission remains required. No shared registration, schema, writer, event or wire changed.

Two native PostgreSQL tests cover owner/World isolation, ordering, anonymous and expired
OPEN visibility, exclusion of ended/held-credit rows, and the exact 100/101 boundary.
Rustfmt and whitespace checks run locally. Cargo compilation and both new PG bodies are
**NOT_RUN**: this workspace has no Cargo executable; no alternate harness or new PC runner
was used. This candidate does not inherit the old component qualification for changed files.
MAIL c8ade87f PASS and the untouched NPC 8196b1e7 job retain their existing evidence.

This completes the OPEN slice, not the last-600-ended slice. The proposed offer schema has
no end-time/sequence field; ordering endings by creation would invent history semantics.
History retention/order therefore needs the CP Market schema decision alongside its
numbered migration. Existing Market Bank/Inbox/custody/Content/wire gates remain open.
Preserve this candidate locally and in a verified bundle outside the checkout until required
checks/publication policy allow its existing-draft push (CP question #2131 comment6224104932).
