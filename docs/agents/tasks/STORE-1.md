# STORE-1 — in-game Store catalogue, purchase and history (task card)

```yaml
task_id: STORE-1
title: "STORE-1: in-game Store catalogue, purchase and history"
mode: IMPLEMENT
status: waiting   # contract verification and lane allocation pending
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 6
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 STORE-ENT-1 (row 123); runtime follow-up to archived Store catalogue authoring"
briefs:
  - docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md
class: "hard, persistence and economy review"
base_branch: main
branch: null
pr: null
needs_migration: true   # allocated at lane start
needs_registry: true   # resource-limit ranges allocated at lane start
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Expose the premium coin balance, admitted offers catalogue, a durable fenced purchase and paged purchase history in game, without real payment integration. Reuse the existing Store authoring package and the Platform-owned balance and purchase ledger; Game must not become a second commercial authority.

## Dependencies

The archived OTV2-20260928-store-catalog-authoring task covers offline schema tooling only, not this runtime mechanic. The accepted Store catalogue owner decision §2 leaves balance and ledger with Platform; §32 / STORE-ENT-1 delivery ownership and the balance/debit/delivery contract must be resolved before purchase activation. Re-verify PREMIUM-1 and native item/entitlement delivery owners at lane start.

This is a waiting plan, not a runtime allocation or a new accepted contract. Live
Issue/PR state and owning contracts must be checked when the lane starts.

## owned_paths (proposed; confirm at lane start)

- `crates/gameplay/economy/src/store/` — pure rules and separate child test files.
- `crates/gameplay/durability/src/durability/store/` — own storage submodule, codecs, receipts and separate tests.
- `crates/gameplay/dispatch/src/gameplay_transport/store/` — separate later wiring slice and dispatch tests.
- `apps/game-server/src/lifted_tests/auto/store_postgres_cases.rs` — auto-mounted PG cases; no shared test mount edits.
- `docs/agents/tasks/STORE-1.md` — this task record.

Migration numbers and resource-limit ranges: **allocated at lane start**; none
reserved here. Exact migration, content input and protocol paths require an explicit
lane allocation; shared `mod.rs`/`lib.rs` changes are registration only. No workflow,
repository-policy or Soul War paths belong to this plan.

## Scope

Read authoritative premium coin balance and paged history through the accepted Platform boundary; serve admitted versioned offers; validate current offer identity, price, currency and eligibility server-side. Persist a Game purchase/delivery attempt and reconciliation receipt in its own store submodule. An accepted contract must bind the authoritative debit to one idempotent delivery; stale session-generation cannot begin a new Character write. Preserve retained purchase identity after disconnect or unknown commit.

## Excluded scope

Real payment providers, money exchange rates, live account operations, coin issuance or administrative credit endpoints, refunds/chargebacks, entitlement lifecycle reownership, a parallel Game coin ledger, client redesign, Soul War files and reference asset redistribution.

## Target layout and slices (ADR-0022)

1. Pure rules in the existing L3 `oteryn-gameplay-economy` rules module
   `economy/src/store/`. Keep policy, plans and arithmetic free of SQL and transport.
2. Durable state in `crates/gameplay/durability/src/durability/store/`, as its
   own submodule. Split transactions, decoding and tests into small child files.
   Reuse recovery admission, session-generation fencing, revision sequencing and
   native value receipts. Schema/grants and their writer are inseparable.
3. Dispatch wiring is a separate later slice in `gameplay_transport/store/`.
   Every added public function needs a production caller and a named end-to-end
   test through real `protocol-oteryn` dispatch before mechanic completion.

No giant mechanic files, speculative framework or new dependency fork. Keep
WorldId and ChannelId distinct and retain Character session-generation fences.

## Reference parity

Canary (`opentibiabr/canary`) source paths to consult as evidence:

- `data/modules/scripts/gamestore/gamestore.lua`
- `data/modules/scripts/gamestore/catalog_loader.lua`
- `data/libs/gamestore/constants.lua`
- `data/libs/gamestore/purchases.lua`
- `data/libs/gamestore/player.lua`
- `data/libs/gamestore/senders.lua`

TibiaWiki page names to consult: **Store, Tibia Coins, Transferable Tibia Coins**.

TODO: currency kinds and transferability, catalogue prices and offer eligibility, delivery restrictions, history pagination/retention and purchase limits. Pin upstream revision and compare the archived catalogue evidence; reference values never decide Game/Platform authority.

Evidence is not runtime authority. Record pinned source revisions and any deliberate
difference in the implementation packet before content admission.

## Acceptance criteria

- [ ] Concurrent purchases cannot overspend the authoritative balance. A stale or modified quote, insufficient balance, unavailable contract or stale fence fails closed. Retry, restart and debit/delivery acknowledgement loss reconcile one retained purchase, never a second debit or mint. History is bounded, account-isolated and consistent with the authoritative ledger.
- [ ] Durable state and retained receipts survive restart, rollback and reconnect.
- [ ] Bounds and lock order are enforced in the writer; concurrent losers are typed.
- [ ] Properties cover checked price arithmetic, no negative authoritative balance, immutable purchase binding, one debit/delivery per purchase and account isolation.
- [ ] Rules, storage and dispatch stay separate; tests live in child files.
- [ ] Production callers and real-dispatch end-to-end tests cover all new public APIs.

## Validation

Planned commands after implementation; these filters must name the actual new tests
and produce nonzero test counts. Run only this mechanic's PostgreSQL cases. The
wrapper supplies `OTERYN_TEST_POSTGRES_ADMIN_URL`; absent routing returns `Ok(())`
without claiming PG qualification. GitHub CI runs the complete registered suites.

```sh
cargo fmt --all --check
cargo clippy --locked -p oteryn-gameplay-economy -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-economy --lib store::
cargo test --locked -p oteryn-gameplay-economy --lib store::property_
cargo test --locked -p oteryn-gameplay-durability --lib store::
cargo test --locked -p oteryn-gameplay-dispatch --lib store::
cargo test --locked -p oteryn-game-server --lib lifted_tests_auto::store_postgres_cases::
python3 tools/agents/validate_governance.py
git diff --check
```

PG cases must exercise the installed migrator/runtime role, commit/reload,
rollback, replay/conflict, stale fences, scope isolation and concurrent writers.
Add mechanic-specific property tests and an end-to-end dispatch case; include
restart/unknown-commit reconciliation rather than storage-only success claims.

## Risks

A distributed debit cannot be made atomic by a local SQL transaction alone. Contract acceptance, external idempotency/reconciliation, Platform history authority and stale catalogue prices are explicit activation gates.
