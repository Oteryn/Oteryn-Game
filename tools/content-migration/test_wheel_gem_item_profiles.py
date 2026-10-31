"""Source-backed Atelier admission and refusal boundaries of the native overlay."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import wheel_gem_item_profiles as wheel

ROOT = Path(__file__).resolve().parents[2]
PROFILES = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'


def encoded(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + '\n').encode()


def order(record):
    key = record['production_definition']['production_key']
    number = int(key.rsplit('i', 1)[1])
    return number == 40450, number


class WheelGemProfilesTests(unittest.TestCase):
    def packet(self):
        return json.loads((ROOT / wheel.PATH).read_bytes())

    def base(self):
        document = json.loads((ROOT / PROFILES).read_bytes())
        document['records'] = [record for record in document['records']
                               if record['production_definition']['production_key'] not in wheel.KEYS]
        return {PROFILES: encoded(document), MANIFEST: (ROOT / MANIFEST).read_bytes()}

    def test_all_seventeen_inputs_keep_source_weights_and_existing_profiles(self):
        generated = self.base()
        before = json.loads(generated[PROFILES])['records']
        wheel.admit(ROOT, generated, PROFILES, MANIFEST, order, encoded)
        after = json.loads(generated[PROFILES])['records']
        retained = [record for record in after
                    if record['production_definition']['production_key'] not in wheel.KEYS]
        self.assertEqual(retained, before)
        added = [record for record in after
                 if record['production_definition']['production_key'] in wheel.KEYS]
        self.assertEqual(len(added), 17)
        self.assertEqual({row['production_definition']['production_key'] for row in added}, wheel.KEYS)
        for record in added:
            item = int(record['production_definition']['production_key'].rsplit('i', 1)[1])
            self.assertEqual(record['semantics']['physical']['value']['weight']['value'], wheel.WEIGHTS[item])
            self.assertEqual(record['semantics']['stack'], wheel.STACK)
            self.assertEqual(record['admission'], wheel.ADMISSION)
        self.assertEqual(json.loads(generated[MANIFEST])['item_profiles']['sha256'],
                         hashlib.sha256(generated[PROFILES]).hexdigest())
        before_retry = copy.deepcopy(generated)
        with self.assertRaisesRegex(ValueError, 'WHEEL_GEM_PROFILE_DUPLICATE'):
            wheel.admit(ROOT, generated, PROFILES, MANIFEST, order, encoded)
        self.assertEqual(generated, before_retry)

    def test_wrong_source_identity_stack_weight_and_admission_refuse_before_outputs(self):
        for mutation in ('source', 'identity', 'stack', 'weight', 'admission'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                packet = self.packet()
                first = packet['records'][0]
                if mutation == 'source':
                    packet['sources'][0]['revision'] = '0' * 40
                elif mutation == 'identity':
                    first['production_definition']['production_key'] = 'oteryn:item.tibia.i46627'
                elif mutation == 'stack':
                    first['semantics']['stack']['value']['stack_max']['value'] = 101
                elif mutation == 'weight':
                    greater = next(row for row in packet['records']
                                   if row['production_definition']['production_key'] == 'oteryn:item.tibia.i46626')
                    greater['semantics']['physical']['value']['weight']['value'] = 10
                else:
                    first['admission']['legal_destinations'].append('Other')
                target = root / wheel.PATH
                target.parent.mkdir(parents=True)
                target.write_bytes(encoded(packet))
                generated = self.base()
                before = copy.deepcopy(generated)
                with self.assertRaises(ValueError):
                    wheel.admit(root, generated, PROFILES, MANIFEST, order, encoded)
                self.assertEqual(generated, before)

    def test_source_capture_rejects_substituted_bytes_before_parsing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / 'data/items/items.xml'
            path.parent.mkdir(parents=True)
            path.write_bytes(b'<items/>')
            with self.assertRaisesRegex(ValueError, 'WHEEL_GEM_SOURCE_BLOB:data/items/items.xml'):
                wheel.capture(root)


if __name__ == '__main__':
    unittest.main()
