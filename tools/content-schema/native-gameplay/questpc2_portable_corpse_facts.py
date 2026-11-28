"""Keep portable corpse Items under their pinned source owner."""
import sys

# Crystal 201189f8 Elvira uses 6013 -> 4164 -> 4165 -> 4143.
# The terminal corpse shares the source-qualified Dwarf custody policy.
IDS = frozenset({3990, 4103, 4104, 4143})


def profiles(root):
    sys.path.insert(0, str(root / 'tools/content-migration'))
    import encounter_loot_item_admissions as source
    return source.facts(root, IDS, allow_containers=True)
