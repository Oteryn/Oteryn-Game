"""Promote source-qualified physical inputs for the durable NPC executor.

Refresh with --source-root pointing at Crystal 201189f's pinned items.xml.
Qualification belongs to the existing batch-three importer and native merger.
"""
import argparse
import json
from pathlib import Path

import quest_batch3_item_profiles as batch
import qengine_reward_item_profiles as shared

PATH = 'tools/content-schema/native-gameplay/engnpc-m05-item-profiles.json'
SCHEMA = 'OTERYN_ENGNPC_M05_ITEM_PROFILES/v1'
KEYS = {'oteryn:item.tibia.i27654'}


def capture(source_root):
    packet = batch.capture(source_root)
    packet['records'] = [r for r in packet['records']
                         if r['production_definition']['production_key'] in KEYS]
    packet['classification'] = [r for r in packet['classification'] if r['key'] in KEYS]
    packet.update(schema=SCHEMA, producer='tools/content-migration/engnpc_m05_item_profiles.py',
                  runtime_enabled=True,
                  decision='ENGNPC_M05: production NPC effect qualification',
                  purpose='Physical custody for source-authored NPC hand-ins; no custom use effects.')
    return packet


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    records = packet['records']
    if (packet['schema'] != SCHEMA or len(records) != len(KEYS)
            or {r['production_definition']['production_key'] for r in records} != KEYS
            or len(packet['classification']) != len(KEYS)
            or {r['key'] for r in packet['classification']} != KEYS
            or any(r['classification'] not in ('QUALIFIED_PLAIN_PORTABLE', 'EXISTING_NATIVE')
                   for r in packet['classification'])):
        raise ValueError('ENGNPC_M05_ITEM_QUALIFICATION')
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, records)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True)
    args = parser.parse_args()
    (batch.ROOT / PATH).write_bytes(batch.grizzly.encode(capture(args.source_root)))
