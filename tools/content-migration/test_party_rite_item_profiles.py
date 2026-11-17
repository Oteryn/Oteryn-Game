"""Native route admission must preserve source collision and reject input drift atomically."""
import json
import tempfile
import unittest
from pathlib import Path

import party_rite_item_profiles as party

ROOT = Path(__file__).resolve().parents[2]


class PartyRiteItemProfiles(unittest.TestCase):
    def test_exact_four_source_items_retain_their_collision(self):
        source = json.loads((ROOT / party.PATH).read_bytes())
        records = {r['production_definition']['production_key']: r for r in source['records']}
        self.assertEqual(set(records), {f'oteryn:item.tibia.i{i}' for i in (1841,2114,2153,3656)})
        for item in (1841,2114,2153):
            self.assertIs(records[f'oteryn:item.tibia.i{item}']['attributes']['blocks_movement'], True)
        self.assertIs(records['oteryn:item.tibia.i3656']['attributes']['blocks_movement'], False)
        self.assertEqual(set(source['source_lines']), {'1841','2114','2153','3656'})

    def test_duplicate_item_refuses_before_output_mutation(self):
        record = json.loads((ROOT / party.PATH).read_bytes())['records'][0]
        output = {'profiles': json.dumps({'records':[record]}).encode(), 'manifest':b'{}'}
        before = dict(output)
        with self.assertRaisesRegex(ValueError, 'PARTY_RITE_ITEM_PROFILE_DUPLICATE'):
            party.admit(ROOT, output, 'profiles', 'manifest', lambda r:0, lambda v:b'')
        self.assertEqual(output,before)

    def test_missing_item_or_foreign_source_refuses_atomically(self):
        original = json.loads((ROOT / party.PATH).read_bytes())
        for change in ('missing','foreign','duplicate_source'):
            source = json.loads(json.dumps(original))
            if change == 'missing':
                source['records'].pop()
            elif change == 'foreign':
                source['source_files'][0]['sha256'] = '0'*64
            else:
                source['source_files'][1] = source['source_files'][0]
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = root / party.PATH
                target.parent.mkdir(parents=True)
                target.write_text(json.dumps(source))
                proof = root / 'tools/content-schema/native-gameplay/canary-ground-proof.json'
                proof.write_bytes((ROOT / 'tools/content-schema/native-gameplay/canary-ground-proof.json').read_bytes())
                output = {'profiles':b'{"records":[]}','manifest':b'{}'}
                before = dict(output)
                with self.assertRaisesRegex(ValueError, 'PARTY_RITE_ITEM_'):
                    party.admit(root, output, 'profiles', 'manifest', lambda r:0, lambda v:b'')
                self.assertEqual(output,before)
