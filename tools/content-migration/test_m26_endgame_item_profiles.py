"""M26 corpse/loot custody through the real native provider and source capture."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import m26_endgame_item_profiles as m26


class SourceSpiteItems(unittest.TestCase):
    def test_real_native_provider_contains_source_corpse_decay_and_map_item_closure(self):
        capture=m26.load_capture()
        rows={r['production_definition']['production_key']:r for r in
              json.loads((m26.ROOT/m26.PROFILE).read_bytes())['records']}
        for source in capture['records']:
            key=source['production_definition']['production_key']
            self.assertTrue(key in rows, 'production provider is missing captured dependency '+key)
            self.assertEqual(rows[key]['production_binding'],source['production_binding'])
            self.assertEqual(rows[key]['admission'],source['admission'])
            for group in ('physical','container','temporal'):
                self.assertFalse(m26.known_conflict(rows[key]['semantics'][group],source['semantics'][group]))
        manifest=json.loads((m26.ROOT/'content/spells.manifest.json').read_bytes())
        self.assertEqual(manifest['item_profiles']['sha256'],hashlib.sha256((m26.ROOT/m26.PROFILE).read_bytes()).hexdigest())

    def test_overlay_is_idempotent_and_preserves_unrelated_profiles(self):
        source=m26.load_capture()['records'][0]
        unrelated=copy.deepcopy(source)
        unrelated['production_definition']['production_key']='oteryn:item.unrelated_fixture'
        generated={'p':m26.canonical({'records':[unrelated]}),'m':m26.canonical({'item_profiles':{}})}
        m26.admit(m26.ROOT,generated,'p','m',lambda r:r['production_definition']['production_key'],m26.canonical)
        first=dict(generated)
        m26.admit(m26.ROOT,generated,'p','m',lambda r:r['production_definition']['production_key'],m26.canonical)
        self.assertEqual(generated,first)
        self.assertIn(unrelated,json.loads(generated['p'])['records'])

    def test_source_capture_substitution_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            target=root/m26.PATH
            target.parent.mkdir(parents=True)
            target.write_bytes((m26.ROOT/m26.PATH).read_bytes()+b' ')
            with self.assertRaisesRegex(ValueError,'M26_ITEM_CAPTURE_DRIFT'):
                m26.load_capture(root)


class SourceRewardBossCorpseCapacity(unittest.TestCase):
    def test_large_source_reward_boss_corpse_reaches_the_real_loot_producer(self):
        import build_full_spell_artifact as loot
        rows=json.loads((m26.ROOT/'content/creatures/definitions/spell-native-profiles.json').read_bytes())['records']
        spite=next(r for r in rows if r['profile']['target']['key']==m26.BOSSES[0])
        profiles=json.loads((m26.ROOT/m26.PROFILE).read_bytes())
        for reward_boss,expected in [(True,True),(False,False)]:
            with self.subTest(reward_boss=reward_boss):
                creature=copy.deepcopy(spite)
                creature['profile']['data']['profile']['details']['system_eligibility']['reward_boss']=reward_boss
                payload=json.loads(loot.loot_tables({'records':[creature]},native_item_profiles=profiles))
                fact=next(r for r in payload['items'] if r['item']['key']=='oteryn:item.tibia.i33867')
                self.assertEqual(fact['materializable'],expected)
                self.assertEqual(fact['container_capacity'],48 if reward_boss else None)
        corpse=next(r for r in profiles['records'] if r['production_definition']['production_key']=='oteryn:item.tibia.i33867')
        identity=corpse['production_binding']['target']
        self.assertIsNone(loot.native_corpse_capacity(identity,corpse), 'ordinary corpses retain the 30-entry bound')
        self.assertIsNone(loot.native_loot_yield_facts(identity,corpse), 'Ground-only admission never permits a carried yield')


if __name__=='__main__':
    unittest.main()
