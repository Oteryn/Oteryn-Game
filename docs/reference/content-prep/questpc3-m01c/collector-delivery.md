# Chosen collector delivery

The owner's 2026-11-09 direction assigns Pilgrimage of Ashes and Stampede
collector delivery to this lane, in addition to the original M01c list.

Pilgrimage uses the existing Guide Jonathan identity for the chosen recipe's
unspecified city guide. His retained Canary voice explicitly names Thais:
`content/npcs/definitions/npcs-00500-00999.json:1`, declaration identity
`oteryn:npc.guide_jonathan`, Canary source revision
`47dfd51f45280a59a1d3e50ba7edd573d7234446`. The six named blessers retain their
own identities and order. Visits are dialogue, without blessing purchases,
as the selected recipe explicitly chooses.

The generated route follows `oteryn:quest.pilgrimage_of_ashes` in
`content/quests/definitions/quests-00100-00199.json`: eight ordered stages and
one final grant of twenty Item 3035 Platinum Coins. This is the selected Oteryn
programme, not a claim that the original game's blessing discounts grant cash.
Native delivery uses the existing Character revision slot, NPC reward owner,
MINT, custody and quest-completion transaction. No second payout store or
collector-side mutable quest state is added.

Stampede follows the chosen personal trial (not daily world-state availability)
in `content/quests/definitions/quests-00200-00299.json:1`,
`oteryn:quest.stampede_mini_world_change`. The existing Crystal raid gives the
native Terrified Elephant identity and the area 32825,32667,7–32888,32675,7:
`content/encounters/raids/catalogue.json:1`, `raid:porthope.stampede`, source
`201189f871d23597b82caf88fd611454a2ddefb8:data-global/raids/port_hope/stampede.xml:6`
(SHA-256 `32ebc61a91bcb60989b2b66829b074909c500dec72b907a5e3a27a07dd547f2e`).
The selected recipe's sources are https://tibia.fandom.com/index.php?curid=76265&oldid=845239
and https://tibia.fandom.com/index.php?curid=76266&oldid=887155.

Chosen Oteryn entry is 32850,32671,7, with five spawn cells
32850,32673; 32851,32672; 32852,32673; 32853,32672; 32854,32673, all at depth 7.
The committed native region has only ground Item 1019 on each of these cells.
The personal trial schedules five elephants after one second and cleans up only
its own surviving creature identities after fifteen minutes. Re-entry before
five kills permits retry without lowering accumulated progress; entry after
five kills refuses. No daily raid definition or world-change state is changed.

Guide Tiko (`oteryn:npc.guide_tiko`, retained voice names Port Hope) is the chosen
collector for this added delivery. The selected reward is the existing
`oteryn:achievement/trail_of_the_ape_god`, not guaranteed tusks. Five native
Terrified Elephant death receipts are required before this one-time achievement
and quest completion commit together in the existing reward owner.
