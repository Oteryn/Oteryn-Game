# Forge candidate and pair preview v1

Existing capability152 adds command6116 as a read projection; no capability is added. Original command181, its51-byte payload, UUIDv7 occurrence receipts and paid writer remain unchanged. A preview is advisory: the writer rechecks current custody, item revisions, resource balances and current authoritative rules. Reading neither creates a receipt nor reserves, charges, rolls, changes tiers or starts paid cooldown.

## Request

Little-endian nonce u64 (nonzero), then tag u8. Tag0 carries an exclusive after-handle u64 (17 bytes total). Tag1 carries the canonical existing51-byte Forge request with zero occurrence and a pair operation (60 bytes total). Distinct nonzero current issued item handles are required. Boost/protect remain normal-fusion-only. Unknown tags, flags, trailing bytes, invalid lengths and noncanonical operations are refused.

## Result

At most32768 bytes. Echoed nonzero nonce u64 then tag:0 refused;1 candidate page;2 pair quote. All scalars little-endian. Item: handle u64, definition_ref u32, bound verified appearance u32 (0 unavailable), classification u8, tier u8, max tier u8, container bool u8, name byte length u16 plus bounded128-byte UTF8 without controls. Containers have zero classification/tier/max; native eligible items carry actual authoritative values. Names and appearance bindings come from the active admitted content, never an unverified definition-index/sprite guess.

Resources: directly carried spendable gold u64, account bank gold u64, bank-spendable bool, dust u32, current dust limit u32, recursive-carried-plus-stash cores u64 and slivers u64. A nonspendable bank balance is displayed separately. Page: after u64,next u64 (0 absent), resources, count u8, up to32 sorted items with handles strictly greater than after. A next handle equals the last item and exists only on a full page. The session sees only already issued live carried item/container handles; open a carried bag through the existing container-tree route before refreshing to browse deeper entries.

Quote: canonical zero-occurrence51-byte pair request; first and second items; resources; base gold u64,dust u32,cores u8; target tier u8,success percent u8,protection percent u8. Pure ForgeQuote validation enforces pair identities, native classification/tier, operation constraints and current maximum tiers. Gold is the base authoritative cost before outcome-dependent convergence bonuses; all outcome effects and actual charges remain writer-owned.

Every read uses a short coherent fenced transaction and current actor/content generation before and after awaiting. Session/lease/item-scope changes invalidate client observations and nonce correlation. Legacy152 servers retain original paid-operation support with explicit preview-unavailable text. New attempts generate UUIDv7 locally using the platform secure random source; unresolved results retain exact original bytes for retry and prohibit editing into another paid attempt.
