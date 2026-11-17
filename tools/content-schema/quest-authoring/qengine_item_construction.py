"""Admit complete source construction recipes through the existing piece owner."""
import xml.etree.ElementTree as ET


def append_glider(packet, source_text, mounts_xml, parser, effect_bindings, source):
    config, _ = parser.table(source_text, 'config')
    source_rule = config[21897]
    if source_rule['TYPE'] != 'TYPE_GLOOTH_GLIDER':
        raise ValueError('Glider source recipe type changed')
    ingredients = [r['id'] for r in source_rule['REQUIRED'].values()]
    if len(ingredients) != 7 or len(set(ingredients)) != 7 or 21897 not in ingredients:
        raise ValueError('Glider source recipe shape changed')
    mount = next(r for r in ET.fromstring(mounts_xml) if int(r.get('id')) == source_rule['ID'])
    grant_key = f"oteryn:item-grant.mount.{source_rule['ID']}"
    donor = source['reference'].removeprefix('crystal:') + ':20-36,112-139'
    effect_id = find_effect(effect_bindings)
    if effect_id is None:
        raise ValueError('Glider source effect binding is not admitted')
    packet['grants'].append({'key': grant_key, 'kind': 'mount',
        'mount_id': source_rule['ID'], 'look_type': int(mount.get('clientid')),
        'donor': donor})
    # Acquisition has no Premium gate in the source; wearing retains XML Premium.
    packet['rules'].append({'item_id': 21897, 'donor': donor, 'mode': 'pieces',
        'premium': False, 'required_items': sorted(ingredients),
        'choices': [{'grants': [grant_key], 'owned': grant_key, 'required': [],
            'cues': [{'binding': 'canary.appearance:effect/magic_green',
                      'effect_id': effect_id}],
            'feedback': source_rule['SUCCESS_MSG'],
            'achievements': ['oteryn:achievement/fabled_construction',
                             'oteryn:achievement/natural_born_cowboy']}]})
    packet['grants'].sort(key=lambda r: r['key'])
    packet['rules'].sort(key=lambda r: r['item_id'])
    packet['native_admission']['construction_sources'] = [source]


def find_effect(value):
    if isinstance(value, dict):
        if value.get('key') == 'canary.appearance:effect/magic_green' and value.get('kind') == 'effect':
            return value['client_id']
        for child in value.values():
            result = find_effect(child)
            if result is not None:
                return result
    elif isinstance(value, list):
        for child in value:
            result = find_effect(child)
            if result is not None:
                return result
    return None
