# Historical stopped rows from the c8b3890e patch

Superseded for this continuation by `seasonal-continuation.md`. Fresh pinned engine
tracing resolves Cake's damage semantics; its current implementation blocker is
protected combat consumers and the protected native-profile pin. Santa's permanent
flag/reward groups are established; current missing producer/activation facts, not
an assumed annual reset, govern its STOP. Ruprecht's independently sourced exchange component is also STOPPED under the
latest open-branch protection of its two real native consumers; see
[ruprecht-native.md](ruprecht-native.md). Its complete source facts are retained,
but no production merchant, package or supervisor feature is claimed.

# W05d source investigation, 2026-11-05

The permitted Fandom MediaWiki `action=query` revision/content request returned
HTTP 402. No page revision was obtained; no wiki claim is accepted from that
request. Request details and exact donor input hashes are in
`seasonal-sources.json`. Donor evidence remains `OTS_HYPOTHESIS_ONLY`.

## Facts recovered

| Fact | Immutable donor locator |
| --- | --- |
| Cake Golem's second attack is named `combat`, damage 0–40, range 7, cake projectile and poison-area effect; no combat `type` is specified. | `crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/quests/a_piece_of_cake/cake_golem.lua:76–82` |
| Halloween Hare is requested on October 31 at 16:00; execution uses `Game.startRaid` and only process-local duplicate suppression. | `crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/globalevents/others/raids_schedule.lua:21–22,43–48` |
| The active Crystal raid registry has no Halloween Hare entry. | `crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/raids/raids.xml:1–196` |
| Retained API Wiki evidence says the Hare appears just after October31 server save, conflicting with the donor's16:00 request; approximate Dwarf Bridge location is not an exact admitted tile. | `Kazordoon Raids`, revid1135570, The Halloween Hare section; `../raid-content-1/admission-evidence/evidence-00000-00121.json`, record36 |
| Retained API Wiki evidence gives Pumpkin raids every four hours from October31 server save until November3 server save, but preserves executable wave composition/native qualification as unresolved. | `The Mutated Pumpkin`, revid1139122, notes/location; `Northern Darama Raids`, revid1135815, The Mutated Pumpkin section; same retained evidence file, record50 |
| Santa's reward thresholds are ordered 90, 70, 30, 10 in an `if/elseif` chain; the last three branches are unreachable and draws 91–100 leave an empty table. | `crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/santa_claus.lua:83–97`; independently `canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/npc/santa_claus.lua:85–99` |
| Santa checks and sets a permanent storage value of 1, with no year in the claim operation. | `crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/santa_claus.lua:103–122`; independently `canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/npc/santa_claus.lua:105–124` |

## Cake Golem — stopped pending source/owner decision

1. Exact pinned donor attack 79 still omits its damage type.
2. The retained engine witness defaults omitted type to undefined (`crystal:00ce02a57ca5a12e48f32a3476e37471167e4c3f:src/creatures/monsters/monsters.hpp:244`, `monsters.cpp:131`), not an authored damage element.
3. The only authorized current wiki route returned 402, so no page+revid resolves the element.
4. No native attack or creature admission is changed: poison visuals alone do not establish earth damage.
5. Needed decision/evidence: a permitted revision specifying the damage element, or an explicit owner rule for the omitted type.

## Halloween — stopped pending source/owner decision

1. The pinned schedule requests a Hare raid at 16:00 on October 31.
2. Its active raid registry supplies no matching definition; retained API Wiki `Kazordoon Raids` revid1135570 instead says just after server save, not16:00.
3. Retained API evidence keeps exact Hare/Pumpkin executable spawn composition unqualified; the fresh permitted wiki request returned402.
4. No undefined raid is admitted and no Pumpkin portal, timer or participation entitlement is invented.
5. Needed decision/evidence: resolve donor16:00 versus Wiki server-save timing and supply exact spawn/lifecycle facts or an explicit authored event specification.

## Christmas — stopped pending source/owner decision

1. Both pinned donors contain the same broken reward branch ordering.
2. Their Santa claim is a permanent storage bit, not an annual entitlement.
3. The permitted wiki request returned 402; no revision resolves reward weights and annual reset semantics.
4. No reward probability, annual claim conversion, goblin raid or seasonal calendar is invented.
5. Needed decision/evidence: corrected reward probabilities and annual claim/calendar rules from a permitted source or owner specification.

No production changes, migrations, capability bits or command IDs are introduced
by this stopped source investigation. Red/green and PostgreSQL cases are not
claimed for mechanics whose source/decision boundary is unresolved.
