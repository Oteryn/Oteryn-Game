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
