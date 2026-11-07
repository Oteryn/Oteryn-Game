"""Exact field-qualified variant mitigation without broad creature/loot aliases."""
import copy
import hashlib
import json
import unittest
from fractions import Fraction
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import wiki_compare as wiki


class MitigationVariantTests(unittest.TestCase):
    def fixture(self, relative=None):
        relative = relative or next(iter(wiki.CREATURE_MITIGATION_VARIANTS))
        route = copy.deepcopy(wiki.CREATURE_MITIGATION_VARIANTS[relative])
        source = {'name': route['display'], 'health': 1000, 'outfit': {'lookTypeEx': 2187},
                  'defenses': {'armor': 50}, 'loot': [{'id': 1, 'chance': 100000}]}
        route['source_table_sha256'] = hashlib.sha256(json.dumps(
            source, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
        fields = {'name': route['title'], 'actualname': route['display'], 'mitigation': '1.31'}
        if route['wiki_race'] is not None:
            fields['race_id'] = route['wiki_race']
        for field, links in route['context_links'].items():
            fields[field] = ' '.join('[[' + title + ']]' for title in links)
        first_field = next(iter(route['context_links']))
        fields[first_field] += ' ' + ' '.join(route['context_phrases'])
        content = '{{Infobox Creature\n' + '\n'.join('| ' + k + ' = ' + v for k, v in fields.items()) + '\n}}'
        cut = {'page_id': route['page_id'], 'title': route['title'], 'revision_id': 123,
               'revision_timestamp': '2026-09-01T00:00:00Z', 'content': content}
        return relative, route, source, {'cut': cut, 'current': cut.copy(), 'retrieved_at': '2026-10-01T00:00:00Z'}

    def resolve(self, relative, route, source, record, name=None):
        with patch.dict(wiki.CREATURE_MITIGATION_VARIANTS, {relative: route}), \
                patch.object(wiki, 'fetch', return_value=record) as fetch:
            result = wiki.creature_page(relative, name or route['registration'], source, None)
        return result, fetch.call_count

    def test_certain_complete_numeric_percent_preserves_zero_and_rejects_ranges(self):
        for raw, expected in [('0', Fraction(0)), ('0.00%', Fraction(0)), (' 1.31 % ', Fraction(131, 100)),
                              ('.08', Fraction(2, 25)), ('100', Fraction(100))]:
            self.assertEqual(expected, wiki.mitigation_number(raw))
        for raw in (None, '', '?', 'unknown', '1–2%', '1-2%', '1.31 approximately', '~1.31', '1.31?',
                    '1.31 (estimated)', '1.31/2', '101', '-1', 'nan', '1e0', True, 0):
            with self.subTest(raw=raw):
                self.assertIsNone(wiki.mitigation_number(raw))

    def test_all_nine_routes_require_dated_creature_and_specific_context(self):
        self.assertEqual(9, len(wiki.CREATURE_MITIGATION_VARIANTS))
        for relative in wiki.CREATURE_MITIGATION_VARIANTS:
            relative, route, source, record = self.fixture(relative)
            (_, binding), calls = self.resolve(relative, route, source, record)
            self.assertEqual('VERIFIED', binding['status'])
            self.assertEqual(['mitigation_percent'], binding['field_scope'])
            self.assertFalse(binding['source_mitigation_inherited'])
            self.assertNotIn('source_race_id', binding)
            self.assertEqual(1, calls)

    def test_source_fingerprint_mutations_fail_before_canonical_lookup(self):
        relative, route, source, record = self.fixture()
        for mutation in ({'name': 'other'}, {'raceId': None}, {'health': 500},
                         {'outfit': {'lookTypeEx': 1}}, {'defenses': {'armor': 10}},
                         {'loot': []}, {'unexpected_source_field': True}):
            (_, proof), calls = self.resolve(relative, route, {**source, **mutation}, record)
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.assertEqual(0, calls)
        (_, proof), calls = self.resolve(relative, route, source, record, route['registration'].lower())
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
        self.assertEqual(0, calls)

    def test_canonical_mutations_reject_npc_wrong_page_cut_context_and_uncertain_value(self):
        for relative in wiki.CREATURE_MITIGATION_VARIANTS:
            relative, route, source, record = self.fixture(relative)
            cut = record['cut'];content = cut['content']
            mutations = [{'page_id': 1}, {'title': 'Other'}, {'revision_timestamp': None},
                         {'revision_timestamp': 'invalid'}, {'revision_timestamp': '2026-10-01T00:00:00Z'},
                         {'content': content.replace('Infobox Creature', 'Infobox NPC')},
                         {'content': content.replace('| actualname = ' + route['display'], '| actualname = other')},
                         {'content': content.replace('| name = ' + route['title'], '| name = other')},
                         {'content': content.replace('| mitigation = 1.31', '| mitigation = 1-2%')}]
            for links in route['context_links'].values():
                mutations.extend({'content': content.replace('[[' + link + ']]', link)} for link in links)
            mutations.extend({'content': content.replace(phrase, 'different context')} for phrase in route['context_phrases'])
            if route['wiki_race'] is not None:
                mutations.append({'content': content.replace('| race_id = ' + route['wiki_race'], '| race_id = 999')})
            for mutation in mutations:
                with self.subTest(relative=relative, mutation=mutation):
                    (_, proof), _ = self.resolve(relative, route, source, {**record, 'cut': {**cut, **mutation}})
                    self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_comparison_exposes_only_mitigation_without_loot_or_abilities(self):
        relative, route, source, record = self.fixture()
        converter = SimpleNamespace(monster_root=Path('/source'), monster_dir='monsters',
            convert=lambda _: (None, {'creature': {'stats': {}}, 'behavior': {}}, None, None,
                               {'entries': [{'resolution': 'COMBAT_UNDEFINEDDAMAGE'}]}, None))
        for raw in ('1.31', '0'):
            record['cut']['content'] = record['cut']['content'].replace('| mitigation = 1.31', '| mitigation = ' + raw)
            with patch.dict(wiki.CREATURE_MITIGATION_VARIANTS, {relative: route}), \
                    patch.object(wiki.cb, 'CONVERTER', converter, create=True), \
                    patch.object(wiki.cb, 'load_monster', return_value=(route['registration'], source, None)), \
                    patch.object(wiki, 'fetch', return_value=record), \
                    patch.object(wiki, 'loot_statistics', side_effect=AssertionError('loot must not be queried')):
                result = wiki.compare(relative, None, None, None)
            self.assertEqual(['mitigation_percent'], result['qualified_fields'])
            self.assertEqual(['mitigation_percent'], [r['field'] for r in result['rows']])
            self.assertNotIn('loot_statistics', result)
            self.assertNotIn('abilities', result)
            self.assertEqual(0 if raw == '0' else 1.31, result['rows'][0]['wiki'])
            self.assertEqual('DIFF', result['rows'][0]['status'])

    def test_other_source_files_and_old_look_variants_never_gain_proxy_routes(self):
        for relative in ('nostalgia/old_spider', 'nostalgia/old_bear', 'other/orc_armor'):
            self.assertNotIn(relative, wiki.CREATURE_MITIGATION_VARIANTS)
            with patch.object(wiki, 'fetch', return_value={'cut': None}) as fetch:
                _, proof = wiki.creature_page(relative, 'Old Spider', {}, None)
            self.assertEqual('WIKI_PAGE_MISSING', proof['status'])
            self.assertEqual('Old Spider', fetch.call_args.args[0])


if __name__ == '__main__':
    unittest.main()
