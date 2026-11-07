"""Source claims may advance an existing owner only through their exact prerequisite."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import quest_claim_progress as tool

ROOT = Path(__file__).resolve().parents[3]


class ClaimGuards(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.row = next(r for r in json.loads((ROOT / tool.PATH).read_text())['bindings']
                        if r.get('source_claim'))
        for path, value in [(tool.PATH, {'schema': 1, 'bindings': [self.row]}),
                            ('tools/content-schema/quest-authoring/native-source-claims.json',
                             json.loads((ROOT / 'tools/content-schema/quest-authoring/native-source-claims.json').read_text()))]:
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(json.dumps(value))
        self.definition = {'identity': {'key': self.row['claim'], 'revision': self.row['claim_revision']},
            'readiness': 'ready', 'provenance': {'pilot_key': self.row['source_claim']},
            'placements': [{'appearance_tibia_id': 2472,
                'source_binding': {'project_position': {'x': 32455, 'y': 31968, 'z': 14},
                    'legacy_unique_ids': [{'server': 'crystalserver', 'unique_id': 9226}]},
                'reward': {'items': [{'item': {'family': 'Item', 'key': self.row['item'],
                                             'revision': 'definition-r1'}, 'count': 1}]}}]}

    def test_existing_owner_retains_initial_value_and_gets_exact_chest_progress(self):
        original = {'key': self.row['track'], 'quest': self.row['quest'],
                    'initial': 0, 'min': 0, 'max': 3}
        quests = [{'quest': self.row['quest'], 'tracks': [copy.deepcopy(original)], 'transitions': []}]
        tool.extend_state(self.root, quests, [{'identity': {'key': self.row['quest']}}])
        self.assertEqual(quests[0]['tracks'], [original])
        transition = quests[0]['transitions'][0]
        self.assertEqual(transition['effects'][0]['from'], {'op': 'EQ', 'value': 2})
        self.assertEqual(transition['effects'][0]['effect'], {'kind': 'SET', 'value': 3})
        tool.apply_claims(self.root, [{'definition': self.definition}], plain=True)
        self.assertEqual(self.definition['placements'][0]['quest_requirements'],
                         [{'track': self.row['track'], 'minimum': 2, 'maximum': 2}])

    def test_missing_native_admission_or_different_reward_cannot_teach_progress(self):
        for mutation in ('unadmitted', 'item', 'position', 'uid'):
            with self.subTest(mutation=mutation):
                definition = copy.deepcopy(self.definition)
                if mutation == 'unadmitted':
                    definition['readiness'] = 'waiting_item_semantics'
                elif mutation == 'item':
                    definition['placements'][0]['reward']['items'][0]['item']['key'] = 'oteryn:item.tibia.i3031'
                elif mutation == 'position':
                    definition['placements'][0]['source_binding']['project_position']['x'] += 1
                else:
                    definition['placements'][0]['source_binding']['legacy_unique_ids'][0]['unique_id'] += 1
                with self.assertRaises(ValueError):
                    tool.apply_claims(self.root, [{'definition': definition}], plain=True)

    def test_a_claim_cannot_steal_another_quests_track(self):
        quests = [{'quest': self.row['quest'], 'tracks': [
            {'key': self.row['track'], 'quest': 'oteryn:quest.someone_else', 'initial': 0, 'min': 0, 'max': 3}],
            'transitions': []}]
        with self.assertRaisesRegex(ValueError, 'owner'):
            tool.extend_state(self.root, quests, [{'identity': {'key': self.row['quest']}}])


if __name__ == '__main__':
    unittest.main()
