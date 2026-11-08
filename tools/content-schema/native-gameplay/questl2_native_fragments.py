"""Read exact Item profiles from the two registered native providers."""
import hashlib
import json


def item_profiles(definitions):
    content = definitions.parents[1]
    manifest = json.loads((content / 'spells.manifest.json').read_bytes())
    records = []
    for name in ('item_profiles', 'starting_loadout_item_profiles'):
        entry = manifest.get(name)
        if entry is None:
            continue
        relative = entry['path']
        if not relative.startswith('items/definitions/') or '..' in relative.split('/'):
            raise ValueError('Native Item fragment outside registered definitions')
        raw = (content / relative).read_bytes()
        if hashlib.sha256(raw).hexdigest() != entry['sha256']:
            raise ValueError('Native Item fragment digest mismatch')
        records.extend(json.loads(raw)['records'])
    return records
