# Store price list v1

Game authors this closed S-5 file; Platform never edits it. Until Game has its own producer, the `offer_key` and currency rules of this file are derived from Game `protocol-oteryn/src/store/catalogue.rs` at aee3f57a; `price-list.valid.json` is a sample in that shape, not yet a copy of the Game catalogue offers. Import requires exact compact UTF-8 JSON bytes with members in schema order, no backslashes, BOM or trailing newline. `catalog_revision` is lowercase SHA-256 of the entire imported file. It is not a member of the file, avoiding a self-referential hash.

The schema bounds one file to 10,000 unique offers, each with 1–8 ordered fulfilment units. Import also checks uint64 upper bounds, signed-64 positive prices and unique offer keys. A file is at most 16 MiB. Unit quantity is per offer; a later purchase must check multiplication by purchase quantity against the unit ceiling. Every offer is quoted in `COINS`; an offer in any other currency, `TRANSFERABLE_COINS` included, makes the whole file invalid.

The two member types this file shares with the Store wire follow owner decision D1. The rows are copied byte for byte from `OTERYN_V2_STORE_WIRE_CONTRACT.md` §2.2, so `§10` names that contract's section:

| Type | Rule |
|---|---|
| `offer_key` | `^[a-z0-9][a-z0-9_.:-]{0,127}$`, copied from the Game catalog; Game keys keep `:`, for example `store:exercise-sword` (owner D1, 2026-11-02) |
| `currency` | `COINS` only (owner D1, 2026-11-02; §10, Q1 resolved for this wire) |

`price-list.valid.json` and `price-list.invalid-*.json` are exact input bytes, intentionally without a trailing newline. The overflow and duplicate examples need semantic validation in addition to JSON Schema.

Import: `php artisan store:catalog:import <file>`. Import alone never activates. Explicit activation: `php artisan store:catalog:activate <catalog_revision>`. The pointer is serialized by a singleton row lock. Re-importing identical bytes is idempotent; imports and offer projections are immutable. No Store flag or Wallet balance is changed by either command.

The owner decision recorded in STORE-IMPL-1 overrides the Proposed wire's numeric catalog revision. Wire fixture changes belong to the contract owner; this lane leaves those fixtures unchanged.
