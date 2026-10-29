# Utility rune reference check, 2026-10-29

The local Global records in `r22-audit/tibiopedia/tibiopedia-spell-facts-2026-10-02.json:1491`
and `:2021` cover Disintegrate and Chameleon requirements. The corresponding
`mechanics-single-facts.json:864` and `:1124` explicitly omit duration and target exceptions.

Current Global working-source cross-checks (search-index excerpts, not an official capture):

- [Chameleon Rune, TibiaWiki BR](https://www.tibiawiki.com.br/Chameleon): 200 seconds.
  [TibiaWiki](https://tibia.fandom.com/wiki/Chameleon_Rune) also excludes creatures,
  NPCs, players and immovable objects.
- [Disintegrate Rune, TibiaWiki BR](https://www.tibiawiki.com.br/index.php?stableid=406921&title=Disintegrate_Rune):
  destroys movable objects and creature bodies.
  [TibiaWiki](https://tibia.fandom.com/wiki/Disintegrate_Rune) specifies adjacent range,
  up to 500 items, exclusion of human bodies, and use in protection zones without PZ block.
- [Global fix reported 2024-04-03](https://www.tibialife.com.br/corrigido-o-bug-da-disintegrate-rune/):
  destroying items in other players' houses was fixed. This supports refusal of a
  channel caster targeting another house; it does not establish an owner's in-house rights.

The existing Tibiopedia records are structured evidence; the additional links above
were checked through unstructured indexed excerpts, not revision-pinned captures.
The permitted Fandom MediaWiki API returned HTTP 403; no retry was made.
These excerpts are supplementary evidence, not `CIPSOFT_OFFICIAL` captures. The local official
manual notes `../tibia-manual/combat.md:88` additionally disable Chameleon in water.

Donor fallback: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data/scripts/runes/chameleon.lua:4,19` (200000 ms; movable item), and
`data/scripts/runes/desintegrate_rune.lua:3,13` (500 visited items; skip immovable,
unique/action tagged items and listed human corpses). These are `OTS_HYPOTHESIS_ONLY`.
Global wins: Disintegrate is a non-aggressive support action allowed in ordinary PZs;
other players' house property is protected despite the donor's lack of a house guard.
