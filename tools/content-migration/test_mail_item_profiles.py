import json
from pathlib import Path
import unittest
from unittest.mock import patch

import mail_item_profiles as mail

ROOT = Path(__file__).resolve().parents[2]
PROFILES = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'
KEYS = [f'oteryn:item.tibia.i{row["id"]}' for row in mail.ROWS]
CARRIER = {'materializable': False, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']}


class MailItemProfilesTest(unittest.TestCase):
    def generated(self):
        # The writable layer's carriers before this layer: letter and label only.
        document = json.loads((ROOT / PROFILES).read_bytes())
        records = []
        for record in document['records']:
            key = record['production_definition']['production_key']
            if key in KEYS[2:5:2]:
                record['admission'] = dict(CARRIER)
            if key not in KEYS or key in KEYS[2:5:2]:
                records.append(record)
        document['records'] = records
        return {PROFILES: json.dumps(document).encode(), MANIFEST: (ROOT / MANIFEST).read_bytes()}

    def admit(self, generated):
        mail.admit(ROOT, generated, PROFILES, MANIFEST,
                   lambda r: int(r['production_definition']['production_key'].split('.i')[-1]),
                   lambda value: (json.dumps(value, sort_keys=True) + '\n').encode())

    def test_parcel_letter_and_label_are_carriable_with_reference_facts(self):
        generated = self.generated()
        self.admit(generated)
        result = {r['production_definition']['production_key']: r
                  for r in json.loads(generated[PROFILES])['records']}
        for row, key in zip(mail.ROWS, KEYS):
            record = result[key]
            self.assertEqual(record['admission'], {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(record['semantics']['physical']['value']['weight']['value'], row['weight'])
        self.assertEqual(result[KEYS[0]]['semantics']['container']['value']['capacity']['value'], 10)
        # A second application meets admitted records, never silently re-admits them.
        with self.assertRaisesRegex(ValueError, 'MAIL_PROFILE_CONFLICT'):
            self.admit(generated)

    def test_substituted_reference_revision_is_rejected(self):
        generated = self.generated()
        read = Path.read_bytes
        def altered(path):
            raw = read(path)
            if path == ROOT / mail.FACTS:
                facts = json.loads(raw)
                facts['source']['revision'] = 'unreviewed'
                return json.dumps(facts).encode()
            return raw
        with patch.object(Path, 'read_bytes', altered):
            with self.assertRaisesRegex(ValueError, 'MAIL_REFERENCE_FACTS'):
                self.admit(generated)


if __name__ == '__main__':
    unittest.main()
