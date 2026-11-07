"""The WVAR Mamma profile is a real native provider input with bounded assumptions."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import wvar_creature_profiles as wvar

ROOT = Path(__file__).resolve().parents[2]


class WvarCreatureProfilesTests(unittest.TestCase):
    def inputs(self):
        return {path: (ROOT / path).read_bytes()
                for path in (wvar.CREATURES, wvar.PRESENTATIONS, wvar.MANIFEST)}

    def test_mamma_uses_observed_stats_and_appearance_and_shared_source_combat(self):
        generated = self.inputs()
        before = copy.deepcopy(generated)
        wvar.admit(ROOT, generated)
        creatures = json.loads(generated[wvar.CREATURES])['records']
        mamma = next(r for r in creatures if r['profile']['target']['key'] == wvar.CREATURE)
        profile = mamma['profile']['data']['profile']
        self.assertEqual((profile['health'], profile['experience']), (1800, 2200))
        self.assertEqual(profile['details']['display_name'], 'Mamma Longlegs')
        self.assertNotIn('bestiary', profile)
        self.assertFalse(profile['details']['system_eligibility']['prey'])
        self.assertFalse(mamma['behavior']['data']['profile']['movement']['pushable'])
        self.assertTrue(mamma['behavior']['data']['profile']['movement']['push_items'])
        closure = mamma['attack_definitions']
        self.assertIn(mamma['profile'], closure['profiles'])
        self.assertIn(mamma['behavior'], closure['profiles'])
        reference = next(r for r in closure['records'] if r['identity']['key'] == wvar.CREATURE)
        self.assertNotIn('loot', reference)
        presentations = json.loads(generated[wvar.PRESENTATIONS])['records']
        presentation = next(r for r in presentations if r['target']['key'] == wvar.PRESENTATION)
        self.assertEqual(presentation['data']['profile']['asset_binding'], 'canary.appearance:outfit/208')
        original = json.loads(before[wvar.CREATURES])['records']
        self.assertEqual([r for r in creatures if r['profile']['target']['key'] != wvar.CREATURE],
                         [r for r in original if r['profile']['target']['key'] != wvar.CREATURE])

    def test_repeat_generation_is_exact_and_changes_only_owned_provider_sections(self):
        generated = self.inputs()
        old_manifest = json.loads(generated[wvar.MANIFEST])
        wvar.admit(ROOT, generated)
        once = copy.deepcopy(generated)
        wvar.admit(ROOT, generated)
        self.assertEqual(generated, once)
        new_manifest = json.loads(generated[wvar.MANIFEST])
        for section in old_manifest:
            if section not in ('creature_profiles', 'presentation_profiles'):
                self.assertEqual(old_manifest[section], new_manifest[section])

    def test_changed_source_revision_is_rejected_before_provider_mutation(self):
        generated = self.inputs()
        before = copy.deepcopy(generated)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / wvar.SOURCE
            path.parent.mkdir(parents=True)
            source = json.loads((ROOT / wvar.SOURCE).read_bytes())
            mamma = next(row for row in source['records'] if row.get('source_title') == 'Mamma Longlegs')
            mamma['source_revision_id'] += 1
            path.write_text(json.dumps(source))
            with self.assertRaisesRegex(ValueError, 'WVAR_MAMMA_SOURCE_CHANGED'):
                wvar.admit(root, generated)
        self.assertEqual(generated, before)


if __name__ == '__main__':
    unittest.main()
