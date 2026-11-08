"""Pinned Unnatural Selection hand-ins cannot skip their world objectives."""
import unittest
import hashlib
import json
import tarfile
from unittest.mock import patch

import qengine_rewards
import questl2_rewards
import questl2_unnatural as source


class UnnaturalNpcAuthoring(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        with patch.object(questl2_rewards, 'NPCS',
                          (*questl2_rewards.NPCS, *source.NPCS)):
            cls.inputs = qengine_rewards.donor_inputs()
        cls.tour_path = 'data-global/scripts/quests/unnatural_selection/movements_mission2_all_around_the_world.lua'
        with tarfile.open(qengine_rewards.ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
            manifest = json.load(corpus.extractfile('corpus-manifest.json'))
            row = next(r for r in manifest['files'] if r['source'] == 'crystalserver'
                       and r['revision'] == qengine_rewards.PIN and r['path'] == cls.tour_path)
            raw = corpus.extractfile(row.get('blob_path', row.get('cache_path'))).read()
            if hashlib.sha256(raw).hexdigest() != row['sha256']:
                raise ValueError('Unnatural tour source pin')
            cls.inputs[cls.tour_path] = raw
        cls.world = source.world_callbacks(cls.inputs, qengine_rewards.PIN)['callbacks']
        cls.rows, cls.offers = [], []

        def add(name, quest, npc, keywords, storage, reply, **kw):
            if kw['source_text'] not in cls.inputs[f'data-global/npc/{npc}.lua'].decode():
                raise ValueError('Unnatural Selection donor reply drift')
            cls.rows.append(dict(key=name, storage=storage, sources=[],
                                 take=[dict(item=f'oteryn:item.tibia.i{i}', count=n)
                                       for i, n in kw.get('take', [])], **{
                                           k: v for k, v in kw.items() if k != 'take'}))

        def state(track, before, after, maximum=None):
            return dict(track='oteryn:quest-progress/quest/'+track, minimum=before,
                        maximum=before if maximum is None else maximum, set=after)

        source.append(add, state, cls.rows, cls.offers, cls.inputs, qengine_rewards.PIN)
        cls.callbacks = {r['key']: r for r in cls.rows}

    def test_all_thirty_tour_locations_require_the_carried_skull(self):
        tour = self.world[:-1]
        self.assertEqual(len(tour), 30)
        self.assertEqual(len({tuple(r['selector']['position'].values()) for r in tour}), 30)
        for row in tour:
            self.assertEqual(row['effects']['present'],
                             [dict(item='oteryn:item.tibia.i10159', count=1)])
            state = row['storage'][0]
            self.assertEqual(state['minimum'], state['maximum'])
            self.assertEqual(state['set'], state['minimum']+1)
        self.assertEqual(sorted({r['storage'][0]['minimum'] for r in tour}), list(range(1, 12)))

    def test_skull_cache_uses_the_donor_uid_and_single_mint(self):
        row = self.world[-1]
        self.assertEqual(row['selector']['unique'], 9259)
        self.assertEqual(row['effects']['item_reward'],
                         dict(item='oteryn:item.tibia.i10159', count=1, contents=[]))
        self.assertEqual((row['storage'][0]['minimum'], row['storage'][0]['set']), (1, 2))

    def test_tour_does_not_silently_relax_the_skull_guard(self):
        inputs = dict(self.inputs)
        inputs[self.tour_path] = inputs[self.tour_path].replace(
            b'player:getItemCount(10159) >= 1', b'player:getItemCount(10159) >= 0')
        self.assertNotEqual(inputs[self.tour_path], self.inputs[self.tour_path])
        with self.assertRaisesRegex(ValueError, 'tour predicate drift'):
            source.world_callbacks(inputs, qengine_rewards.PIN)

    def test_native_dance_and_crystal_are_not_replaced_by_npc_writes(self):
        phases = [(r['storage'][0]['minimum'], r['storage'][0]['set']) for r in self.rows]
        self.assertEqual(phases, [(-1, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6),
                                  (7, 8), (8, 9), (9, 10), (11, 12), (12, 13),
                                  (13, 14), (14, 15)])
        self.assertTrue(all(r['sources'] for r in self.rows))

    def test_skull_return_requires_all_eleven_views_and_consumes_it(self):
        row = self.callbacks['l2-unnatural-return-tour']
        self.assertEqual((row['storage'][1]['minimum'], row['storage'][1]['set']), (12, 13))
        self.assertEqual(row['take'], [dict(item='oteryn:item.tibia.i10159', count=1)])
        self.assertEqual(self.callbacks['l2-unnatural-issue-tour']['give'], (10159, 1))

    def test_sacrifice_is_one_atomic_three_item_exchange(self):
        row = self.callbacks['l2-unnatural-return-sacrifice']
        self.assertEqual(row['take'], [dict(item=f'oteryn:item.tibia.i{i}', count=5)
                                       for i in (10196, 5878, 5876)])

    def test_final_pot_requires_actual_beer_and_grants_the_source_reward(self):
        row = self.callbacks['l2-unnatural-return-beer']
        self.assertEqual(row['take'], [dict(item='oteryn:item.tibia.i3465', count=1,
                                           fluid_subtype=3)])
        self.assertEqual(row['give'], (10198, 1))
        self.assertTrue(row['completes'])


if __name__ == '__main__':
    unittest.main()
