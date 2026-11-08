"""Chief branch provenance and exclusive, ordered Secret Service issuance."""
import unittest

import qengine_rewards
import questl2_secret


class SecretChiefBranches(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = qengine_rewards.donor_inputs()
        cls.rows = []

        def add(name, quest, npc, keywords, storage, reply, **kwargs):
            path = f'data-global/npc/{npc}.lua'
            if kwargs.get('source_text', reply) not in cls.inputs[path].decode():
                raise ValueError('reply missing from pinned donor')
            cls.rows.append(dict(key=name, storage=storage, **kwargs))

        def state(track, before, after, maximum=None):
            return dict(track=track, minimum=before, maximum=before if maximum is None else maximum, set=after)

        questl2_secret.append(add, state, cls.rows, [], cls.inputs, qengine_rewards.PIN)
        cls.callbacks = {row['key']: row for row in cls.rows}

    def test_all_three_factions_have_seven_issued_and_reported_missions(self):
        self.assertEqual(len(self.rows), 45)
        for faction in questl2_secret.FACTIONS:
            issued = self.callbacks[f'l2-secret-{faction}-issue']
            self.assertEqual(len(issued['requires']), 2)
            self.assertTrue(all(g['maximum'] == 0 for g in issued['requires']))
            for number in range(2, 8):
                row = self.callbacks[f'l2-secret-{faction}-issue-{number:02}']
                self.assertEqual(row['storage'][0]['minimum'], 2 * number - 1)
                self.assertEqual(len(row['requires']), 1)
            final = self.callbacks[f'l2-secret-{faction}-finish']
            self.assertTrue(final['completes'])
            self.assertEqual(final['take'], [(396, 1)])
            self.assertEqual(final['storage'][0]['set'], 15)

    def test_every_item_operation_is_present_in_the_pinned_chief(self):
        for spec in questl2_secret.FACTIONS.values():
            source = self.inputs[f'data-global/npc/{spec["npc"]}.lua'].decode()
            for item in spec['take'].values():
                self.assertRegex(source, rf'player:removeItem\({item},\s*1\)')
            for item in [*spec['give'].values(), spec['reward']]:
                self.assertRegex(source, rf'player:addItem\({item},\s*1\)')

    def test_avin_issues_objects_and_consumes_actual_returns(self):
        for name, item in [('issue', 402), ('issue-03', 404), ('issue-06', 405)]:
            self.assertEqual(self.callbacks[f'l2-secret-avin-{name}']['give'], (item, 1))
        self.assertEqual(self.callbacks['l2-secret-avin-return']['take'], [(403, 1)])
        self.assertEqual(self.callbacks['l2-secret-avin-report-05']['take'], [(406, 1)])
        self.assertEqual(self.callbacks['l2-secret-avin-finish']['give'], (899, 1))


if __name__ == '__main__':
    unittest.main()
