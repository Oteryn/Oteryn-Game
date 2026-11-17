"""ITEMHOLD-Q catalogue admission and retained source-conflict checks."""
import json
import copy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/reward-claim-authoring'))
from reward_claim_authoring import stack_problem

RESOLVED = (3572, 3054, 14156, 12369, 39546, 3083, 3056, 2881, 2882, 52732, 33330, 44179)
# Main admits starter Item 34017; retain a held catalogue row and an unknown ID.
HELD = (1000, 54609)


class ItemholdQTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = {}
        for path in (ROOT / 'content/items/definitions').glob('items-*.json'):
            for row in json.loads(path.read_bytes())['records']:
                definition = row['definition']
                cls.items[definition['identity']['key']] = definition

    def test_resolved_items_materialize_at_catalogue_level(self):
        for item in RESOLVED:
            with self.subTest(item=item):
                definition = self.items[f'oteryn:item.tibia.i{item}']
                self.assertIsNone(stack_problem(definition, 1))
                self.assertEqual(definition['stack_class'], 'NonStackable')
                self.assertIsNotNone(stack_problem(definition, 2))

    def test_remaining_holds_still_reject(self):
        for item in HELD:
            with self.subTest(item=item):
                self.assertIsNotNone(stack_problem(self.items.get(f'oteryn:item.tibia.i{item}'), 1))

    def test_charged_definitions_keep_exact_donor_counts(self):
        for item, charges in ((3054, 200), (3056, 200), (3083, 150)):
            self.assertEqual(self.items[f'oteryn:item.tibia.i{item}']['semantics']['charges'],
                             {'state': 'KNOWN', 'value': {'count': {'state': 'KNOWN', 'value': charges}}})

    def test_ornamented_shield_exact_source_charge_keeps_conflicting_witness(self):
        path = ROOT / 'content/interactions/reward_claims/reward-claim-variants-00000-00099.json'
        claim = next(row['definition'] for row in json.loads(path.read_bytes())['records']
                     if row['definition']['identity']['key'].endswith('.kv.ornamentedshield'))
        self.assertEqual(claim['data_holds'], [{'category': 'source', 'code': 'SOURCE_CHARGE_MISMATCH',
                          'placement_index': 0, 'reward_field': 'items', 'reward_index': 2}])
        self.assertTrue(any(c['reason'] == 'SOURCE_CHARGE_MISMATCH'
                            for c in claim['source_variant']['charge_dispositions']))
        self.assertTrue(claim['native_contents']['placements'])
        placement, = claim['native_contents']['placements']
        child = placement['contents'][2]
        self.assertEqual(child['item']['key'], 'oteryn:item.tibia.i3083')
        self.assertEqual(child['count'], 1)
        self.assertEqual(child['charges_remaining'], 1)
        self.assertEqual(claim['native_contents']['source_difference'],
                         'EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS')
        import itemhold_charge_precedence as precedence
        self.assertFalse(precedence.qualifies(self.items['oteryn:item.tibia.i3083'],
            'canary:reward-claim/kv/ornamentedshield', {'x': 32769, 'y': 32302, 'z': 10},
            'oteryn:item.tibia.i3083', 1))

    def test_remaining_sources_apply_precedence_and_reject_corruption(self):
        import itemhold_q_remaining as remaining
        self.assertEqual(remaining.source_facts(ROOT), {33330: 7500, 44179: 60, 52732: 90})
        rows = [copy.deepcopy(self.items[f'oteryn:item.tibia.i{i}']) for i in remaining.IDS]
        remaining.apply_admissions(rows, ROOT)
        once = copy.deepcopy(rows)
        remaining.apply_admissions(rows, ROOT)
        self.assertEqual(rows, once)
        bottle = next(r for r in rows if r['identity']['key'].endswith('.i44179'))
        bottle['semantics']['physical']['value']['weight']['value'] = 0
        with self.assertRaisesRegex(ValueError, 'CONFLICT'):
            remaining.apply_admissions(rows, ROOT)

    def test_full_charge_source_projection_is_closed_to_exact_power_ring(self):
        import itemhold_charge_precedence as precedence
        self.assertEqual(set(precedence.CASES), {'canary:reward-claim/kv/powerring2'})
        for claim, values in precedence.CASES.items():
            item, raw, _, position, *_ = values
            key = f'oteryn:item.tibia.i{item}'
            self.assertTrue(precedence.qualifies(self.items[key], claim, position, key, raw))
            self.assertFalse(precedence.qualifies(self.items[key], claim, position, key, raw + 1))
            self.assertFalse(precedence.qualifies(self.items[key], claim, {**position, 'z': 0}, key, raw))
            self.assertFalse(precedence.qualifies(self.items[key], claim + '-other', position, key, raw))

    def test_remaining_loader_defaults_reject_a_changed_constructor(self):
        import itemhold_q_remaining as remaining
        read = Path.read_bytes

        def altered(path):
            raw = read(path)
            if path == ROOT / remaining.RETAINED:
                document = json.loads(raw)
                record = next(r for r in document['source_files']
                              if r['repository_path'] == 'src/items/items.hpp'
                              and r['revision'] == remaining.CRYSTAL_REVISION)
                record['raw_utf8'] += '\n'
                return json.dumps(document).encode()
            return raw

        with patch.object(Path, 'read_bytes', altered):
            with self.assertRaisesRegex(ValueError, 'CONSTRUCTOR_PIN'):
                remaining.source_facts(ROOT)

    def test_source_overlay_is_idempotent_and_rejects_conflicting_accepted_facts(self):
        import itemhold_q
        definitions = [copy.deepcopy(self.items[f'oteryn:item.tibia.i{i}']) for i in itemhold_q.IDS]
        itemhold_q.apply_admissions(definitions)
        once = copy.deepcopy(definitions)
        itemhold_q.apply_admissions(definitions)
        self.assertEqual(definitions, once)
        for group, leaf in (('physical', 'weight'), ('charges', 'count')):
            conflict = copy.deepcopy(once)
            amulet = next(d for d in conflict if d['identity']['key'].endswith('.i3054'))
            amulet['semantics'][group]['value'][leaf]['value'] += 1
            with self.assertRaisesRegex(ValueError, 'CONFLICT'):
                itemhold_q.apply_admissions(conflict)

    def test_shared_shard_repin_rejects_any_changed_shop_definition(self):
        import hashlib
        import grizzly_shop_profiles as grizzly
        baseline = json.dumps({'records': [
            {'definition': {'identity': {'key': 'shop'}, 'materializable': False}},
            {'definition': {'identity': {'key': 'wolf'}, 'materializable': False}},
        ]}).encode()
        for changed_key in ('wolf', 'shop'):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                packet = root / grizzly.PATH
                packet.parent.mkdir(parents=True)
                packet.write_text(json.dumps({'canonical_inputs': [{
                    'path': 'shard.json', 'sha256': hashlib.sha256(baseline).hexdigest()}]}))
                shard = json.loads(baseline)
                next(row['definition'] for row in shard['records']
                     if row['definition']['identity']['key'] == changed_key)['materializable'] = True
                (root / 'shard.json').write_text(json.dumps(shard))
                with (patch.object(grizzly, 'selected', return_value=({'shop'}, 'digest')),
                      patch.object(grizzly.subprocess, 'check_output', return_value=baseline)):
                    if changed_key == 'shop':
                        with self.assertRaisesRegex(ValueError, 'SELECTED_DEFINITION_CHANGED'):
                            grizzly.refresh_canonical_input_pins(root, 'baseline')
                    else:
                        grizzly.refresh_canonical_input_pins(root, 'baseline')
                        self.assertEqual(json.loads(packet.read_bytes())['canonical_inputs'][0]['sha256'],
                                         hashlib.sha256((root / 'shard.json').read_bytes()).hexdigest())


if __name__ == '__main__':
    unittest.main()
