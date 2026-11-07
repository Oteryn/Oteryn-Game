import os
import unittest
from pathlib import Path
import wvar_item_profiles as wvar

ARCHIVE=Path(os.environ.get("OTERYN_TEST_CRYSTAL_ARCHIVE", "/tmp/wvar-crystal.tar.gz"))

class WvarItemProfilesTests(unittest.TestCase):
    @unittest.skipUnless(ARCHIVE.is_file(), "optional pinned donor archive unavailable")
    def test_pinned_native_sandfish_is_portable_stackable_and_edible(self):
        data=wvar.build_archive(ARCHIVE)
        self.assertEqual(len(data['records']),7)
        row=next(r for r in data['records'] if r['production_definition']['production_key']=='oteryn:item.tibia.i13992')
        self.assertEqual(row['production_definition']['production_key'],'oteryn:item.tibia.i13992')
        self.assertEqual(row['semantics']['consumption'],{'state':'KNOWN','value':{'kind':'FOOD','value':{'regeneration_seconds':660}}})
        self.assertEqual(row['semantics']['physical']['value']['weight'],{'state':'KNOWN','value':660})
        self.assertEqual(row['admission'],{'materializable':True,'stack_class':'StackCapable','legal_destinations':['CharacterInventory','Ground']})
        self.assertEqual(row['semantics']['container'],{'state':'NOT_APPLICABLE'})
    @unittest.skipUnless(ARCHIVE.is_file(), "optional pinned donor archive unavailable")
    def test_only_six_genuine_reward_leaves_join_existing_sandfish(self):
        rows=wvar.build_archive(ARCHIVE)['records']
        keys={r['production_definition']['production_key'] for r in rows}
        self.assertEqual(keys,{f'oteryn:item.tibia.i{i}' for i in [13992,8031,7412,7395,817,3316,3265]})
        self.assertNotIn('oteryn:item.tibia.i44612',keys)

    def test_reward_overlay_replaces_only_the_two_source_actors(self):
        import json
        import wvar_creature_profiles as creatures
        path="tools/content-schema/native-gameplay/loot-tables.json"
        before=json.loads((wvar.ROOT/path).read_bytes())
        generated={}
        creatures.admit_reward_loot(wvar.ROOT,generated)
        after=json.loads(generated[path])
        expected={"oteryn:loot.creature.giant_spider_wyda":2,"oteryn:loot.creature.bibby_bloodbath":17}
        corpses={6008:10}
        for item,capacity in corpses.items():
            fact=next(r for r in after["items"] if r["item"]["key"]==f"oteryn:item.tibia.i{item}")
            self.assertTrue(fact["materializable"])
            self.assertEqual(fact["container_capacity"],capacity)
        originals={r["identity"]["key"]:r for r in before["tables"]}
        for row in after["tables"]:
            key=row["identity"]["key"]
            if key in expected:
                self.assertEqual(len(row["entries"]),expected[key])
                self.assertNotIn("oteryn:item.tibia.i44612",{e["item"]["key"] for e in row["entries"]})
            else:self.assertEqual(row,originals[key])

    def test_reward_canonical_keeps_previously_accepted_marketability(self):
        import json
        definitions=[r["definition"] for p in (wvar.ROOT/"content/items/definitions").glob("items-*.json") for r in json.loads(p.read_bytes())["records"]]
        wvar.apply_canonical_admissions(definitions,wvar.ROOT)
        row=next(d for d in definitions if d["identity"]["key"]=="oteryn:item.tibia.i3316")
        self.assertEqual(row["semantics"]["trade_restrictions"]["value"]["marketable"],{"state":"KNOWN","value":True})

    def test_native_membership_is_installed_once_without_replacing_other_profiles(self):
        import json
        generated={'profiles':b'{"records":[]}', 'manifest':b'{"item_profiles":{"sha256":"old"}}'}
        canonical=lambda value:(json.dumps(value,sort_keys=True)+'\n').encode()
        order=lambda row:row['production_definition']['production_key']
        wvar.admit(wvar.ROOT,generated,'profiles','manifest',order,canonical)
        self.assertEqual(len(json.loads(generated['profiles'])['records']),7)
        with self.assertRaisesRegex(ValueError,'WVAR_ITEM_PROFILE_DUPLICATE'):
            wvar.admit(wvar.ROOT,generated,'profiles','manifest',order,canonical)

    @unittest.skipUnless(ARCHIVE.is_file(), "optional pinned donor archive unavailable")
    def test_altered_donor_bytes_cannot_grant_native_admission(self):
        sources=wvar.read_archive(ARCHIVE)
        sources[0]=(sources[0][0],sources[0][1]+b'\n')
        with self.assertRaisesRegex(ValueError,'WVAR_ITEM_SOURCE_HASH'):
            wvar.build_sources(sources)

class RecipeRewardClosureTests(unittest.TestCase):
    @unittest.skipUnless(ARCHIVE.is_file(), "pinned donor archive unavailable")
    def test_all_recipe_source_rewards_have_exact_physical_dependencies(self):
        data=wvar.build_recipe_reward_archive(ARCHIVE)
        rows={row['creature']:row for row in data['creatures']}
        self.assertEqual(len(rows),41) #42keys minus intentionally authored Mamma
        self.assertEqual(len(rows['oteryn:creature.arthom_the_hunter']['loot']['entries']),15)
        self.assertEqual(len(rows['oteryn:creature.hive_overseer']['loot']['entries']),16)
        self.assertIsNone(rows['oteryn:creature.dog']['loot'])
        self.assertTrue({'oteryn:creature.elf','oteryn:creature.elf_scout','oteryn:creature.elf_arcanist','oteryn:creature.elf_overseer'}<=set(rows))
        self.assertEqual(max(row['capacity'] or 0 for row in data['corpses']),44)
        self.assertEqual(next(row['capacity'] for row in data['corpses'] if row['item']==5977),30)
        self.assertEqual(next(row['capacity'] for row in data['corpses'] if row['item']==4039),20)
        admitted={row['production_definition']['production_key'] for row in data['records']}
        for row in data['records']:
            for field in ['equipment','weapon','protection','skill_modifiers','imbuement','use_transform']:
                self.assertEqual(row['semantics'][field],{'state':'UNKNOWN'},'physical rewards do not qualify unrelated systems')
        for row in data['creatures']:
            for entry in (row['loot'] or {}).get('entries',[]):self.assertIn(entry['item']['key'],admitted)
        self.assertNotIn('oteryn:item.tibia.i44612',admitted)

    def test_generic_projection_preserves_other_creatures_and_admits_every_dependency(self):
        import json
        from unittest.mock import patch
        data=wvar.recipe_reward_overlay()
        paths=['content/items/definitions/spell-native-profiles.json','content/creatures/definitions/spell-native-profiles.json','content/spells.manifest.json']
        generated={p:(wvar.ROOT/p).read_bytes() for p in paths}
        loot_path='tools/content-schema/native-gameplay/loot-tables.json'
        before=json.loads((wvar.ROOT/loot_path).read_bytes())
        wvar.admit_recipe_reward_closure(wvar.ROOT,generated)
        after=json.loads(generated[loot_path]);facts={row['item']['key']:row for row in after['items']}
        selected={row['loot']['identity']['key'] for row in data['creatures'] if row['loot']}
        old_tables={row['identity']['key']:row for row in before['tables']}
        for row in after['tables']:
            if row['identity']['key'] not in selected:self.assertEqual(row,old_tables[row['identity']['key']])
        for row in data['creatures']:
            self.assertTrue(facts[f'oteryn:item.tibia.i{row["corpse"]}']['materializable'])
            self.assertLessEqual(facts[f'oteryn:item.tibia.i{row["corpse"]}']['container_capacity'],44)
            for entry in (row['loot'] or {}).get('entries',[]):self.assertTrue(facts[entry['item']['key']]['materializable'])
        self.assertNotIn('oteryn:item.tibia.i12722',facts)
        self.assertNotIn('oteryn:item.tibia.i12819',facts)

    def test_changed_bindings_refuse_the_stale_recipe_receipt(self):
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            for relative in [wvar.REWARD_PATH,'content/encounters/world-changes/variants.json','content/encounters/world-changes/variants-extra.json','imports/crystalserver/bindings/items.json']:
                destination=root/relative;destination.parent.mkdir(parents=True,exist_ok=True)
                destination.write_bytes((wvar.ROOT/relative).read_bytes())
            path=root/'imports/crystalserver/bindings/items.json';path.write_bytes(path.read_bytes()+b'\n')
            with self.assertRaisesRegex(ValueError,'WVAR_REWARD_BINDINGS_SHA'):
                wvar.recipe_reward_overlay(root)

    def test_altered_archive_cannot_authorize_recipe_reward_facts(self):
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'altered.tar.gz';path.write_bytes(b'not the pinned archive')
            with self.assertRaisesRegex(ValueError,'WVAR_REWARD_ARCHIVE_SHA'):
                wvar.build_recipe_reward_archive(path)

if __name__=='__main__': unittest.main()
