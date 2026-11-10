"""A changed catalogue root cannot bless changed bounded callback semantics."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import build_native_death_event_witness as death
import build_srx3_soul_war_death_events as soul


class NativePinRefreshTests(unittest.TestCase):
    def test_w01_refresh_preserves_every_frozen_source_parent(self):
        path = death.ROOT / 'crates/gameplay/content/src/content/native_w01_death_events_witness.json'
        before = json.loads(path.read_bytes())
        after = death.refresh_native_pin(path)
        self.assertEqual(after['parents'], before['parents'])
        self.assertEqual(after['source_definitions_sha256'], soul.witness_build()['source_definitions_sha256'])

    def test_w01_refresh_rejects_changed_source_event(self):
        path = death.ROOT / 'crates/gameplay/content/src/content/native_w01_death_events_witness.json'
        value = json.loads(path.read_bytes())
        value['parents'][0]['registered_events'].append('foreign callback')
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'capture.json'
            target.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, 'source registration'):
                death.refresh_native_pin(target)

    def test_w01_refresh_rejects_changed_native_body_under_matching_root(self):
        path = death.ROOT / 'crates/gameplay/content/src/content/native_w01_death_events_witness.json'
        capture = json.loads(path.read_bytes())
        native = {'source_definitions_sha256': capture['source_definitions_sha256'],
                  'records': [{'profile': copy.deepcopy(row['native_profile'])} for row in capture['parents']]}
        native['records'][0]['profile']['data']['profile']['health'] += 1
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            definitions = root / 'content/creatures/definitions'
            definitions.mkdir(parents=True)
            (definitions / 'spell-native-profiles.json').write_text(json.dumps(native))
            for shard in (death.ROOT / 'content/creatures/definitions').glob('creatures-*.json'):
                (definitions / shard.name).symlink_to(shard)
            witness = root / 'crates/gameplay/content/src/content/native_monster_callback_witness.json'
            witness.parent.mkdir(parents=True)
            witness.write_text(json.dumps({'source_definitions_sha256': native['source_definitions_sha256']}))
            with patch.object(death, 'ROOT', root), self.assertRaisesRegex(ValueError, 'native body'):
                death.refresh_native_pin(path)

    def test_soul_war_refresh_preserves_the_complete_source_capture(self):
        before = json.loads(soul.OUTPUT.read_bytes())
        after = soul.refresh_native_pin()
        self.assertEqual(after, before)

    def test_soul_war_refresh_rejects_changed_ordered_source_capture(self):
        value = json.loads(soul.OUTPUT.read_bytes())
        value['records'][0]['parent_git_blob'] = '0' * 40
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'capture.json'
            target.write_text(json.dumps(value))
            with patch.object(soul, 'OUTPUT', target), self.assertRaisesRegex(ValueError, 'source capture changed'):
                soul.refresh_native_pin()


if __name__ == '__main__':
    unittest.main()
