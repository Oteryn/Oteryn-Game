"""Donor replay compares payloads and preserves equivalent gzip containers."""
import contextlib
import gzip
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import quest_donor_source_authoring as donor
from donor_sources.output import payload_sha256, quest_source_sha256


class DonorOutputTests(unittest.TestCase):
    def replay(self, existing, generated, check, name='packet.json.gz'):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / 'capture' / name
            target.parent.mkdir()
            if existing is not None:
                target.write_bytes(existing)

            def assemble(_repo, scratch):
                (scratch / name).write_bytes(generated)
                return {'raw_source': {}, 'lua_structure': {}}

            argv = ['donor', '--root', str(root)] + (['--check'] if check else [])
            with patch.object(donor, 'assemble', assemble), \
                    patch.object(donor, 'DIRECTORY', 'capture'), \
                    patch.object(donor, 'OUTPUTS', (name,)), \
                    patch('sys.argv', argv), contextlib.redirect_stdout(io.StringIO()):
                donor.main()
            return target.read_bytes()

    def test_check_accepts_different_gzip_encodings_of_same_payload(self):
        payload = b'{"records": []}\n' * 100
        level6 = gzip.compress(payload, compresslevel=6, mtime=0)
        level9 = gzip.compress(payload, compresslevel=9, mtime=0)
        self.assertNotEqual(level6, level9)
        self.assertEqual(self.replay(level6, level9, True), level6)

    def test_check_rejects_changed_payload(self):
        with self.assertRaisesRegex(ValueError, 'stale donor Source capture'):
            self.replay(gzip.compress(b'old', mtime=0), gzip.compress(b'new', mtime=0), True)

    def test_write_preserves_equivalent_gzip_container(self):
        existing = gzip.compress(b'payload', compresslevel=6, mtime=0)
        generated = gzip.compress(b'payload', compresslevel=9, mtime=0)
        self.assertEqual(self.replay(existing, generated, False), existing)

    def test_write_replaces_changed_payload_and_creates_missing_output(self):
        generated = gzip.compress(b'new', mtime=0)
        self.assertEqual(self.replay(gzip.compress(b'old', mtime=0), generated, False), generated)
        self.assertEqual(self.replay(None, generated, False), generated)

    def test_non_gzip_check_remains_byte_exact(self):
        with self.assertRaisesRegex(ValueError, 'stale donor Source capture'):
            self.replay(b'{"a":1}\n', b'{"a": 1}\n', True, 'packet.json')

    def test_output_hashes_exact_uncompressed_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            for name in ('packet.json', 'packet.json.gz'):
                path = Path(temporary) / name
                raw = b'{"a": 1}\n'
                path.write_bytes(gzip.compress(raw, mtime=0) if path.suffix == '.gz' else raw)
                self.assertEqual(payload_sha256(path), hashlib.sha256(raw).hexdigest())

    def test_recorded_output_hashes_match_payloads(self):
        capture = Path(__file__).parent / 'samples/donor-source'
        for name in ('coverage.json', 'components248/summary.json', 'refinements/summary.json'):
            summary = capture / name
            for descriptor in json.loads(summary.read_bytes())['outputs']:
                with self.subTest(summary=name, output=descriptor['path']):
                    self.assertEqual(payload_sha256(summary.parent / descriptor['path']),
                                     descriptor['sha256'])
        npc_sha = payload_sha256(capture / 'npc.json.gz')
        self.assertEqual(donor.read(capture / 'npc-index.json')['capture_artifact']['sha256'], npc_sha)
        self.assertEqual(donor.read(capture / 'npc-dependencies.json')['input_provenance']['npc_capture_sha256'], npc_sha)
        joins = donor.read(capture / 'refinements/joins.json')
        self.assertEqual(joins['input_sha256']['boss_descriptor_packet'],
                         payload_sha256(capture / 'components248/boss.json.gz'))

    def test_quest_source_hash_excludes_feedback_but_detects_source_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            shard = Path(temporary) / 'quests-00000-00000.json'
            value = {'records': [{'definition': {'identity': {'key': 'quest'},
                                                'source_data': {'progress': []}}}]}
            shard.write_text(json.dumps(value))
            source_sha = quest_source_sha256(shard)
            definition = value['records'][0]['definition']
            definition['oteryn_recipe'] = {'donor_source_data': {'refs': ['generated']}}
            shard.write_text(json.dumps(value, indent=2))
            self.assertEqual(quest_source_sha256(shard), source_sha)
            definition['source_data']['progress'].append({'key': 'new-source-track'})
            shard.write_text(json.dumps(value))
            self.assertNotEqual(quest_source_sha256(shard), source_sha)

            index = Path(temporary) / 'index.json'
            value = {'shards': [shard.name], 'record_count': 1}
            index.write_text(json.dumps(value))
            source_sha = quest_source_sha256(index)
            value['authoring_sources'] = [{'sha256': 'generated-donor-output'}]
            index.write_text(json.dumps(value))
            self.assertEqual(quest_source_sha256(index), source_sha)
            value['shards'].append('new-shard.json')
            index.write_text(json.dumps(value))
            self.assertNotEqual(quest_source_sha256(index), source_sha)


if __name__ == '__main__':
    unittest.main()
