"""CRAFT-1: compose the recipe-owned native cooking policies into the existing provider."""
import hashlib
import json

PATH = 'content/craft/runtime-item-profiles.json'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    if overlay['schema'] != 'OTERYN_NATIVE_COOKING_ITEM_PROFILES/v1':
        raise ValueError('COOKING_ITEM_PROFILES_SCHEMA')
    recipes = json.loads((root / 'content/craft/recipes.json').read_bytes())['records']
    wanted = set()
    for row in recipes:
        recipe = row['recipe']
        if (recipe['tool'] != 'none' or recipe['target']['kind'] != 'map_item'
                or len(recipe['operands']) != 1 or recipe['outputs']['kind'] != 'fixed'):
            continue
        wanted.update(ref['key'] for ref in [recipe['input_item'], recipe['target']['definition'],
                      recipe['operands'][0]['output_item_id']])
        wanted.update(output['item_id']['key'] for output in recipe['outputs']['entries'])
    keys = [row['production_definition']['production_key'] for row in overlay['records']]
    if len(keys) != len(wanted) or set(keys) != wanted:
        raise ValueError('COOKING_ITEM_PROFILES_RECORD_SET')
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    for record in overlay['records']:
        key = record['production_definition']['production_key']
        if key in existing:
            if (existing[key]['production_binding'] != record['production_binding']
                    or existing[key]['production_definition'] != record['production_definition']):
                raise ValueError('COOKING_ITEM_PROFILES_IDENTITY_CONFLICT')
        else:
            document['records'].append(record)
    document['records'].sort(key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
