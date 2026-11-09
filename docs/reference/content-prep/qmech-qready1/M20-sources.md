# M20 crafting: Cursed Crystal World grinder slice

The selected recipe's s2 uses Green Crystal Splinter with Wooden Grinder.
The source World station is already present at (32012,32908,7), top-level
ordinal 3, in `region-z07-x125-y128.b3`. This slice adds no map placement.

Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446:

- [grinder.lua:1-10](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/actions/tools/grinder.lua#L1-L10), SHA256 `780ec83d892ddc1c7041b80a9233ad9a0d36822c0bf54a61d9789e06388fcfeb`: ingress 16122, target 21573.
- [register_actions.lua:1156-1182](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/lib/register_actions.lua#L1156-L1182), SHA256 `43fc4eb36c5e3452038a3cc29e60f73abe7d4667eec32936f57a89931d8cfb17`: consumes one splinter and creates one dust 21507, retains the station. The source recipe mapping supplies the output ID. These are OTS_HYPOTHESIS_ONLY facts.

Native flags/weights are normalized by `build_qready1_craft_items.py` from
the existing pinned Crystal XML, official appearance bytes and exact production
bindings, with the same verifier used for M19. Dust weighs 60 integer units;
the station is immovable, not pickupable and admitted only on Ground.

The craft counter is a receipt epoch, not an artificial cooldown or quest
completion. Each new epoch consumes another plain carried splinter and mints
one native dust through QENGINE in the same fenced transaction. A reusable
item, a replacement that avoids spending the material, or a reward that returns
the same material cannot qualify for this counter epoch.

Scope: this is the reusable World-tool shape. It does not claim the whole Cursed
Crystal walkthrough or completion of the Cradle and Scatterbrained recipes.
QN4's selected carried-recipe proofs/migration 2460 are reused for subsequent
item-on-item wiring; they are not copied into this lane's migration range.

## Cradle and Scatterbrained stations

- [Cradle, Heal with Anti-Mutagen](https://www.tibiawiki.com.br/wiki/Cradle_of_Monsters_Quest#Heal_with_Anti-Mutagen), captured HTML SHA256 `dc87380a577e77185ffd326aef4323354d9d802276d7c8fe873b579af28dcea6`: one nutrient solution, one bottled yolk and fresh hydra herbs make one dose; repeat five times for five victims. The source orders three separate applications to the cauldron.
- [Scatterbrained, Feed](https://www.tibiawiki.com.br/wiki/The_Scatterbrained_Sorcerer_Quest#Feed), captured HTML SHA256 `73304aa175e96a9349a607f192da17786ca0f473ca05f978d586f18cf2ab934c`: one flour, worm, white mushroom, egg and sling herb; place them in the device, operate its lever, obtain fish flakes.
- Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data/items/items.xml` (verified retained corpus digest): 8997–8998 suspicious cauldrons, 9108 magical blending device, 9109 fish flakes (weight 70). The device description specifies placing ingredients in its bowl.
- Unmodified B3: cauldron 8997 at `(33786,32570,-12)`, ordinal 2; blender 9108 at `(32293,32293,-8)`, ordinal 2. No placement is added.

Client 15.30's admitted official appearance data independently names 40016 fresh
hydra herbs, 40598 bottled yolk, 40601 nutrient solution and 40603 flask with
anti-mutagen. Captured Wiki facts supply those explicit item IDs and weights.
The normalizer requires numeric identity AND exact official name to agree.
It reuses the closed Wiki-binding path (as for ITEMHOLD-Q) at the captured Wiki
revisions. It never invents a Crystal/OTS server-ID row.

Selected-route decisions: one native station USE spends the complete declared
recipe atomically and grants its one output through QENGINE. Cradle uses three
carried inputs; Scatterbrained uses five real ground offerings on the device's
tile. This is the selected Oteryn crafting adapter for s8/s9 and s3/s4. Source
sequential applications and lever animation remain source evidence. Neither
binding claims the entire quest walkthrough is complete.

The chosen prepared-material profile keeps these inputs stable, explicitly
scoped in its packet. Source yolk expiry (the walkthrough says two minutes) and
unspecified nutrient expiry remain source observations; no native timing value
is guessed. This records the local selected-recipe decision, rather than a claim
that source materials never expire.

The existing `carried-recipes.json` QN4 first-mixture recipe remains the owner of
11466 + 21504 -> 21505. The new ingress reuses QN4's existing two final-mixture
reward callbacks and migration 2460 for dust + oil -> ointment and monotonic
quest-line progress. It does not register another first-mixture reward owner.

Qualification boundary: the Cradle crafting PG case uses the production Item 8997,
source-witnessed coordinate/ordinal, and a bounded mechanic scene. It does not
qualify the complete native room: its source ground IDs 40231/40232 currently lack
registry admission (the native-crop test rejected donor item/40232). No ground
identity binding or collision properties are guessed. Scatter uses its unchanged
native B3 crop. Complete node-boot topology remains blocked in this workspace by
missing sudo and Docker layer disk capacity; full-manifest decoding passed.

The final mixture's output, special ointment 21506, is admitted through the same
closed Crystal XML / official appearance / exact OTS binding normalizer as the
other crafting Items. Its missing profile caused the initial real-dispatch final
mixture case to fail before grant admission. The six-identity crafting cohort and
its independent equipment pin are regenerated from that source packet.
