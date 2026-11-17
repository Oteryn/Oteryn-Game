# M13 seal-paint tool route

Source: https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua (CrystalServer retained revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua`, blob `e14f4cc41bac08a5e9affeea050bd1e7bce419c2`).
The retained source paints map item 7178 to 7252 for 30000 ms. PaintSeal starts at -1 and reaches 2 on the third use; only that use removes paint 7253 and advances questline 8 to 9 and mission04 1 to 2.

The Native B3 map independently supplies three bare seals at 32380/31157/7, 32380/31158/7 and 32381/31158/7, ordinal 2. `test_engitem_native_seals.py` qualifies these placements against the retained map. Paint 7253 is admitted from its pinned item identity; no invented item ID is used.

The PG fixture begins at the persisted assignment boundary, exercises actual framed item USE, retains the exact tool on the first two occurrences, consumes it on the third, and drives the existing monotonic movement-decay owner without shortening the production timer. It does not claim the full Ice Islands journey or the optional foreign-quest Animal Activist branch.
