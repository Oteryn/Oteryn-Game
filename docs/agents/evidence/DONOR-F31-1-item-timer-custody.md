# DONOR-F31-1 — item timer custody evidence

## Fixture record

- Issue / lane: #710, `DONOR-F31-1`, owner-approved test-only lane.
- Donor locator (regression evidence only): `OpenCoreMMO@25636e7e61e5991008f048d26389934ccc9448f8`, “persist item decay timer through player save/load cycle (#1004)”, as supplied by the task spec. No donor code copied.
- Failure mode: a custody or persistence boundary resets, pauses, double-debits or reinterprets an item's timer, or issues duplicate expiry.
- Invariant: preserve the declared absolute-deadline or active-budget semantics across supported moves and durable boundaries.
- Owning contracts: `docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md` §4 and `docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md` §§4–8, 10. These constrain the broad GAME-ITEM temporal invariant to admitted locations and explicit mode transitions.
- Implementation: `crates/gameplay/domain/src/domain/timed_item.rs`, `crates/gameplay/content/src/content/timed_item_definition.rs`, `crates/gameplay/durability/src/durability/item_timed_state.rs`.

## Temporal modes found

| Kind / location | Mode | Representation and boundary |
| --- | --- | --- |
| Equipped duration items (rings / soft boots) and other hosted active duration lanes | Authoritative active-time budget | `TimedValues.remaining_ms`; stop checkpoints the reduced value; inactive time does not drain it. |
| Lit continuous items directly on ground / house tiles | Durable absolute deadline | `deadline_at` on database time; `remaining_ms` records the budget when set. Ground-to-ground moves preserve it by contract. |
| Lit continuous item carried in an equipment slot | Active lane under TIMED-ITEM-0B §10.2 | Not universally a deadline item. Pickup explicitly clears the deadline and derives remaining time; container insertion puts it out (§10.4). |
| Inactive paired forms in inventory | Stored budget | Preserved until reactivated; no active lane drain. |
| Charged items | Orthogonal counter | `charges`, not a third temporal mode. |

A deadline cannot legally be carried unchanged through the requested inventory/depot/mail chain.
The content adapter excludes deadline-only definitions from the active-budget host. The existing
PostgreSQL guard case also refuses a deadline on a held item.

## Fixtures and statuses

Locators:

- UNIT: `crates/gameplay/domain/src/domain/timed_item.rs::tests::active_budget_survives_stop_and_reload_without_reset_or_double_charge`.
- PG: `apps/game-server/tests/support/item_timed_state_postgres_cases.rs::f31_active_budget_checkpoint_reload_and_expiry_are_not_double_charged`, mounted by existing `durability_postgres` (also `item_timed_state_postgres`).
- GUARD: same cases file, `the_guard_refuses_writes_without_their_record_or_row` (including deadline on a held item).
- MODE: `apps/game-server/src/lifted_tests/domain_timed_item_host_tests.rs::the_resolver_reads_the_active_time_budget_charges_and_decay_target`.

| Step | Status | Fixture type / evidence |
| --- | --- | --- |
| Previous cloud synthetic custody/persistence fixture | SUPERSEDED | Its private structs and identity `persist_reload` did not exercise Oteryn. Rewritten to exercise the production lane. |
| Active-budget debit, stop, unchanged retry, inactive interval, reload, exact boundary expiry, terminal no-second-write | PASS | UNIT; real production lane, simulated write acknowledgements; no custody or database claim. |
| Durable stop checkpoint and replay; repeated row reads; rehydrate lane from committed state | PASS | PG; actual fenced durability writer and PostgreSQL reads, synthetic seeded ring. No fake persistence helper. |
| Active interval checkpoint at 1 ms; expiry at zero; durable expiry replay; one audit event and spent-row refusal | PASS | PG; exact values, revisions and record/event counts. |
| Deadline does not become an active budget / held deadline refusal | PASS | MODE + GUARD, existing tests. |
| Inventory → depot | BLOCKED_OWNER | Integration: migration `0157_depot_transfers.sql:47` refuses items with a timed row as “item loop timed item not hosted”. This is an explicit capability boundary, not a demonstrated timer reset. |
| Persist/reload after actual depot move | BLOCKED_OWNER | Integration depends on the refused move. Supported inventory persistence is qualified separately by PG. |
| Depot → mail send, persist/reload in transit, receive, persist/reload in inbox | BLOCKED_OWNER | Integration depends on unsupported timed depot custody. No successful mail custody is claimed. |
| Mail → inventory, final reload and expiry after the complete custody chain | BLOCKED_OWNER | Integration depends on the previous unsupported steps. |
| Deadline-mode version of the complete inventory/depot/mail chain | BLOCKED_OWNER | Contract only admits a lit deadline on tiles, with explicit pickup/put-out semantics. Enabling or redefining this route exceeds the test-only task. |

## Outcome

The production-backed fixtures cover the supported active-budget persistence/accounting boundary.
The full requested custody fixture remains BLOCKED_OWNER; passing these narrower tests must not
be reported as completion of that chain. No production timer defect has been demonstrated, and no
production behavior, migration, capability or wire format was changed. Validation results are in
`F31C-FINAL.md`; PASS rows correspond to executed checks, not the blocked custody chain.
