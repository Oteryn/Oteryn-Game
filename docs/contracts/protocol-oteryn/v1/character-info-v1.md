# Character info v1 (CHARINFO)

## Request

Command 113 is reserved for the server-side character information family. Every query is scoped
to the controlled actor, or to a character allowed by the native Friends visibility owner. History
requests contain an unsigned 16-bit offset and a page size from 1 through 50. Unlock and select
operations are writes and must pass the current Character session-generation fence.

Requests have a one-byte operation tag. Character IDs are 16 bytes; page offsets are
little-endian `u16`, limits and title IDs are `u8`. The registered maximum is 20 bytes.

| Tag | Operation | Bytes after tag | Total bytes |
| --- | --- | --- | ---: |
| 0 | QuerySelf | offset, limit | 4 |
| 1 | QueryVisible | character ID, offset, limit | 20 |
| 2 | Unlock | title ID | 2 |
| 3 | Select | title ID (0 clears) | 2 |
| 4 | LookVisible | character ID | 17 |

## Result

Replies are bounded to 32 KiB. They may contain base/progression statistics, native death and PvP
history from the preceding 30 days, item counts grouped by inventory/depot/stash/inboxes, appearance
and achievement summary counters, earned titles, and the currently qualified display title. The
query reads those native owners and does not persist derived aggregate data.

Permanent titles remain earned. Conditional titles are displayable only while their predicate is
true. Rotation titles carry the next-rotation expiry. A read or look requalifies the selected title;
the fenced durable owner clears a selection that is no longer qualified.

The stable Game title catalogue appends Pensive Wanderer as ID 5 (MONKQUEST3,
owner decision 2026-10-30). IDs 1–4 retain their meanings. Honouring the Shrine of
Eternity earns this permanent title in the same fenced quest transaction; the
Character title owner supplies its ordinary query, unlock and selection behavior.
The existing `u8` title fields and capability 97 are unchanged.

### Catalogue extension and automatic unlock (PAR-P12a)

The catalogue appends IDs 6–43 (`CharacterTitle::ALL` in
`crates/protocol-oteryn/src/character_info.rs`); IDs 1–5 keep their meanings and
migration 0572 widens both title checks to 1–43. Names, female names,
descriptions and permanence follow Global (TibiaWiki *Cyclopedia*, revision
1197515). Each admitted title has one rule over a native owner: bank balance,
level, rank 1 of the current World highscore snapshot, daily reward streak,
guild leadership, Bosstiary points or unlocked bosses per category, the kill
count of a named creature, or a ten-year-old Character. This original extension
left 69 Global titles outside the admitted catalogue. Their complete P12a
follow-up inventory is retained in
`docs/agents/evidence/OTV2-20261031-achievements/p12-titles-badges.md`.

The fenced durable owner grants every newly qualified title, clears a selection
that no longer qualifies and revokes the failed conditional titles in one
transaction. It runs at login admission, on the actor cadence after a kill
settlement that changed the level, a tracked creature or a boss, and before the
self query. The wire format, the `u8` title fields and capability 97 are
unchanged.

### Existing native sources (P12a, Crystal 201189f)

Migration 0705 appends IDs 44–84 without changing any earlier ID or command
shape. The live unlock command reads qualification and inserts the earned
title in the same session-generation-fenced transaction. A shared Character
root lock serializes the Charm balance read with Bestiary and Charm writers.
The caller supplies the active immutable cosmetics view before the transaction;
the request cannot supply qualification, content provenance or an expiry.
Read/look requalification derives facts under the same transaction and root lock,
so a page cannot revoke a newly completed conditional title using an older snapshot.

| ID | Title | Native rule | Crystal source |
| --- | --- | --- | --- |
| 44 | Beastly | Available Charm Points ≥ 2000; the existing Charm owner subtracts spent points from completed Bestiary rewards | `player_title.cpp:312–314`; `player.cpp:1779–1785` |
| 45 | Challenger of the Iks | At least one credited Ahau kill, from the existing Bestiary/Bosstiary counter | `player_title.cpp:297–299`; `data-global/monster/bosses/ahau.lua:16–18` |
| 46 | Admirer of the Crown | Both looks 1457 and 1456 with addons 1 and 2 | `player_title.cpp:291–293` |
| 47–51 | Beaststrider, Grades 1–5 | At least 10/20/30/40/50 distinct admitted, owned mounts | `game.cpp:327–331`; `player_title.cpp:195–203` |
| 52 | Big Spender | Both looks 1211 and 1210 with addons 1 and 2 | `player_title.cpp:294–296` |
| 53 | Royal Bounacean Advisor | Both looks 1437 and 1436 with addons 1 and 2 | `player_title.cpp:300–303` |
| 54–58 | Tibia's Topmodel, Grades 1–5 | At least 10/20/30/40/50 earned canonical outfit identities, one count per male/female pair | `game.cpp:333–337`; accepted Q3=B |
| 59–79 | Bestiary class titles | Every admitted race of the named class completed at its final threshold | `game.cpp:361–381`; accepted Q2=B |
| 80 | Executioner | Every canonical Bestiary race completed | `game.cpp:382`; accepted Q2=B |
| 81 | Boss Annihilator | Every Nemesis boss at mastery | `game.cpp:384`; accepted Q2=B |
| 82 | Boss Eraser | Every Archfoe boss at mastery | `game.cpp:387`; accepted Q2=B |
| 83 | Boss Executioner | Every canonical boss at mastery | `game.cpp:388`; accepted Q2=B |
| 84 | Boss Slayer | Every Bane boss at mastery | `game.cpp:391`; accepted Q2=B |

IDs 44–58 are permanent. IDs 59–84 are conditional completion titles. Spending Charm Points or later losing an
available cosmetic definition does not revoke an earned title or its selection.
Automatic refresh also runs after a completed Bestiary entry, because its Charm
reward can satisfy Beastly even when the creature is not a named title target.

Cosmetic predicates use the existing account appearance receipt/fact pair and
explicit active-content grant mappings. Compatible quest, NPC and mount-taming
grants authorize ownership; Store facts do not. Full outfits require an
authorized base and both addons for both exact donor looks. The existing
appearance-owner premium policy (D61) remains authoritative. No inferred grant
key naming, stored appearance selection or new unlock table supplies proof.
An unavailable cosmetics view yields no new cosmetic qualification. Production
cosmetic definitions still need their separately owned content admission; the
synthetic authored test packet does not populate that catalogue.

Coordinator answer [#2782, 6547429543](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543)
accepts actual completion of every class/category member (Q2=B), correcting
Crystal's class/category-enum-as-monster-ID implementation at
`player_title.cpp:254–267` / `player.cpp:9761–9766`. The existing validated
Creature decoder supplies 823 Bestiary races in 21 classes and 291 Bosstiary
bosses. Each Bestiary race must reach its existing final kill threshold; each
boss must reach the existing category mastery threshold. Empty catalogues
never qualify. These conditional titles are requalified and revoked, with
selection cleared, when their complete-set predicate no longer holds.

The same answer Q3=B accepts one earned canonical outfit identity per count
for Topmodel. The pair of female/male looks counts once. A compatible base
grant qualifies, as does a compatible earned addon for an available starter
base. Crystal's `addOutfitAddon` calls `addOutfit`, which inserts the
`outfitsMap` entry even when only an addon was earned
(`player_functions.cpp:3306–3311`; `player.cpp:6840–6850`). Starter content alone
and an addon with no available base do not qualify. Counts and completion
state are derived from the existing owners inside the title transaction and
are not copied to new persistence tables.

The current `NOT_ADMITTED_TITLES` list retains 28 titles with no sourced rule
or native owner. Reigning Drome Champion keeps stable ID 4; no rota result
owner is fabricated. Tournament omissions remain out under Q1=A. The retained
inventory records every original exclusion and the accepted deviations.

Coordinator [Q7=A](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547727362)
retains the server and protocol boundary for this change. Native client UI for
capabilities 97 and 199, and activation beyond the bundle world, remain a
separate client migration. This contract adds no capability for that work.
