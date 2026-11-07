"""Tests of crystal_batch.py: the allow-list of the CrystalServer monsters outside its summer_update_2026 directory.

    python -m unittest test_crystal_batch          # from this directory
    OTERYN_CANARY=<checkout> OTERYN_CRYSTAL=<checkout> python -m unittest test_crystal_batch   # also selects the files
"""
import json
import os
import tempfile
import unittest
from pathlib import Path

import crystal_batch
import official_library
import spell_scripts
import wiki_only_candidates

SAMPLE = json.loads(wiki_only_candidates.SAMPLE.read_text(encoding='utf-8'))
WIKI = json.loads(crystal_batch.WIKI.read_text(encoding='utf-8'))
CANARY, CRYSTAL = os.environ.get('OTERYN_CANARY'), os.environ.get('OTERYN_CRYSTAL')


class AllowList(unittest.TestCase):
    def test_is_exactly_the_ordinary_rows_of_the_candidate_sample(self):
        expected = sorted(m['crystal_other_dir'][:-len('.lua')] for m in SAMPLE['monsters']
                          if m.get('kind') == 'real monster' and m['crystal_other_dir']
                          and m.get('canonical_classification', {}).get('source_wiki_identity_status', 'COMPARED') == 'COMPARED')
        self.assertEqual(46, len(expected))
        self.assertEqual(expected, sorted(crystal_batch.EXTRA_MONSTERS))

    def test_has_no_duplicate_and_leaves_the_summer_directory_alone(self):
        self.assertEqual(len(crystal_batch.EXTRA_MONSTERS), len(set(crystal_batch.EXTRA_MONSTERS)))
        self.assertFalse(any(r.startswith('summer_update_2026/') for r in crystal_batch.EXTRA_MONSTERS))
        self.assertTrue(all(not Path(r).is_absolute() and '..' not in Path(r).parts and not r.startswith('.')
                            for r in crystal_batch.EXTRA_MONSTERS))

    def test_quest_event_and_summon_like_rows_stay_out(self):
        others = {m['crystal_other_dir'][:-len('.lua')] for m in SAMPLE['monsters']
                  if m['crystal_other_dir'] and m.get('kind') != 'real monster'}
        self.assertTrue(others)
        self.assertFalse(others & set(crystal_batch.EXTRA_MONSTERS))

    def test_new_regular_classifications_have_dated_canonical_identity(self):
        new = [m for m in SAMPLE['monsters'] if m.get('canonical_classification')
               and m['kind'] == 'real monster'
               and m['canonical_classification']['source_wiki_identity_status'] == 'COMPARED']
        self.assertEqual(18, len(new))
        by_slug = {m['monster']: m for m in WIKI['monsters']}
        for row in new:
            proof = row['canonical_classification']
            self.assertEqual('no', proof['isboss'], row['name'])
            self.assertIn('Regular', proof['spawntype'], row['name'])
            self.assertEqual('2026-09-27', proof['reference_cut'])
            self.assertRegex(proof['content_sha256'], r'^[a-f0-9]{64}$')
            compared = by_slug[row['name'].replace(' ', '_')]
            self.assertEqual('VERIFIED', compared['creature_identity']['status'])
            self.assertEqual(proof['page_id'], compared['page_id'])
            self.assertEqual(proof['revision_id'], compared['cut_revision_id'])
            self.assertEqual(proof['content_sha256'], compared['cut_content_sha256'])

    def test_imperial_missing_wiki_race_uses_the_explicit_staticdata_identity(self):
        imperial = next(m for m in SAMPLE['monsters'] if m['name'] == 'imperial')
        self.assertEqual('real monster', imperial['kind'])
        self.assertEqual('COMPARED', imperial['canonical_classification']['source_wiki_identity_status'])
        self.assertIn(imperial['crystal_other_dir'][:-len('.lua')], crystal_batch.EXTRA_MONSTERS)
        compared = next(m for m in WIKI['monsters'] if m['monster'] == 'imperial')
        self.assertEqual('ExactAcceptedStaticdataAndCanonicalCreature', compared['creature_identity']['method'])
        self.assertEqual('UNSPECIFIED', compared['creature_identity']['wiki_race_id_status'])
        self.assertEqual(2775, compared['creature_identity']['source_race_id'])

    def test_monsters_are_under_the_converter_root(self):
        self.assertTrue(crystal_batch.MONSTER_DIR.startswith(crystal_batch.MONSTER_ROOT + '/'))

    def test_wiki_reference_holds_every_allow_listed_monster(self):
        slugs = {m['monster'] for m in WIKI['monsters']}
        missing = [r for r in crystal_batch.EXTRA_MONSTERS if r.split('/')[-1] not in slugs]
        self.assertEqual([], missing)

    def test_wiki_reference_has_the_sample_values_for_the_allow_listed_monsters(self):
        by_slug = {m['monster']: m for m in WIKI['monsters']}
        checked = 0
        for row in SAMPLE['monsters']:
            if row['crystal_other_dir'] and row.get('kind') == 'real monster' and row['name'].replace(' ', '_') in by_slug:
                rows = {r['field']: r for r in by_slug[row['name'].replace(' ', '_')]['rows']}
                if 'experience' in rows:  # a row is compact only when the wiki differs from or is unknown next to Crystal
                    checked += 1
                    self.assertEqual(row['wiki_exp'], str(rows['experience']['wiki_raw']).replace(',', ''), row['name'])
        self.assertGreater(checked, 15)


class OfficialLibraryExtra(unittest.TestCase):
    def test_sample_covers_the_allow_list_with_a_news_check(self):
        sample = json.loads(official_library.CRYSTAL_EXTRA_SAMPLE.read_text(encoding='utf-8'))
        slugs = {m['monster'] for m in sample['monsters']}
        allowed = {r.split('/')[-1] for r in crystal_batch.EXTRA_MONSTERS}
        self.assertLessEqual(slugs, allowed)
        # The original dated official-library batch covers 27 of the original 28 entries.
        # New ordinary sources have explicit canonical Wiki health; they need no invented library capture.
        new = {m['name'].replace(' ', '_') for m in SAMPLE['monsters']
               if m.get('canonical_classification') and m.get('kind') == 'real monster'}
        self.assertEqual({'ink_splash'}, allowed - slugs - new)
        self.assertEqual('2026-09-30', sample['captured'])
        self.assertIn(8980, {n['id'] for n in sample['news_checked']})
        self.assertTrue(all(m['captured'] == '2026-09-30' and m['fields'] for m in sample['monsters']))

    def test_every_creature_without_wiki_health_has_library_health(self):
        sample = {m['monster'] for m in json.loads(official_library.CRYSTAL_EXTRA_SAMPLE.read_text(encoding='utf-8'))['monsters']}
        for row in SAMPLE['monsters']:
            if row['crystal_other_dir'] and row.get('kind') == 'real monster' and row['wiki_hp'] is None:
                self.assertIn(row['name'].replace(' ', '_'), sample, row['name'])


class ExtraSpellRoot(unittest.TestCase):
    SPELL = 'local s = Spell("instant")\ns:name("%s")\ns:register()\n'

    def test_extra_root_adds_only_names_canary_lacks(self):
        with tempfile.TemporaryDirectory() as tmp:
            canary, extra = Path(tmp, 'canary'), Path(tmp, 'extra')
            (canary / 'data/scripts').mkdir(parents=True)
            (canary / 'data/scripts/a.lua').write_text(self.SPELL % 'Shared')
            (canary / 'data-otservbr-global/scripts').mkdir(parents=True)
            (extra / 'data-global/scripts/spells/monster').mkdir(parents=True)
            (extra / 'data-global/scripts/spells/monster/b.lua').write_text(self.SPELL % 'Only Extra' + self.SPELL % 'Shared')
            (extra / 'data-global/scripts/other.lua').write_text(self.SPELL % 'Elsewhere')
            index = spell_scripts.index_spells(canary, (extra,))
            self.assertEqual({'shared', 'only extra'}, set(index))
            self.assertEqual(canary / 'data/scripts/a.lua', index['shared'][1])
            self.assertEqual({'shared'}, set(spell_scripts.index_spells(canary)))


CRYSTAL_ONLY_SPELLS = ('dark blood wave', 'clouds chain', 'blood ring', 'crypt construct wave', 'crypt x', 'energy cruz',
                       'wave death crypt', 'deathcircle', 'night harpy shielding ball', 'night harpy cone wave',
                       'night harpy scratch', 'skirmisher wave', 'roaming physical ring')


@unittest.skipUnless(CANARY and CRYSTAL, 'set OTERYN_CANARY and OTERYN_CRYSTAL to the pinned checkouts')
class CrystalOnlySpells(unittest.TestCase):
    def test_plain_combat_scripts_evaluate_exactly(self):
        scripts = spell_scripts.SpellScripts(Path(CANARY), extra_roots=(Path(CRYSTAL),))
        for name in CRYSTAL_ONLY_SPELLS:
            info = scripts.evaluate(name)
            self.assertIsNotNone(info, name)
            self.assertNotIn('error', info, name)
            self.assertEqual('P2', info['tier'], name)
            self.assertTrue(info['script'].startswith('data-global/scripts/spells/monster/'), name)
            self.assertFalse(spell_scripts.SpellScripts(Path(CANARY)).evaluate(name), name)

    def test_the_seven_blocked_monsters_have_no_open_row(self):
        conv = crystal_batch.converter(Path(CANARY), Path(CRYSTAL))
        for relative in ('humanoids/gloom_maw', 'humanoids/norcferatu_heartless', 'humanoids/varg',
                         'winter_update_2025/crypt_construct', 'winter_update_2025/crypt_mage',
                         'winter_update_2025/night_harpy', 'winter_update_2025/raubritter_skirmisher'):
            conv.pending_definitions = set()
            manifest = conv.convert(relative)[4]
            self.assertEqual([], [e['source_field'] for e in manifest['entries']
                                  if e['status'] in ('unresolved_semantics', 'unsupported_source_field')], relative)


@unittest.skipUnless(CANARY and CRYSTAL, 'set OTERYN_CANARY and OTERYN_CRYSTAL to the pinned checkouts')
class Selection(unittest.TestCase):
    def test_files_keep_the_summer_files_first_and_append_the_allow_list(self):
        selected = crystal_batch.files(Path(CANARY), Path(CRYSTAL))
        summer = [f for f in selected if f.startswith('summer_update_2026/')]
        self.assertEqual(49, len(summer))
        self.assertEqual(summer + list(crystal_batch.EXTRA_MONSTERS), selected)

    def test_converted_source_file_is_the_crystal_path(self):
        conv = crystal_batch.converter(Path(CANARY), Path(CRYSTAL))
        conv.pending_definitions = set()
        source = conv.convert('inkborn/bluebeak')[5]
        self.assertEqual('data-global/monster/inkborn/bluebeak.lua', source['file'])


if __name__ == '__main__':
    unittest.main()
