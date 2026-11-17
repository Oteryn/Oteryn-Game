# Nomads Land: literal text at the cabinet

Verified source: [Crystal `container_misc.lua`, lines 803–822](https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/startup/tables/container_misc.lua#L803-L822),
revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, full-file SHA-256
`da146858a7bfbef47f601e02fc2b621c5e639ec2d096a6831446800277129bca`.
The same bytes are already retained in the donor corpus as
`blobs/fbb988e2f836cc3b5a2ca611e2dbe38ee0b28bef`; no new reference dump is needed.

Drawers `2434` at `(33220, 32858, 9)` contain bag `2853`, which contains jewel
case `7527` and parchment `2833`. The parchment carries the two literal source
paragraphs, beginning “We are not far from revealing” and ending “without being
overrun by rotworms.” Native floor is `-9`. Lua's initial long-string newline is
omitted. The authoring tool extracts the body from the pinned retained bytes.

This is recipe stage `s5` of `oteryn:quest.authored.nomads_land_quest`, not a new
NPC reward. The native source progress owner is `oteryn:quest.tibia_tales`;
the source's Arito task is 2 at this point. The proposed personal cabinet claim
preserves that track and adds an independent once-only cabinet track. The
once-per-Character receipt is an Oteryn rule for the chosen recipe, rather than
a claim that the donor startup container implements personal progress.

The bounded reward text model now preserves legacy root strings and represents
ordered child text explicitly. The MINT adapter projects each body to its authored
child index, preserving the source container topology. The callback is registered after its complete production World reward PG case
passed (PostgreSQL 18.6, 52.493176599s). The World path binds the quest definition
and same-transaction MINT; migration 2071 also protects NPC source plans using
the new child-text shape. This is a cabinet-mechanic proof, not whole-quest qualification.

The physical parchment promotion reuses the pinned Canary XML importer:
[items.xml:5818–5823](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml#L5818-L5823),
SHA-256 `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`,
and the retained client appearance bank. It qualifies portable custody through the
exact existing binding; it does not infer a write-once target or install a writing
or transformation hook. The existing unresolved target remains UNKNOWN.
