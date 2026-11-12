"""Only the exact, complete canonical donor boss closes native callback custody."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('boss_levers', Path(__file__).with_name('import_boss_levers.py'))
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)
ROOT = Path(__file__).resolve().parents[3]


class BossSourceClosure(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        rows = json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())['records']
        cls.sugar, = [r for r in rows if r['profile']['target']['key'] == 'oteryn:creature.sugar_daddy']

    def test_exact_canonical_sugar_source_closes_only_native_boss_death(self):
        # Canary47df sugar_daddy.lua:97-132; Crystal201189f boss_lever.lua:241.
        self.assertEqual(IMPORTER.native_callback_closure('oteryn:creature.sugar_daddy',
            {'oteryn:creature.sugar_daddy': self.sugar}), (['BossLeverOnDeath'], True))

    def test_changed_creature_or_custom_hook_cannot_reuse_the_source_capture(self):
        for field in ('health', 'event_bindings'):
            with self.subTest(field=field):
                creature = copy.deepcopy(self.sugar)
                if field == 'health':
                    creature['profile']['data']['profile']['health'] += 1
                else:
                    creature['behavior']['data']['profile']['event_bindings'] = [{'event': 'custom'}]
                self.assertEqual(IMPORTER.native_callback_closure('oteryn:creature.sugar_daddy',
                    {'oteryn:creature.sugar_daddy': creature}), ([], False))
        self.assertEqual(IMPORTER.native_callback_closure('oteryn:creature.faceless_bane', {}), ([], False))

    def test_correlated_capture_edit_is_not_qualification(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / IMPORTER.boss_reward_item_profiles.PATH
            path.parent.mkdir(parents=True)
            doc = json.loads((ROOT / IMPORTER.boss_reward_item_profiles.PATH).read_bytes())
            doc['callback_closure']['required_callbacks'] = []
            path.write_text(json.dumps(doc))
            with patch.object(IMPORTER, 'REPOSITORY_ROOT', root):
                with self.assertRaisesRegex(ValueError, 'BOSS_NATIVE_CALLBACK_CAPTURE_DRIFT'):
                    IMPORTER.native_callback_closure('oteryn:creature.sugar_daddy', {})


if __name__ == '__main__':
    unittest.main()
