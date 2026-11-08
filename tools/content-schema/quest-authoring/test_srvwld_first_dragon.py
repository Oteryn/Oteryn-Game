import copy
import json
import tempfile
import unittest
from pathlib import Path

import srvwld_first_dragon as producer


class TamorilSeasonalProgramme(unittest.TestCase):
    def test_checked_programme_is_reproducible_and_all_branches_are_guarded(self):
        value = producer.document()
        self.assertEqual(value, json.loads(producer.TARGET.read_text()))
        self.assertFalse(value['rules'][0]['guards'][0]['within'])
        for rule in value['rules'][1:]:
            self.assertEqual(rule['guards'][0], {
                'kind': 'calendar_window', 'start_month': 1, 'start_day': 14,
                'end_month': 2, 'end_day': 12, 'within': True})
        reset = {effect['track'].rsplit('/', 1)[-1]
                 for rule in value['rules'] for effect in rule['effects']
                 if effect['kind'] == 'set' and effect['value'] == 0}
        self.assertTrue({'dragon_counter', 'gelidrazah_access', 'secrets_counter'} <= reset)

    def test_source_substitution_is_rejected(self):
        source_path = 'content/dialogues/quest-sources/the_first_dragon.json'
        original = json.loads((producer.ROOT / source_path).read_text())
        bad = copy.deepcopy(original)
        entry = next(e for e in bad['entries'] if e['npc_source_name'] == 'tamoril')
        entry['full_source_lua'] = entry['full_source_lua'].replace('day = 14', 'day = 1')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / source_path
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(bad))
            with self.assertRaisesRegex(ValueError, 'source changed'):
                producer.document(root)


if __name__ == '__main__':
    unittest.main()
