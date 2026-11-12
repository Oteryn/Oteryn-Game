# Goblin Merchant: first perfume sample

Sources checked on 2026-11-09:

- https://www.tibiawiki.com.br/wiki/Task:Rafzan describes the four odours in
  order: fresh Slug slime, Swamp Gas, live Rotworm, Mouldy Cheese.
- https://www.tibiawiki.com.br/wiki/Perfume_Gatherer describes the empty,
  nonstackable 3.50 oz gatherer bought from Rafzan.
- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/rafzan.lua
  retains shop item 17670 as the empty perfume gatherer.
- The pinned imported items XML retains perfume variants 17670, 17672–17675;
  17670 is empty and 17675 is full. Item 17671 is a urine bottle, not a perfume
  variant. The registered third-sample edge is 17673 → 17674.

The first edge 17670 → 17672 is authored from that ordered five-state cohort.
It is a derivation, not a claim that a donor Lua action was found. Qualification
copies exact portable physical facts from the pinned item sources; the item
importer does not infer a use action.

The callback selects the original fresh Slug corpse and burns one empty
physical gatherer, mints one first-sample variant and advances a personal 0→1
sample receipt in the same fenced Root transaction. It does not complete the
four-odour task. Swamp Gas and cheese wiring are separate work. Freshness uses
the existing original-corpse/decay boundary rather than an invented timeout.
