# STORE_V1 wire contract

STORE_V1 exposes only bounded client views and intents. Platform remains authoritative for coin balances, Wallet debit, order lines, purchase history, and pending-delivery entitlements. Account and Character identity come from the admitted session and are never accepted in these payloads. Game catalogue content is identified by its exact revision.

All strings are UTF-8. Unknown, duplicate, malformed, or over-bound fields are rejected. UUID fields are RFC 9562 UUIDv7 values. Pages contain at most 32 rows; cursors are opaque and at most 128 bytes.

## StoreCatalogueRequestV1

Command 183 request: required category key and optional opaque page cursor.

## StoreCataloguePageV1

Command 183 response: exact catalog revision, up to 32 offer rows (offer key, display title, positive price, and currency), optional next cursor, and Platform-read total and transferable coin balances. Transferable coins cannot exceed total coins.

## StoreHistoryRequestV1

Command 184 request: optional opaque page cursor. An empty request starts the Platform-owned history.

## StoreHistoryPageV1

Command 184 response: up to 32 Platform purchase rows (order UUIDv7, offer key, catalog revision, positive price, currency, and purchase time) and an optional next cursor.

## StorePurchaseRequestV1

Command 185 request: UUIDv7 purchase request id, offer key, exact catalog revision, positive price, and currency. The same id is retained only for retries of the same confirmation binding.

With selected `HIRELING_MANAGEMENT_V1` (250), optional length-delimited field 6 carries Hireling customization. Its closed nested fields are: 1 = target Hireling UUIDv7; 2 = ASCII name (3–18 bytes); 3 = sex (1 male, 2 female). A new hire requires name and sex, a rename requires target and name, and a sex change requires target only (the sex toggles). Skill/outfit unlocks require no customization. Inapplicable, duplicate, unknown or malformed fields are rejected. The complete request remains bounded to 512 bytes; customization is at most 64 bytes.

Names follow Crystal `201189f` `data/modules/scripts/gamestore/init.lua:1475-1528,1888-1890`: letters/spaces, no leading space, at most one whitespace group, donor blocked words after removing spaces, and title casing. The normalized customization joins the immutable Game purchase binding. Platform continues to receive its existing unmodified purchase contract; it remains the only debit authority.

Hireling categories `houses.hirelings` and `houses.hireling-dresses` use revision `store-hireling-crystal-201189f-v1`, independently of the preserved exercise-item catalogue revision. All sixteen offers are generated from the pinned donor catalogue. Hireling purchases reserve the ten-slot character limit, character-wide entitlement or inactive target before requesting the debit. Uncertain results keep the reservation for a retry of the same request; terminal refusals release it. Accepted native lamp mint, Hireling identity or entitlement/change, immutable Store receipt and receipt outbox commit together. Replays never debit, mint, grant, rename or toggle twice.

Production Store producer activation remains outside E12 under PR #2783 ANSWER 6547428201 Q6; no new endpoint or balance authority is introduced.

For a new Hireling, the pre-debit reservation also checks the source lamp weight against the
existing derived carrying capacity and the20-slot StoreInbox limit. Pending lamp reservations
count toward that limit. The native delivery transaction checks the resulting Inbox count
again; a conflicting later inventory change leaves an accepted Platform operation pending
for idempotent delivery retry. Game never performs a speculative coin refund or a second debit.

The Platform operation, Hireling UUID and lamp item UUID stay stable across delivery retries.
A native Inbox preparation is immutable and bounded by its actual session/command reference.
After a rejected command, a new real command therefore receives a separate native attempt ID,
derived from the stable operation and the complete new CommandRef. It does not reset an earlier
attempt's work budget. A committed Store receipt records its winning `native_transaction_id`;
the native mint, same-transaction audit and Hireling effect prove that one delivery. Exact-command
retries retain their native attempt, and an already committed operation returns its existing
Store receipt before preparing any more native work. Ordinary item Store operations keep their
existing native transaction identity.

## StorePurchaseResultV1

Command 185 has exactly six results: `ACCEPTED`, `STALE_QUOTE`, `OFFER_UNAVAILABLE`, `PURCHASE_DISABLED`, `INSUFFICIENT_BALANCE`, and `ACCOUNT_RESTRICTED`. Only `ACCEPTED` carries required Platform order and operation UUIDv7 values.

## StoreInboxClaimRequestV1

Command 186 request: one pending Store fulfilment-unit key. It neither represents nor reconstructs a purchase.

## StoreInboxClaimReceiptV1

Command 186 response: immutable Platform operation UUIDv7, fulfilment-unit key, closed result (`COMMITTED`, `REVOKED`, `NOT_CLAIMABLE`, or `REJECTED`), positive recovery generation, and decision time. A repeated operation republishes its stored receipt.

Display names follow Crystal `schema.sql:697–713` and `gamestore/init.lua:1475–1528`: they are validated text, not unique identities. Two purchases, including overlapping uncertain requests, may use the same canonical display name and receive distinct durable Hireling/lamp UUIDs.
