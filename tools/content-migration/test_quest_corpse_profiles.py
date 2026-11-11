"""Bounded source qualification, exact capacities and preserved accepted Item owners."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET
from unittest.mock import patch

import quest_corpse_profiles as corpse


class CorpseQualification(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan = corpse.qualifications()
        raw = (corpse.ROOT / corpse.XML).read_bytes()
        cls.xml = corpse.grizzly.source.xml_index(raw)
        client = (corpse.ROOT / corpse.custody.CLIENT).read_bytes()
        cls.appearances = corpse.grizzly.source.engine_items.load_appearance_objects(client)
        cls.policies = {row['server_item_id']: row for row in corpse.grizzly.source.policies(
            client, raw, {int(key.rsplit('i', 1)[1]) for key in cls.plan.records})}
        cls.definitions = corpse.definitions(corpse.ROOT, cls.policies)
        cls.world = corpse.world_objects(corpse.ROOT, cls.policies)
        cls.bindings = corpse.grizzly.source.binding_index(
            (corpse.ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())

    def qualify(self, number, *, definition=None, world=None, appearance=None, bindings=None, fresh=True):
        return corpse.qualify(number, definition or self.definitions[f'oteryn:item.tibia.i{number}'],
                              self.world.get(number) if world is None else world,
                              self.xml[number], appearance or self.appearances[number], self.policies[number],
                              bindings or self.bindings, fresh)

    def test_native_goal_decay_tail_retains_its_non_container_policy(self):
        row = self.qualify(4027, fresh=False)
        self.assertEqual(row['semantics']['container'], corpse.grizzly.source.NA)
        node = copy.deepcopy(self.xml[4027])
        ET.SubElement(node, 'attribute', {'key': 'containerSize', 'value': '8'})
        with self.assertRaisesRegex(ValueError, 'QUEST_CORPSE_CANONICAL_CONTAINER'):
            corpse.qualify(4027, self.definitions['oteryn:item.tibia.i4027'],
                           self.world[4027], node, self.appearances[4027],
                           self.policies[4027], self.bindings, False)

    def test_native_corym_corpse_keeps_exact_source_decay_and_capacity(self):
        root = self.plan.records['oteryn:item.tibia.i17449']
        terminal = self.plan.records['oteryn:item.tibia.i17450']
        self.assertEqual(root['semantics']['container'], corpse.grizzly.source.K({'capacity': corpse.grizzly.source.K(5)}))
        self.assertEqual(root['semantics']['temporal']['value']['duration'], corpse.grizzly.source.K(300000))
        self.assertEqual(terminal['semantics']['temporal']['value']['duration'], corpse.grizzly.source.K(60000))
        self.assertEqual(root['admission']['legal_destinations'], ['Ground'])

    def test_published_loot_admission_matches_requested_corpse_roots(self):
        section = json.loads((corpse.ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        facts = {row['item']['key']: row for row in section['items']}
        for number in self.plan.roots:
            key = f'oteryn:item.tibia.i{number}'
            expected = self.plan.records[key]['semantics']['container']['value']['capacity']['value']
            self.assertTrue(facts[key]['materializable'], key)
            self.assertEqual(facts[key]['container_capacity'], expected, key)

    def test_exact_source_capacities_and_complete_decay_chains(self):
        self.assertEqual(self.plan.roots, {5960, 5962, 5968, 5973, 5975, 5995, 6006, 17449, 30272})
        self.assertEqual(len(self.plan.records), 31)
        for number, capacity, target in [(5960, 7, 3987), (5962, 30, 3989),
                                         (5968, 5, 4007), (5975, 20, 4291)]:
            row = self.plan.records[f'oteryn:item.tibia.i{number}']
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
            self.assertEqual(row['semantics']['container'], corpse.grizzly.source.K(
                {'capacity': corpse.grizzly.source.K(capacity)}))
            temporal = row['semantics']['temporal']['value']
            self.assertEqual(temporal['duration'], corpse.grizzly.source.K(10000))
            self.assertEqual(temporal['decay_target'], corpse.grizzly.source.K(
                {'key': f'oteryn:item.tibia.i{target}', 'revision': 'definition-r1'}))
            self.assertIn(f'oteryn:item.tibia.i{target}', self.plan.records)
        # These are distinct source facts, not a clone of the Rat or a capacity30 template.
        self.assertEqual(self.plan.records['oteryn:item.tibia.i4009']['semantics']['container'],
                         corpse.grizzly.source.NA)
        self.assertEqual(self.plan.records['oteryn:item.tibia.i4292']['semantics']['container'],
                         corpse.grizzly.source.K({'capacity': corpse.grizzly.source.K(20)}))

    def test_queen_projection_retains_source40_and_native30_without_changing_other_corpses(self):
        key = 'oteryn:item.tibia.i30272'
        self.assertEqual(corpse.grizzly.source.attrs(self.xml[30272])['containersize'], '40')
        self.assertEqual(self.plan.projected_source_capacities, {key: 40})
        self.assertEqual(self.plan.records[key]['semantics']['container'],
                         corpse.grizzly.source.K({'capacity': corpse.grizzly.source.K(30)}))
        packet = json.loads((corpse.ROOT / corpse.REQUIREMENTS).read_bytes())
        for field, value in [('source_capacity', 41), ('native_capacity', 31)]:
            bad = copy.deepcopy(packet)
            bad['capacity_projections'][0][field] = value
            with patch.object(Path, 'read_bytes', return_value=json.dumps(bad).encode()):
                with self.assertRaisesRegex(ValueError, 'PROJECTION_BINDING'):
                    corpse.projection_limits(corpse.ROOT, self.plan.roots, self.xml)

    def test_preserves_canonical_data_and_every_unrelated_semantic_group(self):
        before = copy.deepcopy(self.definitions)
        row = self.qualify(5960)
        self.assertEqual(before, self.definitions)
        definition = self.definitions['oteryn:item.tibia.i5960']
        self.assertFalse(definition['materializable'])
        for group in set(row['semantics']) - set(corpse.GROUPS):
            self.assertEqual(row['semantics'][group], definition.get('semantics', {}).get(
                group, corpse.grizzly.source.U))

    def test_rejects_source_kind_binding_and_accepted_fact_substitutions(self):
        for mutation in ['corpse', 'player', 'container', 'pointer', 'movable', 'binding', 'capacity', 'stack']:
            world = copy.deepcopy(self.world[5960])
            appearance = copy.deepcopy(self.appearances[5960])
            definition = copy.deepcopy(self.definitions['oteryn:item.tibia.i5960'])
            bindings = copy.deepcopy(self.bindings)
            if mutation == 'corpse':
                appearance['flags']['flags.corpse'] = False
            elif mutation == 'player':
                appearance['flags']['flags.player_corpse'] = True
            elif mutation == 'container':
                appearance['flags']['flags.container'] = False
            elif mutation == 'pointer':
                world['provenance']['item_pointer']['revision'] = 'definition-r2'
            elif mutation == 'movable':
                world['movable'] = corpse.grizzly.source.K(True)
            elif mutation == 'binding':
                bindings['5960']['disposition'] = 'HYPOTHESIS'
            elif mutation == 'capacity':
                definition.setdefault('semantics', {})['container'] = corpse.grizzly.source.K(
                    {'capacity': corpse.grizzly.source.K(30)})
            else:
                definition['stack_class'] = 'StackCapable'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.qualify(5960, definition=definition, world=world,
                             appearance=appearance, bindings=bindings)

    def test_only_exact_preexisting_item_containers_may_bridge_missing_decay_worldobjects(self):
        self.assertNotIn(4291, self.world)
        self.qualify(4291, fresh=False)
        with self.assertRaises(ValueError):
            self.qualify(4291, fresh=True)
        altered = copy.deepcopy(self.definitions['oteryn:item.tibia.i4291'])
        altered['semantics']['container'] = corpse.grizzly.source.U
        with self.assertRaises(ValueError):
            self.qualify(4291, definition=altered, fresh=False)

    def test_xml_pin_cycle_and_unowned_source_system_fail_closed(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda path: b'<items/>' if path == corpse.ROOT / corpse.XML
                          else original(path)), self.assertRaisesRegex(ValueError, 'XML_PIN'):
            corpse.qualifications()
        xml = copy.deepcopy(self.xml)
        for attribute in xml[5960]:
            if attribute.get('key').lower() == 'decayto':
                attribute.set('value', '5960')
        with self.assertRaisesRegex(ValueError, 'DECAY_CLOSURE'):
            corpse.decay_closure({5960}, xml)
        xml = copy.deepcopy(self.xml)
        ET.SubElement(xml[5960], 'attribute', {'key': 'charges', 'value': '1'})
        with self.assertRaisesRegex(ValueError, 'UNSUPPORTED_SOURCE_SYSTEM'):
            corpse.decay_closure({5960}, xml)

    def test_loot_projection_has_exact_capacities_and_is_atomic_on_conflict(self):
        facts = {key: {'item': {'family': row['production_definition']['family'],
                                     'key': row['production_definition']['production_key'],
                                     'revision': row['production_definition']['revision_ref']}, 'materializable': False,
                       'container_capacity': None} for key, row in self.plan.records.items()}
        with patch.object(corpse, 'qualifications', return_value=self.plan):
            corpse.apply_loot_facts(facts)
            for key, row in self.plan.records.items():
                self.assertTrue(facts[key]['materializable'])
                container = row['semantics']['container']
                self.assertEqual(facts[key]['container_capacity'],
                                 container['value']['capacity']['value'] if container['state'] == 'KNOWN' else None)
            invalid = {key: {'item': {'family': row['production_definition']['family'],
                                       'key': row['production_definition']['production_key'],
                                       'revision': row['production_definition']['revision_ref']}, 'materializable': False,
                             'container_capacity': None} for key, row in self.plan.records.items()}
            invalid['oteryn:item.tibia.i5975']['container_capacity'] = 30
            before = copy.deepcopy(invalid)
            with self.assertRaisesRegex(ValueError, 'CAPACITY_CONFLICT'):
                corpse.apply_loot_facts(invalid)
            self.assertEqual(invalid, before)

    def test_real_loot_generator_facts_use_canonical_identity_not_candidate_provenance(self):
        import build_full_spell_artifact
        # Exercise the actual production item-fact builder and qualifier together.
        # Candidate authoring refs are source provenance, never canonical Item identities.
        facts = build_full_spell_artifact.item_facts()
        for key, row in self.plan.records.items():
            definition = row['production_definition']
            expected = {'family': definition['family'], 'key': definition['production_key'],
                        'revision': definition['revision_ref']}
            self.assertEqual(facts[key]['item'], expected)
            self.assertNotEqual(facts[key]['item'], row['authoring']['item'])
        before = copy.deepcopy(facts)
        corpse.apply_loot_facts(facts)
        for key, row in self.plan.records.items():
            self.assertEqual(facts[key]['item'], before[key]['item'])
            self.assertTrue(facts[key]['materializable'])
            container = row['semantics']['container']
            expected = container['value']['capacity']['value'] if container['state'] == 'KNOWN' else None
            self.assertEqual(facts[key]['container_capacity'], expected)
        self.assertEqual({key: value for key, value in facts.items() if key not in self.plan.records},
                         {key: value for key, value in before.items() if key not in self.plan.records})

    def test_native_overlay_is_idempotent_and_preserves_existing_profiles(self):
        profile = 'content/items/definitions/spell-native-profiles.json'
        manifest = 'content/spells.manifest.json'
        generated = {profile: (corpse.ROOT / profile).read_bytes(),
                     manifest: (corpse.ROOT / manifest).read_bytes()}
        before = {row['production_definition']['production_key']: row
                  for row in json.loads(generated[profile])['records']}
        canonical = lambda value: json.dumps(value, sort_keys=True).encode()
        with patch.object(corpse, 'qualifications', return_value=self.plan):
            corpse.admit(corpse.ROOT, generated, profile, manifest,
                         lambda row: row['production_definition']['production_key'], canonical)
            once = copy.deepcopy(generated)
            corpse.admit(corpse.ROOT, generated, profile, manifest,
                         lambda row: row['production_definition']['production_key'], canonical)
        self.assertEqual(generated, once)
        after = {row['production_definition']['production_key']: row
                 for row in json.loads(generated[profile])['records']}
        self.assertEqual({key: after[key] for key in before}, before)
        self.assertEqual(set(after) - set(before), set(self.plan.records) - set(before))

class BossQuestAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan = corpse.qualifications()

    def test_boss_then_quest_admission_preserves_the_qualified_terminal_and_replays(self):
        import boss_corpse_profiles as boss
        generated = {'profiles': boss.encode({'records': []}),
                     'manifest': boss.encode({'item_profiles': {}})}
        order = lambda row: row['production_definition']['production_key']

        def admit(generated):
            # Reuse this class's actual source-qualified plan at the admission
            # seam; parsing the same immutable inputs again is not the assertion.
            with patch.object(corpse, 'qualifications', return_value=self.plan):
                corpse.admit(corpse.ROOT, generated, 'profiles', 'manifest', order, boss.encode)

        boss.admit(corpse.ROOT, generated, 'profiles', 'manifest', order, boss.encode)
        before = {row['production_definition']['production_key']: row
                  for row in json.loads(generated['profiles'])['records']}
        admit(generated)
        once = copy.deepcopy(generated)
        admit(generated)
        self.assertEqual(generated, once)
        after = {row['production_definition']['production_key']: row
                 for row in json.loads(generated['profiles'])['records']}
        self.assertEqual({key: after[key] for key in before}, before)
        self.assertEqual(set(after), set(before) | set(self.plan.records))
        self.assertEqual(after['oteryn:item.tibia.i4027']['semantics']['container'],
                         {'state': 'NOT_APPLICABLE'})

        # The later owner still rejects a real disagreement without publishing
        # any partial profile or manifest change.
        conflict = copy.deepcopy(generated)
        document = json.loads(conflict['profiles'])
        terminal = next(row for row in document['records']
                        if row['production_definition']['production_key'] == 'oteryn:item.tibia.i4027')
        terminal['semantics']['container'] = corpse.grizzly.source.K(
            {'capacity': corpse.grizzly.source.K(8)})
        conflict['profiles'] = boss.encode(document)
        refused = copy.deepcopy(conflict)
        with self.assertRaisesRegex(ValueError, 'QUEST_CORPSE_EXISTING_NATIVE_CONFLICT'):
            admit(conflict)
        self.assertEqual(conflict, refused)


class SelectionBounds(unittest.TestCase):
    def test_active_rule_objective_creature_join_and_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'content/quests').mkdir(parents=True)
            (root / corpse.REQUIREMENTS).parent.mkdir(parents=True)
            (root / corpse.REQUIREMENTS).write_text(json.dumps({'schema': 1, 'objectives': [], 'sources': []}))
            objectives = {'schema': 1, 'objectives': [{'key': 'oteryn:quest-death/animal',
                                                     'creature': 'oteryn:creature.troll'}]}
            rules = {'schema': 1, 'rules': [{'objective': 'oteryn:quest-death/animal'}]}
            (root / corpse.OBJECTIVES).write_text(json.dumps(objectives))
            creatures = {'records': [{'profile': {'target': {'family': 'Creature',
                          'key': 'oteryn:creature.troll', 'revision': 'definition-r1'},
                          'data': {'profile': {'details': {'corpse_item': {'family': 'Item',
                          'key': 'oteryn:item.tibia.i5960', 'revision': 'definition-r1'}}}}}}]}
            (root / corpse.RULES).write_text(json.dumps(rules))
            self.assertEqual(corpse.required(root, creatures), {5960})
            # Ordinary native rewards request the same exact objective/Creature/corpse
            # without inventing a quest-loot rule or changing its rewards.
            (root / corpse.RULES).write_text(json.dumps({'schema': 1, 'rules': []}))
            request = {'schema': 1, 'objectives': ['oteryn:quest-death/animal'], 'sources': ['fixture:bound-source']}
            (root / corpse.REQUIREMENTS).write_text(json.dumps(request))
            self.assertEqual(corpse.required(root, creatures), {5960})
            request['objectives'] = ['oteryn:quest-death/foreign']
            (root / corpse.REQUIREMENTS).write_text(json.dumps(request))
            with self.assertRaisesRegex(ValueError, 'ORDINARY_OBJECTIVE_BINDING'):
                corpse.required(root, creatures)
            (root / corpse.REQUIREMENTS).write_text(json.dumps({'schema': 1, 'objectives': [], 'sources': []}))
            rules['rules'].append(rules['rules'][0])
            (root / corpse.RULES).write_text(json.dumps(rules))
            with self.assertRaisesRegex(ValueError, 'OBJECTIVE_BINDING'):
                corpse.required(root, creatures)
            rules['rules'] *= 17
            (root / corpse.RULES).write_text(json.dumps(rules))
            with self.assertRaisesRegex(ValueError, 'RULE_BOUNDS'):
                corpse.required(root, creatures)


if __name__ == '__main__':
    unittest.main()
