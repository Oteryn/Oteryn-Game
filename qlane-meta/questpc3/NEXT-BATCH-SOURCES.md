# R–S continuation source packet

Pinned current Crystal revision: 201189f871d23597b82caf88fd611454a2ddefb8.
The reference-only NPC files below were fetched by HTTPS GET, with SHA256 witnesses
in reference/npc-source-witnesses.json. The wiki base and spoiler revisions are in
wiki/revisions.json. No donor Lua is executed by Game.

The R batch's remaining alphabetical start is Rascoohan Outfits -> Rathleton ->
Ravenous Madness. Rascoohan's base depends on A Pirate's Tail (A-lane content),
with Soap/Scrubbing Brush addons in the existing item-cosmetic owner. Keep R–S
reward wiring separate from the A-lane quest content. Existing item cosmetics are
source actions; canonical outfit catalogue admission must also be proved.

For a later S barter batch, the complete source branches are now locally present:

| Quest | Source | Exact debit -> reward | Dialogue |
|---|---|---|---|
| Short Sword | reference/crystal/data-global/npc/amber.lua:399-424 | Item2821 x1 -> Item3294 x1 | hi, book/notebook, yes; repeatable; donor checks Item2821 only, while wiki says the quest notebook: retain this source difference when selecting native admission |
| Small Health Potion | reference/crystal/data-global/npc/billy.lua:134-142 | Item3466 x1 -> Item7876 x1 | hi, pan, yes; repeatable |
| Studded Shield | reference/crystal/data-global/npc/willie.lua:258-282 | Item3587 x1 -> Item3426 x1 | hi, banana, yes; repeatable |
| Studded Legs | reference/crystal/data-global/npc/lee_delle.lua:139-150 | Item2984 x1 -> Item3362 x1 | honey flower; immediate exchange, not a new yes confirmation |

The chest prerequisite paths of Short Sword and Studded Shield are already covered
in the current 20-case batch. Do not mark those quests DONE before their real NPC
exchanges commit atomically, replay exactly, and reject stale session generations.
No new command/capability number is needed: NPC_TALK already exists. Reuse the
existing item transaction owner for debit+grant and its audit/receipt semantics;
never split consume and mint into independent commits or manufacture a free shop
purchase. Keep reply/context identity recoverable after a committed, unacknowledged
exchange, including when the source item was consumed.

The current Spectulus and Sinclair programs were also captured for Research and
Development / Sea of Light / Spirithunters; Rottin Wood and Hector the Mentor are
captured for their corresponding R chains. All files are evidence only.
