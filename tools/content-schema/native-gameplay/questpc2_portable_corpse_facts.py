"""Keep main's three portable corpse Items under their pinned source owner."""
import sys

IDS = frozenset({3990, 4103, 4104})


def profiles(root):
    sys.path.insert(0, str(root / 'tools/content-migration'))
    import encounter_loot_item_admissions as source
    return source.facts(root, IDS, allow_containers=True)
