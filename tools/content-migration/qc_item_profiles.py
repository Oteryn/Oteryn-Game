"""Lane qc: source-qualified physical custody for Twenty Miles and Lizard Sanctuary items.

Refresh with --source-root pointing at Crystal 201189f's pinned items.xml.
Qualification belongs to the existing batch-three importer and native merger.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from import_cyclopedia_global import apply_profile_weights, load_weights

import qengine_reward_item_profiles as shared
import quest_batch3_item_profiles as batch

PATH = 'tools/content-schema/native-gameplay/qc-item-profiles.json'
SCHEMA = 'OTERYN_QC_ITEM_PROFILES/v1'
INPUTS = (
    'content/quests/npc-callbacks/qc-twenty-miles.json',
    'content/quests/world-callbacks/qc-lizard-sanctuary.json',
    'content/quests/world-callbacks/qc-twenty-miles.json',
)
KEYS = {f'oteryn:item.tibia.i{i}' for i in
        (21469, 21745, 21746, 21753, 21754, 21770, 21795, 21861, 21862)}
# Brandon's wedding ring is a ring: the existing equipment importer owns its facts.
EQUIPMENT = {21745}
# Crystal omits the nacre slab weight; the Global client weight (ITEMGLOB) is the
# canonical 1.00 oz. Admit only while the client weight equals the canonical one.
UNWEIGHED = {21753, 21861, 21862}


def unweighed(source_root, root=batch.ROOT):
    source = batch.grizzly.source
    xml_raw = (source_root / 'items.xml').read_bytes()
    client_raw = (root / batch.reward.CLIENT).read_bytes()
    if (hashlib.sha256(xml_raw).hexdigest() != batch.XML_SHA256
            or hashlib.sha256(client_raw).hexdigest() != batch.reward.CLIENT_SHA):
        raise ValueError('QC_ITEM_SOURCE_PIN')
    xml = source.xml_index(xml_raw)
    appearances = source.engine_items.load_appearance_objects(client_raw)
    physical = {row['server_item_id']: row
                for row in source.policies(client_raw, xml_raw, UNWEIGHED)}
    bindings = source.binding_index(
        (root / 'imports/crystalserver/bindings/items.json').read_bytes())
    keys = {f'oteryn:item.tibia.i{i}' for i in UNWEIGHED}
    definitions = {row['definition']['identity']['key']: row['definition']
                   for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
                   for row in json.loads(path.read_bytes())['records']
                   if row['definition']['identity']['key'] in keys}
    weights = load_weights(root)
    records = []
    for item_id in sorted(UNWEIGHED):
        definition = definitions[f'oteryn:item.tibia.i{item_id}']
        canonical = definition['semantics']['physical']['value']['weight']
        if ('weight' in batch.grizzly.source.attrs(xml[item_id])
                or canonical != {'state': 'KNOWN', 'value': weights[item_id]}):
            raise ValueError('QC_SLAB_WEIGHT_DECISION_DRIFT')
        donor = copy.deepcopy(definition)
        donor['semantics']['physical']['value']['weight'] = {'state': 'KNOWN', 'value': 0}
        record = batch.qualify(item_id, xml[item_id], appearances[item_id],
                               physical[item_id], bindings, donor)
        apply_profile_weights([record], root)
        if record['semantics']['physical'] != definition['semantics']['physical']:
            raise ValueError('QC_SLAB_PHYSICAL_DRIFT')
        records.append(record)
    return records


def capture(source_root):
    packet = batch.capture(source_root, additional_inputs=INPUTS)
    packet['records'] = [r for r in packet['records']
                         if r['production_definition']['production_key'] in KEYS]
    packet['classification'] = [r for r in packet['classification'] if r['key'] in KEYS]
    equipment = shared.capture(item_ids=EQUIPMENT)
    packet['records'].extend(equipment['records'])
    packet['records'].extend(unweighed(source_root))
    packet['records'].sort(key=lambda r: int(r['production_definition']['production_key'].rsplit('i', 1)[1]))
    packet['sources'].extend(equipment['sources'])
    for row in packet['classification']:
        item_id = int(row['key'].rsplit('i', 1)[1])
        if item_id in EQUIPMENT:
            row.update(classification='QUALIFIED_NATIVE_EQUIPMENT', reason=None,
                       profile_source=equipment['sources'])
        elif item_id in UNWEIGHED:
            row.update(classification='QUALIFIED_GLOBAL_WEIGHT', reason=None)
    packet.update(schema=SCHEMA, producer='tools/content-migration/qc_item_profiles.py',
                  runtime_enabled=True,
                  decision='Lane qc: Twenty Miles and Lizard Sanctuary hand-in items',
                  purpose='Physical custody for source-authored quest hand-ins; no custom use effects.')
    return packet


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    records = packet['records']
    if (packet['schema'] != SCHEMA or len(records) != len(KEYS)
            or {r['production_definition']['production_key'] for r in records} != KEYS
            or len(packet['classification']) != len(KEYS)
            or {r['key'] for r in packet['classification']} != KEYS
            or any(r['classification'] not in ('QUALIFIED_PLAIN_PORTABLE', 'QUALIFIED_NATIVE_EQUIPMENT',
                                               'QUALIFIED_GLOBAL_WEIGHT')
                   for r in packet['classification'])):
        raise ValueError('QC_ITEM_QUALIFICATION')
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, records)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True)
    args = parser.parse_args()
    (batch.ROOT / PATH).write_bytes(batch.grizzly.encode(capture(args.source_root)))
