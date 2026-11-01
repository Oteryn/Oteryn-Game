"""Four exact portable identities; condition use never masquerades as HP/MP restoration."""
import copy
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import condition_potion_profiles as tool


@contextmanager
def packet_root(packet):
    """Keep the independent canonical definitions and binding table unchanged."""
    original = json.loads((tool.ROOT / tool.PATH).read_bytes())
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        paths = [row['path'] for row in original['canonical_inputs']]
        paths.append(tool.BINDINGS.relative_to(tool.ROOT).as_posix())
        for name in paths:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((tool.ROOT / name).read_bytes())
        path = root / tool.PATH
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(packet))
        yield root


class ConditionPotionProfiles(unittest.TestCase):
    def test_all_four_profiles_are_source_bound_and_condition_only(self):
        packet = json.loads((tool.ROOT / tool.PATH).read_bytes())
        rows = packet['records']
        self.assertEqual([r['production_definition']['production_key'] for r in rows],
                         [f'oteryn:item.tibia.i{i}' for i in (7439, 7440, 7443, 49271)])
        self.assertEqual(packet['source_revisions']['crystal'], '201189f871d23597b82caf88fd611454a2ddefb8')
        for row in rows:
            self.assertEqual(row['production_binding']['target']['key'], row['production_definition']['production_key'])
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertTrue(row['admission']['materializable'])
            self.assertIn('CharacterInventory', row['admission']['legal_destinations'])
            self.assertNotIn('consumption', row['semantics'])
            self.assertEqual(row['semantics']['container']['state'], 'NOT_APPLICABLE')
        self.assertEqual(rows[0]['semantics']['use_requirements']['value']['vocations']['value'], ['KNIGHT'])
        self.assertEqual(rows[1]['semantics']['use_requirements']['value']['vocations']['value'], ['DRUID', 'SORCERER'])
        self.assertEqual(rows[2]['semantics']['use_requirements']['value']['vocations']['value'], ['PALADIN'])
        self.assertEqual(rows[3]['semantics']['use_requirements']['value']['vocations']['state'], 'NOT_APPLICABLE')

    def test_registration_adds_four_and_rejects_duplicate_without_partial_write(self):
        planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        canonical = lambda v: (json.dumps(v, sort_keys=True) + '\n').encode()
        order = lambda r: r['production_definition']['production_key']
        tool.admit(tool.ROOT, planned, 'profiles', 'manifest', order, canonical)
        self.assertEqual(len(json.loads(planned['profiles'])['records']), 4)
        self.assertEqual(json.loads(planned['manifest'])['item_profiles']['sha256'],
                         hashlib.sha256(planned['profiles']).hexdigest())
        before = copy.deepcopy(planned)
        with self.assertRaisesRegex(ValueError, 'CONDITION_POTION_PROFILE_DUPLICATE'):
            tool.admit(tool.ROOT, planned, 'profiles', 'manifest', order, canonical)
        self.assertEqual(planned, before)

    def test_missing_identity_fails_closed_before_writing_any_output(self):
        packet = json.loads((tool.ROOT / tool.PATH).read_bytes())
        packet['records'].pop()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / tool.PATH
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(packet))
            planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
            before = dict(planned)
            with self.assertRaisesRegex(ValueError, 'CONDITION_POTION_PROFILE_SET'):
                tool.admit(root, planned, 'profiles', 'manifest', lambda r: 0, lambda v: b'')
            self.assertEqual(planned, before)

    def test_mutated_identity_and_physical_policy_fail_before_output(self):
        original = json.loads((tool.ROOT / tool.PATH).read_bytes())
        mutations = [
            (('production_binding', 'external_id'), '7440'),
            (('production_binding', 'target', 'family'), 'Monster'),
            (('production_binding', 'target', 'revision'), 'definition-r2'),
            (('production_definition', 'family'), 'Monster'),
            (('authoring', 'item', 'key'), 'candidate:item/7440'),
            (('semantics', 'physical', 'value', 'weight', 'value'), 201),
            (('semantics', 'stack', 'value', 'stack_max', 'value'), 101),
            (('semantics', 'container'), {'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': 8}}}),
        ]
        for path, value in mutations:
            with self.subTest(path=path):
                packet = copy.deepcopy(original)
                target = packet['records'][0]
                for part in path[:-1]:
                    target = target[part]
                target[path[-1]] = value
                with packet_root(packet) as root:
                    planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
                    before = dict(planned)
                    with self.assertRaisesRegex(ValueError, 'CONDITION_POTION_'):
                        tool.admit(root, planned, 'profiles', 'manifest', lambda r: 0, lambda v: b'')
                    self.assertEqual(planned, before)

    def test_removed_canonical_evidence_and_changed_source_pin_fail_closed(self):
        original = json.loads((tool.ROOT / tool.PATH).read_bytes())
        for name in ('canonical_inputs', 'source_revisions', 'sources'):
            with self.subTest(name=name):
                packet = copy.deepcopy(original)
                packet[name] = {} if name == 'source_revisions' else []
                with packet_root(packet) as root:
                    planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
                    before = dict(planned)
                    with self.assertRaisesRegex(ValueError, 'CONDITION_POTION_'):
                        tool.admit(root, planned, 'profiles', 'manifest', lambda r: 0, lambda v: b'')
                    self.assertEqual(planned, before)

    def test_duplicate_canonical_identity_is_not_resolved_by_last_write(self):
        packet = json.loads((tool.ROOT / tool.PATH).read_bytes())
        with packet_root(packet) as root:
            definitions = root / 'content/items/definitions'
            original = json.loads(next(definitions.glob('items-*.json')).read_bytes())
            keys = {f'oteryn:item.tibia.i{i}' for i in tool.IDS}
            selected = next(row for row in original['records'] if row['definition']['identity']['key'] in keys)
            (definitions / 'items-99999-99999.json').write_text(json.dumps({'records': [selected]}))
            planned = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
            before = dict(planned)
            with self.assertRaisesRegex(ValueError, 'CONDITION_POTION_CANONICAL_IDENTITIES'):
                tool.admit(root, planned, 'profiles', 'manifest', lambda r: 0, lambda v: b'')
            self.assertEqual(planned, before)


if __name__ == '__main__':
    unittest.main()
