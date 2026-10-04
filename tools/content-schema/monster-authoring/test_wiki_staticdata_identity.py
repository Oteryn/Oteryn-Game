"""Only the exact accepted Imperial staticdata row supplements an absent Wiki race ID."""
import copy
import hashlib
import json
import unittest
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import wiki_compare as wiki


class StaticdataCreatureIdentityTests(unittest.TestCase):
    relative = 'winter_update_2025/imperial'

    def fixture(self, relative=None):
        route = copy.deepcopy(wiki.CREATURE_STATICDATA_ROUTES[relative or self.relative])
        source = {'raceId': route['race_id'], 'outfit': {'lookType': route['look_type']}, 'health': 100,
                  'maxHealth': 100, 'experience': 0}
        route['source_table_sha256'] = hashlib.sha256(json.dumps(
            source, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
        content = '{{Infobox Creature\n| name = ' + route['title']
        if route['actualname'] is not None:
            content += '\n| actualname = ' + route['actualname']
        content += '\n| hp = 100\n| exp = 0\n}}'
        route['cut_content_sha256'] = hashlib.sha256(content.encode()).hexdigest()
        cut = {'page_id': route['page_id'], 'title': route['title'], 'revision_id': 1195192,
               'revision_timestamp': '2026-07-10T23:14:44Z', 'content': content}
        return route, source, {'cut': cut, 'current': cut.copy(), 'retrieved_at': '2026-10-01T00:00:00Z'}

    def resolve(self, route, source, record, relative=None, name='Imperial'):
        route_key = relative if relative in wiki.CREATURE_STATICDATA_ROUTES else self.relative
        with patch.dict(wiki.CREATURE_STATICDATA_ROUTES, {route_key: route}), \
                patch.object(wiki, 'fetch', return_value=record) as fetch:
            result = wiki.creature_page(relative or self.relative, name, source, None)
        return result, fetch.call_count

    def test_exact_accepted_staticdata_row_binds_absent_public_race_without_inventing_it(self):
        self.assertEqual(38, len(wiki.CREATURE_STATICDATA_ROUTES))
        self.assertIn(self.relative, wiki.CREATURE_STATICDATA_ROUTES)
        route, source, record = self.fixture()
        (_, proof), calls = self.resolve(route, source, record)
        self.assertEqual('VERIFIED', proof['status'])
        self.assertEqual('ExactAcceptedStaticdataAndCanonicalCreature', proof['method'])
        self.assertEqual(2775, proof['source_race_id'])
        self.assertEqual(1914, proof['source_outfit_look_type'])
        self.assertEqual(797, proof['staticdata_source_index'])
        self.assertEqual('UNSPECIFIED', proof['wiki_race_id_status'])
        self.assertNotIn('wiki_race_id', proof)
        self.assertNotIn('field_scope', proof)
        self.assertEqual(1, calls)

    def test_altered_source_id_outfit_registration_or_table_fails_before_lookup(self):
        route, source, record = self.fixture()
        for change in ({'raceId': 2776}, {'raceId': True}, {'outfit': {'lookType': 1915}},
                       {'name': 'Other'}, {'health': 101}, {'loot': []}):
            (_, proof), calls = self.resolve(route, {**source, **change}, record)
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.assertEqual(0, calls)
        (_, proof), calls = self.resolve(route, source, record, name='imperial')
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
        self.assertEqual(0, calls)

    def test_staticdata_file_digest_change_fails_closed(self):
        route, source, record = self.fixture()
        with patch.object(Path, 'read_bytes', return_value=b'{}'):
            (_, proof), calls = self.resolve(route, source, record)
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
        self.assertEqual(0, calls)

    def test_matching_file_digest_cannot_substitute_another_staticdata_row(self):
        route, source, record = self.fixture()
        row = copy.deepcopy(route['staticdata_record'])
        for change in ({'source_id': 2776}, {'source_index': 798}, {'name': 'Other'},
                       {'look': {'hex': '08fb0e'}}):
            raw = json.dumps({'family': 'Creature', 'records': [{**row, **change}]}).encode()
            route['staticdata_file_sha256'] = hashlib.sha256(raw).hexdigest()
            with patch.object(Path, 'read_bytes', return_value=raw):
                (_, proof), calls = self.resolve(route, source, record)
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
            self.assertEqual(0, calls)

    def test_staticdata_look_is_decoded_and_not_only_a_copied_metadata_claim(self):
        route, source, record = self.fixture()
        route['staticdata_record']['look']['hex'] = '08fb0e'  # protobuf look=1915
        raw = json.dumps({'family': 'Creature', 'records': [route['staticdata_record']]}).encode()
        route['staticdata_file_sha256'] = hashlib.sha256(raw).hexdigest()
        with patch.object(Path, 'read_bytes', return_value=raw):
            (_, proof), calls = self.resolve(route, source, record)
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
        self.assertEqual(0, calls)

    def test_canonical_page_type_declared_names_cut_and_content_are_bound(self):
        route, source, record = self.fixture()
        content = record['cut']['content']
        for change in ({'page_id': 1}, {'title': 'Other'}, {'revision_timestamp': None},
                       {'revision_timestamp': '2026-10-01T00:00:00Z'},
                       {'content': content.replace('Infobox Creature', 'Infobox NPC')},
                       {'content': content.replace('| name = Imperial', '| name = Other')},
                       {'content': content.replace('| actualname = imperial', '| actualname = stag')},
                       {'content': content.replace('| hp = 100', '| hp = 101')}):
            (_, proof), _ = self.resolve(route, source, {**record, 'cut': {**record['cut'], **change}})
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_current_raw_page_cannot_add_a_conflicting_race_or_change_creature_identity(self):
        route, source, record = self.fixture()
        content = record['current']['content']
        for raw in (content.replace('\n}}', '\n| race_id = 2776\n}}'),
                    content.replace('Infobox Creature', 'Infobox NPC'),
                    content.replace('| actualname = imperial', '| actualname = stag'), None):
            (_, proof), _ = self.resolve(route, source,
                                         {**record, 'current': {**record['current'], 'content': raw or ''}})
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_public_race_if_present_must_match_and_remains_a_public_fact(self):
        route, source, record = self.fixture()
        for number, expected in ((2775, 'VERIFIED'), (2776, 'WIKI_IDENTITY_UNKNOWN')):
            candidate = copy.deepcopy(record)
            candidate['cut']['content'] = candidate['cut']['content'].replace('\n}}',
                                                        '\n| race_id = ' + str(number) + '\n}}')
            route['cut_content_sha256'] = hashlib.sha256(candidate['cut']['content'].encode()).hexdigest()
            (_, proof), _ = self.resolve(route, source, candidate)
            self.assertEqual(expected, proof['status'])
            if expected == 'VERIFIED':
                self.assertEqual(2775, proof['wiki_race_id'])
                self.assertEqual('PRESENT_MATCHES', proof['wiki_race_id_status'])

    def test_other_source_files_keep_the_generic_missing_race_guard(self):
        route, source, record = self.fixture()
        (_, proof), _ = self.resolve(route, source, record, relative='other/imperial')
        self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
        self.assertEqual('Creature race ID missing or mismatched', proof['reason'])

    def test_all_generated_routes_bind_exact_client_rows_and_dated_names(self):
        for relative, stored in wiki.CREATURE_STATICDATA_ROUTES.items():
            if stored.get('pinned_callback_keys'):
                continue
            route, source, record = self.fixture(relative)
            if route.get('reference_cut_only'):
                record.pop('current')
            (_, proof), _ = self.resolve(route, source, record, relative, route['registration'])
            self.assertEqual('VERIFIED', proof['status'], relative)
            self.assertEqual(route['race_id'], proof['source_race_id'])
            self.assertNotIn('wiki_race_id', proof)
            if not route['actualname']:
                self.assertEqual('UNSPECIFIED_NOT_INVENTED', proof['wiki_actualname_status'])

    def test_cut_only_routes_reject_contradictory_current_identity_when_supplied(self):
        for relative in ('winter_update_2025/cyclursus', 'devoted_radiant/devoted_radiant_paragon'):
            if relative not in wiki.CREATURE_STATICDATA_ROUTES:
                relative = next(k for k, r in wiki.CREATURE_STATICDATA_ROUTES.items()
                                if r['registration'] == 'Devoted Radiant Paragon')
            route, source, record = self.fixture(relative)
            record['current']['content'] = record['current']['content'].replace('\n}}', '\n| race_id = 999\n}}')
            (_, proof), _ = self.resolve(route, source, record, relative, route['registration'])
            self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])

    def test_callback_route_requires_exact_pinned_source_file_and_real_lua_callback(self):
        from lupa.luajit21 import LuaRuntime
        relative = 'winter_update_2025/stag'
        route, source, record = self.fixture(relative)
        source['onSpawn'] = LuaRuntime().eval('function() return true end')
        serial = {**source, 'onSpawn': {'pinned_source_callback': 'onSpawn'}}
        route['source_table_sha256'] = hashlib.sha256(json.dumps(
            serial, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / route['source_path']
            path.parent.mkdir(parents=True)
            pinned = b'local pinned_source = true\n'
            path.write_bytes(pinned)
            route['source_file_sha256'] = hashlib.sha256(pinned).hexdigest()
            converter = SimpleNamespace(monster_root=Path(directory), monster_dir='data-global/monster',
                                        source=route['source_repository'])
            with patch.object(wiki.cb, 'CONVERTER', converter, create=True), \
                    patch.object(wiki.subprocess, 'check_output', return_value=pinned):
                (_, proof), _ = self.resolve(route, source, record, relative, 'Stag')
                self.assertEqual('VERIFIED', proof['status'])
                path.write_bytes(b'changed source\n')
                (_, proof), _ = self.resolve(route, source, record, relative, 'Stag')
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])
                path.write_bytes(pinned)
                (_, proof), _ = self.resolve(route, {**source, 'onSpawn': lambda: True}, record, relative, 'Stag')
                self.assertEqual('WIKI_IDENTITY_UNKNOWN', proof['status'])


if __name__ == '__main__':
    unittest.main()
