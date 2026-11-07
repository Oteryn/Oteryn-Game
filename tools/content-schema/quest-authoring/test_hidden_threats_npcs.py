"""Source custody and eight-distinct-rescuer regression checks."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import hidden_threats_npcs as tool


class HiddenThreatsNpcTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = tool.document()

    def test_canary_rescuers_are_eight_distinct_once_flags(self):
        rules = [r for r in self.document['rules'] if r['key'].endswith('/rescue')]
        self.assertEqual(len(rules), 8)
        flags = [e['track'] for r in rules for e in r['effects']
                 if e['kind'] == 'set' and 'corym_rescued' in e['track']]
        self.assertEqual(len(flags), len(set(flags)))
        self.assertEqual({key.rsplit('/', 1)[1] for key in flags},
                         {f'corym_rescued{n:02}' for n in range(1, 9)})
        gifts = [(r['npc'], e['item'], e['count']) for r in rules
                 for e in r['effects'] if e['kind'] == 'give_item']
        self.assertEqual(gifts, [('oteryn:npc.corym_worker_1', 'oteryn:item.tibia.i3030', 1),
                                ('oteryn:npc.corym_worker_3', 'oteryn:item.tibia.i3032', 1),
                                ('oteryn:npc.corym_worker_5', 'oteryn:item.tibia.i3029', 1)])

    def test_fresh_source_pin_mismatch_refuses_emission(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            capsule = copy.deepcopy(json.loads((tool.ROOT / tool.CAPSULE).read_text()))
            capsule['sources'][0]['sha256'] = '0' * 64
            path = root / tool.CAPSULE
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(capsule))
            archive = root / tool.CORPUS
            archive.parent.mkdir(parents=True)
            archive.symlink_to(tool.ROOT / tool.CORPUS)
            with self.assertRaisesRegex(ValueError, 'SOURCE_CUSTODY'):
                tool.document(root)
            self.assertFalse((root / tool.OUTPUT).exists())

    def test_programme_identity_preserves_the_canonical_aggregate_owner(self):
        self.assertEqual(self.document['quest'], 'oteryn:quest.tibia_tales')
        self.assertEqual(self.document['program_key'], 'oteryn:quest-npc-program.hidden_threats')
        self.assertEqual(len(self.document['source_files']), 10)
        self.assertTrue(all(r['revision'] == tool.REVISION for r in self.document['source_files']))


if __name__ == '__main__':
    unittest.main()
