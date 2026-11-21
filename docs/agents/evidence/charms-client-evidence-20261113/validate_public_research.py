"""Validate research provenance and prevent leads being promoted to combat proof."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess
import unittest
from urllib.parse import urlparse

HERE = Path(__file__).resolve().parent


def require(ok, label):
    if not ok:
        raise ValueError(label)


def validate(data, repository=None):
    require(data['schema'] == 'OTERYN_CHARMS_PUBLIC_RESEARCH/v1', 'SCHEMA')
    require(data['source_head'] == '3e168ce6afe374485da20ee0ac13d3749558e2f8', 'HEAD')
    require(not data['runtime_patch_included'] and not data['game_actions_by_worker']
            and not data['current_global_combat_parity_verified']
            and data['mechanic_status_promotions'] == [], 'RESEARCH_BOUNDARY')
    sources = {row['id']: row for row in data['sources']}
    require(len(sources) == len(data['sources']) == 17, 'SOURCE_POPULATION')
    for row in sources.values():
        require(urlparse(row['url']).scheme == 'https' and row['source_class']
                and row['read_method'] and row['accessed_utc'] and row['supports']
                and row['does_not_establish'] and row['current_global_runtime_verified'] is False,
                'SOURCE_QUALIFICATION')
        for stamp in row['timestamps']:
            require(stamp['verification'] in ['AUTO_TRANSCRIPT_READ', 'SOURCE_LINK_ONLY']
                    and stamp['start_seconds'] >= 0, 'TIMESTAMP_QUALIFICATION')
    require(len(data['topics']) == 5 and len({row['id'] for row in data['topics']}) == 5,
            'TOPIC_POPULATION')
    receipt_paths = {row['path'] for row in data['source_receipts']}
    for row in data['topics']:
        require(set(row['sources']) <= sources.keys() and row['unresolved']
                and row['comparison'] and row['decision'], 'TOPIC_QUALIFICATION')
        require(all(path in receipt_paths and line > 0 for path, line in row['source_pointers']),
                'SOURCE_POINTER')
    for row in data['source_receipts']:
        require(row['head'] == data['source_head'] and re.fullmatch('[0-9a-f]{64}',row['sha256']),
                'RECEIPT')
        if repository:
            raw = subprocess.check_output(['git','show',row['head']+':'+row['path']],cwd=repository)
            require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256'],
                    'SOURCE_BYTES')
            lines = raw.decode('utf-8').splitlines()
            require(all(line <= len(lines) for topic in data['topics']
                        for path,line in topic['source_pointers'] if path == row['path']), 'SOURCE_LINE')
    return {'sources':17,'topics':5,'runtime_promotions':0,'source_blobs':len(receipt_paths)}


class BoundaryTests(unittest.TestCase):
    def setUp(self):
        self.data = copy.deepcopy(json.loads((HERE/'public-research-source-matrix.json').read_text(encoding='utf-8')))

    def test_packet(self):
        validate(self.data)

    def test_source_link_is_not_frame_proof(self):
        self.data['sources'][10]['timestamps'][0]['verification'] = 'FRAME_VERIFIED'
        with self.assertRaisesRegex(ValueError,'TIMESTAMP_QUALIFICATION'):
            validate(self.data)

    def test_creator_lead_is_not_current_global_proof(self):
        self.data['sources'][3]['current_global_runtime_verified'] = True
        with self.assertRaisesRegex(ValueError,'SOURCE_QUALIFICATION'):
            validate(self.data)

    def test_research_does_not_promote_mechanic_status(self):
        self.data['mechanic_status_promotions'] = ['cleanse.agony']
        with self.assertRaisesRegex(ValueError,'RESEARCH_BOUNDARY'):
            validate(self.data)


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository',type=Path)
    parser.add_argument('--test',action='store_true')
    args=parser.parse_args()
    if args.test:
        unittest.main(argv=['validate_public_research'])
    else:
        print(json.dumps(validate(json.loads((HERE/'public-research-source-matrix.json').read_text(encoding='utf-8')),args.repository),sort_keys=True))
