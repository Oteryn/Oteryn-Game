"""Frozen actual Rotten closure and source identity boundaries."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

P = Path(__file__).with_name('build_srx3_rotten_blood_witness.py')
spec = importlib.util.spec_from_file_location('rotten_witness', P)
w = importlib.util.module_from_spec(spec)
spec.loader.exec_module(w)


class RottenWitnessTests(unittest.TestCase):
    def test_actual_bounded_source_closure_and_bindings(self):
        data = w.build()
        self.assertEqual(len(data['records']), 27)
        self.assertEqual(len(data['source_bindings']), 27)
        self.assertEqual(len(data['callbacks']), 34)
        self.assertEqual(data['mechanics_source_revision'], w.PIN)
        pillars = [b for b in data['source_bindings'] if b['target']['key'].endswith('.pillar_of_dark_energy')]
        self.assertEqual(len(pillars), 1)
        self.assertEqual(pillars[0]['identity_namespace'], 'crystalserver/monster-file')

    def test_explicit_item_use_movement_and_corpse_callbacks_are_source_bound(self):
        callbacks=w.build()['callbacks']
        for path in (
            'data-global/monster/quests/rotten_blood_quest/rottenEssences.lua',
            'data-global/scripts/quests/rotten_blood_quest/movements_fields.lua',
            'data-global/scripts/quests/rotten_blood_quest/bakragore_mechanics_death.lua',
        ):
            self.assertTrue(path in callbacks, f"source callback is absent: {path}")
            self.assertEqual(len(callbacks[path]),64)

    def test_spliced_existing_source_body_refuses(self):
        original = w.ROOT
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            definitions = root / 'content/creatures/definitions'
            definitions.mkdir(parents=True)
            for path in (original / 'content/creatures/definitions').glob('creatures-*.json'):
                (definitions / path.name).symlink_to(path)
            data = json.loads((original / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())
            record = next(r for r in data['records'] if r['profile']['target']['key'] == 'oteryn:creature.murcion')
            record['profile']['data']['profile']['health'] += 1
            (definitions / 'spell-native-profiles.json').write_text(json.dumps(data))
            try:
                w.ROOT = root
                with self.assertRaisesRegex(ValueError, 'body changed'):
                    w.build()
            finally:
                w.ROOT = original

    def test_generated_capture_matches_registered_tool(self):
        expected = (json.dumps(w.build(), sort_keys=True, indent=2, ensure_ascii=False) + '\n').encode()
        self.assertEqual(w.OUTPUT.read_bytes(), expected)


if __name__ == '__main__':
    unittest.main()
