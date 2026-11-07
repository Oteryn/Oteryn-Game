"""Horse mitigation follows exact Bestiary race IDs, not mismatched color names."""
import copy
import hashlib
import json
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import canary_batch as cb
import wiki_compare as wiki


class HorseMitigationTests(unittest.TestCase):
    def fixture(self, relative):
        route = copy.deepcopy(wiki.CREATURE_MITIGATION_RACE_ROUTES[relative])
        source = {'name': 'Horse', 'raceId': route['race_id'],
                  'outfit': {'lookType': route['look_type']}, 'defenses': {'armor': 10}}
        route['source_table_sha256'] = hashlib.sha256(json.dumps(
            source, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
        content = ('{{Infobox Creature\n| name = ' + route['title'] + '\n| actualname = horse'
                   '\n| race_id = ' + str(route['race_id']) + '\n| mitigation = 0.08\n| armor = 2\n}}')
        cut = {'page_id': route['page_id'], 'title': route['title'], 'revision_id': 123,
               'revision_timestamp': '2026-06-25T00:00:00Z', 'content': content}
        return route, source, {'cut': cut, 'current': cut.copy(), 'retrieved_at': '2026-10-01T00:00:00Z'}

    def resolve(self, relative, route, source, record, name=None):
        with patch.dict(wiki.CREATURE_MITIGATION_RACE_ROUTES, {relative: route}), \
                patch.object(wiki, 'fetch', return_value=record) as fetch:
            result = wiki.creature_page(relative, name or route['registration'], source, None)
        return result, fetch.call_count

    def test_exact_race_mapping_ignores_misleading_registration_color(self):
        expected = {'mammals/grey_horse': (751, 434, 'Horse (Brown)', 85687),
                    'mammals/brown_horse': (752, 436, 'Horse (Taupe)', 53979)}
        self.assertEqual(set(expected), set(wiki.CREATURE_MITIGATION_RACE_ROUTES))
        for relative, identity in expected.items():
            route, source, record = self.fixture(relative)
            self.assertEqual(identity, (route['race_id'], route['look_type'], route['title'], route['page_id']))
            (_, binding), calls = self.resolve(relative, route, source, record)
            self.assertEqual('VERIFIED', binding['status'])
            self.assertEqual(['mitigation_percent'], binding['field_scope'])
            self.assertEqual(identity[0], binding['source_race_id'])
            self.assertTrue(binding['canonical_color_name_differs_from_registration'])
            self.assertEqual(1, calls)

    def test_wrong_race_outfit_name_or_fingerprint_prevents_lookup(self):
        relative = 'mammals/grey_horse'
        route, source, record = self.fixture(relative)
        for mutation in ({'raceId': 752}, {'raceId': True}, {'name': 'Horse (Brown)'},
                         {'outfit': {'lookType': 436}}, {'defenses': {'armor': 2}}, {'health': 100}):
            (_, binding), calls = self.resolve(relative, route, {**source, **mutation}, record)
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', binding['status'])
            self.assertEqual(0, calls)
        (_, binding), calls = self.resolve(relative, route, source, record, 'Brown Horse')
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', binding['status'])
        self.assertEqual(0, calls)

    def test_wrong_canonical_race_page_date_or_numeric_token_is_rejected(self):
        for relative in wiki.CREATURE_MITIGATION_RACE_ROUTES:
            route, source, record = self.fixture(relative)
            content = record['cut']['content']
            for mutation in ({'page_id': 85688}, {'title': 'Horse (Grey)'},
                             {'revision_timestamp': '2026-10-01T00:00:00Z'},
                             {'content': content.replace('Infobox Creature', 'Infobox NPC')},
                             {'content': content.replace('| race_id = ' + str(route['race_id']), '| race_id = 750')},
                             {'content': content.replace('| actualname = horse', '| actualname = donkey')},
                             {'content': content.replace('| mitigation = 0.08', '| mitigation = 0.08?')}):
                (_, binding), _ = self.resolve(relative, route, source,
                                               {**record, 'cut': {**record['cut'], **mutation}})
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', binding['status'])

    def test_comparison_and_adoption_change_only_mitigation(self):
        for relative in wiki.CREATURE_MITIGATION_RACE_ROUTES:
            route, source, record = self.fixture(relative)
            original = {'creature': {'stats': {'armor': 10, 'max_health': 75}},
                        'behavior': {}, 'presentation': {}}
            converter = SimpleNamespace(monster_root=Path('/source'), monster_dir='monsters',
                convert=lambda _: (None, copy.deepcopy(original), None, None, {'entries': []}, None))
            with patch.dict(wiki.CREATURE_MITIGATION_RACE_ROUTES, {relative: route}), \
                    patch.object(wiki.cb, 'CONVERTER', converter, create=True), \
                    patch.object(wiki.cb, 'load_monster', return_value=(route['registration'], source, None)), \
                    patch.object(wiki, 'fetch', return_value=record), \
                    patch.object(wiki, 'loot_statistics', side_effect=AssertionError('no loot query')):
                result = wiki.compare(relative, None, None, None)
            self.assertEqual(['mitigation_percent'], result['qualified_fields'])
            self.assertEqual(['mitigation_percent'], [r['field'] for r in result['rows']])
            self.assertEqual(0.08, result['rows'][0]['wiki'])
            adopter = object.__new__(cb.Converter)
            adopter.wiki = {result['monster']: result}
            monster = copy.deepcopy(original)
            rows = [{'source_index': 0, 'source_file': relative + '.lua',
                     'source_field': 'maxHealth', 'status': 'mapped'}]
            adopter.adopt_wiki(result['monster'], monster, rows,
                               [{'repository': cb.REPOSITORY, 'revision': cb.REVISION}], set(), lambda _: 1)
            expected = copy.deepcopy(original)
            expected['creature']['stats']['mitigation_percent'] = {'numerator': 2, 'denominator': 25}
            self.assertEqual(expected, monster)


if __name__ == '__main__':
    unittest.main()
