import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('shared_variant_repair',Path(__file__).with_name('repair_shared_monster_presentation_variant.py'))
r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)


def documents(projectile=True):
    effect={'identity':{'key':r.EFFECT,'revision':'source-r1'},'operation':'presentation_only',
            'presentation':{'impact_asset_binding':'canary.appearance:effect/sound_purple'}}
    if projectile:effect['presentation']['projectile_asset_binding']='canary.appearance:missile/suddendeath'
    ability={'identity':{'key':r.ABILITY,'revision':'source-r1'},'kind':'spell','range_tiles':0,
             'needs_target':False,'needs_direction':True,'effects':[dict(effect['identity'],family='Effect')]}
    return {'monster.json':{'behavior':{'attacks':[{'ability':dict(ability['identity'],family='Ability'),'chance_percent':25}]},
                            'creature':{'health':42,'armor':7},'loot':{'entries':[{'chance_percent':1}]}},
            'dependencies.json':{'abilities':[ability],'effects':[effect],
                                 'formulas':[{'identity':{'key':'canary:formula/shared','revision':'r1'},'kind':'range'}],
                                 'documents':[{'content':[r.ABILITY]}],'items':[],'loot_tables':[]},
            'catalog.json':{'assets':['canary.appearance:missile/suddendeath'],
                            'definitions':[dict(ability['identity'],family='Ability')]}}


class SharedVariantRepairTests(unittest.TestCase):
    def test_all_typed_references_localized_and_shared_mechanics_preserved(self):
        before=documents();after,changes=r.localize(before)
        self.assertEqual(after['monster.json']['behavior']['attacks'][0]['ability']['key'],r.LOCAL_ABILITY)
        self.assertEqual(after['dependencies.json']['abilities'][0]['effects'][0]['key'],r.LOCAL_EFFECT)
        self.assertEqual(after['dependencies.json']['effects'][0]['identity']['key'],r.LOCAL_EFFECT)
        self.assertEqual(after['catalog.json']['definitions'][0]['key'],r.LOCAL_ABILITY)
        self.assertEqual(after['dependencies.json']['formulas'],before['dependencies.json']['formulas'])
        self.assertEqual(after['dependencies.json']['documents'],before['dependencies.json']['documents'])
        self.assertEqual(after['monster.json']['loot'],before['monster.json']['loot'])
        self.assertEqual(after['monster.json']['creature'],before['monster.json']['creature'])
        self.assertEqual(before,documents())
        self.assertEqual(len(changes),5)

    def test_native_collision_fence_rejects_shared_variant_and_accepts_local_variant(self):
        original=documents(False)['dependencies.json'];variant=documents()['dependencies.json']
        for d in (original,variant):d['formulas']=[];d['documents']=[]
        stage=r.native.Stage(r.native.Mapper({}));stage.stage_dependencies(original,'other')
        with self.assertRaises(r.native.StageError):stage.stage_dependencies(variant,'deaththrower')
        repaired,_=r.localize(documents());variant=repaired['dependencies.json'];variant['formulas']=[];variant['documents']=[]
        stage=r.native.Stage(r.native.Mapper({}));stage.stage_dependencies(original,'other');stage.stage_dependencies(variant,'deaththrower')
        self.assertEqual(len(stage.profiles),2)
        self.assertIn(('Ability','oteryn:ability.creature.deaththrower.dark_torturer_skill_reducer'),stage.profiles)

    def test_wrong_projectile_and_preexisting_local_identity_rejected(self):
        with self.assertRaisesRegex(ValueError,'projectile'):r.localize(documents(False))
        value=documents();value['dependencies.json']['abilities'].append({'identity':{'key':r.LOCAL_ABILITY}})
        with self.assertRaisesRegex(ValueError,'already exists'):r.localize(value)

    def test_extra_effect_chain_is_not_silently_rewritten(self):
        value=documents();value['dependencies.json']['abilities'][0]['effects'].append({'family':'Effect','key':'canary:effect/other','revision':'r1'})
        with self.assertRaisesRegex(ValueError,'one known'):r.localize(value)

    def test_exact_prepared_index_guard_fails_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);baseline=root/'baseline';baseline.mkdir();(baseline/'population-index.json').write_text('{}')
            with self.assertRaisesRegex(ValueError,'drifted'):
                r.build(baseline,root/'nonexistent-source.json',root/'collision.json',root/'out')
            self.assertFalse((root/'out').exists())


if __name__=='__main__':unittest.main()
