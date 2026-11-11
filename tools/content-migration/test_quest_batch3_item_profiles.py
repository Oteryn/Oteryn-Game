"""Bounds and source custody checks for the batch-three physical Item cohort."""
import json
import hashlib
import copy
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path
from unittest.mock import patch

import quest_batch3_item_profiles as profiles


class QuestBatchThreeItemProfiles(unittest.TestCase):
    def test_recipe_admission_is_bounded_and_retains_state_holds(self):
        generated = {'profiles': json.dumps({'records': []}).encode(),
                     'manifest': json.dumps({'item_profiles': {}}).encode()}
        profiles.admit_recipes(profiles.ROOT, generated, 'profiles', 'manifest',
                               lambda row: row['production_definition']['production_key'],
                               profiles.grizzly.encode)
        admitted = {row['production_definition']['production_key']
                    for row in json.loads(generated['profiles'])['records']}
        self.assertIn('oteryn:item.tibia.i3595', admitted)
        self.assertNotIn('oteryn:item.tibia.i4363', admitted)
        self.assertNotIn('oteryn:item.tibia.i2874', admitted)
        self.assertNotIn('oteryn:item.tibia.i11701', admitted)

    def test_recipe_admission_does_not_replace_conflicting_native_facts(self):
        packet = json.loads((profiles.ROOT / profiles.PATH).read_bytes())
        existing = copy.deepcopy(next(row for row in packet['records']
                             if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3595'))
        existing['semantics']['stack']['value']['stackable']['value'] = False
        generated = {'profiles': json.dumps({'records': [existing]}).encode(),
                     'manifest': json.dumps({'item_profiles': {}}).encode()}
        with self.assertRaisesRegex(ValueError, 'NPC_RECIPE_EXISTING_PROFILE_CONFLICT'):
            profiles.admit_recipes(profiles.ROOT, generated, 'profiles', 'manifest',
                                   lambda row: row['production_definition']['production_key'],
                                   profiles.grizzly.encode)

    def fixture(self):
        return (6093, ET.fromstring('<item id="6093" name="fixture"><attribute key="weight" value="50"/></item>'),
                {'flags': {'flags.take': True}}, {'movable': True, 'pickupable': True}, {}, {})

    def test_plain_portable_item_uses_existing_exact_binding_qualifier(self):
        values = self.fixture()
        with patch.object(profiles.grizzly, 'qualify', return_value={'proof': 'source'}) as qualifier:
            self.assertEqual(profiles.qualify(*values), {'proof': 'source'})
            qualifier.assert_called_once_with(*values)

    def test_fluid_never_reaches_plain_item_qualification(self):
        for flag in ('flags.liquidcontainer', 'flags.liquidpool'):
            values = list(self.fixture())
            values[2]['flags'][flag] = True
            with patch.object(profiles.grizzly, 'qualify') as qualifier:
                with self.assertRaisesRegex(ValueError, 'FLUID_SUBTYPE_OWNER_REQUIRED'):
                    profiles.qualify(*values)
                qualifier.assert_not_called()

    def test_source_charge_and_decay_keep_their_existing_owner(self):
        for key, value in (('charges', '5'), ('duration', '60'), ('decayto', '3995')):
            values = list(self.fixture())
            ET.SubElement(values[1], 'attribute', {'key': key, 'value': value})
            with patch.object(profiles.grizzly, 'qualify') as qualifier:
                with self.assertRaisesRegex(ValueError, 'INSTANCE_STATE_OWNER_REQUIRED'):
                    profiles.qualify(*values)
                qualifier.assert_not_called()

    def test_accepted_semantic_hold_precedes_all_new_source_qualification(self):
        values = self.fixture()
        with patch.object(profiles.grizzly, 'qualify') as qualifier:
            with self.assertRaisesRegex(ValueError, 'ACCEPTED_SEMANTIC_HOLD:partial charge'):
                profiles.qualify(*values, hold_reasons=['partial charge'])
            qualifier.assert_not_called()

    def test_xml_line_binding_handles_single_ids_and_explicit_ranges(self):
        self.assertEqual(profiles.xml_lines(b'<items>\n<item id="6093"/>\n<item fromid="6105" toid="6106"/>\n</items>'),
                         {6093: 2, 6105: 3, 6106: 3})

    def test_only_explicit_canonical_identities_enter_item_set(self):
        document = {'sources': ['pinned.lua:10'], 'item': 'oteryn:item.tibia.i6093',
                    'reply': 'Bring item 6095.', 'position': {'x': 6096}}
        self.assertEqual(list(profiles.item_references(document)),
                         [('oteryn:item.tibia.i6093', '/item', ['pinned.lua:10'])])

    def test_retained_uniform_choice_items_are_facts_without_runtime_admission(self):
        document = {'sources': ['pinned.lua:74'], 'kind': 'uniform_choice',
                    'items_by_choice': [5926, 6098, 6097, 6126], 'minimum': 1, 'maximum': 4}
        keys = {key for key, _, _ in profiles.item_references(document)}
        self.assertEqual(keys, {'oteryn:item.tibia.i5926', 'oteryn:item.tibia.i6098',
                                'oteryn:item.tibia.i6097', 'oteryn:item.tibia.i6126'})

    def test_only_published_authoring_can_enter_item_inputs(self):
        approved = 'tools/content-schema/quest-authoring/batch3_b/npc/the_shattered_isles_quest.json'
        withdrawn = 'tools/content-schema/quest-authoring/batch3_c/boss_rooms.json'
        transient = 'tools/content-schema/quest-authoring/batch3_c/npc/temporary_review.json'
        with tempfile.TemporaryDirectory(prefix='quest3-item-scope-') as directory:
            root = Path(directory)
            for relative, item_id in ((approved, 6093), (withdrawn, 59999), (transient, 60000)):
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(json.dumps({'item': f'oteryn:item.tibia.i{item_id}',
                                            'sources': ['pinned.lua:10']}))
            producer = root / 'tools/content-migration/quest_batch3_item_profiles.py'
            producer.parent.mkdir(parents=True)
            producer.write_text('source qualifier fixture')
            with patch.object(profiles, '__file__', str(producer)):
                references, inputs = profiles.selected(root)
            self.assertIn('oteryn:item.tibia.i6093', references)
            self.assertNotIn('oteryn:item.tibia.i59999', references)
            self.assertNotIn('oteryn:item.tibia.i60000', references)
            paths = {row['path'] for row in inputs}
            self.assertIn(approved, paths)
            self.assertNotIn(withdrawn, paths)
            self.assertNotIn(transient, paths)


class RetainedQn3CensusTests(unittest.TestCase):
    KEY = 'oteryn:item.tibia.i27654'
    SOURCE = 'content/dialogues/quest-rules/quest-batch3-qn3-dangerous_depths.json'

    def test_retained_qn3_source_and_jar_match_all_existing_owners(self):
        packet = json.loads((profiles.ROOT / profiles.PATH).read_bytes())
        source = (profiles.ROOT / self.SOURCE).read_bytes()
        descriptor = {'path': self.SOURCE, 'sha256': hashlib.sha256(source).hexdigest()}
        self.assertEqual([row for row in packet['authoring_inputs']
                          if row['path'] == self.SOURCE], [descriptor])
        references, inputs = profiles.selected(profiles.ROOT)
        self.assertIn(descriptor, inputs)
        self.assertIn(self.KEY, references)
        classified = [row for row in packet['classification'] if row['key'] == self.KEY]
        self.assertEqual(len(classified), 1)
        self.assertEqual(classified[0]['uses'], references[self.KEY])
        self.assertEqual(classified[0]['classification'], 'EXISTING_NATIVE')
        self.assertTrue(classified[0]['existing_native_profile'])
        self.assertEqual(classified[0]['holds'], [])
        self.assertIsNone(classified[0]['reason'])
        rows = [row for row in packet['records']
                if row['production_definition']['production_key'] == self.KEY]
        self.assertEqual(len(rows), 1)
        for path in (
            profiles.NATIVE,
            'tools/content-schema/native-gameplay/engnpc-m05-item-profiles.json',
            'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json',
        ):
            with self.subTest(owner=path):
                owner = json.loads((profiles.ROOT / path).read_bytes())
                self.assertEqual([row for row in owner['records']
                                  if row['production_definition']['production_key'] == self.KEY], rows)
        self.assertFalse(packet['runtime_enabled'])

    def test_retained_census_does_not_add_a_recipe_item_owner(self):
        generated = {'profiles': json.dumps({'records': []}).encode(),
                     'manifest': json.dumps({'item_profiles': {}}).encode()}
        profiles.admit_recipes(profiles.ROOT, generated, 'profiles', 'manifest',
                              lambda row: row['production_definition']['production_key'],
                              profiles.grizzly.encode)
        admitted = {row['production_definition']['production_key']
                    for row in json.loads(generated['profiles'])['records']}
        self.assertIn('oteryn:item.tibia.i3595', admitted)
        self.assertNotIn(self.KEY, admitted)


if __name__ == '__main__':
    unittest.main()
