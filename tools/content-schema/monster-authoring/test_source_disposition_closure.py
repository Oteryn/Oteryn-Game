"""Pinned-default normalization and complete source dispositions, never live config guesses."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import canary_batch as cb
from validate_monster import validate

PLAYER_TYPES = ('PHYSICALDAMAGE', 'ENERGYDAMAGE', 'EARTHDAMAGE', 'FIREDAMAGE', 'ICEDAMAGE', 'HOLYDAMAGE', 'DEATHDAMAGE')


def lua(value):
    if isinstance(value, dict):return '{'+','.join(k+'='+lua(v) for k,v in value.items())+'}'
    if isinstance(value, list):return '{'+','.join(lua(v) for v in value)+'}'
    if isinstance(value, bool):return 'true' if value else 'false'
    return json.dumps(value)


class SourceDispositionClosure(unittest.TestCase):
    def fixture(self, fields, settings=(200,-200,200), missing=False, source=None):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup);root=Path(temporary.name)
        assignments={'maxDamageReflection':settings[0], 'minElementalResistance':settings[1], 'maxElementalResistance':settings[2]}
        if not missing:
            (root/'config.lua.dist').write_text('\n'.join(k+' = '+str(v) for k,v in assignments.items()))
            loader=root/'src/config/configmanager.cpp';loader.parent.mkdir(parents=True)
            loader.write_text('\n'.join(f'loadIntConfig(L, {enum}, "{name}", {value});' for name,enum,value in
                [('maxDamageReflection','MAX_DAMAGE_REFLECTION',200),('minElementalResistance','MIN_ELEMENTAL_RESISTANCE',-200),('maxElementalResistance','MAX_ELEMENTAL_RESISTANCE',200)]))
        path=root/cb.MONSTER_DIR/'example.lua';path.parent.mkdir(parents=True)
        body={'health':100,'maxHealth':100,'race':'fire','outfit':{'lookType':42},**fields}
        path.write_text('local mType=Game.createMonsterType("Example")\nlocal monster={}\n'+
                        '\n'.join('monster.'+k+'='+lua(v) for k,v in body.items())+'\nmType:register(monster)')
        with patch.object(cb,'load_effect_constants',return_value=({},{})):
            converter=cb.Converter(root,{}, {}, {}, {})
        if source:converter.source=source
        converter.pending_definitions=set()
        with patch.object(cb.subprocess,'check_output',side_effect=lambda args,**kw:(root/args[-1].split(':',1)[1]).read_bytes()):
            result=converter.convert('example')
        return root,result

    def test_reflection_300_clamps_to_pinned_distribution_200_with_raw_provenance(self):
        _,result=self.fixture({'reflects':[{'type':'@COMBAT_ICEDAMAGE','percent':300}]})
        self.assertEqual(result[1]['creature']['damage_reflection'][0]['percent'],cb.ratio(200))
        row=next(e for e in result[4]['entries'] if e['source_field']=='reflects[1]')
        self.assertIn('Raw source percent=300',row['resolution']);self.assertIn('OTS_HYPOTHESIS_ONLY',row['resolution'])
        self.assertTrue(any(e['source_file']=='config.lua.dist' for e in result[4]['entries']))
        self.assertEqual(validate(*result[1:5]),[])

    def test_explicit_bounds_and_zero_are_respected_without_healing_clamp(self):
        for cap,expected in ((50,50),(400,300),(0,0)):
            _,r=self.fixture({'reflects':[{'type':'@COMBAT_ENERGYDAMAGE','percent':300}],
                              'heals':[{'type':'@COMBAT_ENERGYDAMAGE','percent':500}]},(cap,-200,200))
            self.assertEqual(r[1]['creature']['damage_reflection'][0]['percent'],cb.ratio(expected))
            self.assertEqual(r[1]['creature']['healing_from_damage'][0]['percent'],cb.ratio(500))

    def test_missing_config_is_explicitly_unresolved_without_native_300(self):
        _,r=self.fixture({'reflects':[{'type':'@COMBAT_ICEDAMAGE','percent':300}]},missing=True)
        self.assertFalse(r[1]['creature']['damage_reflection'])
        self.assertTrue(any(e['status']=='unresolved_semantics' and '300' in e['resolution'] for e in r[4]['entries']))
        self.assertTrue(validate(*r[1:5]))

    def test_mutated_distribution_is_rejected_against_exact_commit(self):
        root,_=self.fixture({});committed={f:(root/f).read_bytes() for f in ('config.lua.dist','src/config/configmanager.cpp')}
        (root/'config.lua.dist').write_text('maxDamageReflection=999')
        with patch.object(cb.subprocess,'check_output',side_effect=lambda args,**kw:committed[args[-1].split(':',1)[1]]):
            with self.assertRaisesRegex(ValueError,'differs from pinned'):cb.default_registration_profile(root)

    def test_element_default_clamp_and_all_seven_immune_exception(self):
        immune=[{'type':'@COMBAT_'+kind,'percent':100} for kind in PLAYER_TYPES]
        self.assertFalse(cb.elemental_can_clip(immune));self.assertTrue(cb.elemental_can_clip(immune[:-1]))
        for elements,expected in (([{'type':'@COMBAT_DROWNDAMAGE','percent':-400}],-200),
                                  (immune+[{'type':'@COMBAT_DROWNDAMAGE','percent':-300}],-300)):
            _,r=self.fixture({'elements':elements})
            row=next(e for e in r[1]['creature']['resistances'] if e['damage_type']=='drowning')
            self.assertEqual(row['reduction_percent'],cb.ratio(expected))

    def test_empty_collections_zero_corpse_and_obsolete_fields_have_explicit_dispositions(self):
        _,r=self.fixture({'corpse':0,'loot':[],'attacks':[],'voices':[],'summon':{},'summons':[],'maxSummons':0})
        entries={e['source_field']:e for e in r[4]['entries']}
        for field in ('corpse','loot','attacks','voices','summon','summons','maxSummons'):
            self.assertEqual(entries[field]['status'],'approved_omission')
            self.assertNotIn('destination',entries[field])
        self.assertNotIn('corpse_item',r[1]['creature']);self.assertEqual(validate(*r[1:5]),[])

    def test_explicit_empty_events_has_exact_disposition_but_absent_field_does_not(self):
        for fields, count in (({'events':[]},1), ({},0)):
            root,result=self.fixture(fields)
            rows=[e for e in result[4]['entries'] if e['source_field']=='events']
            self.assertEqual(len(rows),count)
            if rows:
                self.assertEqual(rows[0]['status'],'approved_omission')
                self.assertNotIn('destination',rows[0])
                text=(root/cb.MONSTER_DIR/'example.lua').read_text().splitlines()
                self.assertTrue(text[rows[0]['source_line']-1].startswith('monster.events='))
                self.assertEqual(result[1]['behavior']['event_bindings'],[])

    def test_three_actual_empty_event_monsters_preserve_source_file_and_line(self):
        source=Path('/workspace/monster-reference-sources/canary')
        self.assertTrue((source/'data/items/items.xml').exists(),'pinned population fixture is required')
        objects=cb.load_appearance_objects(source/'data/items/appearances.dat')
        items=cb.load_items_xml(source/'data/items/items.xml');names,index=cb.name_index(objects,items)
        converter=cb.Converter(source,objects,items,names,index)
        for relative in ('magicals/candy_horror','magicals/nibblemaw','quests/grave_danger/bosses/sir_nictros'):
            converter.pending_definitions=set();result=converter.convert(relative)
            row,=[e for e in result[4]['entries'] if e['source_field']=='events']
            self.assertEqual(row['source_file'],cb.MONSTER_DIR+'/'+relative+'.lua')
            self.assertEqual(row['status'],'approved_omission');self.assertNotIn('destination',row)
            text=(source/row['source_file']).read_text().splitlines()
            self.assertIn('monster.events',text[row['source_line']-1]);self.assertEqual(result[1]['behavior']['event_bindings'],[])

    def test_invisible_and_night_spawn_existing_carriers_have_parent_dispositions(self):
        _,r=self.fixture({'outfit':{'lookType':0,'lookHead':0},'respawnType':{'period':'@RESPAWNPERIOD_NIGHT','underground':False}})
        entries=[e['source_field'] for e in r[4]['entries'] if e['status']=='mapped']
        self.assertIn('outfit',entries);self.assertIn('respawnType',entries)
        self.assertEqual(r[1]['presentation']['appearance']['selection'],'invisible')
        self.assertEqual(r[1]['creature']['spawn_eligibility']['period'],'night')

    def test_crystal_profile_provenance_keeps_canary_engine_source_separate(self):
        source={'repository':'zimbadev/crystalserver','revision':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
        _,r=self.fixture({'reflects':[{'type':'@COMBAT_ICEDAMAGE','percent':300}]},source=source)
        entry=next(e for e in r[4]['entries'] if e['source_file']=='config.lua.dist')
        self.assertEqual(r[4]['sources'][0],source)
        self.assertEqual(r[4]['sources'][entry['source_index']],{'repository':cb.REPOSITORY,'revision':cb.REVISION})
        self.assertEqual(validate(*r[1:5]),[])


if __name__=='__main__':unittest.main()
