"""Sugar Daddy's physical body and complete existing loot table have native custody."""
import copy
import hashlib
import json
import sys
import unittest

import boss_reward_item_profiles as boss
sys.path.insert(0, str(boss.ROOT / 'tools/content-schema/native-gameplay'))
from build_full_spell_artifact import native_corpse_capacity, native_loot_yield_facts


class BossRewardItemProfiles(unittest.TestCase):
    def test_closed_source_capture_covers_real_corpse_and_every_existing_loot_yield(self):
        raw = (boss.ROOT / boss.PATH).read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), boss.CAPTURE_SHA256)
        doc = json.loads(raw)
        table, ids = boss.selected(boss.ROOT)
        self.assertEqual(doc['item_ids'], sorted(ids))
        self.assertEqual(len(ids), 22)
        self.assertEqual(doc['loot_sha256'], hashlib.sha256(boss.canonical(table)).hexdigest())
        for record in doc['records']:
            identity = record['production_binding']['target']
            if identity['key'] == 'oteryn:item.tibia.i48416':
                self.assertEqual(record['production_binding'], boss.body_binding())
                self.assertEqual(native_corpse_capacity(identity, record), 8)
                self.assertIsNone(native_loot_yield_facts(identity, record),
                    'Source Ground corpse admission cannot authorize carried loot')
            else:
                self.assertIsNotNone(native_loot_yield_facts(identity, record), identity['key'])

    def test_production_native_provider_and_loot_section_close_the_boss_chain(self):
        _, ids = boss.selected(boss.ROOT)
        native = json.loads((boss.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        selected = {r['production_definition']['production_key']: r for r in native['records']}
        loot = json.loads((boss.ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        facts = {r['item']['key']: r for r in loot['items']}
        for item in ids:
            key = f'oteryn:item.tibia.i{item}'
            self.assertIn(key, selected)
            self.assertTrue(facts[key]['materializable'], key)
            identity = selected[key]['production_binding']['target']
            if item == boss.BODY:
                self.assertEqual(native_corpse_capacity(identity, selected[key]), 8)
                self.assertEqual(facts[key]['container_capacity'], 8)
            else:
                self.assertIsNotNone(native_loot_yield_facts(identity, selected[key]), key)
        binding, = [r for r in loot['creature_loot'] if r['creature']['key'] == boss.BOSS]
        self.assertEqual(binding['loot']['key'], 'oteryn:loot.creature.sugar_daddy')

    def test_existing_quiver_gains_unknown_custody_facts_without_losing_owner_patterns(self):
        overlay = json.loads((boss.ROOT / boss.PATH).read_bytes())
        record, = [copy.deepcopy(r) for r in overlay['records'] if r['production_definition']['production_key'] == 'oteryn:item.tibia.i45644']
        record['semantics']['classification'] = {'state':'UNKNOWN'}
        record['semantics']['physical']['value']['movable'] = {'state':'UNKNOWN'}
        patterns = record['semantics']['equipment']['value']['patterns']['value']
        second = copy.deepcopy(patterns[0])
        second['pattern_id'] = 2
        second['primary_slot'] = boss.source.K('EXTRA')
        patterns.append(second)
        accepted_patterns = copy.deepcopy(patterns)
        generated = {'profiles': json.dumps({'records':[record]}).encode(), 'manifest': b'{"item_profiles":{}}'}
        boss.admit(boss.ROOT, generated, 'profiles', 'manifest', lambda r:r['production_definition']['production_key'], boss.canonical)
        current, = [r for r in json.loads(generated['profiles'])['records'] if r['production_definition'] == record['production_definition']]
        self.assertEqual(current['semantics']['equipment']['value']['patterns']['value'], accepted_patterns)
        self.assertEqual(current['semantics']['physical']['value']['movable'], boss.source.K(True))
        self.assertIsNotNone(native_loot_yield_facts(current['production_binding']['target'], current))

    def test_existing_owner_semantics_and_identity_are_preserved(self):
        overlay = json.loads((boss.ROOT / boss.PATH).read_bytes())
        record = copy.deepcopy(overlay['records'][0])
        record['semantics']['presentation'] = boss.source.K({'name': boss.source.K('accepted'),
            'description': boss.source.K('accepted owner description')})
        original = copy.deepcopy(record)
        generated = {'profiles': json.dumps({'records':[record]}).encode(), 'manifest': b'{"item_profiles":{}}'}
        boss.admit(boss.ROOT, generated, 'profiles', 'manifest', lambda r:r['production_definition']['production_key'], boss.canonical)
        output = json.loads(generated['profiles'])['records']
        current, = [r for r in output if r['production_definition'] == record['production_definition']]
        self.assertEqual(current['semantics']['presentation'], original['semantics']['presentation'])
        self.assertEqual(current['production_binding'], original['production_binding'])
        self.assertEqual(len(output), 22)


if __name__ == '__main__':
    unittest.main()
