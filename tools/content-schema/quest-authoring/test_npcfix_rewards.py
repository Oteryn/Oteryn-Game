"""Source closure must fill missing lifetime facts without rewriting other owners."""
import copy
import json
import sys
import unittest

import qengine_rewards as q

sys.path.insert(0, str(q.ROOT / 'tools/content-migration'))
import qengine_reward_item_profiles as items
import register_spell_families as native


class NpcfixRewardProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs = q.build()

    def profiles(self):
        outputs = copy.deepcopy(self.outputs)
        profiles = json.loads(outputs[items.PROFILE])
        pick = next(row for row in profiles['records']
                    if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3456')
        return outputs, profiles, pick

    def test_lifetime_enrichment_retains_every_other_existing_pick_fact(self):
        outputs, profiles, pick = self.profiles()
        pick['semantics']['temporal'] = {'state': 'UNKNOWN'}
        before = copy.deepcopy(pick)
        outputs[items.PROFILE] = q.encoded(profiles)
        items.admit(q.ROOT, outputs, items.PROFILE, 'content/spells.manifest.json',
                    native.provider_order, q.encoded)
        after = next(row for row in json.loads(outputs[items.PROFILE])['records']
                     if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3456')
        self.assertEqual(after['semantics']['temporal'], {'state': 'NOT_APPLICABLE'})
        after['semantics']['temporal'] = before['semantics']['temporal']
        self.assertEqual(after, before)

    def test_workshop_items_use_source_facts_without_replacing_the_existing_bag(self):
        packet = json.loads(self.outputs[items.PROOF])
        records = {r['production_definition']['production_key']: r for r in packet['records']}
        for number in (43854, 43855):
            row = records[f'oteryn:item.tibia.i{number}']
            self.assertEqual(row['semantics']['physical']['value']['weight']['value'], 100)
        self.assertNotIn('oteryn:item.tibia.i43895', records)
        self.assertIn('00ce02a57ca5a12e48f32a3476e37471167e4c3f', packet['supplement']['url'])
        callbacks = json.loads(self.outputs['content/quests/shared-reward-callbacks.json'])['callbacks']
        by_key = {c['key']: c for c in callbacks}
        altar = by_key['oteryn:quest-reward/npcfix/altar-bag']
        self.assertEqual(altar['fee'], {'gold': 50_000_000, 'include_bank': True})
        self.assertEqual(altar['take'], [{'item':'oteryn:item.tibia.i43855','count':5},
                                        {'item':'oteryn:item.tibia.i43854','count':5}])
        submit = by_key['oteryn:quest-reward/npcfix/koshei-submit']
        claim = by_key['oteryn:quest-reward/npcfix/koshei-claim']
        self.assertIsNone(submit['give'])
        self.assertEqual(submit['fee']['gold'], 5000)
        self.assertTrue(submit['clocks'][0]['set_now'])
        self.assertEqual(claim['clocks'][0]['track'], submit['clocks'][0]['track'])
        self.assertEqual(claim['clocks'][0]['elapsed_seconds'], 86400)
        self.assertFalse(claim['clocks'][0]['set_now'])

    def test_existing_known_weight_conflict_is_never_overridden(self):
        outputs, profiles, pick = self.profiles()
        pick['semantics']['physical']['value']['weight']['value'] += 1
        outputs[items.PROFILE] = q.encoded(profiles)
        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
            items.admit(q.ROOT, outputs, items.PROFILE, 'content/spells.manifest.json',
                        native.provider_order, q.encoded)

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('npcfix_rewards', HERE / 'npcfix_rewards.py')
PRODUCER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PRODUCER)


class NpcfixRewardAuthoring(unittest.TestCase):
    def fixture(self, root):
        schema = root / 'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json'
        schema.parent.mkdir(parents=True)
        schema.write_bytes((HERE / 'shared-reward-callbacks.schema.json').read_bytes())

    def test_fresh_checkout_materializes_only_the_owned_source_callback(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            outputs = PRODUCER.outputs(root)
            callbacks = json.loads(outputs[root / 'content/quests/shared-reward-callbacks.json'])['callbacks']
            by_npc = {row['npc']: row for row in callbacks}
            self.assertEqual(len(by_npc), 1)
            self.assertEqual(by_npc['oteryn:npc.a_fading_memory']['cooldown']['seconds'], 72_000)
            self.assertEqual(by_npc['oteryn:npc.a_fading_memory']['give']['item'], 'oteryn:item.tibia.i8746')
            self.assertEqual(by_npc['oteryn:npc.a_fading_memory']['storage'][0]['maximum'], 1)

    def test_reproduction_preserves_unrelated_lane_records(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            path = root / 'content/quests/shared-reward-callbacks.json'
            path.parent.mkdir(parents=True)
            other = json.loads(PRODUCER.INPUT.read_text())['reference_callbacks'][0]
            other['key'] = 'oteryn:quest-reward/another-lane'
            other['npc'] = 'oteryn:npc.another_lane'
            path.write_text(json.dumps({'schema': 1, 'callbacks': [other]}))
            first = PRODUCER.outputs(root)
            self.assertEqual(json.loads(first[path])['callbacks'][0], other)
            for target, content in first.items():
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(content)
            self.assertEqual(first, PRODUCER.outputs(root))


    def test_shared_generation_retains_deadlines_and_achievement_once(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            callbacks = json.loads((HERE.parents[2] / 'content/quests/shared-reward-callbacks.json').read_text())['callbacks']
            soul = next(row for row in callbacks if row['npc'] == 'oteryn:npc.a_wandering_soul')
            path = 'content/quests/shared-reward-callbacks.json'
            generated = {path: json.dumps({'schema': 1, 'callbacks': [soul]}).encode(), 'foreign-provider': b'unchanged'}
            result = PRODUCER.overlay_generated(generated, root)
            document = json.loads(result[path])
            self.assertEqual(document['callbacks'][0], soul)
            self.assertTrue(soul['achievement_once'])
            self.assertEqual(soul['give']['decay']['duration_millis'], 604_800_000)
            self.assertEqual(document['callbacks'][1]['cooldown']['seconds'], 72_000)
            self.assertEqual(len(document['tracks']), 1)
            self.assertEqual(result['foreign-provider'], b'unchanged')


if __name__ == '__main__':
    unittest.main()
