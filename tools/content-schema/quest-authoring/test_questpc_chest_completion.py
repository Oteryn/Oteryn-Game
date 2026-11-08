"""Standalone claim lifecycle bindings must remain exact and fail closed."""
import copy
import unittest

import questpc_chest_completion as completion
import quest_state_lowering as lowering


class StandaloneCompletionTests(unittest.TestCase):
    def test_each_owned_claim_completes_only_its_own_once_track(self):
        records = [{'identity': {'key': key}} for key in completion.BINDINGS.values()]
        unrelated = {'quest': 'oteryn:quest.unrelated', 'tracks': [], 'transitions': []}
        quests = [copy.deepcopy(unrelated)]
        completion.append(quests, records)
        lowering.validate(quests)
        self.assertEqual(quests[0], unrelated)
        self.assertEqual(len(quests), 6)
        for marker, key in completion.BINDINGS.items():
            quest = next(q for q in quests if q['quest'] == key)
            transition, = quest['transitions']
            effect, = transition['effects']
            self.assertTrue(transition['completes'])
            self.assertEqual(transition['source']['claim'],
                             'oteryn:reward-claim.' + marker.replace('/', '.'))
            self.assertEqual(effect['from'], {'op': 'EQ', 'value': 0})
            self.assertEqual(effect['effect'], {'kind': 'SET', 'value': 1})
            self.assertEqual(quest['tracks'][0]['max'], 1)

    def test_missing_definition_and_existing_programme_are_refused(self):
        with self.assertRaisesRegex(ValueError, 'Missing owned standalone quest'):
            completion.append([], [])
        records = [{'identity': {'key': key}} for key in completion.BINDINGS.values()]
        first = next(iter(completion.BINDINGS.values()))
        with self.assertRaisesRegex(ValueError, 'already has a progress programme'):
            completion.append([{'quest': first}], records)
