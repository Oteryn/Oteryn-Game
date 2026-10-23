# MARKET-1 — world market (skeleton task record)

```yaml
task_id: MARKET-1
title: "MARKET-1: world market"
mode: IMPLEMENT
status: IN_PROGRESS
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
checkpoint: BOUNDED (storage/query/selection components only; see last section)
```

## Goal

World Market: operations, offers, escrow, book lock, place/accept/cancel/expiry and matching, market event and every §8 delta (MARKET-1); marketable facts (MARKET-CONTENT-1); MARKET_V1 wire and Inbox view (MARKET-WIRE-1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

MARKET-CONTENT-1 (content) -> MARKET-1 (hard, persistence/economy/security) -> MARKET-WIRE-1 (Reg, after DEPOT-WIRE-1). INBOX-1b after DEPOT-1. MARKET-TIER-1 belongs to ITEM-FORGE-1.

## Dependencies (verified against origin/main `43914f183`, 2026-10-23)

Every row was checked using `git log --oneline origin/main --grep <ID>` and
searches of `crates/` and `apps/`; a reference-only commit does not establish delivery.
The owner's current pass supersedes historical slot-only restrictions below for
rules, the additive migration, split registry entries and auto-mounted PG cases.

| Dependency | Real state and evidence |
|---|---|
| ITEM-LOOP-1 | MERGED: `a43c37f12` (#2190); existing item dispatch and native item owner. |
| NPC-ECON-1 | MERGED: `5e7a3b771` (#2197); existing NPC trade/Bank composition. |
| PREMIUM-1 / PREM-WIRE-1 | MERGED: `d8b3191b2` (#2110); existing fail-closed Premium gate. |
| ECON-RET-0 / MARKET-RET-0 | MERGED: `19effa26e` (#1748); economy/market retention profiles. |
| BANK-1 | MERGED: #1766; native `durability/bank.rs`, migration 0071 and Bank audit. Market operation ledger integration remains absent. |
| GOLD-FEE-2 | MERGED: `77197a1f8` (#1781), activation `2f8d35bcb` (#1850); Market's bank-only fee integration remains absent. |
| INBOX-1a | MERGED: `69f171fcb` (#1785), migration 0076 and `character_inbox.rs`. |
| MARKET-CONTENT-1 | BLOCKING: grep only finds reference/task preparation (`f93aecf6d`, #2293). No admitted `market_category` field exists in native Rust. Existing `marketable` promotion alone is insufficient for MARKET0 §3.1. |
| DEPOT-1 | MERGED: `43914f183` (#2334), migration 0151, single `character_depot` owner and native Inbox delivery target. Dormant storage, no locker/fenced move writer. |
| DEPOT-WIRE-1 / INBOX-1b | BLOCKING: no Market/depot wire seam, capability or handler; depot view admission and Inbox out-shapes remain absent. DEPOT-0 references are not implementation. |
| acceptance of MARKET0 | ACCEPTED: `e2da9805d` (#2209), retained MARKET0 contract. |
| ITEM-IMBUE-1 (additional current owner) | MERGED: `3bb98bcbc` (#2333), migration 0150. No parallel imbuement or depot owner is introduced. |

The genuinely unmerged content/admission dependencies select the owner's fallback:
pure rules plus own dormant storage only. No schema/API from an unmerged dependency
is assumed. Atomic value movement cannot qualify until those owners and DUR-03's
custody, Bank/fee, Inbox reservation and event proof are composed together.

## Reference parity

Reference snapshot: Canary
[`04b83b512114bfd888000d6e1433ed8ecaec7c5b`](https://github.com/opentibiabr/canary/tree/04b83b512114bfd888000d6e1433ed8ecaec7c5b).
Read the source files on 2026-10-23. The current official manual now describes
2.5% / 2,500,000, while the task and accepted MARKET0 require this pinned 2% /
1,000,000 ruleset; no silent update to today's values is made.

| Rule from reference | Value implemented | Table-driven L3 assertion |
|---|---|---|
| [Offer limit, config.lua.dist](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/config.lua.dist#L444) | 100 open offers per Character, buys and sells together | `market_reference_numeric_rules`, open offers row; native race: `market_storage_capacity_serializes_last_slot_across_sides_and_books` |
| [Stackable amount, game.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp#L10696) | 1–64,000 units; MARKET0 further limits custody to 100 touched native items | `market_reference_numeric_rules`, units row + bounds cases |
| [Piece price, game.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp#L10676) | 1–999,999,999,999; MARKET0 §3.2 also caps total at this value | `market_reference_numeric_rules`, price row + total boundary cases |
| [Minimum fee, game.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp#L10741) | 20 gold | `market_reference_numeric_rules`, minimum fee row |
| [Fee rate, game.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp#L10740) | 2% of total | `market_reference_numeric_rules`, 2 percent row |
| [Integer fee conversion, game.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp#L10740) | Floor fractional gold using exact integer division by 50 (avoids reference floating-point imprecision) | `market_reference_numeric_rules`, floor fractional gold row |
| [Maximum fee, game.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp#L10742) | 1,000,000 gold | `market_reference_numeric_rules`, maximum fee row |
| [Duration, config.lua.dist](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/config.lua.dist#L440) | 30 days, exact expiry boundary | `market_reference_numeric_rules`, duration row; `market_reference_sides_anonymity_and_expiry` |
| [Both sides and anonymous flag, game.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp#L10717) | Buy/sell accept either flag, retaining private owner identity; public name redaction awaits wire | `market_reference_sides_anonymity_and_expiry`, side × anonymity rows |
| [Expiry handling, iomarket.cpp](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/io/iomarket.cpp#L237) | Pure plan returns remaining sell units or buy gold at expiry; durable delivery remains unfinished | `market_reference_sides_anonymity_and_expiry`, expiry rows |

## owned_paths (builder edits freely)

- `crates/gameplay/domain/src/domain/market.rs`
- `crates/gameplay/durability/src/durability/market.rs`
- `crates/gameplay/durability/src/durability/market_audit.rs`
- `crates/gameplay/durability/src/durability/market_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/market.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/market_tests.rs`
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
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- crates/gameplay/durability/src/durability/core_modules.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- durability/bank.rs + bank_audit.rs (BANK-1) for escrow debit/credit; reuse, do not fork
- durability/character_inbox.rs for delivery
- crates/gameplay/content/src/content/item_market_true_promotion.rs (existing market flag promotion)
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
- crates/gameplay/durability/src/durability/bank.rs
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

### Owned book-query completion (2026-10-16)

Returned MARKET to AUTHORING from local `9be1efda` while preserving its verified bundle.
Completed the second accepted MARKET0 §10 read: private book pages of 32 offers with a
33rd-row continuation sentinel, best price first (sell ascending, buy descending), then
creation time and offer id. Keyset continuation binds World/ware/side; invalid cursor
prices/times or cross-book/side reuse fail closed. A single-statement timestamp excludes
expired and future OPEN rows. Stored anonymous rows remain private adapter input; native
depot admission, Content marketability and anonymous wire redaction remain caller gates.
No offers, value, custody, operation, schema, wire or shared registration are mutated.

Added two native PG tests for both price directions, time/id ties, the exact 32+3 split,
continuation, cross-book/side cursor rejection, and expiry/future/ended/ware/World filters.
Rustfmt/whitespace checks pass; all four new PG tests and Cargo remain **NOT_RUN**.
The existing reviewed PC runners admit only their exact frozen MAIL/NPC candidates, not
this Market candidate; no runner was rewritten, restarted or bypassed to test it. Local
Cargo is absent. Request a compatible native validation allocation through #2131; keep
this owned checkpoint local/bundled pending actual validation and publication rules.

Independent review found P2 in initial book pagination: the text projection alias would
make ORDER BY sort lexically. Fixed both sides to name the numeric table column and changed
the price fixture to 2/9/10/100/101 so an actual PG run detects the regression. Disposition:
fixed in source; execution remains NOT_RUN until compatible native Cargo validation.

### CP D1160/D1122: own module registrations and checkpoint (2026-10-16)

Status NOT_FULLY_QUALIFIED. CP comment6224912485 grants minimal own-module registrations
without a slot; register domain::market and durability::market so library CI compiles the
actual components. No other shared line is changed. This does not grant an alternate
composition of standalone roots: their Content/private-domain ABI compatibility remains
unqualified until real Cargo checks run. New own reads/four PG tests: NOT_RUN; fmt/diff
PASS and static review only. D1122 comment6224930201 supersedes the earlier publication
hold for one coherent existing-draft checkpoint; it does not grant ready/merge or full PASS.

## Documentation-only successor and predecessor CI — 2026-10-17

Status remains **NOT_FULLY_QUALIFIED**. The predecessor GitHub read confirmed draft #2134 at
`3e061eb9784d307cb73d4e69b59c17c6bf1d0d82`, branch `skeleton/market-1`.

CP D1310 permits this documentation-only successor checkpoint on the existing
draft branch. This increment changes only this task record; Rust implementation
and test sources remain unchanged from the predecessor above. **New successor
Cargo/PG/CI checks: NOT_RUN.** The recovered results below remain bound to the
stated predecessor SHA and are not new-commit qualification. Publication permission
does not grant runtime activation, readiness, merge or shared integration.

The current source supersedes the historical statement that the last-600-ended
reader is absent: `ended_character_offers` is implemented using stored ending
time and sequence, and three ending-history cases are present. This is a private
reader over a proposed schema, not an installed ending writer.

The current canonical registration mounts
`tests/support/market_postgres_cases.rs` from
`tests/character_authority_postgres.rs`. Its inventory is **15 PostgreSQL cases**:
two own-open-offer, two book-page, three ended-history, seven original native
storage/lock/selection/operation/escrow/schema cases, and one persisted-definition
validation case. Five pure cases remain in the ordinary domain module.
The owned source was inspected once in this continuation; no demonstrated product
defect or reason to alter the read components was found.

GitHub Actions Rust job
[117216200836](https://github.com/Oteryn/Oteryn-Game/actions/runs/38973020053/job/117216200836)
explicitly checked out the SHA above. Strict all-target Clippy passed. Its library
run reported **3,155 passed / 0 failed / 29 ignored**, and the operations binary
**6 passed**. All **five MARKET pure cases** appear as `ok`.
The exact-head check collection contains **15 success / one skipped** (World
bundle), with no pending or failed check at the read.

Canonical PostgreSQL on that implementation predecessor remains **NOT_RUN in this continuation**.
The inspected Rust job ran unit/bin targets; the successful S3-B job ran only its
selected composition case. Neither executes `character_authority_postgres`.
Earlier native component evidence does not qualify these later query and private
schema-fixture additions.

Focused canonical command, after configuring the allocated disposable PG service:

```bash
cargo +1.94.0 test --locked --offline -p oteryn-game-server --test character_authority_postgres market_postgres_cases:: -- --test-threads=1
```

The existing harness requires `OTERYN_TEST_POSTGRES_ADMIN_URL` ending exactly in
`/postgres`, PostgreSQL 17.6 (`server_version_num=170006`), and privileges to
create/drop isolated test databases, apply native migrations and seed the disclosed
administrative fixtures. Without that variable the harness reports noncanonical
execution and returns early; that outcome is not PG qualification.

The production Market writer, migration/privileges, ending metadata producer,
Bank/fee/Inbox/custody/conservation/audit integration, current Content and
compatibility, depot/actor admission, jobs and wire remain undelivered. Shared
changes belong to the owning control-plane slot. No runtime activation, ready,
merge, full-family completion or new paid review is claimed.

## CP bounded checkpoint — 2026-10-17

`MERGE_AUTHORITY: CP_ONLY`. Status stays **NOT_FULLY_QUALIFIED** for the task as a
whole. This PR is offered as a bounded checkpoint: it lands the checked offer domain
rules and the private storage/query/selection components, nothing that commits value.

- Merged current main (`f094e032`); the only conflict was the `domain/mod.rs`
  registration line (kept both `mail` and `market`).
- Compiled code: `domain::market` and `durability::market` are registered modules;
  `tests/support/market_postgres_cases.rs` is mounted from
  `tests/character_authority_postgres.rs`. No uncompiled Rust source is in the PR.
- Validation on the merged tree (shared PostgreSQL 17.6 through `in-pg-netns.sh`):
  `cargo fmt --all --check` PASS; `cargo clippy --locked -p oteryn-game-server
  --all-targets -- -D warnings` PASS; `cargo test --locked -p oteryn-game-server --lib`
  3,377 passed / 0 failed / 29 ignored (all five MARKET pure cases ok);
  `cargo test --locked -p oteryn-game-server --test character_authority_postgres
  market_postgres_cases:: -- --test-threads=1` 15 passed / 0 failed against
  PostgreSQL 17.6 (73.8 s). Governance validator and `tools/agents/tests` PASS.

Out of this checkpoint and still open (MARKET-2 and the owners named above): the
atomic place/accept/cancel/expiry writer, production migration and privileges,
Bank ledger/fee (`FeeBurnCause::MarketFee`), Inbox delivery/return, custody,
audit/outbox, the ending-metadata producer, Content facts, admission, jobs and wire.
The acceptance criteria above remain unchecked.

## Completion + wiring (post-checkpoint)

Imported from closed draft #2229 and rebased onto the ADR-0022 layout (`docs/architecture/ADR-0022-gameplay-crate-layering.md`); every existing path below was verified on origin/main `59785d76`. The checkpoint merged unfinished; nothing in the running game calls it. The completion PR must not merge until the mechanic is playable end to end through the real dispatch path, CI is green and Codex review is clean. Dispatch chain: `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` (command `else if` chain) and the `GATED` table in `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` (highest capability id on main: 30). Wire modules live in `crates/protocol-oteryn/src/`. Migration numbers: next free number at allocation time (only one migration PR in the queue); 0130 MAP-PICKUP-1, 0131 BAGS-USE-1 and 0132 (provisional TIMED-RT-1c) are reserved.

Structure rules (binding acceptance criteria):
- [ ] One mechanic per module file in its ADR-0022 layer (pure rules L3, PostgreSQL in `durability`, runtime L4, dispatch L5).
- [ ] New tests in their own child files, not appended to existing files.
- [ ] Edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] No file over ~1000 lines grows by more than ~150 lines (`connection.rs` is ~8,260 lines).

### Present on main (#2134)
- Pure offer rules: `crates/gameplay/economy/src/market.rs` (+ `market_tests.rs`) and `crates/gameplay/domain/src/domain/market.rs`; private storage/query/selection: `crates/gameplay/durability/src/durability/market.rs`. Its only consumer is `apps/game-server/tests/support/market_postgres_cases.rs` (15 PG cases).
- No migration, no protocol module, no capability, no handler, no job, no event, no content facts.

### Checklist
- [ ] **Persistence/migration**: Market migration (next free number at allocation time; only one migration PR in the queue): operations, offers, escrow, book-lock rows, ledger links, ending metadata, game-role privileges. Install the same schema in the PG test harness instead of the private per-test DDL in `market_postgres_cases.rs`.
- [ ] **Atomic writers** in `crates/gameplay/durability/src/durability/market.rs` with a new sibling `market_audit.rs` (new): place, accept, cancel, expiry/return. Escrow = Bank debit (`crates/gameplay/durability/src/durability/bank.rs`) + offer row + ledger in one transaction; fee burn via `FeeBurnCause::MarketFee` with balance ceiling; item custody via `crates/gameplay/durability/src/durability/character_inbox.rs` (delivery and return); losing concurrent writer typed; restart-safe expiry that returns escrow exactly once.
- [ ] **Event/audit/outbox**: market event in `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`, limits in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, ending-metadata producer.
- [ ] **Content**: MARKET-CONTENT-1 `trade.marketable` / `trade.market_category` facts; regenerate content and pins via `tools/content-migration/regenerate_content.py`; reconcile with `crates/gameplay/content/src/content/item_market_true_promotion.rs`.
- [ ] **Protocol**: new `crates/protocol-oteryn/src/market.rs` (+ `market_tests.rs`): `CAPABILITY_MARKET_V1` (next free id), browse / my offers / place / accept / cancel commands, state domain/delta types; register in `crates/protocol-oteryn/src/lib.rs` and `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`; error codes in `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`.
- [ ] **Runtime hookup**: new handler `crates/gameplay/dispatch/src/gameplay_transport/market.rs` (new); wiring-only dispatch branch in `connection.rs` (alongside `COMMAND_TYPE_NPC_TRADE_INTENT`) and a `GATED` row for `MARKET_V1` in `capabilities.rs`; admission/session-generation checks as in `crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs`; depot/NPC range rule from MARKET0.
- [ ] **Jobs**: restart-safe World job for offer expiry registered at node startup (`apps/game-server/src/node/serve.rs`).
- [ ] **Client/UI** (`apps/client`): market panel out of scope unless the MARKET-WIRE-1 owner says otherwise; the server path must at least be drivable by a protocol-level test client.
- [ ] **Tests**: unit (domain, wire codec); PG durability for every MARKET0 §8 delta; double-accept race; expiry-after-restart; **end-to-end through `connection.rs` dispatch**: login, place offer, second character accepts, gold and item moved, Inbox delivery, capability off => command refused.
- [ ] Acceptance boxes in the Acceptance criteria section ticked only with evidence.

### Blockers and dependencies (re-verified on origin/main `59785d76`)
- ITEM-LOOP-1: MERGED (#2190). NPC-ECON-1: MERGED (#2197).
- MARKET-CONTENT-1: BLOCKING (content draft #2234 open, not merged).
- DEPOT-1 / DEPOT-WIRE-1: BLOCKING — ITEM-STORAGE-1 depot reads re-implementation pending (draft #2156 closing; card `docs/agents/tasks/ITEM-STORAGE-1.md`). INBOX-1b: BLOCKING (not merged).
- No MARKET-WIRE-1 PR exists; the completion PR owns the wire unless CP splits it.
- MARKET-TIER-1 (ITEM-FORGE-1) depends on this.

## Current pass — 2026-10-23

Status **IN_PROGRESS**. Existing owners found before writing: L3
`economy/market.rs` and tests; the domain definition bridge; L4 `durability/market.rs`
(private readers, book locks, matching/end plans and operation reads); fifteen
native proposed-schema PG cases; content's partial `marketable` promotion;
Bank/fee, Inbox, depot and imbuement owners. No Market protocol seam exists on
this main. The old proposed schema was not a production migration.

Delivered in this dependency fallback:

- Named L3 bounds and pinned reference parity; existing buy/sell, fee, duration,
  anonymity, maker-price, expiry and held-credit rules remain the single owner.
  Deterministic `test-support::property` tests cover checked arithmetic, fee
  bounds, match gold conservation and remaining items/escrow on expiry.
- Migration **0155** (highest on this main: 0152) installs the existing offer-book,
  operation and structural escrow schema. Character-root locking enforces the
  shared 100-open-offer limit across ware books and sides, including expired OPEN
  backlog. Account/World binding and immutable terms, ending metadata and sequence
  uniqueness, reader indexes, numeric bounds and immutable operations are enforced.
  Runtime/control have SELECT only: no new value-moving write grant exists.
- The existing L4 owner is split into types/rules, reads, book selection and
  operation history, each under 600 lines. Fifteen PG cases are preserved and
  moved once from the authority target to the required auto-mounted library
  directory, replacing their private schema with the native migration. Three
  new PG cases qualify admission races, schema/ending constraints and role grants.
  Intentional corrupt-history fixtures remain isolated to decoder tests; separate
  migration cases assert production guards are enabled. No test is skipped.
- Resource-limit fragments **0670–0676** register MARKET0 bounds. No protocol or
  capability ID is allocated without an existing wire seam. `MECHANICS_STATUS.md`
  does not exist on main and is not created.

### Unfinished

- MARKET-CONTENT-1 admitted market categories/default-instance eligibility,
  including refusal of imbuements using the merged ITEM-IMBUE owner.
- Current depot view/session-generation admission and depot/Inbox transfer
  out-shapes (DEPOT-WIRE-1 / INBOX-1b).
- Atomic place/accept/cancel/expiry, Bank ledger/MarketFee burn, native exclusive
  escrow custody and provenance, inbox reservation/delivery/return, held-credit
  release, audit/event/outbox and conservation. Structural tables and pure plans
  do **not** implement settlement or exactly-once expiry return.
- Runtime jobs and dispatch/wire E2E. No Market protocol seam is present; this pass
  adds no new production `pub fn` or independently reachable mutation path.
- Independent exact-head review and CI belong to the integrator. No commit/push,
  external publication, content regeneration or activation is performed here.

### Validation of this working tree

- `cargo fmt --all -- --check`: PASS through the queued wrapper.
  Auto-mounted files also pass explicit `rustfmt --edition 2024 --check
  apps/game-server/src/lifted_tests/auto/market_postgres_cases.rs` (Cargo's
  generated include is not traversed by rustfmt).
- `cargo clippy --locked --offline -p oteryn-gameplay-economy -p
  oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet --
  -D warnings`: PASS through the queued wrapper.
- `cargo test --locked --offline -p oteryn-gameplay-economy --lib --quiet`:
  **22 passed / 0 failed** through the queued wrapper.
- `cargo test --locked --offline -p oteryn-game-server --lib
  domain_market_tests:: --quiet`: **5 passed / 0 failed** through the queued wrapper.
- `cargo test --locked --offline -p oteryn-game-server --lib
  market_postgres_cases:: --quiet -- --test-threads=1`: **18 passed / 0 failed /
  0 ignored**, actual PostgreSQL 18.6 through the queued wrapper, 125.35 seconds.
  Only Market cases ran. An earlier login-shell rustup invocation had no PG
  route and is explicitly not counted as database evidence; the wrapper was
  subsequently selected with `login=false`.
- `cargo test --locked --offline -p oteryn-gameplay-durability --lib market
  --quiet`: compiles; 0 matching tests (not counted as behavioral coverage).
  Native L4 qualification is the auto-mounted 18-case run above.
- `python3 tools/repository/validate_repository_policy.py`: PASS.
  Governance validator and all 124 governance unit tests: PASS.
  Both protocol/resource fragment trees pass `registry_fragments.validate`.
- Additional optional `python3 -m unittest discover -s tools/contracts/tests
  -p test_registry_fragments.py`: 4 pass, 1 pre-existing failure in
  `test_split_of_the_aggregate_is_the_committed_tree`. Reconstructed unchanged
  HEAD fragments in memory reproduce the first mismatch:
  `0609-PARTYPVP0-RL-04.json` would be renumbered to `0598-PARTYPVP0-RL-04.json`.
  Existing ordinals and the validator are preserved; integrator follow-up.
- Whole working-tree diff reviewed against
  `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and repository checklist; all 15
  original PG case names preserved. Workspace-wide `apps/`, `crates/`, `tools/`
  searches checked moved Market type/constructor/caller sites: no production
  enum variants, fields, trait methods or public functions were added, and the
  existing public paths remain re-exported. `git diff --check`: PASS.
