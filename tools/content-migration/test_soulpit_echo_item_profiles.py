"""Source capture and protected Item admission stay exact before native output changes."""
import copy
import hashlib
import json
import re
import tempfile
import unittest
from pathlib import Path

import soulpit_echo_item_profiles as soulpit

ROOT = Path(__file__).resolve().parents[2]
PROFILES = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'
BINDINGS = 'imports/crystalserver/bindings/items.json'


def canonical(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False) + '\n').encode()


class SoulpitEchoItemProfilesTest(unittest.TestCase):
    def setUp(self):
        self.capture = (ROOT / soulpit.PATH).read_bytes()
        self.overlay = json.loads(self.capture)
        self.bindings = (ROOT / BINDINGS).read_bytes()

    def generated(self):
        document = json.loads((ROOT / PROFILES).read_bytes())
        keys = {row['production_definition']['production_key'] for row in self.overlay['records']}
        document['records'] = [row for row in document['records']
                               if row['production_definition']['production_key'] not in keys]
        return {PROFILES: canonical(document), MANIFEST: (ROOT / MANIFEST).read_bytes()}

    def root_copy(self, directory, overlay=None, bindings=None):
        root = Path(directory)
        for path, raw in ((soulpit.PATH, self.capture if overlay is None else canonical(overlay)),
                          (BINDINGS, self.bindings if bindings is None else canonical(bindings))):
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
        return root

    def admit(self, root, generated):
        soulpit.admit(root, generated, PROFILES, MANIFEST,
                      lambda row: row['production_definition']['production_key'], canonical)

    def assert_capture_refused(self, altered):
        generated = self.generated()
        before = dict(generated)
        with tempfile.TemporaryDirectory() as directory:
            root = self.root_copy(directory, altered)
            with self.assertRaisesRegex(ValueError, 'SOULPIT_ITEM_CAPTURE_DRIFT'):
                self.admit(root, generated)
        self.assertEqual(generated, before)

    def test_exact_runtime_catalogue_and_allowed_profiles_keep_prism_and_unbound_draws(self):
        self.assertEqual(hashlib.sha256(self.capture).hexdigest(), soulpit.CAPTURE_SHA256)
        cores = self.overlay['core_catalogue']
        ids = [row['id'] for row in cores]
        self.assertEqual(len(ids), 835)
        self.assertEqual(ids, sorted(set(ids)))
        self.assertIn(49164, ids)  # Runtime market.category26 includes Soul Prism.
        self.assertEqual(next(row['name'] for row in cores if row['id'] == 49038), 'white lion')
        unbound = [47899, *range(53142, 53156)]
        self.assertEqual(self.overlay['unadmitted_source_ids'], unbound)
        self.assertTrue(set(unbound).issubset(ids))
        records = self.overlay['records']
        admitted = [int(row['production_binding']['external_id']) for row in records]
        self.assertEqual(len(records), 845)
        self.assertEqual(admitted, sorted((set(ids) | soulpit.EXTRA_IDS) - set(unbound)))
        self.assertTrue(all(row['admission']['materializable'] is True for row in records))
        self.assertTrue(all(row['admission']['legal_destinations'] == ['CharacterInventory', 'Ground']
                            for row in records))

    def test_rust_runtime_catalogue_matches_every_captured_source_id_and_name(self):
        self.assertEqual(hashlib.sha256(self.capture).hexdigest(), soulpit.CAPTURE_SHA256)
        catalogue = (ROOT / 'crates/gameplay/domain/src/domain/soulpit/items/catalog.rs').read_text(
            encoding='utf-8')
        ids = re.search(
            r'pub\s+const\s+SOUL_CORE_SOURCE_IDS\s*:\s*&\[u32\]\s*=\s*&\[(.*?)\];',
            catalogue, re.DOTALL)
        names = re.search(
            r'const\s+NAMES\s*:\s*&\[\(u32,\s*&str\)\]\s*=\s*&\[(.*?)\];',
            catalogue, re.DOTALL)
        self.assertIsNotNone(ids, 'the runtime draw must retain its explicit source ID array')
        self.assertIsNotNone(names, 'the runtime name lookup must retain its source tuple array')
        # Exact ordered equality catches drift even when both arrays still contain 835 rows.
        runtime_ids = [int(value.strip()) for value in ids.group(1).split(',') if value.strip()]
        runtime_names = [
            {'id': int(item_id), 'name': json.loads(name)}
            for item_id, name in re.findall(
                r'\(\s*(\d+)\s*,\s*("(?:[^"\\]|\\.)*")\s*,?\s*\)', names.group(1))
        ]
        self.assertEqual(runtime_ids, [row['id'] for row in self.overlay['core_catalogue']])
        self.assertEqual(runtime_names, self.overlay['core_catalogue'])

    def test_admission_preserves_existing_profiles_and_pins_exact_output_bytes(self):
        generated = self.generated()
        original = json.loads(generated[PROFILES])
        before_manifest = json.loads(generated[MANIFEST])
        with tempfile.TemporaryDirectory() as directory:
            root = self.root_copy(directory)
            self.admit(root, generated)
            result = json.loads(generated[PROFILES])
            by_key = {row['production_definition']['production_key']: row for row in result['records']}
            self.assertEqual(len(by_key), len(original['records']) + 845)
            for row in original['records']:
                self.assertEqual(by_key[row['production_definition']['production_key']], row)
            for row in self.overlay['records']:
                self.assertEqual(by_key[row['production_definition']['production_key']], row)
            manifest = json.loads(generated[MANIFEST])
            expected_manifest = copy.deepcopy(before_manifest)
            expected_manifest['item_profiles']['sha256'] = hashlib.sha256(generated[PROFILES]).hexdigest()
            self.assertEqual(manifest, expected_manifest)
            # Matching profiles are retained; repeated admission cannot duplicate or rewrite them.
            once = dict(generated)
            self.admit(root, generated)
            self.assertEqual(generated, once)

    def test_identity_rebind_in_capture_is_refused_without_partial_output(self):
        altered = copy.deepcopy(self.overlay)
        row = altered['records'][0]
        row['production_binding']['target']['key'] = 'oteryn:item.tibia.i999999'
        row['production_definition']['production_key'] = 'oteryn:item.tibia.i999999'
        self.assert_capture_refused(altered)

    def test_protected_binding_table_cannot_be_rebound_even_with_unchanged_capture(self):
        bindings = json.loads(self.bindings)
        wanted = self.overlay['records'][0]['production_binding']['external_id']
        row = next(row for row in bindings['bindings'] if row['external_id'] == wanted)
        row['target']['revision'] = 'unreviewed-revision'
        generated = self.generated()
        before = dict(generated)
        with tempfile.TemporaryDirectory() as directory:
            root = self.root_copy(directory, bindings=bindings)
            with self.assertRaisesRegex(ValueError, 'Item binding table digest mismatch'):
                self.admit(root, generated)
        self.assertEqual(generated, before)

    def test_source_name_custody_and_binding_metadata_drift_are_refused(self):
        mutations = (
            ('source_revision', lambda value: value['sources'][0].update(revision='0' * 40)),
            ('source_digest', lambda value: value['sources'][0].update(sha256='0' * 64)),
            ('core_name', lambda value: value['core_catalogue'][0].update(name='invented soul core')),
            ('binding_revision', lambda value: value['records'][0]['production_binding'].update(source_revision='0' * 40)),
            ('weight', lambda value: value['records'][0]['semantics']['physical']['value']['weight'].update(value=0)),
            ('stack', lambda value: value['records'][0]['semantics']['stack']['value']['stackable'].update(value=False)),
            ('materializable', lambda value: value['records'][0]['admission'].update(materializable=False)),
            ('destination', lambda value: value['records'][0]['admission'].update(legal_destinations=['Ground'])),
        )
        for label, mutate in mutations:
            with self.subTest(label=label):
                altered = copy.deepcopy(self.overlay)
                mutate(altered)
                self.assert_capture_refused(altered)

    def test_correlated_pool_and_record_removal_cannot_shrink_the_draw(self):
        altered = copy.deepcopy(self.overlay)
        core = altered['core_catalogue'].pop(0)['id']
        altered['records'] = [row for row in altered['records']
                              if int(row['production_binding']['external_id']) != core]
        self.assert_capture_refused(altered)

    def test_missing_duplicate_and_reordered_output_sets_are_refused(self):
        mutations = (
            ('missing_profile', lambda value: value['records'].pop()),
            ('duplicate_profile', lambda value: value['records'].append(copy.deepcopy(value['records'][0]))),
            ('reordered_catalogue', lambda value: value['core_catalogue'].reverse()),
            ('hidden_unbound_member', lambda value: value['unadmitted_source_ids'].pop()),
        )
        for label, mutate in mutations:
            with self.subTest(label=label):
                altered = copy.deepcopy(self.overlay)
                mutate(altered)
                self.assert_capture_refused(altered)

    def test_conflicting_existing_native_custody_is_not_overwritten(self):
        generated = self.generated()
        document = json.loads(generated[PROFILES])
        conflicting = copy.deepcopy(self.overlay['records'][0])
        conflicting['semantics']['physical']['value']['weight']['value'] += 1
        document['records'].append(conflicting)
        generated[PROFILES] = canonical(document)
        before = dict(generated)
        with tempfile.TemporaryDirectory() as directory:
            root = self.root_copy(directory)
            with self.assertRaisesRegex(ValueError, 'SOULPIT_ITEM_PROFILE_CONFLICT'):
                self.admit(root, generated)
        self.assertEqual(generated, before)


if __name__ == '__main__':
    unittest.main()
