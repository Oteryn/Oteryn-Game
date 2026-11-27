"""Generator admission for the retained Twenty Miles runtime inputs."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import qc_item_profiles
import register_spell_families as spells


ROOT = Path(__file__).resolve().parents[2]
SWIMMING = 'tools/content-schema/native-gameplay/qc-swimming-appearance.json'


class TwentyMilesOverlayTests(unittest.TestCase):
    def stage(self, path, mutate):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name)
        target = root / path
        target.parent.mkdir(parents=True)
        document = json.loads((ROOT / path).read_bytes())
        mutate(document)
        target.write_text(json.dumps(document))
        return root

    def test_changed_swimming_look_with_old_qualification_is_rejected(self):
        root = self.stage(SWIMMING, lambda packet: packet['record'].update(look_type=268))
        destination = spells.spell_import.PROVIDER_DESTINATIONS['spell_appearances']
        existing = json.loads((ROOT / destination).read_bytes())
        existing['records'] = [row for row in existing['records'] if row['look_type'] != 267]
        generated = {destination: json.dumps(existing).encode(),
                     spells.NATIVE_MANIFEST: (ROOT / spells.NATIVE_MANIFEST).read_bytes()}
        baseline = copy.deepcopy(generated)
        with self.assertRaisesRegex(ValueError, 'QC_SWIMMING_APPEARANCE_QUALIFICATION'):
            spells.admit_swimming_appearance(root, generated)
        self.assertEqual(generated, baseline)

    def test_unknown_field_on_new_item_is_rejected(self):
        root = self.stage(qc_item_profiles.PATH,
                          lambda packet: packet['records'][0].update(unknown_field=True))
        generated = {
            'profiles': b'{"records":[]}',
            'manifest': b'{"item_profiles":{}}',
        }
        baseline = copy.deepcopy(generated)
        with self.assertRaisesRegex(ValueError, 'QC_ITEM_RECORD_QUALIFICATION'):
            qc_item_profiles.admit(root, generated, 'profiles', 'manifest',
                                   lambda row: row['production_definition']['production_key'],
                                   spells.spell_import.canonical_bytes)
        self.assertEqual(generated, baseline)
