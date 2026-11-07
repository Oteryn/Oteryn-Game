"""Source-qualified Bestiary whole-profile adoption; compact absence never proves a fact."""
import copy
import json
import os
import unittest
from pathlib import Path

import canary_batch as cb
import crystal_batch
import validate_monster as vm

MATRIX = json.loads((cb.ROOT / 'samples/source-bestiary-matrix-2026-09-27.json').read_text())
CANARY, CRYSTAL = os.environ.get('OTERYN_CANARY'), os.environ.get('OTERYN_CRYSTAL')
REGULAR = [(0, 1, [5, 10, 25]), (1, 5, [10, 100, 250]), (2, 15, [25, 250, 500]),
           (3, 25, [50, 500, 1000]), (4, 50, [100, 1000, 2500]), (5, 100, [200, 2000, 5000])]
RARE_POINTS = [5, 10, 30, 50, 100, 200]


def fixture(difficulty='easy', occurrence='common'):
    record = {'status': 'COMPARED', 'creature_identity': {'status': 'VERIFIED'}, 'wiki_title': 'Example',
              'page_id': 1, 'cut_revision_id': 2, 'cut_content_sha256': 'a' * 64,
              'qualified_bestiary_facts': {'page_id': 1, 'revision_id': 2, 'content_sha256': 'a' * 64,
                  'difficulty': {'value': difficulty, 'raw': difficulty.title(), 'wiki_line': 10},
                  'occurrence': {'value': occurrence, 'raw': occurrence.replace('_', ' ').title(), 'wiki_line': 11}}}
    creature = {'bestiary': {'class': 'Human', 'taxonomy': 'human', 'difficulty': 'trivial',
                            'occurrence': 'common', 'stars': 1, 'charm_points': 999,
                            'kill_thresholds': [1, 5, 5], 'locations': 'source location'}}
    sources = [{'repository': 'example/source', 'revision': 'a' * 40},
               {'kind': 'mediawiki', 'api': 'https://example.com/api.php', 'title': 'Example',
                'page_id': 1, 'revision_id': 2, 'content_sha256': 'a' * 64}]
    rows = []
    return record, creature, rows, sources


def adopt(record, creature, rows, sources):
    cb.Converter.adopt_wiki_bestiary_matrix(record, creature, rows, sources, 1)


class QualifiedMatrix(unittest.TestCase):
    def test_all_regular_difficulties_use_public_thresholds_and_points(self):
        for stars, points, thresholds in REGULAR:
            difficulty = cb.DIFFICULTY[stars]
            for occurrence in ('common', 'uncommon', 'rare'):
                with self.subTest(difficulty=difficulty, occurrence=occurrence):
                    r, c, rows, sources = fixture(difficulty, occurrence)
                    adopt(r, c, rows, sources)
                    self.assertEqual(stars, c['bestiary']['stars'])
                    self.assertEqual(points, c['bestiary']['charm_points'])
                    self.assertEqual(thresholds, c['bestiary']['kill_thresholds'])
                    self.assertEqual(occurrence, c['bestiary']['occurrence'])
                    self.assertEqual('source location', c['bestiary']['locations'])
                    self.assertEqual('human', c['bestiary']['taxonomy'])
                    self.assertTrue(all(e['status'] == 'mapped' for e in rows))
                    self.assertTrue(all(e['source_index'] < len(sources) for e in rows))

    def test_very_rare_uses_the_public_triplet_and_rarity_points(self):
        self.assertTrue(MATRIX['very_rare_triplet_qualified'])
        for stars, points in enumerate(RARE_POINTS):
            r, c, rows, sources = fixture(cb.DIFFICULTY[stars], 'very_rare')
            adopt(r, c, rows, sources)
            self.assertEqual([2, 3, 5], c['bestiary']['kill_thresholds'])
            self.assertEqual(points, c['bestiary']['charm_points'])
            self.assertEqual(stars, c['bestiary']['stars'])
            self.assertTrue(any(e['source_file'] == 'Template:Charm Points' for e in rows))

    def test_no_compact_diff_or_match_is_not_a_qualified_whole_profile(self):
        r, c, rows, sources = fixture()
        del r['qualified_bestiary_facts']
        r['rows'] = [{'field': 'bestiary.difficulty', 'status': 'MATCH'}]
        before = copy.deepcopy((c, rows, sources))
        adopt(r, c, rows, sources)
        self.assertEqual(before, (c, rows, sources))

    def test_partial_identity_or_mitigation_only_scope_cannot_adopt_bestiary(self):
        for status, identity, scope in [('WIKI_UNCERTAIN', 'VERIFIED', None),
                                       ('COMPARED', 'WIKI_IDENTITY_UNKNOWN', None),
                                       ('COMPARED', 'VERIFIED', ['mitigation_percent'])]:
            r, c, rows, sources = fixture()
            r['status'], r['creature_identity']['status'] = status, identity
            r['qualified_fields'] = scope
            before = copy.deepcopy((c, rows, sources))
            adopt(r, c, rows, sources)
            self.assertEqual(before, (c, rows, sources))

    def test_creature_without_a_declared_bestiary_is_not_assigned_one(self):
        r, c, rows, sources = fixture()
        del c['bestiary']
        adopt(r, c, rows, sources)
        self.assertNotIn('bestiary', c)
        self.assertFalse(rows)

    def test_mutated_page_revision_digest_or_labels_fail_closed(self):
        for path, value in [('page_id', 3), ('revision_id', 3), ('content_sha256', 'b' * 64)]:
            r, c, rows, sources = fixture()
            r['qualified_bestiary_facts'][path] = value
            before = copy.deepcopy((c, rows, sources))
            with self.assertRaises(ValueError):
                adopt(r, c, rows, sources)
            self.assertEqual(before, (c, rows, sources))
        for value in ('?', 'Easy?', 'Unknown'):
            r, c, rows, sources = fixture()
            r['qualified_bestiary_facts']['difficulty']['raw'] = value
            with self.assertRaises(ValueError):
                adopt(r, c, rows, sources)


@unittest.skipUnless(CANARY and CRYSTAL, 'set both pinned source checkouts')
class ActualProfiles(unittest.TestCase):
    def test_three_malformed_event_profiles_become_data_valid_with_source_pins(self):
        conv = crystal_batch.converter(Path(CANARY), Path(CRYSTAL))
        conv.wiki = {m['monster']: m for m in json.loads(crystal_batch.WIKI.read_text())['monsters']}
        for relative, points in [('humans/doomsday_cultist', 30), ('humans/bride of night', 50),
                                 ('humans/midnight_warrior', 50)]:
            conv.pending_definitions = set()
            _, monster, deps, catalog, manifest, _ = conv.convert(relative)
            profile = monster['creature']['bestiary']
            self.assertEqual([2, 3, 5], profile['kill_thresholds'])
            self.assertEqual(points, profile['charm_points'])
            self.assertFalse(vm.validate(monster, deps, catalog, manifest), relative)
            self.assertTrue(any(s.get('title') == 'Template:Kills to Unlock' for s in manifest['sources']))
            self.assertNotIn(relative, crystal_batch.EXTRA_MONSTERS)  # event/quest admission is separate
