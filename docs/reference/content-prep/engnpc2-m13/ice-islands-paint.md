# Ice Islands painting: checked source boundary

Source: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua
SHA-256: `599666e12f2960ab22b820a84a2b79c0ea40db7a0e2b39e7e3ef4cb2931272a5`.

The chosen recipe is `oteryn:quest.the_ice_islands_quest`, stage s4, count3, target_items Baby Seal and Vial of Paint. Stage s3 releases ants on a ship mast; it does not require capturing a live Creature.

The pinned paint action checks target Item7178, transforms it to7252, and restores7178 after30000ms. It requires Questline8 and increments PaintSeal; the third successful operation (legacy unset value -1) advances Questline9/Mission04=2 and removes Item7253. The source itself lacks a personal distinct-target guard; the chosen request explicitly requires three different seals.

Thus the Formorgar/Ice Islands consumer needs native map-object target identity and personal distinct progress. It must not fabricate a Creature identity for Item7178. The earlier live-Creature owner-borrow change remains relevant to the Goblin Merchant live targets.

The initial distinct-target planner is bounded to eight signed masks and504 stable target indices. It rejects repeated targets, invalid population shapes and unknown bits. It is a pure planner, with no gameplay authority or persistence by itself; production callback/transaction wiring is subsequent work.
