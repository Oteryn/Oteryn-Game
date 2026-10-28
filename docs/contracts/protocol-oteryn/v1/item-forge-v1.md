# Item Forge V1

Capability 0152 exposes command type 0181. The server remains authoritative for custody,
costs, tiers and outcomes. The current production slice is **DustLimit and QueryHistory only**,
using the existing native dust ledger, reservation, receipt and bounded history owner. Fusion,
transfer and item conversions are staged and return `Rejected`; migration 0320's SQL prototype
has no runtime privileges and is never called by dispatch. Canary prices are reference evidence,
not activated Content recipes.

## Request
Exactly 51 bytes: operation (`u8`), occurrence UUID (16), item-presence flags (`u8`), first and
second item UUIDs (16 each), history limit (`u8`, at most 50). All unused bytes are zero.
Operations: 0 QueryHistory, 1 Fusion, 2 ConvergenceFusion, 3 Transfer, 4 ConvergenceTransfer,
5 DustToSlivers, 6 SliversToCores, 7 DustLimit. Operations 1–4 require flags 3 and both items;
all others require flags 0. QueryHistory uses a zero occurrence; mutations use their occurrence
and a zero history limit. DustLimit occurrences are UUIDv7, bound to the authenticated Character.

## Reply
A bounded JSON result or history array (maximum 50 entries). DustLimit returns `operation`,
`succeeded`, and `result` containing the native dust balance and dust limit. A terminal
`REVISION_CHANGED` receipt returns `succeeded: false` and `result: null`. A replay under a new
CommandId returns the retained result and never advances the limit twice. QueryHistory returns
only the admitted native DustLimit history subset, newest first (`ordinal`, `receipt`); a zero
limit returns an empty array. No prototype wallet or item table is gameplay truth.

Current session, lease, scope and node fences are revalidated under the native transaction
locks. Rejections use command status `Rejected` with no payload. Future fusion/transfer require
native item and fee composition, Content admission and receipt-bound server RNG per
IMBUE-FORGE-0; client-chosen predictable SQL hashes cannot authorize gameplay rolls.
