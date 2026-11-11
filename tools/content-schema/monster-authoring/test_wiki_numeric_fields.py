"""Integer wiki counts retain grouping without changing decimal percentages."""
import copy
import unittest
from fractions import Fraction
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import canary_batch as cb
import wiki_br_fill
import wiki_compare as wiki


class WikiNumericFieldTests(unittest.TestCase):
    def compare(self, **fields):
        original = {
            'creature': {
                'stats': {'max_health': 10000, 'initial_health': 10000, 'experience': 0,
                          'armor': 80, 'speed': 0, 'mitigation_percent': {'numerator': 0, 'denominator': 1}},
                'resistances': [], 'immunities': {'damage_types': [], 'conditions': []},
                'summoning': {'summonable': False, 'convinceable': False},
                'flags': {'illusionable': False}},
            'behavior': {'movement': {'pushable': False, 'push_items': False},
                         'targeting': {'sense_invisible': False, 'flee_health': 0}}}
        content = '{{Infobox Creature\n| name = Soul Sphere\n' + '\n'.join(
            '| ' + key + ' = ' + value for key, value in fields.items()) + '\n}}'
        cut = {'page_id': 1, 'title': 'Soul Sphere', 'revision_id': 123,
               'revision_timestamp': '2026-09-01T00:00:00Z', 'content': content}
        record = {'cut': cut, 'current': cut.copy(), 'retrieved_at': '2026-10-01T00:00:00Z'}
        converter = SimpleNamespace(monster_root=Path('/source'), monster_dir='monsters',
            convert=lambda _: (None, copy.deepcopy(original), None, None, {'entries': []}, None))
        with patch.object(cb, 'CONVERTER', converter, create=True), \
                patch.object(cb, 'load_monster', return_value=('Soul Sphere', {'loot': []}, None)), \
                patch.object(wiki, 'creature_page', return_value=(record, {'status': 'VERIFIED'})), \
                patch.object(wiki, 'loot_statistics', side_effect=AssertionError('no loot query')):
            result = wiki.compare('quests/soul_war/soul_sphere', None, None, None)
        return original, result

    def test_grouped_sphere_health_reaches_existing_adoption_as_fifteen_thousand(self):
        # Raw hp spelling retained by the frozen Soul Sphere manifest, Fandom revision 1002465.
        original, result = self.compare(hp='15.000', exp='1,234,567', armor='3.000')
        fields = {row['field']: row for row in result['rows']}
        self.assertEqual(fields['max_health']['wiki'], 15000)
        self.assertEqual(fields['max_health']['wiki_raw'], '15.000')
        self.assertEqual(fields['max_health']['status'], 'DIFF')
        adopter = object.__new__(cb.Converter)
        adopter.wiki = {result['monster']: result}
        rows = [{'source_index': 0, 'source_file': 'quests/soul_war/soul_sphere.lua',
                 'source_field': field, 'status': 'mapped'} for field in ('health', 'maxHealth', 'experience', 'defenses.armor')]
        monster = copy.deepcopy(original)
        sources = [{'repository': cb.REPOSITORY, 'revision': cb.REVISION}]
        adopter.adopt_wiki(result['monster'], monster, rows, sources, set(), lambda _: 1)
        self.assertEqual(monster['creature']['stats']['max_health'], 15000)
        self.assertEqual(monster['creature']['stats']['initial_health'], 15000)
        self.assertEqual(monster['creature']['stats']['experience'], 1234567)
        self.assertEqual(monster['creature']['stats']['armor'], 3000)
        hp_proof = next(row for row in rows if row['source_field'] == 'Infobox Creature.hp')
        self.assertIn('"15.000"', hp_proof['resolution'])
        self.assertEqual(sources[hp_proof['source_index']]['content_sha256'], result['cut_content_sha256'])

    def test_integer_field_parser_rejects_partial_or_malformed_tokens(self):
        for raw in ('15.00', '15,00', '1.234,000', '1,234.000', '15.000 HP',
                    '15.000%', '15.000/2', '15.000-20.000', '0.500'):
            with self.subTest(raw=raw):
                _, result = self.compare(hp=raw, exp=raw, armor=raw, speed=raw, summon=raw, runsat=raw)
                fields = {row['field']: row for row in result['rows']}
                for field in ('max_health', 'experience', 'armor', 'speed', 'summon_mana_cost', 'flee_health'):
                    self.assertEqual(fields[field]['status'], 'WIKI_UNPARSED')

    def test_decimal_resistance_and_mitigation_remain_decimal(self):
        _, result = self.compare(hp='15.000', mitigation='3.400', physicalDmgMod='99.500%')
        fields = {row['field']: row for row in result['rows']}
        self.assertEqual(fields['mitigation_percent']['wiki'], 3.4)
        self.assertEqual(fields['resistance.physical']['wiki'], 0.5)
        self.assertEqual(wiki.number('15.000'), Fraction(15))

    def test_grouped_mana_and_flee_health_use_integer_context(self):
        _, result = self.compare(summon='2.000', convince='2,000', runsat='1.000')
        fields = {row['field']: row for row in result['rows']}
        for field in ('summon_mana_cost', 'convince_mana_cost'):
            self.assertEqual(fields[field]['wiki'], 2000)
        self.assertEqual(fields['flee_health']['wiki'], 1000)

    def test_br_fallback_uses_the_same_certain_integer_rule(self):
        capture = {'pages': [{'title': 'Soul Sphere', 'page_title': 'Soul Sphere', 'page_id': 1,
            'revision_id': 123, 'revision_timestamp': '2026-09-01T00:00:00Z', 'content_sha256': 'a' * 64,
            'fields': {'hp': '15.000'}, 'field_lines': {'hp': 6}}]}
        sample = {'monsters': [{'monster': 'soul_sphere', 'wiki_title': 'Soul Sphere', 'status': 'COMPARED'}]}
        for raw in ('15.00', '15.000%', '1.23.000'):
            with self.subTest(raw=raw):
                filled = wiki_br_fill.fill(capture, sample, lambda _: {'hp': raw})
                self.assertEqual(len(filled), 1)
                self.assertEqual(filled[0]['fields']['max_health']['br'], 15000)
        for raw in ('15000', '15.000', '15,000'):
            self.assertEqual(wiki_br_fill.fill(capture, sample, lambda _: {'hp': raw}), [])

    def test_br_grouping_cannot_promote_fractional_or_malformed_health(self):
        for raw in ('0.500', '15.00', '1.23.000', '15,000'):
            with self.subTest(raw=raw):
                self.assertIsNone(wiki_br_fill.plain_number(raw))


if __name__ == '__main__':
    unittest.main()
