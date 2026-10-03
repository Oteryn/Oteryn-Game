"""Exact source field guards and precedence; secondary Lua is never executed here."""
import copy
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import secondary_mitigation as sm

PACKET = json.loads(sm.SAMPLE.read_text())
ROOT = Path(os.environ.get('OTERYN_CANARY', '/workspace/monster-reference-sources/canary'))
CRYSTAL = Path(os.environ.get('OTERYN_CRYSTAL', '/workspace/monster-reference-sources/crystal'))


class LiteralWitness(unittest.TestCase):
    SOURCE = 'local mType = Game.createMonsterType("Test")\nlocal monster = {}\nmonster.defenses = {armor=10,\n mitigation=7.16,}\nmType:register(monster)\n'

    def test_exact_literal_and_physical_source_line(self):
        self.assertEqual({'literal':'7.16','line':4,'table_line':2,'registration_line':5}, sm.literal_witness(self.SOURCE,'Test'))

    def test_expressions_boolean_uncertainty_range_and_reassignment_are_rejected(self):
        for value in ['true', '0/0', 'math.random()', '7.16 * 1.5', '-1', '101', '7.16, -- TODO confirm']:
            with self.subTest(value=value), self.assertRaises(ValueError):
                sm.literal_witness(self.SOURCE.replace('7.16', value), 'Test')
        for extra in ['monster.defenses.mitigation=8\n', 'local alias=monster.defenses\n', 'monster.defenses={}\n']:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                sm.literal_witness(self.SOURCE.replace('mType:register', extra+'mType:register'), 'Test')

    def test_conditional_duplicate_and_wrong_registration_are_rejected(self):
        for text in [self.SOURCE.replace('monster.defenses', 'if x then monster.defenses'),
                     self.SOURCE+'mType:register(monster)', self.SOURCE.replace('"Test"','"Other"')]:
            with self.assertRaises(ValueError):sm.literal_witness(text, 'Test')


@unittest.skipUnless(ROOT.exists() and CRYSTAL.exists(), 'exact public pinned source caches unavailable')
class PinnedAdoption(unittest.TestCase):
    def attempt(self, row, packet=None, raw_mutate=None, existing=None, primary=None):
        raw = copy.deepcopy(row['primary_source']['fingerprint'])
        name = raw.pop('registration_name');raw['name']=raw.pop('display_name')
        raw['defenses']={}
        if raw_mutate:raw_mutate(raw)
        creature={'stats':{'armor':105,'speed':250}}
        if existing is not None:creature['stats']['mitigation_percent']=existing
        rows=[];sources=[{'repository':sm.CANARY[0],'revision':sm.CANARY[1]}]
        before=(copy.deepcopy(creature),copy.deepcopy(rows),copy.deepcopy(sources))
        with tempfile.TemporaryDirectory() as tmp:
            sample=Path(tmp)/'sample.json';sample.write_text(json.dumps(packet or PACKET))
            with patch.object(sm,'SAMPLE',sample):
                changed=sm.adopt_secondary_mitigation(row['relative'],name,raw,ROOT/row['primary_source']['path'],
                    creature,rows,sources,CRYSTAL,primary or sources[0])
        if not changed:self.assertEqual(before,(creature,rows,sources))
        return changed,creature,rows,sources

    def test_all_six_literal_base_percent_values_and_field_only_provenance(self):
        expected={'Bakragore':(204,25),'Chagorz':(179,25),'Ichgahal':(179,25),
                  'Murcion':(179,25),'Vemiath':(179,25),'Overcharged Demon':(387,50)}
        for row in PACKET['candidates']:
            with self.subTest(name=row['primary_source']['fingerprint']['registration_name']):
                changed,creature,rows,sources=self.attempt(row);self.assertTrue(changed)
                n,d=expected[row['primary_source']['fingerprint']['registration_name']]
                self.assertEqual({'numerator':n,'denominator':d},creature['stats']['mitigation_percent'])
                self.assertEqual(105,creature['stats']['armor']);self.assertEqual(250,creature['stats']['speed'])
                self.assertEqual(1,len(rows));self.assertEqual('mapped',rows[0]['status'])
                self.assertIn(sm.QUALIFICATION,rows[0]['resolution']);self.assertIn('Global value UNKNOWN',rows[0]['resolution'])
                self.assertIn('full combat balance differs',rows[0]['resolution'])
                self.assertEqual(row['secondary_source']['path'],rows[0]['source_file'])
                self.assertEqual(row['literal_witness']['line'],rows[0]['source_line'])
                self.assertEqual({'repository':sm.CRYSTAL[0],'revision':sm.CRYSTAL[1]},sources[rows[0]['source_index']])

    def test_primary_zero_and_existing_wiki_value_always_take_precedence(self):
        row=PACKET['candidates'][0]
        for value in [{'numerator':0,'denominator':1},{'numerator':7,'denominator':2}]:
            self.assertFalse(self.attempt(row,existing=value)[0])
        self.assertFalse(self.attempt(row,raw_mutate=lambda raw:raw['defenses'].update(mitigation=0))[0])

    def test_unknown_missing_and_ambiguous_source_are_not_filled(self):
        row=PACKET['candidates'][0]
        for mutate in [lambda p:p.update(candidates=[]),lambda p:p['candidates'].append(copy.deepcopy(p['candidates'][0])),
                       lambda p:p['candidates'][0].update(unique_same_registration_secondary_count=2)]:
            packet=copy.deepcopy(PACKET);mutate(packet);self.assertFalse(self.attempt(row,packet=packet)[0])
        self.assertIn('mushroom',PACKET['rejected_identity_conflicts'])

    def test_secondary_bool_nonfinite_out_of_range_and_unspecified_values_are_rejected(self):
        row=PACKET['candidates'][0]
        for value in [True,False,float('nan'),float('inf'),-1,101,None,'7.16']:
            packet=copy.deepcopy(PACKET);packet['candidates'][0]['secondary_source']['mitigation']=value
            with self.subTest(value=value):self.assertFalse(self.attempt(row,packet=packet)[0])

    def test_forged_matching_identity_cannot_adopt_actual_incompatible_mushroom(self):
        import hashlib
        import canary_batch
        row=copy.deepcopy(PACKET['candidates'][0])
        primary_path='data-otservbr-global/monster/quests/rotten_blood/mushroom.lua'
        secondary_path='data-global/monster/quests/rotten_blood_quest/mushroom.lua'
        name,raw,_=canary_batch.load_monster(ROOT/primary_path,[])
        actual_fingerprint=sm.fingerprint(name,raw)
        for family,root,path,epoch in [('primary_source',ROOT,primary_path,sm.CANARY),
                                     ('secondary_source',CRYSTAL,secondary_path,sm.CRYSTAL)]:
            content=(root/path).read_bytes();errors=[]
            canary_batch.load_monster(root/path,errors)
            row[family]={'repository':epoch[0],'revision':epoch[1],'path':path,
                'sha256':hashlib.sha256(content).hexdigest(),
                'git_blob':hashlib.sha1(b'blob '+str(len(content)).encode()+b'\0'+content).hexdigest(),
                'fingerprint':copy.deepcopy(actual_fingerprint),'bounded_eval_errors':errors}
        row['relative']='quests/rotten_blood/mushroom'
        row['secondary_source']['mitigation']=3.16
        row['literal_witness']=sm.literal_witness((CRYSTAL/secondary_path).read_text(),name)
        packet=copy.deepcopy(PACKET);packet['candidates']=[row]
        self.assertFalse(self.attempt(row,packet=packet)[0])

    def test_source_pin_hash_path_registration_and_full_fingerprint_substitutions_fail_closed(self):
        row=PACKET['candidates'][0]
        for family,key,value in [('primary_source','revision','0'*40),('secondary_source','revision','0'*40),
             ('primary_source','sha256','0'*64),('secondary_source','sha256','0'*64),('secondary_source','git_blob','0'*40),
             ('secondary_source','path','../other.lua'),('secondary_source','repository',sm.CANARY[0])]:
            packet=copy.deepcopy(PACKET);packet['candidates'][0][family][key]=value
            with self.subTest(family=family,key=key):self.assertFalse(self.attempt(row,packet=packet)[0])
        for key in row['secondary_source']['fingerprint']:
            packet=copy.deepcopy(PACKET);packet['candidates'][0]['secondary_source']['fingerprint'][key]='SUBSTITUTED'
            with self.subTest(fingerprint=key):self.assertFalse(self.attempt(row,packet=packet)[0])
        self.assertFalse(self.attempt(row,primary={'repository':sm.CRYSTAL[0],'revision':sm.CRYSTAL[1]})[0])
