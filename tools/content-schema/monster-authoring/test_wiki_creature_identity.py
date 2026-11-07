"""Wiki creature binding needs a Creature infobox and exact source variant identity."""
import unittest
import json
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import wiki_compare as wiki


def record(title, page_id, content):
    revision = {'title': title, 'page_id': page_id, 'revision_id': 100,
                'revision_timestamp': '2026-09-04T10:00:00Z', 'content': content}
    return {'cut': revision, 'current': revision.copy(), 'retrieved_at': '2026-10-01T12:00:00Z'}


def creature(title, page_id, race_id):
    return record(title, page_id, '{{Infobox Creature\n| race_id = ' + str(race_id)
                  + '\n| hp = 150\n| mitigation = 0.64\n}}')


class WikiCreatureIdentity(unittest.TestCase):
    def setUp(self):
        self.pages = {'Nomads': record('Nomads', 23774,
                                      '{{disambig}}\n[[Nomad (Basic)]]\n'
                                      '[[Nomad (Blue)|Blue]]\n[[Nomad (Female)]]')}
        for _, (_, race_id, title, page_id) in wiki.CREATURE_PAGE_ROUTES.items():
            self.pages[title] = creature(title, page_id, race_id)

    def resolve(self, relative='humans/nomad', name='Nomad', source=None):
        with patch.object(wiki, 'fetch', side_effect=lambda title, _: self.pages.get(title, {'cut': None})) as fetch:
            result = wiki.creature_page(relative, name, {'raceId': 310} if source is None else source, None)
        return result, [call.args[0] for call in fetch.call_args_list]

    def test_three_variants_use_disambiguation_and_exact_race_ids(self):
        for relative in ('humans/nomad', 'humans/nomad_blue', 'humans/nomad_female'):
            name, race_id, title, _ = wiki.CREATURE_PAGE_ROUTES[relative]
            with self.subTest(relative=relative):
                (page, proof), calls = self.resolve(relative, name, {'raceId': race_id})
                self.assertEqual('VERIFIED', proof['status'])
                self.assertEqual('DisambiguationAndExactRaceId', proof['method'])
                self.assertEqual(race_id, proof['wiki_race_id'])
                self.assertEqual(title, page['cut']['title'])
                self.assertEqual(['Nomads', title], calls)

    def test_same_name_npc_is_never_a_creature_binding(self):
        self.pages['Nomad'] = record('Nomad', 80830, '{{Infobox NPC\n| name = Nomad\n}}')
        (page, proof), _ = self.resolve('other/nomad')
        self.assertEqual(80830, page['cut']['page_id'])
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_comparison_rejects_npc_without_emitting_adoptable_rows(self):
        converter = SimpleNamespace(monster_root=Path('/source'), monster_dir='monsters',
                                    convert=lambda _: (None, {}, None, None, {'entries': []}, None))
        npc = record('Nomad', 80830, '{{Infobox NPC\n| name = Nomad\n}}')
        with patch.object(wiki.cb, 'CONVERTER', converter, create=True), \
                patch.object(wiki.cb, 'load_monster', return_value=('Nomad', {'raceId': 310}, None)), \
                patch.object(wiki, 'fetch', return_value=npc):
            result = wiki.compare('other/nomad', None, None, None)
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', result['status'])
        self.assertEqual([], result['rows'])

    def test_general_noncreature_pages_never_reach_loot_comparison(self):
        converter = SimpleNamespace(monster_root=Path('/source'), monster_dir='monsters',
                                    convert=lambda _: (None, {}, None, None, {'entries': []}, None))
        source = {'loot': [{'name': 'Gold Coin', 'chance': 100000}]}
        for content in (
                '{{Infobox NPC\n| name = Pythius The Rotten\n'
                '| loot = {{Loot Item|Gold Coin}}\n}}',
                '{{Infobox Item\n| name = Fish\n| itemid = 3578\n}}',
                '{{Disambig}}\n[[Monk (Creature)]]\n[[Monk (Vocation)]]'):
            with self.subTest(content=content), \
                    patch.object(wiki.cb, 'CONVERTER', converter, create=True), \
                    patch.object(wiki.cb, 'load_monster', return_value=('Pythius The Rotten', source, None)), \
                    patch.object(wiki, 'fetch', return_value=record('Pythius The Rotten', 31558, content)), \
                    patch.object(wiki, 'loot_statistics', side_effect=AssertionError('noncreature loot lookup')):
                result = wiki.compare('bosses/pythius_the_rotten', None, None, None)
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', result['status'])
                self.assertEqual([], result['rows'])
                self.assertNotIn('loot_statistics', result)
                self.assertNotIn('loot_chances', result)

    def test_variant_wrong_source_name_or_race_rejects_before_page_lookup(self):
        for name, source in [('Nomad Blue', {'raceId': 310}), ('Nomad', {'raceId': 776}),
                             ('Nomad', {}), ('Nomad', {'raceId': True})]:
            with self.subTest(name=name, source=source):
                (_, proof), calls = self.resolve(name=name, source=source)
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
                self.assertEqual([], calls)

    def test_wrong_page_title_id_type_or_race_is_unknown(self):
        for title, page_id, race_id in [('Nomad', 12143, 310), ('Nomad (Basic)', 80830, 310),
                                      ('Nomad (Basic)', 12143, 776), ('Nomad (Basic)', 12143, '?'),
                                      ('Nomad (Basic)', 12143, '310 and 776')]:
            with self.subTest(title=title, page_id=page_id, race_id=race_id):
                self.pages['Nomad (Basic)'] = creature(title, page_id, race_id)
                (_, proof), _ = self.resolve()
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
        self.pages['Nomad (Basic)'] = record('Nomad (Basic)', 12143, '{{Infobox NPC\n| raceid = 310\n}}')
        (_, proof), _ = self.resolve()
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_missing_disambiguation_link_or_page_proof_is_unknown(self):
        for page in [record('Nomads', 99, '[[Nomad (Basic)]]'),
                     record('Nomad', 23774, '[[Nomad (Basic)]]'),
                     record('Nomads', 23774, '[[Nomad (Blue)]]'), {'cut': None}]:
            with self.subTest(page=page):
                self.pages['Nomads'] = page
                (_, proof), _ = self.resolve()
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_missing_variant_page_is_explicitly_missing(self):
        del self.pages['Nomad (Basic)']
        (_, proof), _ = self.resolve()
        self.assertEqual('WIKI_PAGE_MISSING', proof['status'])

    def test_regular_creature_without_source_race_preserves_fallback(self):
        self.pages['Dragon'] = creature('Dragon', 123, 34)
        (_, proof), calls = self.resolve('dragons/dragon', 'Dragon', {})
        self.assertEqual('VERIFIED', proof['status'])
        self.assertEqual('CreatureInfobox', proof['method'])
        self.assertNotIn('source_race_id', proof)
        self.assertEqual(['Dragon'], calls)

    def test_redirect_alias_requires_exact_race_when_source_has_one(self):
        self.pages['Alias'] = creature('Canonical Creature', 987, 34)
        (_, proof), _ = self.resolve('dragons/alias', 'Alias', {'raceId': 34})
        self.assertEqual('VERIFIED', proof['status'])
        self.assertEqual('CreatureInfoboxAndExactRaceId', proof['method'])
        (_, proof), _ = self.resolve('dragons/alias', 'Alias', {'raceId': 35})
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_missing_malformed_and_postcut_timestamps_fail_closed_for_both_pages(self):
        for title in ('Nomads', 'Nomad (Basic)'):
            original = dict(self.pages[title]['cut'])
            for timestamp in (None, '', 'not-a-timestamp', '2026-09-27',
                              '2026-09-27T12:00:00', 123, '2026-09-28T00:00:00.001Z',
                              '2026-09-27T23:30:00-01:00'):
                with self.subTest(title=title, timestamp=timestamp):
                    self.pages[title]['cut'] = {**original, 'revision_timestamp': timestamp}
                    (_, proof), _ = self.resolve()
                    self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.pages[title]['cut'] = {k: v for k, v in original.items() if k != 'revision_timestamp'}
            (_, proof), _ = self.resolve()
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.pages[title]['cut'] = original

    def test_cut_boundary_and_equivalent_aware_offsets_are_eligible(self):
        for timestamp in ('2026-09-28T00:00:00Z', '2026-09-28T02:00:00+02:00',
                          '2026-09-27T23:59:59.999Z'):
            with self.subTest(timestamp=timestamp):
                for title in ('Nomads', 'Nomad (Basic)'):
                    self.pages[title]['cut']['revision_timestamp'] = timestamp
                (_, proof), _ = self.resolve()
                self.assertEqual('VERIFIED', proof['status'])

    def test_captured_source_pointers_resolve_exact_canonical_pages(self):
        for relative, (name, race_id, title, page_id) in wiki.CREATURE_PAGE_ROUTES.items():
            pointer_title, pointer_id = wiki.CREATURE_PAGE_POINTERS[relative]
            if pointer_title == 'Nomads':
                continue
            link_title = wiki.CREATURE_PAGE_POINTER_LINKS.get(relative, title)
            content = ('{{Disambiguation|disambig_title=' + title + '}}' if name == 'Monk'
                       else 'For the creature see [[' + link_title + ']].')
            self.pages[pointer_title] = record(pointer_title, pointer_id, content)
            source = {} if race_id is None else {'raceId': race_id}
            if relative in wiki.CREATURE_SOURCE_LOOKS:
                source['outfit'] = {'lookType': wiki.CREATURE_SOURCE_LOOKS[relative]}
            (page, proof), calls = self.resolve(relative, name, source)
            self.assertEqual('VERIFIED', proof['status'])
            self.assertEqual(page_id, page['cut']['page_id'])
            self.assertEqual([pointer_title, title], calls)
            self.assertEqual(relative, proof['source_file'])
            self.assertEqual('SourcePointerAndSourceFile' if race_id is None else
                             'SourcePointerAndExactRaceId', proof['method'])
            if race_id is None:
                self.assertNotIn('source_race_id', proof)
                self.assertNotIn('wiki_race_id', proof)

    def test_pointer_routes_without_source_race_reject_added_race_or_other_source_file(self):
        relative = 'bosses/pythius_the_rotten'
        name, _, title, _ = wiki.CREATURE_PAGE_ROUTES[relative]
        pointer_title, pointer_id = wiki.CREATURE_PAGE_POINTERS[relative]
        self.pages[pointer_title] = record(pointer_title, pointer_id, '[[' + title + ']]')
        for source in ({'raceId': 534}, {'raceId': False}, {'raceId': '534'}):
            (_, proof), calls = self.resolve(relative, name, source)
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.assertEqual([], calls)
        (_, proof), calls = self.resolve('other/pythius_the_rotten', name, {})
        self.assertEqual('WIKI_PAGE_MISSING', proof['status'])
        self.assertEqual([name], calls)

    def test_wrong_pointer_and_target_metadata_fail_closed(self):
        relative = 'aquatics/fish'
        name, race_id, title, page_id = wiki.CREATURE_PAGE_ROUTES[relative]
        pointer_title, pointer_id = wiki.CREATURE_PAGE_POINTERS[relative]
        original = record(pointer_title, pointer_id, '[[' + title + ']]')
        for mutation in ({'page_id': pointer_id + 1}, {'title': 'Other'},
                         {'content': '[[Fish]]'}, {'content': ['[[' + title + ']]']},
                         {'revision_id': True}, {'page_id': str(pointer_id)},
                         {'revision_timestamp': '2026-10-01T12:00:00Z'}):
            self.pages[pointer_title] = {**original, 'cut': {**original['cut'], **mutation}}
            (_, proof), calls = self.resolve(relative, name, {'raceId': race_id})
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.assertEqual([pointer_title], calls)
        self.pages[pointer_title] = original
        target = self.pages[title]
        for mutation in ({'page_id': page_id + 1}, {'title': 'Fish'}, {'content': None},
                         {'revision_id': 0}, {'revision_timestamp': ''}):
            self.pages[title] = {**target, 'cut': {**target['cut'], **mutation}}
            (_, proof), _ = self.resolve(relative, name, {'raceId': race_id})
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def case_pages(self):
        for _, (name, title, page_id) in wiki.CREATURE_CASE_ROUTES.items():
            self.pages[title] = record(title, page_id, '{{Infobox Creature\n| name = ' + title
                                       + '\n| actualname = ' + name.lower() + '\n| hp = 150\n}}')

    def test_twelve_explicit_case_routes_bind_dated_declared_names_without_source_race(self):
        self.case_pages()
        for relative, (name, title, page_id) in wiki.CREATURE_CASE_ROUTES.items():
            with self.subTest(relative=relative):
                (page, proof), calls = self.resolve(relative, name, {})
                self.assertEqual('VERIFIED', proof['status'])
                self.assertEqual('ExplicitCaseVariantAndSourceFile', proof['method'])
                self.assertEqual(relative, proof['source_file'])
                self.assertEqual(page_id, page['cut']['page_id'])
                self.assertEqual([title], calls)
                self.assertNotIn('source_race_id', proof)
                self.assertNotIn('wiki_race_id', proof)

    def test_case_routes_reject_changed_registration_or_added_race_before_lookup(self):
        self.case_pages()
        for name, source in [('Raging Mage', {}), ('Raging mage', {'raceId': None}),
                             ('Raging mage', {'raceId': 718}), ('Raging mage', {'raceId': False})]:
            with self.subTest(name=name, source=source):
                (_, proof), calls = self.resolve('bosses/raging_mage', name, source)
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
                self.assertEqual([], calls)

    def test_case_route_requires_exact_page_and_both_literal_creature_names(self):
        self.case_pages()
        original = self.pages['Raging Mage']
        for mutation in ({'title': 'Raging mage'}, {'page_id': 1},
                         {'content': '{{Infobox NPC|name=Raging Mage|actualname=Raging Mage}}'},
                         {'content': '{{Infobox Creature|name=Other|actualname=Raging Mage}}'},
                         {'content': '{{Infobox Creature|name=Raging Mage|actualname=Other}}'},
                         {'content': '{{Infobox Creature|name=Raging Mage}}'},
                         {'content': '{{Infobox Creature|actualname=Raging Mage}}'},
                         {'content': '{{Infobox Creature|name=Raging Mage|actualname=Raging  Mage}}'},
                         {'revision_timestamp': None}, {'revision_timestamp': '2026-10-01T00:00:00Z'}):
            with self.subTest(mutation=mutation):
                self.pages['Raging Mage'] = {**original, 'cut': {**original['cut'], **mutation}}
                (_, proof), _ = self.resolve('bosses/raging_mage', 'Raging mage', {})
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_case_hint_is_never_applied_to_another_source_file(self):
        self.case_pages()
        (_, proof), calls = self.resolve('other/raging_mage', 'Raging mage', {})
        self.assertEqual('WIKI_PAGE_MISSING', proof['status'])
        self.assertEqual(['Raging mage'], calls)

    def test_compact_population_roundtrips_metadata_after_monsters_and_unicode_aliases(self):
        reports = [
            {'source': 'Fandom', 'monsters': [
                {'monster': 'pinata_dragon', 'wiki_title': 'Piñata Dragon'},
                {'monster': 'echo', 'rows': [{'field': 'name', 'wiki': 'Żółw'}]}],
             'recovery': {'method': 'ExplicitCaseVariant', 'aliases': ['Piñata', 'Żółw']},
             'target_cut': '2026-09-27'},
            {'monsters': []},
        ]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'population.json'
            for report in reports:
                with self.subTest(report=report):
                    wiki.write_compact_population(path, report)
                    text = path.read_text(encoding='utf-8')
                    self.assertEqual(report, json.loads(text))
                    record_lines = [line for line in text.splitlines() if line.startswith('    {')]
                    self.assertEqual(len(report['monsters']), len(record_lines))
                    if report['monsters']:
                        self.assertIn('Piñata Dragon', text)
                        self.assertIn('Żółw', text)

    def actor_source_and_page(self, relative):
        route = wiki.CREATURE_CONTEXT_ROUTES[relative]
        source = {'health': route['health'], 'maxHealth': route['health'], 'speed': 0,
                  'outfit': {'lookTypeEx': route['look_type_ex']}}
        if route['event']:
            source['events'] = [route['event']]
        content = ('{{Infobox Creature\n| name = ' + route['title'] + '\n| actualname = '
                   + route['registration'].lower() + '\n| ' + route['wiki_context_field'] + ' = [['
                   + route['wiki_context_link'] + ']]\n| hp = 7500?\n}}')
        self.pages[route['title']] = record(route['title'], route['page_id'], content)
        return route, source

    def test_pink_butterfly_requires_source_race_render_and_exact_pointer(self):
        self.pages['Butterfly'] = record('Butterfly', 38909, '[[Butterfly (Purple)]]')
        relative = 'quests/the_explorer_society/pink_butterfly'
        (page, proof), calls = self.resolve(relative, 'Pink Butterfly',
                                           {'raceId': 213, 'outfit': {'lookType': 213}})
        self.assertEqual('VERIFIED', proof['status'])
        self.assertEqual(3235, page['cut']['page_id'])
        self.assertEqual(['Butterfly', 'Butterfly (Purple)'], calls)
        self.assertEqual(213, proof['source_outfit_look_type'])
        for outfit in ({'lookType': 214}, {'lookType': True}, {'lookTypeEx': 213},
                       {'lookType': 213, 'lookTypeEx': 391}, None):
            (_, proof), calls = self.resolve(relative, 'Pink Butterfly', {'raceId': 213, 'outfit': outfit})
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.assertEqual([], calls)

    def test_encounter_actor_proof_keeps_source_race_absent_and_uncertain_hp_unadoptable(self):
        for relative in wiki.CREATURE_CONTEXT_ROUTES:
            route, source = self.actor_source_and_page(relative)
            (page, proof), calls = self.resolve(relative, route['registration'], source)
            self.assertEqual('VERIFIED', proof['status'])
            self.assertEqual('EncounterActorAndSourceFile', proof['method'])
            self.assertEqual([route['title']], calls)
            self.assertEqual(route['wiki_context_link'], proof['wiki_context_link'])
            self.assertNotIn('source_race_id', proof)
            self.assertNotIn('wiki_race_id', proof)
            self.assertIsNone(wiki.number(wiki.infobox(page['cut']['content'])['hp']))

    def test_actor_source_variant_mutations_and_zero_health_fail_before_lookup(self):
        for relative in wiki.CREATURE_CONTEXT_ROUTES:
            route, original = self.actor_source_and_page(relative)
            mutations = [{'outfit': {'lookTypeEx': 1}}, {'outfit': {'lookTypeEx': True}},
                         {'outfit': {'lookTypeEx': route['look_type_ex'], 'lookType': 1}},
                         {'health': 0}, {'maxHealth': 0}, {'speed': 1}, {'raceId': None},
                         {'raceId': 213}, {'events': ['OtherEncounter']}]
            if route['event']:
                mutations.append({'events': []})
            for mutation in mutations:
                with self.subTest(relative=relative, mutation=mutation):
                    (_, proof), calls = self.resolve(relative, route['registration'], {**original, **mutation})
                    self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
                    self.assertEqual([], calls)
            (_, proof), calls = self.resolve(relative, route['registration'].lower(), original)
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.assertEqual([], calls)

    def test_actor_target_requires_qualified_names_page_cut_and_encounter_link(self):
        for relative in wiki.CREATURE_CONTEXT_ROUTES:
            route, source = self.actor_source_and_page(relative)
            original = self.pages[route['title']]
            valid_content = original['cut']['content']
            for mutation in ({'page_id': 1}, {'title': route['registration']},
                             {'revision_timestamp': '2026-10-01T00:00:00Z'},
                             {'content': valid_content.replace('Infobox Creature', 'Infobox NPC')},
                             {'content': valid_content.replace(route['registration'].lower(), 'other creature')},
                             {'content': valid_content.replace('[[' + route['wiki_context_link'] + ']]', '[[Other Encounter]]')},
                             {'content': valid_content.replace('[[' + route['wiki_context_link'] + ']]', route['wiki_context_link'])}):
                with self.subTest(relative=relative, mutation=mutation):
                    self.pages[route['title']] = {**original, 'cut': {**original['cut'], **mutation}}
                    (_, proof), _ = self.resolve(relative, route['registration'], source)
                    self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_wine_cask_context_conflict_does_not_enable_suffix_lookup(self):
        self.pages['Wine Cask'] = record('Wine Cask', 14810, '{{Infobox Object|name=Wine Cask}}')
        self.pages['Wine Cask (Creature)'] = record('Wine Cask (Creature)', 82338,
                                                   '{{Infobox Creature|name=Wine Cask (Creature)|actualname=wine cask}}')
        (_, proof), calls = self.resolve('quests/cults_of_tibia/bosses/wine_cask', 'Wine Cask', {})
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
        self.assertEqual(['Wine Cask'], calls)

    def encounter_form_source_and_page(self, relative):
        route = wiki.CREATURE_ENCOUNTER_FORM_ROUTES[relative]
        source = {**route['source_fields'], 'outfit': dict(route['outfit']),
                  'elements': [{'type': '@COMBAT_PHYSICALDAMAGE', 'percent': route['physical_percent']}]}
        if route['display'] != route['registration']:
            source['name'] = route['display']
        fields = {'name': route['title'], 'actualname': route['actualname'], 'hp': '?'}
        for field, links in route['context_links'].items():
            fields[field] = ' '.join('[[' + link + ']]' for link in links)
        for field, phrases in route['context_phrases'].items():
            fields[field] = fields.get(field, '') + ' ' + ' '.join(phrases)
        self.pages[route['title']] = record(route['title'], route['page_id'],
            '{{Infobox Creature\n' + '\n'.join('| ' + k + ' = ' + v for k, v in fields.items()) + '\n}}')
        return route, source

    def test_final_encounter_forms_bind_only_exact_phase_and_source_file(self):
        for relative in wiki.CREATURE_ENCOUNTER_FORM_ROUTES:
            route, source = self.encounter_form_source_and_page(relative)
            (page, proof), calls = self.resolve(relative, route['registration'], source)
            self.assertEqual('VERIFIED', proof['status'])
            self.assertEqual('EncounterFormAndSourceFile', proof['method'])
            self.assertEqual(route['page_id'], page['cut']['page_id'])
            self.assertEqual([route['title']], calls)
            self.assertNotIn('wiki_race_id', proof)
            self.assertNotIn('source_race_id', proof)
            if 'hermit' in relative:
                self.assertIn('not adopted', proof['mechanic_context_conflict'])
            (_, proof), calls = self.resolve('other/' + relative, route['registration'], source)
            self.assertNotEqual('VERIFIED', proof['status'])
            self.assertEqual([route['registration']], calls)

    def test_encounter_form_source_mutations_do_not_lookup_aliases(self):
        for relative in wiki.CREATURE_ENCOUNTER_FORM_ROUTES:
            route, source = self.encounter_form_source_and_page(relative)
            mutations = [{'name': 'Other Form'}, {'raceId': None}, {'raceId': 107}, {'events': []},
                         {'health': 0}, {'maxHealth': True}, {'experience': 99}, {'corpse': 1},
                         {'outfit': {**source['outfit'], 'lookBody': 97 if source['outfit']['lookBody'] != 97 else 85}},
                         {'outfit': {**source['outfit'], 'lookType': True}},
                         {'outfit': {**source['outfit'], 'lookTypeEx': 391}},
                         {'elements': [{'type': '@COMBAT_PHYSICALDAMAGE', 'percent': 1}]},
                         {'elements': []}, {'elements': None}, {'elements': {}}]
            for mutation in mutations:
                with self.subTest(relative=relative, mutation=mutation):
                    (_, proof), calls = self.resolve(relative, route['registration'], {**source, **mutation})
                    self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
                    self.assertEqual([], calls)

    def test_encounter_form_target_mutations_reject_npc_phase_and_postcut(self):
        for relative in wiki.CREATURE_ENCOUNTER_FORM_ROUTES:
            route, source = self.encounter_form_source_and_page(relative)
            original = self.pages[route['title']];content = original['cut']['content']
            mutations = [{'page_id': 1}, {'title': 'Goblin'}, {'revision_timestamp': None},
                         {'revision_timestamp': 'invalid'}, {'revision_timestamp': '2026-10-01T00:00:00Z'},
                         {'content': content.replace('Infobox Creature', 'Infobox NPC')},
                         {'content': content.replace('| actualname = ' + route['actualname'], '| actualname = other')},
                         {'content': content.replace('| name = ' + route['title'], '| name = other')}]
            for field, links in route['context_links'].items():
                for link in links:
                    mutations.append({'content': content.replace('[[' + link + ']]', link)})
            for field, phrases in route['context_phrases'].items():
                for phrase in phrases:
                    mutations.append({'content': content.replace(phrase, 'different encounter')})
            for mutation in mutations:
                with self.subTest(relative=relative, mutation=mutation):
                    self.pages[route['title']] = {**original, 'cut': {**original['cut'], **mutation}}
                    (_, proof), _ = self.resolve(relative, route['registration'], source)
                    self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_regular_creature_cached_postcut_revision_is_unknown(self):
        self.pages['Dragon'] = creature('Dragon', 123, 34)
        self.pages['Dragon']['cut']['revision_timestamp'] = '2026-10-01T12:00:00Z'
        (_, proof), _ = self.resolve('dragons/dragon', 'Dragon', {})
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])


if __name__ == '__main__':
    unittest.main()
