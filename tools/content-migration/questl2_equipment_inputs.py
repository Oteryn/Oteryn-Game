"""Read the same three digest-qualified Item providers as NativeGameplayInput."""
import hashlib
import json


def keys(read):
    manifest = json.loads(read('content/spells.manifest.json'))
    def load(name):
        pin = manifest.get(name)
        if pin is None:
            return None, None
        path = pin['path']
        if not path.startswith('items/definitions/') or '..' in path.split('/'):
            raise ValueError('EQUIPMENT_PROVIDER_PATH')
        raw = read('content/' + path)
        if hashlib.sha256(raw).hexdigest() != pin['sha256']:
            raise ValueError('EQUIPMENT_PROVIDER_PIN')
        return json.loads(raw), pin['sha256']
    base, base_sha = load('item_profiles')
    if base is None or base['schema'] != 'OTERYN_NATIVE_ITEM_PROFILES/v1':
        raise ValueError('EQUIPMENT_BASE')
    def unique(records):
        result = {r['production_definition']['production_key'] for r in records}
        if len(result) != len(records):
            raise ValueError('EQUIPMENT_DUPLICATE_KEY')
        return result
    result = unique(base['records'])
    updates, _ = load('item_profile_updates')
    if updates is not None:
        if updates['schema'] != 'OTERYN_NATIVE_ITEM_UPDATES/v1' or updates['base_sha256'] != base_sha:
            raise ValueError('EQUIPMENT_UPDATE_BASE_PIN')
        result |= unique(updates['records'])
    starter, _ = load('starting_loadout_item_profiles')
    if starter is not None:
        added = unique(starter['records'])
        if starter['schema'] != base['schema'] or result & added:
            raise ValueError('EQUIPMENT_STARTER_COLLISION')
        result |= added
    return result
