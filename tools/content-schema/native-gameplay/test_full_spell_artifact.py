"""Source pin and actual artifact composition regressions; no runtime authority."""
import argparse
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('full_spell_artifact', Path(__file__).with_name('build_full_spell_artifact.py'))
full = importlib.util.module_from_spec(spec)
spec.loader.exec_module(full)
FAMILIARS = Path('/workspace/spells-r21-implemented/familiar-profiles')


@unittest.skipUnless(FAMILIARS.exists(), 'requires the explicit local Familiar source export')
class SourceCompositionTests(unittest.TestCase):
    def test_actual_source_identity_and_custom_operation_preserved(self):
        creatures, presentations, proof = full.merged_profiles(FAMILIARS)
        self.assertEqual(6, len(creatures['records']))
        self.assertTrue(all(r['profile']['target']['revision'] == 'canary-47dfd51f' for r in creatures['records']))
        closure = full.dependency_report(creatures, presentations, FAMILIARS, proof)
        self.assertEqual(83, closure['declared_dependency_count'])
        self.assertEqual(2, len(closure['unresolved_source_operations']))
        self.assertIn({'family': 'Ability', 'key': 'canary:ability/spell/summon_challenge', 'revision': 'canary-47dfd51f'}, closure['missing_exact_declarations'])

    def test_skeleton_exact_formal_source_and_all_dependencies_preserved(self):
        skeleton = Path('/workspace/spells-r21-implemented/skeleton-profiles')
        creatures, presentations, proof = full.merged_profiles(FAMILIARS)
        source = full.merge_skeleton(creatures, presentations, skeleton)
        self.assertEqual(7, len(creatures['records']))
        self.assertEqual(full.FORMAL, source['unchanged_full_definitions'][0]['definition_revision'])
        self.assertEqual(('Creature', 'canary:creature/skeleton', 'canary-47dfd51f'),
            full.identity(creatures['records'][-1]['profile']['target']))
        closure = full.dependency_report(creatures, presentations, FAMILIARS, proof, skeleton)
        self.assertEqual(89, closure['declared_dependency_count'])
        self.assertFalse(any(ref['key'].startswith('canary:ability/skeleton/') for ref in closure['missing_exact_declarations']))

    def test_complete_illusionable_appearance_closure_is_generated_from_actual_source(self):
        import build_spell_appearances
        creatures, presentations, _ = full.merged_profiles(FAMILIARS)
        full.merge_skeleton(creatures, presentations, Path('/workspace/spells-r21-implemented/skeleton-profiles'))
        provider = build_spell_appearances.build(Path('/workspace/spell-sources/canary'), creatures)
        closure = full.qualify_appearance_closure(creatures, presentations, provider)
        self.assertEqual(2, closure['illusionable_creature_count'])
        skeleton = next(row for row in provider['records'] if row['creature'] and row['creature']['key']=='canary:creature/skeleton')
        self.assertEqual(33, skeleton['look_type'])
        self.assertEqual('22b05bff44dddfdd71e743fa7ecf7884cc1b6b01', skeleton['source']['git_blob'])
        self.assertEqual('canary-47dfd51f', skeleton['creature']['revision'])
        stale = copy.deepcopy(provider)
        stale['records'].remove(skeleton)
        with self.assertRaisesRegex(ValueError, 'Missing exact illusionable'):
            full.qualify_appearance_closure(creatures, presentations, stale)
        substituted = copy.deepcopy(provider)
        next(row for row in substituted['records'] if row['creature']==skeleton['creature'])['look_type']=21
        with self.assertRaisesRegex(ValueError, 'differs from exact Creature Presentation'):
            full.qualify_appearance_closure(creatures, presentations, substituted)

    def test_all_246_exact_item_refs_require_independent_production_bindings(self):
        _, catalog = full.read(full.SAMPLES/'executable-spell-catalog.json')
        _, provider = full.read(Path('/workspace/spells-r21-implemented/item-profiles/item-profiles.json'))
        closure = full.qualify_item_closure(catalog, provider)
        self.assertEqual(85, closure['item_ref_count'])
        used = closure['exact_used_bindings'][0]['authoring']
        changed = copy.deepcopy(provider)
        row = next(row for row in changed['records'] if row['authoring']['item'] == used)
        row['production_definition']['revision_ref'] = 'forged-revision'
        with self.assertRaisesRegex(ValueError, 'production definition/binding mismatch'):
            full.qualify_item_closure(catalog, changed)
        row['authoring']['item']['revision'] = 'spell-p2-r20'
        with self.assertRaisesRegex(ValueError, 'Missing exact authored Item'):
            full.qualify_item_closure(catalog, changed)

    def test_complete_40450_qualification_packet_cannot_be_truncated(self):
        _, catalog = full.read(full.SAMPLES/'executable-spell-catalog.json')
        _, provider = full.read(Path('/workspace/spells-r21-implemented/item-profiles/item-profiles.json'))
        provider = copy.deepcopy(provider)
        row = next(row for row in provider['records'] if row['authoring']['item']['key'] == 'candidate:item/40450')
        row['production_binding_qualification']['qualification']['sources'].pop()
        with self.assertRaisesRegex(ValueError, 'changed or truncated'):
            full.qualify_item_closure(catalog, provider)

    def test_changed_source_export_pin_rejects(self):
        original = full.read
        def changed(path):
            raw, value = original(path)
            if path.name == 'familiar-profile-source-proof.json':
                value = copy.deepcopy(value)
                value['outputs']['familiar-creature-profiles.json'] = '0' * 64
            return raw, value
        with patch.object(full, 'read', side_effect=changed):
            with self.assertRaisesRegex(ValueError, 'export pin mismatch'):
                full.merged_profiles(FAMILIARS)

    def test_unchanged_blob_without_definition_revision_rejects(self):
        original = full.read
        def changed(path):
            raw, value = original(path)
            if path.name == 'familiar-profile-source-proof.json':
                value = copy.deepcopy(value)
                value['unchanged_full_definitions'][0]['definition_revision'] = full.CANARY
            return raw, value
        with patch.object(full, 'read', side_effect=changed):
            with self.assertRaisesRegex(ValueError, 'unchanged source proof mismatch'):
                full.merged_profiles(FAMILIARS)

    def test_pins_bind_exact_utf8_provider_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            catalog = {'bundles': [{'identity': 'test'}], 'revision': 'r1'}
            cat_raw = full.encoded(catalog)
            (base / 'catalog.json').write_bytes(cat_raw)
            (base / 'selection.json').write_bytes(full.encoded({'revision': 'r1', 'catalog_sha256': full.digest(cat_raw), 'selections': []}))
            item_raw = b'{ "schema":"OTERYN_NATIVE_ITEM_PROFILES/v1", "records":[{"proof":"source bytes"}] }\n'
            (base / 'items.json').write_bytes(item_raw)
            args = argparse.Namespace(out=base/'out', catalog=base/'catalog.json', selection=base/'selection.json', expected_spells=1, familiars=FAMILIARS, items=base/'items.json', build_revision='build-content-r1', appearances=full.NATIVE/'spell_appearances.json', training=full.NATIVE/'build-training.json', familiar_config=full.NATIVE/'familiar-config.json', familiar_defenses=full.NATIVE/'familiar-defenses.json', wheel=full.NATIVE/'wheel-profile.json', source_world=None, native_map_profile='accepted-entry-r1')
            proof = full.build(args)
            self.assertEqual(item_raw, (args.out/'item-profiles.json').read_bytes())
            manifest = json.loads((args.out/'manifest.json').read_bytes())
            self.assertEqual(full.digest(item_raw), manifest['item_profiles']['sha256'])
            self.assertEqual(full.digest((args.out/'manifest.json').read_bytes()), proof['manifest_sha256'])
            self.assertEqual(6, proof['creature_count'])


if __name__ == '__main__':
    unittest.main()
