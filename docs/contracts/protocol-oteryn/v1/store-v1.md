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

## StorePurchaseResultV1

Command 185 has exactly six results: `ACCEPTED`, `STALE_QUOTE`, `OFFER_UNAVAILABLE`, `PURCHASE_DISABLED`, `INSUFFICIENT_BALANCE`, and `ACCOUNT_RESTRICTED`. Only `ACCEPTED` carries required Platform order and operation UUIDv7 values.

## StoreInboxClaimRequestV1

Command 186 request: one pending Store fulfilment-unit key. It neither represents nor reconstructs a purchase.

## StoreInboxClaimReceiptV1

Command 186 response: immutable Platform operation UUIDv7, fulfilment-unit key, closed result (`COMMITTED`, `REVOKED`, `NOT_CLAIMABLE`, or `REJECTED`), positive recovery generation, and decision time. A repeated operation republishes its stored receipt.
