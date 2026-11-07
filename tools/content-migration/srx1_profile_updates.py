"""Generate the small, pinned SRX-1 Item update set; preserve the base provider."""
import hashlib
import json

PATH = 'content/items/definitions/srx1-item-updates.json'


def generate(root, generated, profile_path, manifest_path, order, canonical):
    import dispenser_item_profiles
    import srx1_potion_variants
    import writing_item_admission
    base = generated[profile_path]
    overlay = {profile_path: base, manifest_path: generated[manifest_path]}
    for module in (dispenser_item_profiles, srx1_potion_variants, writing_item_admission):
        module.admit(root, overlay, profile_path, manifest_path, order, canonical)
    original = {row['production_definition']['production_key']: row
                for row in json.loads(base)['records']}
    updates = [row for row in json.loads(overlay[profile_path])['records']
               if original.get(row['production_definition']['production_key']) != row]
    if not updates or len(updates) > 128:
        raise ValueError('SRX1_ITEM_UPDATE_BOUNDS')
    data = ('{\n  "schema": "OTERYN_NATIVE_ITEM_UPDATES/v1",\n'
            '  "base_sha256": "' + hashlib.sha256(base).hexdigest() + '",\n'
            '  "records": [\n'
            + ',\n'.join('    ' + json.dumps(row, ensure_ascii=False, sort_keys=True,
                                             separators=(',', ':'))
                        for row in sorted(updates, key=order))
            + '\n  ]\n}\n').encode()
    if len(data) > 200_000:
        raise ValueError('SRX1_ITEM_UPDATE_SIZE')
    manifest = json.loads(generated[manifest_path])
    manifest['item_profile_updates'] = {
        'path': 'items/definitions/srx1-item-updates.json',
        'sha256': hashlib.sha256(data).hexdigest(),
    }
    generated[PATH] = data
    generated[manifest_path] = canonical(manifest)
