# Tainted Souls captured-stack wiring

The retained [Crystal action](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_tainted_soul/actions_star_herb.lua)
transforms antlers 10297 into talisman 22008 and removes one star herb 3736.
The source-qualified recipe captures the entire target stack; it produces one
non-stackable talisman rather than one talisman per antler.

Independent item evidence:

* [Star Herb, revision 391844](https://www.tibiawiki.com.br/index.php?curid=8480&oldid=391844): stackable, 0.30 oz.
* [Antlers, revision 379046](https://www.tibiawiki.com.br/index.php?curid=6884&oldid=379046): stackable, 0.90 oz.
* [Antler Talisman, revision 379044](https://www.tibiawiki.com.br/index.php?curid=36261&oldid=379044): non-stackable, 2.40 oz.

`craft-authoring/tainted_souls.py` registers this recipe through the canonical
craft catalogue. `qmech_npcfix2_item_profiles.py` admits the output's retained
XML/client custody policy. Both source packets retain their input hashes.

The focused PostgreSQL case locks an actual carried stack of 37 antlers and
passes its quantity to qm22's `carried_cooking_conversion_with_quantity` API.
It checks a target debit of 37 and one talisman. Before registration it failed
with `source-qualified carried recipe` after 55.382473 seconds.
After regeneration and rebuilding the embedded craft catalogue, it passed in
56.797752 seconds; the final-source rerun passed in 55.858795 seconds.

The durable command's consumption of this quantity witness is owned by qm22.
This case checks recipe admission and the quantity-aware resolver; it does not
claim that the durable transformation has committed. The legacy quantity-blind
resolver refuses this recipe, avoiding a partial-stack conversion.
