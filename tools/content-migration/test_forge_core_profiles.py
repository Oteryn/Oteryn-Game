"""P18 core identity/source facts, digest refresh and fail-closed overlay admission."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import forge_core_profiles as cores


def canonical(value):
    return (json.dumps(value, sort_keys=True) + '\n').encode()


class ForgeCoreProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.overlay = json.loads((cores.ROOT / cores.PATH).read_bytes())

    def generated(self):
        return {'profiles': canonical({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': []}),
                'manifest': canonical({'item_profiles': {'path': 'profiles', 'sha256': '0' * 64}})}

    def test_exact_existing_core_identity_and_pinned_physical_stack_facts(self):
        record, = self.overlay['records']
        self.assertEqual(record['production_definition'], {
            'family': 'Item', 'production_key': 'oteryn:item.tibia.i37110', 'revision_ref': 'definition-r1'})
        self.assertEqual(record['production_binding']['disposition'], 'EXACT')
        self.assertEqual(record['semantics']['physical'], cores.PHYSICAL)
        self.assertEqual(record['semantics']['stack'], cores.STACK)
        self.assertEqual(record['semantics']['container'], {'state': 'NOT_APPLICABLE'})
        self.assertEqual(record['admission'], cores.ADMISSION)
        self.assertEqual(self.overlay['sources'][:3], cores.provenance('crystal'))
        active = json.loads((cores.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        admitted = [row for row in active['records']
                    if row['production_definition']['production_key'] == 'oteryn:item.tibia.i37110']
        self.assertEqual(len(admitted), 1, 'production native provider must admit exactly one core')
        self.assertEqual(admitted[0]['production_binding'], record['production_binding'])
        self.assertEqual(admitted[0]['semantics']['stack'], cores.STACK)
        self.assertEqual(admitted[0]['semantics']['physical'], cores.PHYSICAL)

    def test_admission_refreshes_digest_and_rejects_a_second_core_profile(self):
        generated = self.generated()
        order = lambda record: record['production_definition']['production_key']
        cores.admit(cores.ROOT, generated, 'profiles', 'manifest', order, canonical)
        self.assertEqual(json.loads(generated['profiles'])['records'], self.overlay['records'])
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         hashlib.sha256(generated['profiles']).hexdigest())
        with self.assertRaisesRegex(ValueError, 'FORGE_CORE_PROFILE_DUPLICATE'):
            cores.admit(cores.ROOT, generated, 'profiles', 'manifest', order, canonical)

    def test_source_and_materialization_drift_fail_before_modifying_generated_bytes(self):
        for field in ['source', 'stack', 'weight', 'binding']:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                overlay = copy.deepcopy(self.overlay)
                record = overlay['records'][0]
                if field == 'source':
                    overlay['sources'][0]['revision'] = '0' * 40
                elif field == 'stack':
                    record['semantics']['stack']['value']['stack_max']['value'] = 101
                elif field == 'weight':
                    record['semantics']['physical']['value']['weight']['value'] = 0
                else:
                    record['production_binding']['target']['key'] = 'oteryn:item.tibia.i37109'
                path = root / cores.PATH
                path.parent.mkdir(parents=True)
                path.write_bytes(canonical(overlay))
                generated = self.generated()
                before = dict(generated)
                with self.assertRaises(ValueError):
                    cores.admit(root, generated, 'profiles', 'manifest',
                                lambda row: row['production_definition']['production_key'], canonical)
                self.assertEqual(generated, before)


if __name__ == '__main__':
    unittest.main()
