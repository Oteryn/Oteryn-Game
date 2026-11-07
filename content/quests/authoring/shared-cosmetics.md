# Shared quest and item cosmetics

The 2026-11-07 main refresh combines the accepted NX5 catalogue from
`bbb3e06d04875fc1e63f024b853194f7580cc875` with the quest catalogue already
published on questpc4. Preserve all 14 item outfits, eight mounts, 63 quest
outfits and their existing public keys. Different published keys can project
the same source appearance; this merge does not rename persisted entitlements.

The quest authoring tool extends maintained authoring instead of replacing it.
It preserves item grants, palette, fallbacks and per-field promotion evidence.
The two shared legacy base grants retain main's targets only after their
appearance pair, grant kind, addon and compatible provenance match the quest
definition exactly. A disagreement is an authoring error.

The combined catalogue contains 77 outfits, eight mounts and 232 grants.
Runtime input remains the bounded descriptor produced by the canonical
cosmetics tool; reference appearance shards are not embedded in committed
outputs. The character gameplay content revision continues to bind this
catalogue. These are merge decisions, not new donor gameplay claims.
