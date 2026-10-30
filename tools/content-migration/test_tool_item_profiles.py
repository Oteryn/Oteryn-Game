"""Admission refuses drift or duplicate production identities before changing output."""
import json
import tempfile
import unittest
from pathlib import Path

import tool_item_profiles as tools

ROOT = Path(__file__).resolve().parents[2]


class ToolItemProfileTests(unittest.TestCase):
    def test_source_profile_replay_is_bounded_and_has_required_physics(self):
        overlay = json.loads((ROOT / tools.PATH).read_bytes())
        profiles = {r['production_definition']['production_key']: r for r in overlay['records']}
        self.assertEqual(len(profiles), 17)
        for item in (3695, 3701):
            self.assertIs(profiles[f'oteryn:item.tibia.i{item}']['attributes']['blocks_movement'], False)
        for item in (3308, 3483, 3492):
            self.assertTrue(profiles[f'oteryn:item.tibia.i{item}']['admission']['materializable'])

    def test_duplicate_native_identity_is_refused_without_a_partial_write(self):
        record = json.loads((ROOT / tools.PATH).read_bytes())['records'][0]
        planned = {'profiles': json.dumps({'records': [record]}).encode(), 'manifest': b'{}'}
        before = dict(planned)
        with self.assertRaisesRegex(ValueError, 'TOOL_ITEM_PROFILE_DUPLICATE'):
            tools.admit(ROOT, planned, 'profiles', 'manifest', lambda row: 0, lambda v: b'')
        self.assertEqual(planned, before)

    def test_missing_source_record_is_refused_without_a_partial_write(self):
        overlay = json.loads((ROOT / tools.PATH).read_bytes())
        overlay['records'].pop()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / tools.PATH
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps(overlay))
            planned = {'profiles': b'{"records":[]}', 'manifest': b'{}'}
            before = dict(planned)
            with self.assertRaisesRegex(ValueError, 'TOOL_ITEM_PROFILE_SET'):
                tools.admit(root, planned, 'profiles', 'manifest', lambda row: 0, lambda v: b'')
            self.assertEqual(planned, before)
