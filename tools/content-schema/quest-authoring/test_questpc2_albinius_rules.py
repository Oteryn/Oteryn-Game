import json
from pathlib import Path
import unittest

import questpc2_albinius_rules as source

ROOT = Path(__file__).resolve().parents[3]


class AlbiniusRules(unittest.TestCase):
    def test_source_pin_reproduces_exact_runtime_program(self):
        for path, text in source.outputs(ROOT).items():
            self.assertEqual((ROOT / path).read_text(), text)

    def test_only_five_exact_tomes_can_earn_one_progress_flag(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        self.assertEqual({t['key'] for t in doc['tracks']},
                         {source.TRACK} | {source.TRACK.rsplit('/', 1)[0] + '/' + p[1]
                                           for p in source.PORTALS})
        contribution = next(r for r in doc['rules'] if r['key'].endswith('/contribute'))
        self.assertEqual(contribution['topics'], [1])
        self.assertEqual(contribution['guards'][1],
                         dict(kind='item', item=source.ITEM, count=5, op='ge'))
        self.assertEqual(contribution['effects'], [
            dict(kind='take_item', item=source.ITEM, count=5, optional=False),
            dict(kind='set', track=source.TRACK, value=1)])
        self.assertTrue(all(r['continue_matching'] for r in doc['rules']))
        self.assertTrue(all(set(source.UNOWNED) <= set(r['exclude_keywords']) for r in doc['rules']))
        self.assertFalse(any(e['kind'] == 'give_item' for r in doc['rules'] for e in r['effects']))
        missing = next(r for r in doc['rules'] if r['key'].endswith('/missing'))
        self.assertEqual(missing['effects'], [])
        self.assertEqual(missing['guards'][1]['op'], 'lt')

    def test_each_portal_consumes_exactly_fifty_before_its_matching_access_write(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        for name, suffix, raw_item, number, _ in source.PORTALS:
            accept = next(r for r in doc['rules'] if r['key'].endswith('/' + name + '-accept'))
            track = source.TRACK.rsplit('/', 1)[0] + '/' + suffix
            item = f'oteryn:item.tibia.i{raw_item}'
            self.assertEqual(accept['topics'], [number])
            self.assertEqual(accept['effects'], [
                dict(kind='take_item', item=item, count=50, optional=False),
                dict(kind='set', track=track, value=1)])
            self.assertEqual(accept['guards'], [
                dict(kind='storage', track=track, op='lt', value=1),
                dict(kind='item', item=item, count=50, op='ge')])
            decline = next(r for r in doc['rules'] if r['key'].endswith('/' + name + '-decline'))
            self.assertEqual(decline['effects'], [])
            self.assertIn('yes', decline['exclude_keywords'])

    def test_ice_requires_both_quests_and_debits_exactly_fifty_fish(self):
        doc = json.loads(source.outputs(ROOT)[source.OUTPUT])
        rules = {r['key'].rsplit('/', 1)[-1]: r for r in doc['rules']}
        tomes = source.TRACK
        door = 'oteryn:quest-progress/quest/u8_0/the_ice_islands/formorgar_mines_door'
        access = 'oteryn:quest-progress/quest/u11_02/forgotten_knowledge/access_ice'
        fish = 'oteryn:item.tibia.i3578'
        guard = lambda track, op: dict(kind='storage', track=track, op=op, value=1)
        self.assertEqual(rules['ice-offer']['guards'], [guard(tomes, 'eq'), guard(door, 'eq')])
        self.assertEqual(rules['ice-offer']['effects'], [dict(kind='set_topic', value=2)])
        refusal = ["Sorry, you first need to bring my Heavy Old Tomes or do the quest before continuing."]
        self.assertEqual(rules['ice-prerequisite']['guards'], [guard(tomes, 'ne')])
        self.assertEqual(rules['ice-prerequisite']['reply'], refusal)
        self.assertEqual(rules['ice-contactman']['guards'], [guard(tomes, 'eq'), guard(door, 'ne')])
        self.assertEqual(rules['ice-contactman']['reply'], refusal)
        self.assertEqual(rules['ice-accept']['topics'], [2])
        self.assertEqual(rules['ice-accept']['guards'], [
            guard(access, 'lt'), dict(kind='item', item=fish, count=50, op='ge')])
        self.assertEqual(rules['ice-accept']['effects'], [
            dict(kind='take_item', item=fish, count=50, optional=False),
            dict(kind='set', track=access, value=1)])
