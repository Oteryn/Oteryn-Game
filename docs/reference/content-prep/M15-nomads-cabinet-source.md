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
child index, preserving the source container topology. The callback remains
unregistered until its durable SQL source-plan guard and complete cabinet PG
case pass; the parser/model slice is not a whole-quest qualification.
