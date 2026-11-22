"""Source-qualified physical custody for the Lizard Sanctuary offering.

Refresh with --source-root pointing at Crystal 201189f's pinned items.xml.
Qualification belongs to the existing batch-three importer and native merger.
"""
import argparse
import json
from pathlib import Path

import qengine_reward_item_profiles as shared
import quest_batch3_item_profiles as batch

PATH = 'tools/content-schema/native-gameplay/qc-item-profiles.json'
SCHEMA = 'OTERYN_QC_ITEM_PROFILES/v1'
INPUTS = ('content/quests/world-callbacks/qc-lizard-sanctuary.json',)
KEY = 'oteryn:item.tibia.i21469'


def capture(source_root):
    packet = batch.capture(source_root, additional_inputs=INPUTS)
    packet['records'] = [r for r in packet['records']
                         if r['production_definition']['production_key'] == KEY]
    packet['classification'] = [r for r in packet['classification'] if r['key'] == KEY]
    packet.update(schema=SCHEMA, producer='tools/content-migration/qc_item_profiles.py',
                  runtime_enabled=True,
                  decision='Lizard Sanctuary egg offering',
                  purpose='Physical custody for the egg selected at the altar.')
    return packet


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    records = packet['records']
    classifications = packet['classification']
    if (packet['schema'] != SCHEMA or len(records) != 1
            or records[0]['production_definition']['production_key'] != KEY
            or len(classifications) != 1 or classifications[0]['key'] != KEY
            or classifications[0]['classification'] != 'QUALIFIED_PLAIN_PORTABLE'):
        raise ValueError('QC_ITEM_QUALIFICATION')
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, records)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True)
    args = parser.parse_args()
    (batch.ROOT / PATH).write_bytes(batch.grizzly.encode(capture(args.source_root)))
