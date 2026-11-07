# Store wire v1 conformance fixtures

Originally copied verbatim from `/home/mole/wave-a/platstore/docs/contracts/fixtures/store-wire-v1/`
(Platform: `docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md`).

Source `manifest.json` SHA-256:
`bacf54e69b95dee10e2bf12f736501cef961a9a3ce0572de3c7e425d0c58c7f5`.

Fixtures are pretty printed; canonical bytes compact JSON in file member order,
without a trailing newline. Restore fixtures belong to the existing separate restore
exchange and are not decoded by the Store client.

Owner decision D1 (2026-11-02) narrows the local currency schemas to `COINS`
and the balance to exactly one entry. Runtime conformance tests also enforce D1.


Owner decision D2(b) (2026-11-02) changes `catalog_revision` to the lowercase
hex SHA-256 (64 characters) of the canonical catalog document: UTF-8 JSON,
object keys sorted recursively, no insignificant whitespace. The revision is
derived from the complete catalog document; that document has no
`catalog_revision` member. Compare revisions for equality only, never order them.
Numeric and named catalog revisions are invalid.

Offer keys retain Game identities, including `:`; the exact pattern is
`^[a-z0-9][a-z0-9_.:-]{0,127}$`.

The local schemas, examples, dependent request/binding digests and manifest
are derived for D1/D2(b); the source manifest hash above records provenance.
Local manifest SHA-256: `fb46ea97101f2f6ff0356076d0beb9058633b87075861123ad3f68cf50a688f5`.
Catalog hashing sorts keys; Store wire messages retain schema member order.
