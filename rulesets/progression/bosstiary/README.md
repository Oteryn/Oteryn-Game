# PKG-P4/P02 — boss count progression

`categories.json` is the one runtime table for category kill thresholds and
per-stage point awards. Points are cumulative; counts continue after mastery.
Stages are derived (0 = below prowess, 1 = prowess, 2 = expertise, 3 = mastery).

Evidence (reference repositories are evidence only):

- Crystal (OTS_HYPOTHESIS_ONLY) `201189f871d23597b82caf88fd611454a2ddefb8`,
  `src/io/io_bosstiary.cpp:181-207`: credit the kill, compare stages and award
  that stage's points; `:283-308`: derive the stage from the counter.
- Same donor, `src/io/io_bosstiary.hpp:52-56`: category thresholds and points
  (the table is in the header, rather than a data config).
- Same donor, `data-global/monster/raids/ferumbras.lua:16-19`: monster Bosstiary
  ID 231 and Nemesis category. Category metadata already exists in Game content.
- Global cross-check (TIBIAWIKI_STRUCTURED, indexed table only), retrieved 2026-10-29:
  <https://tibia.fandom.com/api.php?action=parse&page=Bosstiary/Categories&prop=wikitext&format=json>.
  UNVERIFIED: the target-date MediaWiki revision could not be fetched (API unavailable).
  The indexed table agrees with the donor: Bane 25/100/300 and 5/15/30;
  Archfoe 5/20/60 and 10/30/60; Nemesis 1/3/5 and 10/30/60.
  Global wins on conflict. No conflicting numeric rule is used in this slice.

The kill principal, occurrence and credit window come from the same captured
Combat death settlement used for Bestiary
(`crates/gameplay/combat/src/combat/death_reward.rs:822`, `:860`, `:957`).
Bosstiary runs alongside it at `:826` and `:864`; production reward metadata
is passed at `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs:1675`.
Bosstiary is an independent descendant:
a missing/saturated Bestiary entry does not suppress boss credit, and a failed
boss credit does not roll back loot or XP. The native character fence is checked
in the writing transaction, including on replay. Like Animus completion this
counter does not advance CharacterRevision; the existing held revision slot and
locked root serialize it with the other settlement descendants.

Boss category metadata is retained independently of reward admission in
`crates/gameplay/content/src/content/creature_reward.rs`. An inadmissible corpse,
loot table, or missing corpse does not suppress a boss kill: the existing queue
captures a boss-only entry (`crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs:734`)
and the same fenced settlement helper credits it at `:1517`. Such an entry has
no reward row and writes no XP or item receipts. Unavailable writes retry under
the existing queue; occurrence receipts make the retries idempotent.

Migration 0532 adds an occurrence receipt to the existing 0445 counter table.
The existing boss-cooldown command/capability already returns counts and total
boss points (`crates/gameplay/durability/src/durability/boss_encounter/cooldown_list.rs:35`,
`:169`); no wire allocation or protocol change is made. Stage is derived
from category and count, rather than adding a wire field.

Residue: boss slots, reroll, loot bonus and boosted boss are outside this slice.
