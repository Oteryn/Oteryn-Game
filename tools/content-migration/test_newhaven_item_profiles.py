"""Newhaven admission keeps existing quiver rules and refuses partial overlays."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import newhaven_item_profiles as profiles

ROOT = Path(__file__).resolve().parents[2]


class NewhavenItemProfilesTests(unittest.TestCase):
    def generated(self):
        overlay = json.loads((ROOT / profiles.PATH).read_bytes())
        records = [copy.deepcopy(r) for r in overlay['records']
                   if int(r['production_definition']['production_key'].rsplit('.i', 1)[1]) in profiles.EXISTING]
        quiver = next(r for r in records if r['production_definition']['production_key'].endswith('.i35562'))
        pattern = copy.deepcopy(quiver['semantics']['equipment']['value']['patterns']['value'][0])
        pattern['pattern_id'] = 2
        pattern['primary_slot']['value'] = 'EXTRA'
        quiver['semantics']['equipment']['value']['patterns']['value'].append(pattern)
        return {'profiles': json.dumps({'records': records}).encode(), 'manifest': b'{"item_profiles":{}}'}

    def test_quiver_extra_slot_survives_source_physical_enrichment(self):
        generated = self.generated()
        profiles.admit(ROOT, generated, 'profiles', 'manifest', lambda r: r['production_definition']['production_key'], lambda v: json.dumps(v).encode())
        quiver = next(r for r in json.loads(generated['profiles'])['records'] if r['production_definition']['production_key'].endswith('.i35562'))
        self.assertEqual([p['primary_slot']['value'] for p in quiver['semantics']['equipment']['value']['patterns']['value']], ['SHIELD', 'EXTRA'])
        self.assertEqual(len(json.loads(generated['profiles'])['records']), len(profiles.IDS))

    def test_missing_operand_refuses_without_changing_generated_output(self):
        overlay = json.loads((ROOT / profiles.PATH).read_bytes())
        overlay['records'].pop()
        generated = self.generated()
        before = dict(generated)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / profiles.PATH
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(overlay))
            with self.assertRaisesRegex(ValueError, 'NEWHAVEN_ITEM_PROFILE_SET'):
                profiles.admit(root, generated, 'profiles', 'manifest', lambda r: 0, lambda v: b'')
        self.assertEqual(generated, before)


if __name__ == '__main__':
    unittest.main()
