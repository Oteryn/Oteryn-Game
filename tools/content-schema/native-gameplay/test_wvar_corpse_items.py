import copy
import json
from pathlib import Path
import tempfile
import unittest

import wvar_corpse_items as wvar

ROOT = Path(__file__).resolve().parents[3]
SHARDS = ('content/items/definitions/items-24500-24999.json',
          'content/items/definitions/items-29500-29999.json')


class WvarCorpseItemsTests(unittest.TestCase):
    def definitions(self):
        return [r['definition'] for path in SHARDS
                for r in json.loads((ROOT / path).read_bytes())['records']]

    def test_only_related_corpse_chain_is_admitted_with_exact_source_decay(self):
        definitions = self.definitions()
        before = copy.deepcopy(definitions)
        wvar.apply_admissions(definitions)
        by_key = {d['identity']['key']: d for d in definitions}
        for item, capacity, duration, target in wvar.CHAIN:
            definition = by_key[f'oteryn:item.tibia.i{item}']
            self.assertTrue(definition['materializable'])
            self.assertEqual(definition['stack_class'], 'NonStackable')
            semantics = definition['semantics']
            self.assertEqual(semantics['container'],
                             {'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': capacity}}}
                             if capacity is not None else {'state': 'NOT_APPLICABLE'})
            temporal = semantics['temporal']['value']
            self.assertEqual(temporal['duration'], {'state': 'KNOWN', 'value': duration})
            self.assertEqual(temporal['decay_target'],
                             {'state': 'KNOWN', 'value': {'key': f'oteryn:item.tibia.i{target}', 'revision': 'definition-r1'}}
                             if target else {'state': 'NOT_APPLICABLE'})
            self.assertEqual({key for key, value in semantics.items() if value['state'] != 'UNKNOWN'},
                             {'physical', 'stack', 'container', 'temporal'})
        owned = {f'oteryn:item.tibia.i{item}' for item in wvar.OWNED}
        self.assertEqual([d for d in definitions if d['identity']['key'] not in owned],
                         [d for d in before if d['identity']['key'] not in owned])
        once = copy.deepcopy(definitions)
        wvar.apply_admissions(definitions)
        self.assertEqual(definitions, once)

    def test_world_object_substitution_refuses_before_item_writes(self):
        definitions = self.definitions()
        before = copy.deepcopy(definitions)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in wvar.OBJECT_SHARDS:
                destination = root / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                doc = json.loads((ROOT / path).read_bytes())
                for row in doc['records']:
                    if row.get('provenance', {}).get('source_item_id') == 4039:
                        row['corpse']['player_corpse'] = {'state': 'KNOWN', 'value': True}
                destination.write_text(json.dumps(doc))
            with self.assertRaisesRegex(ValueError, 'WVAR_CORPSE_WORLD_OBJECT_BINDING'):
                wvar.apply_admissions(definitions, root)
        self.assertEqual(definitions, before)

    def test_conflicting_admitted_policy_is_not_overwritten(self):
        definitions = self.definitions()
        wvar.apply_admissions(definitions)
        target = next(d for d in definitions if d['identity']['key'] == 'oteryn:item.tibia.i4039')
        target['semantics']['temporal']['value']['duration']['value'] += 1
        before = copy.deepcopy(definitions)
        with self.assertRaisesRegex(ValueError, 'WVAR_CORPSE_ADMITTED_POLICY_CONFLICT'):
            wvar.apply_admissions(definitions)
        self.assertEqual(definitions, before)


if __name__ == '__main__':
    unittest.main()
