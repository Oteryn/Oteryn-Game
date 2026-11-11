"""Bounded Soul War Item admission; frozen potion SOURCE receipts stay intact."""
import native_goal_loot_items as shared

BOSSES = frozenset('oteryn:creature.goshnar_s_' + name for name in (
    'malice', 'spite', 'greed', 'hatred', 'cruelty',
    'megalomania_blue', 'megalomania_green'))
PROTECTED = frozenset({49271})
# Rebind only against the committed pre-generation tree and unchanged selected rows.
BASE = 'HEAD'


def selected(root):
    return shared.selected(root, BOSSES) - PROTECTED


def apply_admissions(definitions, root):
    keys = {f'oteryn:item.tibia.i{i}' for i in selected(root)}
    records = [d for d in definitions if d['identity']['key'] in keys and not d['materializable']]
    if records:
        ids = {int(d['identity']['key'].rsplit('.i', 1)[1]) for d in records}
        shared.importer(root).apply_admissions(records, root, ids, source_core=True)


def admit_native(root, generated, profile_path, manifest_path, order, canonical):
    shared.admit_native(root, generated, profile_path, manifest_path, order, canonical,
                        item_ids=selected(root))


def refresh_input_pins(root):
    import grizzly_shop_profiles as shop
    import condition_potion_profiles as potion
    shop.refresh_canonical_input_pins(root, BASE)
    shop.refresh_canonical_input_pins(root, BASE, packet_path=potion.PATH,
                                    keys={f'oteryn:item.tibia.i{i}' for i in potion.IDS})
