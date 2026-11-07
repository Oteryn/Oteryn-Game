"""Promote only W05d's two newly qualified admitted Santa toy states into real death-loot facts.

The maintained reference Item definitions stay unchanged. The active source-qualified native
Item provider is the existing admission authority, as for the Forge resource below this call.
Immovable machine targets never gain inventory/loot admission from this overlay.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SOURCE = 'tools/content-schema/native-gameplay/seasonal-item-profiles.json'
IDS = {6511, 6568}


def admit(facts, definitions):
    profiles = definitions / 'spell-native-profiles.json'
    if not profiles.is_file():
        return
    source = json.loads((ROOT / SOURCE).read_bytes())
    if (source['schema'] != 'OTERYN_NATIVE_SEASONAL_ITEM_PROFILES/v1'
            or source['source_revision'] != '47dfd51f45280a59a1d3e50ba7edd573d7234446'
            or source['crosscheck_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8'):
        raise ValueError('SRV7 loot source qualifier changed')
    allowed = {'oteryn:item.tibia.i'+str(i) for i in IDS}
    captured = {row['production_definition']['production_key']: row for row in source['records'] if row['production_definition']['production_key'] in allowed}
    if set(captured) != allowed:
        raise ValueError('SRV7 seasonal toy source coverage')
    native = json.loads(profiles.read_bytes())
    if native['schema'] != 'OTERYN_NATIVE_ITEM_PROFILES/v1':
        raise ValueError('SRV7 loot requires the native Item provider')
    for row in native['records']:
        ref = row['production_definition']
        key = ref['production_key']
        fact = facts.get(key)
        if key not in allowed or fact is None:
            continue
        expected = {'family': ref['family'], 'key': key, 'revision': ref['revision_ref']}
        binding = row['production_binding']
        if (ref['family'] != 'Item' or fact['item'] != expected
                or binding['disposition'] != 'EXACT' or binding['target'] != expected):
            raise ValueError('SRV7 loot requires an exact production Item binding: ' + key)
        admission = row['admission']
        physical = row['semantics']['physical']
        original = captured[key]
        if (row['production_binding'] != original['production_binding']
                or physical != original['semantics']['physical']
                or admission != original['admission']):
            raise ValueError('SRV7 seasonal toy physics/source binding changed: ' + key)
        if (admission['materializable'] is not True
                or 'CharacterInventory' not in admission['legal_destinations']
                or physical.get('state') != 'KNOWN'
                or physical['value']['pickupable'] != {'state': 'KNOWN', 'value': True}):
            continue
        fact['materializable'] = True
