# STORE_COSMETIC_TOKENS_V1

Owner decision 2026-11-09, official Tibia news 8998: Cosmetic Tokens have
one Tibia Coin's value and belong to the earning character. Eligible Store
categories are outfits, mounts and house decoration/furniture. Prices remain
the admitted offer's coin price; the client never supplies eligibility.

## balance

Capability 479 requires Store 154 and is offered with its configured Store
port. Command 6187 has an empty request and returns the fenced character/World
available token balance as eight little-endian bytes (0..i64::MAX).

## purchase

Command 6188 takes the existing StorePurchaseRequest, with currency exactly
COSMETIC_TOKEN, and returns the existing StorePurchaseResult. It rejects stale
quotes, non-cosmetic offers and insufficient funds. Platform coins are never
consulted or debited by this command. The restore purchase switch applies.

The daily-credit ledger stays immutable. Available tokens are its existing
balance minus a character/World spend projection. Each successful spend has
an immutable purchase receipt and advances a checked predecessor chain.
Debit, character-only cosmetic grant, request answer and Store display history
commit together under current recovery/node/session-generation/gameplay fences.
Accepted request IDs replay the original result; changed bindings fail closed.
Insufficient-funds and eligibility refusals move no value. Coin purchases keep
the Platform-owned debit and delivery path, with native cosmetic delivery.

The bounded first admission is Field Surgeon full outfit (base and both addons),
870 coins or tokens. Evidence: the PROVEN full_store_price and male/female look
fields of wiki:108239:1181114 in
`docs/reference/content-prep/outfits-content-1/admission-evidence/evidence-00000-00135.json`,
and the existing native Field Surgeon definition. Offers without verified prices
or active appearance definitions are excluded; no mount/decor price is inferred.
