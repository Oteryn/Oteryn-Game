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

## NPC price request

Command 6360 requires capability 599 and uses the byte-for-byte command 734
request format. Clients may request prices when resolving newly observed loot.
The server resolves each reference against the same pinned Item index as 734.

## NPC price reply

A u8 count followed by that many pairs of little-endian u32 Item reference and
u64 mean NPC buy price in whole gold, in request order. Zero indicates no
positive whole-gold mean for that exact definition and revision. Each admitted
positive gold BuyFromPlayer quote in the Channel service catalogue contributes
one sample. Divide a stack quote by its unit count and round down before
averaging the samples. Round the arithmetic mean down. Held offers and prices NPCs
charge players are excluded. This metric is advisory and independent of
character-specific shop access. Legacy command 734 retains its original reply.
