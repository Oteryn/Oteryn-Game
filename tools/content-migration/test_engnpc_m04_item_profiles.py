"""M04 profile publication must retain accepted cores and reject forged facts."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import engnpc_m04_item_profiles as profiles
import quest_reward_item_semantics as rewards


class TerminalItemProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads((profiles.ROOT / profiles.PATH).read_bytes())

    def test_four_exact_profiles_are_reproducible_from_pinned_sources(self):
        self.assertEqual(profiles.rebuild(profiles.ROOT, self.packet['xml_witnesses']),
                         self.packet)
        rows = self.packet['records']
        self.assertEqual([r['production_definition']['production_key'] for r in rows],
                         ['oteryn:item.tibia.i901', 'oteryn:item.tibia.i902', 'oteryn:item.tibia.i3212',
                          'oteryn:item.tibia.i3214'])
        for row in rows:
            self.assertEqual(row['admission'], {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['CharacterInventory', 'Ground'],
            })
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')

    def test_marlin_retains_sealed_definition_and_unknown_source_stack_semantics(self):
        definitions = profiles.definitions(profiles.ROOT)
        marlin = definitions['oteryn:item.tibia.i901']
        before = rewards.canonical(marlin)
        with patch.object(profiles, 'definitions', return_value=definitions):
            profiles.rebuild(profiles.ROOT, self.packet['xml_witnesses'])
        self.assertEqual(rewards.canonical(marlin), before)
        self.assertEqual(hashlib.sha256(before).hexdigest(), profiles.MARLIN_CORE_SHA)
        self.assertEqual(self.packet['records'][0]['semantics']['stack'],
                         {'state': 'UNKNOWN'})
        self.assertEqual(self.packet['records'][0]['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 1800})
        self.assertIsNone(self.packet['records'][2]['attributes']['speed_bonus'])
        self.assertEqual(self.packet['records'][2]['semantics']['skill_modifiers'],
                         {'state': 'UNKNOWN'})

    def test_source_scope_and_each_xml_witness_are_closed(self):
        for item in ('901', '902', '3212', '3214'):
            with self.subTest(item=item):
                changed = dict(self.packet['xml_witnesses'])
                changed[item] += ' '
                with self.assertRaisesRegex(ValueError, 'M04_CRYSTAL_XML_WITNESS_DRIFT'):
                    profiles.rebuild(profiles.ROOT, changed)
        changed = dict(self.packet['xml_witnesses'])
        changed['3031'] = changed['901']
        with self.assertRaisesRegex(ValueError, 'M04_SOURCE_ITEM_SET'):
            profiles.rebuild(profiles.ROOT, changed)

    def test_canonical_marlin_drift_is_not_a_general_conflict_override(self):
        original = profiles.definitions(profiles.ROOT)['oteryn:item.tibia.i901']
        for field, value in [('stack_class', 'StackCapable'), ('materializable', False)]:
            changed = copy.deepcopy(original)
            changed[field] = value
            with self.assertRaisesRegex(ValueError, 'M04_ACCEPTED_MARLIN_CORE_DRIFT'):
                profiles.qualify(901, None, {'flags': {'flags.cumulative': True}},
                                 None, None, changed)

    def publication(self, packet):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        (root / 'content').symlink_to(profiles.ROOT / 'content', target_is_directory=True)
        (root / 'imports').symlink_to(profiles.ROOT / 'imports', target_is_directory=True)
        path = root / profiles.PATH
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(packet))
        return root

    def test_publication_recomputes_every_fact_before_native_merging(self):
        packet = copy.deepcopy(self.packet)
        packet['records'][0]['admission']['stack_class'] = 'StackCapable'
        root = self.publication(packet)
        generated = {'profiles': b'unchanged', 'manifest': b'unchanged'}
        with self.assertRaisesRegex(ValueError, 'M04_ITEM_PROFILE_DRIFT'):
            profiles.admit(root, generated, 'profiles', 'manifest', str, json.dumps)
        self.assertEqual(generated, {'profiles': b'unchanged', 'manifest': b'unchanged'})

    def test_valid_publication_pins_profiles_and_rejects_conflicting_native_owner(self):
        root = self.publication(self.packet)
        carrier = copy.deepcopy(next(row for row in self.packet['records']
                                    if row['production_definition']['production_key'] == profiles.DIARY))
        carrier['admission'] = {'materializable': False, 'stack_class': 'NonStackable',
                                'legal_destinations': ['Ground']}
        carrier['semantics']['readable_writeable'] = copy.deepcopy(profiles.DIARY_TEXT)
        generated = {'profiles': json.dumps({'records': [carrier]}).encode(),
                     'manifest': (profiles.ROOT / 'content/spells.manifest.json').read_bytes()}
        def canonical(value):
            return json.dumps(value).encode()

        def order(row):
            return row['production_definition']['production_key']

        profiles.admit(root, generated, 'profiles', 'manifest', order, canonical)
        published = json.loads(generated['profiles'])['records']
        self.assertEqual(len(published), 4)
        diary = next(row for row in published
                     if row['production_definition']['production_key'] == profiles.DIARY)
        self.assertEqual(diary['semantics']['readable_writeable'], profiles.DIARY_TEXT)
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         hashlib.sha256(generated['profiles']).hexdigest())
        records = copy.deepcopy(self.packet['records'])
        records[0]['admission']['stack_class'] = 'StackCapable'
        records[2] = carrier
        generated['profiles'] = json.dumps({'records': records}).encode()
        before = dict(generated)
        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT:admission'):
            profiles.admit(root, generated, 'profiles', 'manifest', order, canonical)
        self.assertEqual(generated, before)

    def test_diary_promotion_cannot_replace_a_different_text_owner(self):
        root = self.publication(self.packet)
        carrier = copy.deepcopy(next(row for row in self.packet['records']
                                    if row['production_definition']['production_key'] == profiles.DIARY))
        carrier['admission'] = {'materializable': False, 'stack_class': 'NonStackable',
                                'legal_destinations': ['Ground']}
        carrier['semantics']['readable_writeable'] = copy.deepcopy(profiles.DIARY_TEXT)
        carrier['semantics']['readable_writeable']['value']['writeable'] = profiles.shared.source.K(True)
        generated = {'profiles': json.dumps({'records': [carrier]}).encode()}
        with self.assertRaisesRegex(ValueError, 'M04_DIARY_CARRIER_DRIFT'):
            profiles.admit(root, generated, 'profiles', 'manifest', str, json.dumps)

    def generated(self, records):
        return {
            'profiles': profiles.shared.encode({
                'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}),
            'manifest': (profiles.ROOT / 'content/spells.manifest.json').read_bytes(),
        }

    def publish(self, root, generated):
        profiles.admit(root, generated, 'profiles', 'manifest',
                       lambda row: row['production_definition']['production_key'],
                       profiles.shared.encode)

    def legacy_diary(self):
        carrier = copy.deepcopy(next(row for row in self.packet['records']
                                    if row['production_definition']['production_key'] == profiles.DIARY))
        carrier['admission'] = {'materializable': False, 'stack_class': 'NonStackable',
                                'legal_destinations': ['Ground']}
        carrier['semantics']['readable_writeable'] = copy.deepcopy(profiles.DIARY_TEXT)
        return carrier

    def test_own_qualified_output_replays_byte_identically(self):
        root = self.publication(self.packet)
        generated = self.generated([self.legacy_diary()])
        self.publish(root, generated)
        first = dict(generated)
        self.publish(root, generated)
        self.assertEqual(generated, first)
        rows = json.loads(generated['profiles'])['records']
        self.assertEqual({row['production_definition']['production_key'] for row in rows},
                         {row['production_definition']['production_key'] for row in self.packet['records']})
        self.assertEqual(len(rows), 4)
        diary = next(row for row in rows
                     if row['production_definition']['production_key'] == profiles.DIARY)
        self.assertEqual(diary['semantics']['readable_writeable'], profiles.DIARY_TEXT)

    def test_source_qualified_qengine_prefix_is_preserved(self):
        root = self.publication(self.packet)
        # The earlier QENGINE owner selects i3212 through questl2_passages.
        # Use its real source qualification, without running full registration.
        prefix = profiles.merger.capture(profiles.ROOT, item_ids={3212})
        self.assertEqual([row['production_definition']['production_key']
                          for row in prefix['records']], [profiles.DIARY])
        diary = prefix['records'][0]
        self.assertEqual(diary['semantics']['readable_writeable'], profiles.DIARY_TEXT)
        generated = self.generated(copy.deepcopy(prefix['records']))
        self.publish(root, generated)
        rows = json.loads(generated['profiles'])['records']
        self.assertEqual(len(rows), 4)
        self.assertEqual(next(row for row in rows
                              if row['production_definition']['production_key'] == profiles.DIARY), diary)
        for expected in self.packet['records']:
            key = expected['production_definition']['production_key']
            if key != profiles.DIARY:
                self.assertEqual(next(row for row in rows
                                      if row['production_definition']['production_key'] == key), expected)
        first = dict(generated)
        self.publish(root, generated)
        self.assertEqual(generated, first)

    def test_promoted_diary_refusals_leave_every_generated_byte_unchanged(self):
        root = self.publication(self.packet)
        original = self.generated([self.legacy_diary()])
        self.publish(root, original)
        rows = json.loads(original['profiles'])['records']

        def diary(records):
            return next(row for row in records
                        if row['production_definition']['production_key'] == profiles.DIARY)

        def wrong_text(records):
            diary(records)['semantics']['readable_writeable']['value']['writeable'] = profiles.shared.source.K(True)

        def wrong_admission(records):
            diary(records)['admission']['legal_destinations'] = ['CharacterInventory']

        def duplicate(records):
            records.append(copy.deepcopy(diary(records)))

        def missing(records):
            records.remove(diary(records))

        def wrong_binding(records):
            diary(records)['production_binding']['target']['revision'] = 'unqualified-revision'

        def wrong_physical(records):
            diary(records)['semantics']['physical']['value']['weight'] = profiles.shared.source.K(1301)

        def wrong_other_owner(records):
            next(row for row in records if row['production_definition']['production_key']
                 == 'oteryn:item.tibia.i901')['admission']['stack_class'] = 'StackCapable'

        for mutate, error in (
            (wrong_text, 'M04_DIARY_CARRIER_DRIFT'),
            (wrong_admission, 'M04_DIARY_CARRIER_DRIFT'),
            (duplicate, 'M04_DIARY_CARRIER_DRIFT'),
            (missing, 'M04_DIARY_CARRIER_DRIFT'),
            (wrong_binding, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT:production_binding'),
            (wrong_physical, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT:semantics'),
            (wrong_other_owner, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT:admission'),
        ):
            with self.subTest(mutation=mutate.__name__):
                changed = copy.deepcopy(rows)
                mutate(changed)
                generated = self.generated(changed)
                before = dict(generated)
                with self.assertRaisesRegex(ValueError, error):
                    self.publish(root, generated)
                self.assertEqual(generated, before)


if __name__ == '__main__':
    unittest.main()
