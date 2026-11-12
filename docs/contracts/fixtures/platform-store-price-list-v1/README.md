# Store price list v1

Game authors this closed S-5 file; Platform never edits it. Import requires exact compact UTF-8 JSON bytes with members in schema order, no backslashes, BOM or trailing newline. `catalog_revision` is lowercase SHA-256 of the entire imported file. It is not a member of the file, avoiding a self-referential hash.

The schema bounds one file to 10,000 unique offers, each with 1–8 ordered fulfilment units. Import also checks uint64 upper bounds, signed-64 positive prices and unique offer keys. A file is at most 16 MiB. Unit quantity is per offer; a later purchase must check multiplication by purchase quantity against the unit ceiling. Transferable prices may be imported for provenance but cannot be spent by the v1 Wallet.

`price-list.valid.json` and `price-list.invalid-*.json` are exact input bytes, intentionally without a trailing newline. The overflow and duplicate examples need semantic validation in addition to JSON Schema.

Import: `php artisan store:catalog:import <file>`. Import alone never activates. Explicit activation: `php artisan store:catalog:activate <catalog_revision>`. The pointer is serialized by a singleton row lock. Re-importing identical bytes is idempotent; imports and offer projections are immutable. No Store flag or Wallet balance is changed by either command.

The owner decision recorded in STORE-IMPL-1 overrides the Proposed wire's numeric catalog revision. Wire fixture changes belong to the contract owner; this lane leaves those fixtures unchanged.
