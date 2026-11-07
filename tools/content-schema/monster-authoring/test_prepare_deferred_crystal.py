"""Regression coverage for preparation that must never silently admit deferred sources."""
import json
import os
import tempfile
import unittest
from pathlib import Path

import crystal_batch
import prepare_deferred_crystal as prep
import wiki_only_candidates

SAMPLE = json.loads(wiki_only_candidates.SAMPLE.read_text(encoding='utf-8'))
CANARY, CRYSTAL = os.environ.get('OTERYN_CANARY'), os.environ.get('OTERYN_CRYSTAL')
# Exact source sets approved by the dated canonical classification recheck.
RETAINED_DEFERRED = {
    'bride of night': 'humans/bride of night',
    'cake golem': 'quests/a_piece_of_cake/cake_golem',
    'doomsday cultist': 'humans/doomsday_cultist',
    'midnight warrior': 'humans/midnight_warrior',
}
CANONICAL_ORDINARY = {
    'newhaven_update_2025/corrupted_ghost', 'newhaven_update_2025/corrupted_skeleton',
    'quests/cults_of_tibia/goldhanded_cultist_bride', 'winter_update_2025/imperial',
    'targuna/crimson_court/infernoid_blob', 'targuna/crimson_court/infernoid_hound',
    'targuna/crimson_court/infernoid_soul', 'targuna/crimson_court/infernoid_spiritual',
    'targuna/hidden_lizard_temple/lizard_commander',
    'targuna/hidden_lizard_temple/lizard_executioner',
    'targuna/hidden_lizard_temple/lizard_henchman',
    'targuna/hidden_lizard_temple/lizard_magician',
    'targuna/hidden_lizard_temple/lizard_swordmaster',
    'targuna/aragonia/pirate_cook', 'targuna/aragonia/pirate_gunner',
    'targuna/aragonia/pirate_navigator', 'targuna/aragonia/pirate_quartermaster',
    'targuna/aragonia/sea_captain',
}


class Preparation(unittest.TestCase):
    def test_deferred_selection_leaves_default_roster_unchanged(self):
        before = tuple(crystal_batch.EXTRA_MONSTERS)
        rows = prep.deferred_rows(SAMPLE)
        self.assertEqual(RETAINED_DEFERRED,
                         {row['name']: row['crystal_other_dir'][:-4] for row in rows})
        self.assertFalse({row['crystal_other_dir'][:-4] for row in rows} & set(crystal_batch.EXTRA_MONSTERS))
        self.assertEqual({'event/quest creature'}, {row['kind'] for row in rows})
        ordinary = {row['crystal_other_dir'][:-4] for row in SAMPLE['monsters']
                    if row.get('canonical_classification') and row['kind'] == 'real monster'
                    and row['canonical_classification']['source_wiki_identity_status'] == 'COMPARED'}
        self.assertEqual(CANONICAL_ORDINARY, ordinary)
        self.assertTrue(CANONICAL_ORDINARY <= set(crystal_batch.EXTRA_MONSTERS))
        self.assertEqual(before, tuple(crystal_batch.EXTRA_MONSTERS))

    def test_rejects_unsafe_and_duplicate_source_paths(self):
        for path in ('../secret.lua', '/tmp/a.lua', 'a\\b.lua', 'a.json'):
            with self.subTest(path=path), self.assertRaises(ValueError):
                prep.deferred_rows({'monsters': [{'kind': 'quest creature', 'crystal_other_dir': path}]})
        row = {'kind': 'quest creature', 'crystal_other_dir': 'quest/a.lua'}
        with self.assertRaises(ValueError):
            prep.deferred_rows({'monsters': [row, row]})

    def test_rejects_repository_outputs(self):
        for out in (prep.REPO, prep.REPO / 'content' / 'drafts'):
            with self.assertRaises(ValueError):
                prep.outside_repository(out)

    def test_probe_conflicts_and_unknowns_remain_explicit(self):
        row = {'wiki_hp': None, 'wiki_exp': '100', 'wiki_summon': '260', 'wiki_convince': '260'}
        evidence = {'fields': {'experience': {'value': 0}, 'flags': {'value': {'summonable': False,
                     'convinceable': False}}}, 'callbacks': {'onDeath': 'function'}}
        issues = prep.preparation_issues(row, evidence)
        self.assertTrue(any('wiki_hp unknown' in issue for issue in issues))
        self.assertEqual(3, sum('conflicts' in issue for issue in issues))
        self.assertTrue(any('callbacks' in issue for issue in issues))
        self.assertEqual(False, evidence['fields']['flags']['value']['summonable'])

    def test_missing_quest_configuration_preserves_literal_facts_only(self):
        row = {'name': 'blocked', 'crystal_other_dir': 'quest/blocked.lua'}
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / crystal_batch.MONSTER_ROOT / row['crystal_other_dir']
            path.parent.mkdir(parents=True)
            path.write_text('local mType = Game.createMonsterType("Blocked")\nlocal monster = {}\n'
                            'monster.health = 100\nmonster.speed = 85 -- known speed\n'
                            'monster.loot = QuestConfig.Items[1]\n')
            evidence = prep.source_evidence(row, Path(tmp))
            self.assertEqual(100, evidence['fields']['health']['value'])
            self.assertEqual(85, evidence['fields']['speed']['value'])
            self.assertIn('loot', evidence['source_unevaluated_fields'])
            self.assertNotIn('loot', evidence['source_absent_fields'])
            self.assertTrue(evidence['evaluation_error'])


@unittest.skipUnless(CANARY and CRYSTAL, 'set both pinned source checkout paths')
class PinnedPreparation(unittest.TestCase):
    def test_every_deferred_source_has_evidence_and_blockers(self):
        with tempfile.TemporaryDirectory() as tmp:
            before = tuple(crystal_batch.EXTRA_MONSTERS)
            report = prep.prepare(Path(CANARY), Path(CRYSTAL), Path(tmp))
            self.assertFalse(report['admission_authorized'])
            by_name = {row['name']: row for row in report['monsters']}
            self.assertEqual(set(RETAINED_DEFERRED), set(by_name))
            self.assertEqual(before, tuple(crystal_batch.EXTRA_MONSTERS))
            prepared_sample = {row['name']: row['preparation'] for row in SAMPLE['monsters']
                               if row['name'] in RETAINED_DEFERRED}
            self.assertEqual(set(by_name), set(prepared_sample))
            for row in report['monsters']:
                self.assertFalse(row['admission_authorized'])
                self.assertEqual(crystal_batch.REVISION, row['source_evidence']['revision'])
                self.assertTrue(row['source_evidence']['blob_sha1'])
                self.assertTrue(row['preparation_issues'])
                self.assertEqual(row['source_evidence'], prepared_sample[row['name']]['source_evidence'])
                self.assertTrue(prepared_sample[row['name']]['qualification_snapshot_scope']
                                .startswith('HISTORICAL_ROUND2_SOURCE_DRAFT'))
                self.assertEqual('draft_schema_valid', row['status'])
                self.assertEqual([], row['structure_errors'])
                self.assertTrue((Path(tmp) / row['bundle_directory'] / 'manifest.json').is_file())
            # Source evidence is immutable; current canonical Bestiary repair
            # supersedes the old malformed-threshold draft status only.
            for name in ('bride of night', 'doomsday cultist', 'midnight warrior'):
                current = by_name[name]
                monster = json.loads((Path(tmp) / current['bundle_directory'] / 'monster.json').read_text())
                self.assertEqual([2, 3, 5], monster['creature']['bestiary']['kill_thresholds'])
                self.assertEqual([], current['readiness_errors'])
                self.assertEqual([], current['open_manifest_rows'])
            cake = by_name['cake golem']
            self.assertEqual('draft_schema_valid', cake['status'])
            self.assertFalse(cake['admission_authorized'])
            self.assertTrue(cake['source_evidence']['source_unevaluated_fields'])
            self.assertTrue(cake['readiness_errors'])
            self.assertTrue(any(row['status'] == 'unresolved_semantics'
                                and row['source_field'] == 'top-level script before mType:register'
                                and 'before registration' in row['resolution']
                                for row in cake['open_manifest_rows']))
            monster = json.loads((Path(tmp) / cake['bundle_directory'] / 'monster.json').read_text())
            self.assertEqual(444, monster['creature']['stats']['max_health'])
            self.assertEqual(444, monster['creature']['stats']['initial_health'])
            self.assertEqual(100, monster['creature']['stats']['experience'])


if __name__ == '__main__':
    unittest.main()
