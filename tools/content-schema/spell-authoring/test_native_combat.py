"""Qualification, strict rejection, source conflicts and independent numerical facts."""
from copy import deepcopy
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from jsonschema import Draft202012Validator,ValidationError
import native_combat as nc
from validate_spell import evaluate


class CombatTests(unittest.TestCase):
    def setUp(self):
        d=tempfile.TemporaryDirectory();self.addCleanup(d.cleanup);self.root=Path(d.name)
        self.specs=deepcopy(nc.SPECS);self.texts={};self.records={}
        for name,sources in self.specs.items():
            self.records[name]={}
            for source,spec in sources.items():
                text=f'qualified full fixture {source} {name}\n';data=text.encode()
                self.texts[source,spec['file']]=text
                spec['sha256']=hashlib.sha256(data).hexdigest()
                spec['blob']=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
                self.records[name][source]={'name':name,'spell_type':'instant',
                    'source_root':self.root if source=='canary' else self.root/'crystal',
                    'file':spec['file'],'blob':spec['blob'],'revision':nc.PINS[source]}
        (self.root/'support.txt').write_bytes(b'pinned core helper')
        crystal_helper=self.root/'crystal/data/scripts/lib/register_spells.lua'
        crystal_helper.parent.mkdir(parents=True)
        crystal_helper.write_bytes(b'pinned crystal healing helper')
        for target,value in [('SPECS',self.specs),
              ('HELPERS',{'support.txt':hashlib.sha256(b'pinned core helper').hexdigest()}),
              ('CRYSTAL_HEALING_HELPER_SHA256',hashlib.sha256(b'pinned crystal healing helper').hexdigest()),
              ('_head',lambda root:nc.PINS['crystal'] if str(root).endswith('crystal') else nc.PINS['canary'])]:
            p=patch.object(nc,target,value);p.start();self.addCleanup(p.stop)

    def build(self,name):return nc.build(name,'instant',self.records[name],self.texts)

    def test_twenty_candidates_validate_and_metadata_stays_separate(self):
        counts={}
        for name in self.records:
            r=self.build(name);self.assertIsNotNone(r,name);self.assertEqual(set(r),{'key','parameters'})
            counts[r['key']]=counts.get(r['key'],0)+1
            Draft202012Validator(nc.schemas()[r['key']]).validate(r['parameters'])
            e=nc.evidence(name,'instant',self.records[name],self.texts)
            self.assertTrue(e['authoring_complete']);self.assertNotIn('source_evidence',r['parameters'])
            self.assertEqual(e['runtime_admission'],'rejected_until_native_contract_and_owner_integration')
        self.assertEqual(counts,{'wheel_combat':9,'avatar_state':5,'monster_ai_override':4,'mass_spirit_mend':1,'mana_shield_capacity':1})

    def test_source_and_helper_mutations_wrong_records_and_unrelated_names_reject(self):
        for name in self.records:
            for source,spec in self.specs[name].items():
                bad=dict(self.texts);bad[source,spec['file']]+='altered behavior'
                self.assertIsNone(nc.build(name,'instant',self.records[name],bad))
                bad.pop((source,spec['file']));self.assertIsNone(nc.build(name,'instant',self.records[name],bad))
                for field,value in [('revision','different'),('blob','0'*40),('name','different')]:
                    records=deepcopy(self.records[name]);records[source][field]=value
                    self.assertIsNone(nc.build(name,'instant',records,self.texts))
            self.assertIsNone(nc.build(name,'rune',self.records[name],self.texts))
        self.assertIsNone(nc.build('energy strike','instant',{},{}))
        (self.root/'support.txt').write_bytes(b'new core semantics');self.assertIsNone(self.build('energy beam'))

    def test_strict_schemas_reject_extra_missing_and_changed_semantics(self):
        for name in self.records:
            r=self.build(name);v=Draft202012Validator(nc.schemas()[r['key']])
            bad=deepcopy(r['parameters']);bad['unrepresented_effect']=True
            with self.assertRaises(ValidationError):v.validate(bad)
            bad=deepcopy(r['parameters']);bad.pop(next(iter(bad)))
            with self.assertRaises(ValidationError):v.validate(bad)
        bad=deepcopy(self.build('energy beam')['parameters']);bad['beam_mastery']['side_damage_percent']['3']=80
        with self.assertRaises(ValidationError):Draft202012Validator(nc.schemas()['wheel_combat']).validate(bad)

    def test_accepted_updates_supersede_old_ots_numbers(self):
        self.assertEqual(self.build('energy beam')['parameters']['beam_mastery']['side_damage_percent'],{'0':0,'1':25,'2':40,'3':70})
        c=self.build('chivalrous challenge')['parameters']
        self.assertEqual(c['chain']['further_targets'],3);self.assertEqual(c['challenge_ms'],6000);self.assertNotIn('wheel',c)
        self.assertEqual(self.build("executioner's throw")['parameters']['chain']['further_targets'],{'0':0,'1':2,'2':3,'3':4})
        mend=self.build('mass spirit mend')['parameters'];self.assertFalse(mend['harmony_spender'])
        self.assertEqual(mend['cooldown_ms']['2'],8000);self.assertEqual(len(mend['area']['orthogonal']),11)
        self.assertTrue(nc.evidence('chivalrous challenge','instant',self.records['chivalrous challenge'],self.texts)['attributed_conflicts'])

    def test_level_curve_and_callback_route_numerics(self):
        env={'level':1200,'magic_level':100,'attack_skill':100,'attack_value':50,'maximum_mana':15000}
        f=self.build('energy beam')['parameters']['formula']
        self.assertEqual(evaluate(f['minimum'],env),405);self.assertEqual(evaluate(f['maximum'],env),533)
        mend=self.build('mass spirit mend')['parameters']
        self.assertAlmostEqual(evaluate(mend['caster_formula']['minimum'],env),980)
        self.assertAlmostEqual(evaluate(mend['others_formula']['minimum'],env),3252.6)
        s=self.build('magic shield')['parameters'];self.assertEqual(evaluate(s['capacity'],env),10300)
        env['maximum_mana']=1000;self.assertEqual(evaluate(s['capacity'],env),1000)
        self.assertFalse(s['wheel_capacity_multiplier'])

    def test_geometry_thresholds_and_target_filters(self):
        b=self.build('ice burst')['parameters'];cells=b['base_area']['orthogonal']
        self.assertEqual(cells[len(cells)//2][len(cells[0])//2],2)
        self.assertEqual(b['health_bonus']['comparison'],'greater_than')
        self.assertEqual(b['health_bonus']['health_percent_rounding'],'nearest_half_away_from_zero')
        a=self.build('balanced brawl')['parameters'];self.assertIsNone(a['reward_boss_cast_refusal']);self.assertEqual(a['forced_melee_ms'],16000)
        self.assertEqual(self.build('challenge')['parameters']['challenge_ms'],6000)
        a=self.build('divine dazzle')['parameters'];self.assertEqual(a['reward_boss_cast_refusal']['dx_max'],11)
        self.assertEqual(a['forced_melee_ms']['2'],12000);self.assertEqual(a['chain']['further_targets']['1'],4)

    def test_build_does_not_share_mutable_templates(self):
        self.build('avatar of steel')['parameters']['duration_ms']=1
        self.assertEqual(self.build('avatar of steel')['parameters']['duration_ms'],15000)


    def test_independent_pinned_base_component_vectors(self):
        vectors=[{'bounds': [247, 427],
          'case': 'baseline/sorcerer-103',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 1, 'magic_level': 50},
          'name': 'energy wave'},
         {'bounds': [380, 560],
          'case': 'baseline/sorcerer-121',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 699, 'magic_level': 50},
          'name': 'energy wave'},
         {'bounds': [287, 467],
          'case': 'baseline/sorcerer-186',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 200, 'magic_level': 50},
          'name': 'energy wave'},
         {'bounds': [281, 416],
          'case': 'baseline/sorcerer-103',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 1, 'magic_level': 50},
          'name': 'great energy beam'},
         {'bounds': [414, 549],
          'case': 'baseline/sorcerer-121',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 699, 'magic_level': 50},
          'name': 'great energy beam'},
         {'bounds': [321, 456],
          'case': 'baseline/sorcerer-186',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 200, 'magic_level': 50},
          'name': 'great energy beam'},
         {'bounds': [225, 404],
          'case': 'baseline/druid-069',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 1, 'magic_level': 50},
          'name': 'strong ice wave'},
         {'bounds': [358, 537],
          'case': 'baseline/druid-087',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 699, 'magic_level': 50},
          'name': 'strong ice wave'},
         {'bounds': [265, 444],
          'case': 'baseline/druid-184',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 200, 'magic_level': 50},
          'name': 'strong ice wave'},
         {'bounds': [191, 326],
          'case': 'baseline/druid-069',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 1, 'magic_level': 50},
          'name': 'ice burst'},
         {'bounds': [324, 459],
          'case': 'baseline/druid-087',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 699, 'magic_level': 50},
          'name': 'ice burst'},
         {'bounds': [231, 366],
          'case': 'baseline/druid-184',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 200, 'magic_level': 50},
          'name': 'ice burst'},
         {'bounds': [191, 326],
          'case': 'baseline/druid-069',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 1, 'magic_level': 50},
          'name': 'terra burst'},
         {'bounds': [324, 459],
          'case': 'baseline/druid-087',
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 699, 'magic_level': 50},
          'name': 'terra burst'},
         {'bounds': [231, 366],
          'case': 'baseline/druid-184',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 200, 'magic_level': 50},
          'name': 'terra burst'},
         {'bounds': [82, 137],
          'case': 'baseline/knight-001',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 1, 'magic_level': 50},
          'name': 'front sweep'},
         {'bounds': [215, 270],
          'case': 'baseline/knight-021',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 701, 'magic_level': 50},
          'name': 'front sweep'},
         {'bounds': [122, 177],
          'case': 'baseline/knight-177',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 200, 'magic_level': 50},
          'name': 'front sweep'},
         {'bounds': [75, 89],
          'case': 'baseline/knight-001',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 1, 'magic_level': 50},
          'name': "executioner's throw"},
         {'bounds': [208, 222],
          'case': 'baseline/knight-021',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 701, 'magic_level': 50},
          'name': "executioner's throw"},
         {'bounds': [115, 129],
          'case': 'baseline/knight-177',
          'input': {'attack_skill': 80, 'attack_value': 14, 'level': 200, 'magic_level': 50},
          'name': "executioner's throw"},
         {'bounds': [None, None],
          'case': 'baseline/sorcerer-103',
          'contains': True,
          'got': [101, 169],
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 1, 'magic_level': 50},
          'name': 'energy beam',
          'nominal': 135},
         {'bounds': [None, None],
          'case': 'baseline/sorcerer-104',
          'contains': True,
          'got': [102, 170],
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 8, 'magic_level': 50},
          'name': 'energy beam',
          'nominal': 136},
         {'bounds': [None, None],
          'case': 'baseline/sorcerer-105',
          'contains': True,
          'got': [117, 185],
          'input': {'attack_skill': 80, 'attack_value': 0, 'level': 80, 'magic_level': 50},
          'name': 'energy beam',
          'nominal': 151}]
        for row in vectors:
            f=nc.TEMPLATES[row['name']]['parameters']['formula']
            got=[int(evaluate(f[k],row['input'])) for k in ('minimum','maximum')]
            if row['bounds'][0] is None:
                self.assertLessEqual(got[0],row['nominal']);self.assertGreaterEqual(got[1],row['nominal'])
                nominal=row['input']['magic_level']*60/25+60/4
                from validate_spell import level_base_damage_healing
                self.assertEqual(level_base_damage_healing(row['input']['level'])+int(nominal),row['nominal'])
            else:
                self.assertEqual(got,row['bounds'],row['case'])
        mass=nc.TEMPLATES['mass healing']['parameters']['formula']
        env={'level':1200,'magic_level':100}
        self.assertEqual([int(evaluate(mass[k],env)) for k in ('minimum','maximum')],[840,1296])
        self.assertEqual(nc._formula_evidence('mass healing')[0]['authority'],'OtsHypothesisOnly')

if __name__=='__main__':unittest.main()
