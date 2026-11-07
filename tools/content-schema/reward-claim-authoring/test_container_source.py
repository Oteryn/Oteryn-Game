"""Finite SOURCE snapshot regressions; native implementation stays pending."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import jsonschema
import container_source as c

BASE = Path(c.__file__).resolve().parent
SAMPLES = BASE / 'samples/container-source' if (BASE / 'samples/container-source').exists() else BASE
SCHEMA = Path(__file__).resolve().with_name('container_source.schema.json')


class ContainerSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (SAMPLES / 'input.json').read_bytes()
        cls.digest = hashlib.sha256(cls.raw).hexdigest()
        cls.source = json.loads(cls.raw)
        cls.output = c.compile_snapshot(cls.source, cls.digest)
        cls.validator = jsonschema.Draft202012Validator(json.loads(SCHEMA.read_text()))

    def rows(self, output=None):
        return [(r, p) for r in (output or self.output)['records'] for p in r['placements']]

    def test_whole_snapshot_recompute_and_originals_unchanged(self):
        snapshot = copy.deepcopy(self.source); before = copy.deepcopy(snapshot)
        self.assertEqual(c.compile_snapshot(snapshot, self.digest), self.output)
        self.assertEqual(self.digest, c.INPUT_SHA256)
        self.assertEqual(self.output, json.loads((SAMPLES / 'compiled.json').read_bytes()))
        self.assertEqual(self.output['summary'], dict(records=65, placements=68, logical_roles=227, AUTHORED=65, WAITING_DATA=0, CONFLICT=3))
        originals = {r['source_claim']['identity']['key']: r for r in self.source['records']}
        for row in self.output['records']:
            self.assertEqual(row['original_record'], originals[row['original_record']['source_claim']['identity']['key']])
            self.assertEqual(row['native_lowering'], row['original_record']['native_lowering'])
            self.assertEqual(row['native_lowering']['status'], 'WAITING_IMPLEMENTATION')
        self.assertEqual(snapshot, before)
        self.validator.validate(self.output)

    def test_fluids_are_explicit_water_not_empty_or_name_guesses(self):
        fluid_entries = [(p, e) for _, p in self.rows() for e in p['entries'] if 'source_fluid' in e]
        self.assertEqual(len(fluid_entries), 7)
        self.assertEqual({e['item']['key'] for _, e in fluid_entries}, {'oteryn:item.tibia.i2874', 'oteryn:item.tibia.i2881', 'oteryn:item.tibia.i2882'})
        for p, e in fluid_entries:
            self.assertEqual(e['source_raw_argument'], 1)
            self.assertEqual(e['normalized_quantity'], {'state': 'KNOWN', 'value': 1})
            self.assertEqual(e['source_fluid'], {'subtype': 1, 'symbol': 'WATER'})
            definition = next(i for i in self.source['items'] if i['identity'] == e['item'])
            self.assertEqual(definition['semantics']['fluid']['state'], 'UNKNOWN')
            children = [n for n in p['item_tree']['value']['contents'] if n['item'] == e['item']]
            self.assertTrue(children)
            self.assertTrue(all(n['source_fluid'] == e['source_fluid'] for n in children))
            self.assertIn({'kind': 'FLUID_INSTANCE_NATIVE_ADMISSION_UNRESOLVED', 'item': e['item']}, p['native_constraints'])

    def test_charge_conflicts_and_only_proven_quantity_normalization(self):
        charged = [p for _, p in self.rows() if 'SOURCE_CHARGE_MISMATCH' in p['source_checks']]
        self.assertEqual(len(charged), 2)
        roles = [e for p in charged for e in p['entries'] if e['quantity_basis'] == 'SOURCE_CHARGES_SUBTYPE']
        self.assertEqual(len(roles), 3)
        self.assertTrue(all(e['normalized_quantity'] == {'state': 'KNOWN', 'value': 1} for e in roles))
        self.assertTrue(all(p['item_tree'] == {'state': 'UNKNOWN'} for p in charged))
        proven = [e for _, p in self.rows() for e in p['entries'] if e['item']['key'] == 'oteryn:item.tibia.i3081' and e['source_raw_argument'] == 5]
        self.assertTrue(proven)
        self.assertTrue(all(e['normalized_quantity'] == {'state': 'KNOWN', 'value': 1} for e in proven))
        runes = [e for _, p in self.rows() for e in p['entries'] if e['item']['key'] in ('oteryn:item.tibia.i3155', 'oteryn:item.tibia.i3160')]
        self.assertEqual(sorted(e['source_raw_argument'] for e in runes), [2, 3, 3])
        self.assertTrue(all(e['quantity_basis'] == 'SOURCE_LITERAL' and e['normalized_quantity'] == {'state': 'KNOWN', 'value': e['source_raw_argument']} for e in runes))

    def test_wiki_authority_separates_barbarian_recipe_from_helheim_conflict(self):
        roles = [(r['original_record']['source_claim']['identity']['key'], p, e) for r, p in self.rows() for e in p['entries'] if e['source_raw_argument'] == 200 and e['item']['key'] in ('oteryn:item.tibia.i3450', 'oteryn:item.tibia.i3031')]
        self.assertEqual(len(roles), 2)
        for key, p, e in roles:
            self.assertEqual(e['source_executed_quantity'], {'state': 'KNOWN', 'value': 100})
            if key.endswith('reward_scrapper_bp'):
                self.assertEqual(e['normalized_quantity'], {'state': 'KNOWN', 'value': 200})
                self.assertEqual(e['quantity_basis'], 'SOURCE_LITERAL_CORROBORATED_BY_PINNED_WIKI')
                self.assertEqual([n['count'] for n in p['item_tree']['value']['contents'] if n['item'] == e['item']], [100, 100])
            else:
                self.assertEqual(e['normalized_quantity'], {'state': 'UNKNOWN'})
                self.assertEqual(p['item_tree'], {'state': 'UNKNOWN'})
                self.assertEqual(p['data_status'], 'CONFLICT')

    def test_native_capacity_and_materializable_holds_do_not_erase_recipes(self):
        caps = [p for _, p in self.rows() if p['definition_capacity'] == {'state': 'KNOWN', 'value': 24}]
        self.assertEqual(len(caps), 1)
        self.assertEqual(caps[0]['data_status'], 'AUTHORED')
        self.assertTrue(any(h['kind'] == 'CONTAINER_CAPACITY_ABOVE_ADMITTED_MAXIMUM' and h['value'] == 24 for h in caps[0]['native_constraints']))
        nodes = [n for _, p in self.rows() if p['item_tree']['state'] == 'KNOWN' for n in p['item_tree']['value']['contents']]
        self.assertEqual(sum(n.get('contents') == [] for n in nodes if 'contents' in n), 3)
        holds = {h['item']['key'] for _, p in self.rows() for h in p['native_constraints'] if h['kind'] == 'ITEM_NOT_MATERIALIZABLE'}
        self.assertTrue({i['identity']['key'] for i in self.source['items'] if not i['materializable']} <= holds)
        self.assertTrue(any(h['kind'] == 'CONTAINER_CAPACITY_ABOVE_ADMITTED_MAXIMUM' and h['value'] == 22 for _, p in self.rows() for h in p['native_constraints']))

    def test_duplicate_indices_references_and_charge_witnesses_rejected(self):
        mutations = [lambda s: s['records'].append(copy.deepcopy(s['records'][0])), lambda s: s['items'].append(copy.deepcopy(s['items'][0])), lambda s: s['provenance']['item_shards'][0]['definitions'].append(copy.deepcopy(s['provenance']['item_shards'][0]['definitions'][0])), lambda s: s['records'][0]['item_bindings'].append(copy.deepcopy(s['records'][0]['item_bindings'][0]))]
        mutations += [lambda s: s['records'][0]['item_bindings'][0].update(placement_index=999), lambda s: s['records'][0]['item_bindings'][0].update(placement_index=True), lambda s: s['records'][0]['item_bindings'][0]['canonical_item'].update(revision='forged'), lambda s: next(r for r in s['records'] if r['charge_dispositions'])['charge_dispositions'][0].update(normalized_quantity={'state': 'KNOWN', 'value': 999})]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                snapshot = copy.deepcopy(self.source); mutate(snapshot)
                with self.assertRaises(ValueError): c.compile_snapshot(snapshot, self.digest)

    def test_zero_stack_max_is_unknown_never_implicit_100(self):
        snapshot = copy.deepcopy(self.source)
        item = next(i for i in snapshot['items'] if i['identity']['key'] == 'oteryn:item.tibia.i3031')
        item['semantics']['stack']['value']['stack_max']['value'] = 0
        output = c.compile_snapshot(snapshot, self.digest)
        affected = [p for _, p in self.rows(output) if any(e['item']['key'] == item['identity']['key'] for e in p['entries'])]
        self.assertTrue(affected)
        self.assertTrue(all('ITEM_STACK_SEMANTICS_UNKNOWN' in p['source_checks'] and p['item_tree'] == {'state': 'UNKNOWN'} for p in affected))

    def test_strict_schema_rejects_native_ready_unknown_values_and_guessed_fluid(self):
        for mutate in (lambda o: o.update(scope='NATIVE_READY'), lambda o: o['records'][0]['native_lowering'].update(status='READY'), lambda o: o['records'][0]['placements'][0]['entries'][0].update(normalized_quantity={'state': 'UNKNOWN', 'value': 1}), lambda o: next(e for _, p in self.rows(o) for e in p['entries'] if 'source_fluid' in e)['source_fluid'].update(symbol='MILK')):
            output = copy.deepcopy(self.output); mutate(output)
            self.assertTrue(list(self.validator.iter_errors(output)))

    def test_cli_rejects_changed_raw_input_and_forged_compiled_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, target = Path(tmp) / 'input.json', Path(tmp) / 'compiled.json'
            source.write_bytes(self.raw + b' ')
            with patch('sys.argv', ['container_source.py', '--input', str(source), '--output', str(target)]):
                with self.assertRaisesRegex(ValueError, 'immutable input'): c.main()
            source.write_bytes(self.raw)
            forged = copy.deepcopy(self.output); forged['summary']['logical_roles'] += 1
            target.write_text(c.encoded(forged))
            with patch('sys.argv', ['container_source.py', '--input', str(source), '--output', str(target), '--check']), patch.object(c, 'LOCAL', SCHEMA.parent):
                with self.assertRaisesRegex(ValueError, 'full offline recomputation'): c.main()


if __name__ == '__main__': unittest.main()
