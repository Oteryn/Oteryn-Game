#!/usr/bin/env python3
"""Corpse wave S1: corpse and loot Item closure for spawned Newhaven creatures.

The S1 creatures spawn on the Newhaven main island and settle no kill today:
their corpse Items (and some loot Items) are not materializable. This module
reuses the native-goal closures unchanged, only with an explicit creature set:
corpse capacity, duration and decay edges come from the pinned Canary
items.xml (encounter_corpse_items.XML/SHA256, Canary
04b83b512114bfd888000d6e1433ed8ecaec7c5b); loot facts come from the accepted
encounter loot importer. Every corpse root's capacity is within the runtime
corpse cap of 30. The per-Item capacities and their source lines are listed in
docs/reference/parity/corpses.md and checked by test_creature_corpse_items.py.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
CREATURES = frozenset('oteryn:creature.' + name for name in (
    'dark_carnisylvan', 'ghost', 'hulking_carnisylvan',
    'hunter', 'muglex_clan_assassin', 'muglex_clan_footman', 'mummy', 'poisonous_carnisylvan',
    'smuggler', 'stone_golem', 'wild_warrior'))


def _shared(root):
    path = str(root / 'tools/content-schema/native-gameplay')
    if path not in sys.path:
        sys.path.insert(0, path)
    import native_goal_corpse_items
    import native_goal_loot_items
    return native_goal_corpse_items, native_goal_loot_items


def apply_admissions(definitions, root=ROOT):
    corpses, loot = _shared(root)
    definitions = list(definitions)
    corpses.apply_admissions(definitions, root, CREATURES)
    loot.apply_admissions(definitions, root, CREATURES)


def admit_native(root, generated, profile_path, manifest_path, order, canonical):
    corpses, loot = _shared(root)
    corpses.admit_native(root, generated, profile_path, manifest_path, order, canonical, CREATURES)
    loot.admit_native(root, generated, profile_path, manifest_path, order, canonical,
                      item_ids=loot.selected(root, CREATURES))
