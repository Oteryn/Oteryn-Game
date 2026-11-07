"""Generate the small, pinned SRX-1 Item update set; preserve the base provider."""
import copy
import hashlib
import json

PATH = 'content/items/definitions/srx1-item-updates.json'


def coalesce_dispenser_top_order(document, source_records, house_overlay):
    """Coalesce only exact dispenser owner facts plus verified source top flags.

    Crystal items.cpp:203-208 enriches the generic House admission first. The
    strict dispenser compiler must still compare every other source fact.
    """
    import house_decoration_actions
    expected = copy.deepcopy(source_records)
    house_decoration_actions.apply_top_order(expected, house_overlay)
    enriched = {row['production_definition']['production_key']: row for row in expected}
    original = {row['production_definition']['production_key']: row for row in source_records}
    for index, row in enumerate(document['records']):
        key = row['production_definition']['production_key']
        if key in enriched and row == enriched[key]:
            document['records'][index] = copy.deepcopy(original[key])


def generate(root, generated, profile_path, manifest_path, order, canonical):
    import dispenser_item_profiles
    import house_decoration_actions
    import srx1_potion_variants
    import writing_item_admission
    base = generated[profile_path]
    overlay = {profile_path: base, manifest_path: generated[manifest_path]}
    house_overlay = json.loads((root / house_decoration_actions.PROFILES).read_bytes())
    document = json.loads(overlay[profile_path])
    dispenser_source = json.loads((root / dispenser_item_profiles.PATH).read_bytes())
    coalesce_dispenser_top_order(document, dispenser_source['records'], house_overlay)
    overlay[profile_path] = canonical(document)
    for module in (dispenser_item_profiles, srx1_potion_variants, writing_item_admission):
        module.admit(root, overlay, profile_path, manifest_path, order, canonical)
    # W18 source top-order facts enrich the final specialized rows only. Reapplying
    # generic furniture transforms here would overwrite the writing owner's policy.
    enriched = json.loads(overlay[profile_path])
    house_decoration_actions.apply_top_order(
        enriched['records'], json.loads((root / house_decoration_actions.PROFILES).read_bytes()))
    overlay[profile_path] = canonical(enriched)
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
