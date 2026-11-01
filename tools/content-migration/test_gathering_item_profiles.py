"""The existing mechanical-rod owner needs its exact portable source identity."""
import copy
from contextlib import contextmanager
import json
from pathlib import Path
import tempfile
import unittest

import gathering_item_profiles as tool

ROOT = Path(__file__).resolve().parents[2]
KEY = 'oteryn:item.tibia.i9306'
CANONICAL = 'content/items/definitions/items-33000-33499.json'
WIKI = 'imports/tibiawiki/facts/items-stats.json'
BINDINGS = 'imports/crystalserver/bindings/items.json'


@contextmanager
def packet_root(packet):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        for name in (CANONICAL, WIKI, BINDINGS):
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((ROOT / name).read_bytes())
        path = root / tool.PATH
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(packet))
        yield root


class GatheringItemProfileTests(unittest.TestCase):
    def packet(self):
        return json.loads((ROOT / tool.PATH).read_bytes())

    def mechanical(self, packet):
        rows = [r for r in packet['records'] if r['production_definition']['production_key'] == KEY]
        self.assertEqual(len(rows), 1, 'the existing ice-hole mechanic needs one admitted rod profile')
        return rows[0]

    def test_mechanical_rod_keeps_existing_qualified_weight_and_identity(self):
        packet = self.packet()
        row = self.mechanical(packet)
        self.assertEqual(row['production_binding']['external_id'], '9306')
        self.assertEqual(row['production_binding']['target'], {
            'family': 'Item', 'key': KEY, 'revision': 'definition-r1'})
        self.assertEqual(row['semantics']['physical']['value']['weight']['value'], 920)
        self.assertFalse(row['semantics']['stack']['value']['stackable']['value'])
        self.assertEqual(row['semantics']['container'], {'state': 'NOT_APPLICABLE'})
        self.assertEqual(row['admission'], {'materializable': True, 'stack_class': 'NonStackable',
                                          'legal_destinations': ['CharacterInventory', 'Ground']})
        proof, = packet['qualified_overrides']
        self.assertEqual((proof['donor_value'], proof['qualified_value']), (919, 920))
        self.assertEqual((proof['page_id'], proof['revision_id']), (36707, 1114794))
        planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        tool.admit(ROOT, planned, 'profiles', 'manifest',
                   lambda r: r['production_definition']['production_key'], lambda v: json.dumps(v).encode())
        self.assertEqual(len(json.loads(planned['profiles'])['records']), 29)

    def test_mutated_rod_weight_or_binding_is_rejected_without_partial_output(self):
        for field in ('weight', 'binding'):
            with self.subTest(field=field):
                packet = self.packet()
                row = self.mechanical(packet)
                if field == 'weight':
                    row['semantics']['physical']['value']['weight']['value'] = 919
                else:
                    row['production_binding']['external_id'] = '3483'
                with packet_root(packet) as root:
                    planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
                    before = copy.deepcopy(planned)
                    with self.assertRaisesRegex(ValueError, 'GATHERING_MECHANICAL_'):
                        tool.admit(root, planned, 'profiles', 'manifest', lambda r: 0, lambda v: b'')
                    self.assertEqual(planned, before)

    def test_changed_canonical_weight_or_wiki_revision_requires_new_qualification(self):
        packet = self.packet()
        self.mechanical(packet)
        for field in ('canonical', 'wiki'):
            with self.subTest(field=field), packet_root(packet) as root:
                path = root / (CANONICAL if field == 'canonical' else WIKI)
                document = json.loads(path.read_bytes())
                if field == 'canonical':
                    row = next(r for r in document['records'] if r['definition']['identity']['key'] == KEY)
                    row['definition']['semantics']['physical']['value']['weight']['value'] = 921
                else:
                    document['records'][KEY]['observations'][0]['revision_id'] = 1114795
                path.write_text(json.dumps(document))
                planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
                before = dict(planned)
                with self.assertRaisesRegex(ValueError, 'GATHERING_MECHANICAL_'):
                    tool.admit(root, planned, 'profiles', 'manifest', lambda r: 0, lambda v: b'')
                self.assertEqual(planned, before)


if __name__ == '__main__':
    unittest.main()
