# Analyser item metadata

CLIENTGAP-ANALYTICS adds optional read-only command 734 under capability 10.
Existing fact and snapshot bytes are unchanged. The query has no persistent effect.
The normal ingress capability, command-id and rate gates apply; the owner rechecks
current actor, session, lease, connection and runtime scope under the runtime lock.
The active generation's pinned Item index resolves every reference. Unknown references
reject the request. Names are qualified content presentation, never caller strings.

## Request

One u8 count (1–16), followed by that many distinct nonzero little-endian u32 Item
references. Exactly `1 + count * 4` bytes; no trailing bytes.

## Reply

One u8 count (1–16), then for each requested reference in request order: u32 Item
reference, u16 UTF-8 name byte length (0–120), name bytes without control characters,
u64 coin denomination. Integers are little-endian. Empty name means unavailable.
Coin denomination is 0 for non-currency, or 1/100/10000 gold for canonical currency.
Replies contain exactly the requested distinct references and no trailing bytes.
Maximum reply size is 2145 bytes.

Clients distinguish reported gold from the coin stacks also present in `kill_loot`.
Only confirmed non-currency identities contribute custom-priced item values on top of
gold. Unresolved/unpriced quantities stay explicit. A custom price change revalues
retained counts and minute buckets, without changing any server state.
