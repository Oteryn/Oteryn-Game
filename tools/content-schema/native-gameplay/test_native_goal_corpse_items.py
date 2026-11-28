import copy
import hashlib
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
    def test_bonelord_has_an_admitted_corpse_and_exact_decay_chain(self):
        sources, _ = native.policies()
        self.assertEqual([sources.get(item) for item in (5992, 4089, 4090, 4091)], [
            (32, 10000, 4089), (32, 300000, 4090), (32, 300000, 4091),
            (None, 60000, None),
        ])
        root = native.ROOT
        profiles = json.loads((root / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        corpse = next(row for row in profiles['records']
                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i5992')
        self.assertEqual(corpse['admission']['legal_destinations'], ['Ground'])
        loot = json.loads((root / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        facts = next(row for row in loot['items'] if row['item']['key'] == 'oteryn:item.tibia.i5992')
        self.assertTrue(facts['materializable'])
        self.assertEqual(facts['container_capacity'], 32)

    def test_served_rat_corpse_and_decay_resolve_to_active_item_profiles(self):
        root = native.ROOT
        manifest = json.loads((root / 'content/spells.manifest.json').read_bytes())

        def provider(name):
            pin = manifest[name]
            raw = (root / 'content' / pin['path']).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), pin['sha256'])
            return json.loads(raw)['records']

        rat, = [r['profile'] for r in provider('creature_profiles')
                if r['profile']['target']['key'] == native.RAT]
        corpse = rat['data']['profile']['details']['corpse_item']
        self.assertEqual(corpse['family'], 'Item')
        items = provider('item_profiles')
        policies, _ = native.policies(root, creatures={native.RAT})
        self.assertEqual(int(corpse['key'].rsplit('.i', 1)[1]), next(iter(policies)))
        for item, (capacity, duration, successor) in policies.items():
            with self.subTest(item=item):
                record, = [r for r in items if r['production_definition'] == {
                    'family': 'Item', 'production_key': f'oteryn:item.tibia.i{item}',
                    'revision_ref': corpse['revision']}]
                self.assertTrue(record['admission']['materializable'])
                self.assertIn('Ground', record['admission']['legal_destinations'])
                for flag in ('blocks_movement', 'blocks_projectile', 'immovable_block_solid'):
                    self.assertIs(type(record['attributes'][flag]), bool)
                semantics = record['semantics']
                if capacity is not None:
                    self.assertEqual(semantics['container']['value']['capacity']['value'], capacity)
                temporal = semantics['temporal']['value']
                self.assertEqual(temporal['duration']['value'], duration)
                self.assertEqual(temporal['decay_target'], {
                    'state': 'KNOWN', 'value': {
                        'key': f'oteryn:item.tibia.i{successor}', 'revision': corpse['revision']},
                } if successor else {'state': 'NOT_APPLICABLE'})

    def test_portable_native_profiles_remain_exact_and_conflicts_are_atomic(self):
        root = native.ROOT
        profiles = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        generated = {path: (root / path).read_bytes() for path in (profiles, manifest)}
        original = json.loads(generated[profiles])
        keys = {f'oteryn:item.tibia.i{i}' for i in (3990, 4103, 4104, 4143)}
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
        profiles = source.facts(native.ROOT, {3990, 4103, 4104, 4143}, allow_containers=True)
        items = {d['identity']['key']: d for d in definitions()}
        for key, profile in profiles.items():
            with self.subTest(item=key):
                for group in ('physical', 'container'):
                    self.assertEqual(items[key]['semantics'][group], profile['semantics'][group])

    def test_full_decay_closure_keeps_source_weights_and_capacities(self):
        policies, weights = native.policies()
        self.assertEqual(len(policies), 52)
        self.assertEqual({item: policies[item] for item in (7535, 7537, 7538, 7539, 7540)}, {
            7535: (7, 10000, 7537),
            7537: (7, 10000, 7538),
            7538: (7, 300000, 7539),
            7539: (7, 300000, 7540),
            7540: (None, None, None),
        })
        self.assertEqual(policies[5973], (12, 10000, 4025))
        self.assertEqual(policies[5963], (10, 10000, 3990))
        self.assertEqual(policies[4013], (10, 60000, None))
        self.assertEqual([policies[item] for item in (5984, 4062, 4063, 4064)], [
            (56, 10000, 4062), (56, 300000, 4063), (56, 300000, 4064), (None, 60000, None),
        ])
        self.assertEqual(weights[3990], 60000)
        self.assertEqual(weights[4041], 120000)
        self.assertEqual(weights[4013], 85000)
        self.assertEqual(weights[5973], 0)
        self.assertEqual(policies[36847], (25, 10000, 36846))
        self.assertEqual(policies[36846], (25, 300000, 36845))
        self.assertEqual(policies[36845], (25, 300000, 36844))
        self.assertEqual(policies[36844], (None, 60000, None))

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
                                 'oteryn:item.tibia.i4103', 'oteryn:item.tibia.i4104',
                                 'oteryn:item.tibia.i4143'})
            for group, accepted in old.get('semantics', {}).items():
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
