"""Current Crystal potions.lua:65-68 variants, through the existing potion owner.

Physical facts are replayed from digest-checked XML/official appearances. Existing
identities and already owner-selected Global potion rules remain unchanged.
"""
import argparse
import hashlib
import json
from pathlib import Path
import dispenser_item_profiles as source

PATH = 'tools/content-schema/native-gameplay/srx1-potion-profiles.json'
SCHEMA = 'OTERYN_NATIVE_SRX1_POTION_PROFILES/v1'
IDS = (53162, 53163, 53164)
RULE_SHA = '6eb180d90f8bfa8d14480b5703cf0b59dccc142caf1cbf5789219b61698d0527'
RULES = ((100, 240, 360, ['DRUID', 'MONK', 'PALADIN', 'SORCERER']),
         (130, 240, 360, None), (200, 425, 575, None))


def records(rows, root=source.ROOT):
    result = source.records(rows, root, IDS)
    known = lambda value: {'state': 'KNOWN', 'value': value}
    for record, (level, low, high, vocations) in zip(result, RULES):
        record['semantics']['consumption'] = known({'kind': 'POTION', 'value': {
            'empty_flask': known({'key': 'oteryn:item.tibia.i284', 'revision': 'definition-r1'}),
            'restores': [{'resource': 'MANA', 'min': low, 'max': high}]}})
        record['semantics']['use_requirements'] = known({
            'enforcement_mode': 'ON_USE', 'min_level': known(level),
            'min_magic_level': {'state': 'NOT_APPLICABLE'},
            'vocations': known(vocations) if vocations else {'state': 'NOT_APPLICABLE'}})
    return result


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    if (packet['schema'] != SCHEMA or packet['source_revision'] != source.REVISION or packet['source_xml_sha256'] != source.XML_SHA
            or packet['potion_rule_sha256'] != RULE_SHA
            or packet['records'] != records(packet['source_rows'], root)):
        raise ValueError('SRX1_CURRENT_POTION_SOURCE_DRIFT')
    world = {row['identity']['key']: row for row in json.loads(
        (root / 'content/world/definitions/reference.json').read_bytes())['records']}
    for record in packet['records']:
        key = record['production_definition']['production_key']
        if world[key]['semantics']['physical'] != record['semantics']['physical']:
            raise ValueError('SRX1_CURRENT_POTION_CANONICAL_PHYSICAL_DRIFT')
    document = json.loads(generated[profile_path])
    keys = {row['production_definition']['production_key'] for row in packet['records']}
    if keys.intersection(row['production_definition']['production_key'] for row in document['records']):
        raise ValueError('SRX1_CURRENT_POTION_OWNER_CONFLICT')
    document['records'] = sorted(document['records'] + packet['records'], key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--xml', type=Path, required=True)
    parser.add_argument('--potions', type=Path, required=True)
    args = parser.parse_args()
    if hashlib.sha256(args.potions.read_bytes()).hexdigest() != RULE_SHA:
        raise SystemExit('SRX1_CURRENT_POTION_RULE_DIGEST')
    packet = source.capture(args.xml, ids=IDS)
    packet['schema'] = SCHEMA
    packet['potion_rule_sha256'] = RULE_SHA
    packet['records'] = records(packet['source_rows'])
    (source.ROOT / PATH).write_text(source.capture_json(packet))
