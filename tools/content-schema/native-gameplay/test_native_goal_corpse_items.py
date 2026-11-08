import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import native_goal_corpse_items as native


def definitions():
    policies, _ = native.policies()
    keys = {f'oteryn:item.tibia.i{i}' for i in policies} | {'oteryn:item.tibia.i39530'}
    return [row['definition'] for path in (native.ROOT / 'content/items/definitions').glob('items-*.json')
            for row in json.loads(path.read_bytes())['records'] if row['definition']['identity']['key'] in keys]


class NativeGoalCorpseProof(unittest.TestCase):
    def test_portable_native_profiles_remain_exact_and_conflicts_are_atomic(self):
        root = native.ROOT
        profiles = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        generated = {path: (root / path).read_bytes() for path in (profiles, manifest)}
        original = json.loads(generated[profiles])
        keys = {f'oteryn:item.tibia.i{i}' for i in (3990, 4103, 4104)}
        selected = lambda doc: {r['production_definition']['production_key']: r
            for r in doc['records'] if r['production_definition']['production_key'] in keys}
        order = lambda r: r['production_definition']['production_key']
        encode = lambda doc: (json.dumps(doc, sort_keys=True) + '\n').encode()
        native.admit_native(root, generated, profiles, manifest, order, encode)
        self.assertEqual(selected(json.loads(generated[profiles])), selected(original))
        tampered = json.loads(generated[profiles])
        selected(tampered)['oteryn:item.tibia.i3990']['semantics']['physical']['value']['pickupable']['value'] = False
        generated[profiles] = encode(tampered)
        before = copy.deepcopy(generated)
        with self.assertRaisesRegex(ValueError, 'PORTABLE_SOURCE_CONFLICT'):
            native.admit_native(root, generated, profiles, manifest, order, encode)
        self.assertEqual(generated, before)

    def test_merged_portable_source_corpses_keep_exact_main_facts(self):
        import sys
        sys.path.insert(0, str(native.ROOT / 'tools/content-migration'))
        import encounter_loot_item_admissions as source
        profiles = source.facts(native.ROOT, {3990, 4103, 4104}, allow_containers=True)
        items = {d['identity']['key']: d for d in definitions()}
        for key, profile in profiles.items():
            with self.subTest(item=key):
                for group in ('physical', 'container'):
                    self.assertEqual(items[key]['semantics'][group], profile['semantics'][group])

    def test_full_decay_closure_keeps_source_weights_and_capacities(self):
        policies, weights = native.policies()
        self.assertEqual(len(policies), 20)
        self.assertEqual(policies[5973], (12, 10000, 4025))
        self.assertEqual(policies[5963], (10, 10000, 3990))
        self.assertEqual(policies[4013], (10, 60000, None))
        self.assertEqual(weights[3990], 60000)
        self.assertEqual(weights[4041], 120000)
        self.assertEqual(weights[4013], 85000)
        self.assertEqual(weights[5973], 0)

    def test_admission_preserves_unowned_items_and_unrelated_semantics(self):
        items = definitions()
        before = copy.deepcopy(items)
        native.apply_admissions(items)
        for old, new in zip(before, items):
            if old['identity']['key'].endswith('i39530'):
                self.assertEqual(new, old)
                continue
            self.assertTrue(new['materializable'])
            self.assertEqual(new['semantics']['physical']['value']['pickupable']['value'],
                             new['identity']['key'] in {'oteryn:item.tibia.i3990',
                                 'oteryn:item.tibia.i4103', 'oteryn:item.tibia.i4104'})
            for group, accepted in old['semantics'].items():
                if group not in native.GROUPS:
                    self.assertEqual(new['semantics'][group], accepted)
        once = copy.deepcopy(items)
        native.apply_admissions(items)
        self.assertEqual(items, once)

    def test_known_fact_conflict_refuses_without_partial_admission(self):
        items = definitions()
        item = next(d for d in items if d['identity']['key'].endswith('i6011'))
        item['semantics']['physical'] = {'state': 'KNOWN', 'value': {
            'weight': {'state': 'KNOWN', 'value': 999}}}
        before = copy.deepcopy(items)
        with self.assertRaisesRegex(ValueError, 'KNOWN_FACT_CONFLICT'):
            native.apply_admissions(items)
        self.assertEqual(items, before)

    def test_wrong_worldobject_corpse_class_refuses_before_any_write(self):
        items = definitions()
        before = copy.deepcopy(items)
        read = Path.read_bytes
        def changed(path):
            raw = read(path)
            if path.parent == native.ROOT / 'content/world/objects' and path.name.startswith('objects-'):
                document = json.loads(raw)
                for row in document['records']:
                    if row.get('provenance', {}).get('item_pointer', {}).get('key') == 'oteryn:item.tibia.i6011':
                        row['corpse']['player_corpse'] = {'state': 'KNOWN', 'value': True}
                return json.dumps(document).encode()
            return raw
        with patch.object(Path, 'read_bytes', changed):
            with self.assertRaisesRegex(ValueError, 'WORLDOBJECT_PROOF'):
                native.apply_admissions(items)
        self.assertEqual(items, before)


if __name__ == '__main__':
    unittest.main()
