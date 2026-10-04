import copy
import math
import json
from pathlib import Path
import unittest
import project_party_summon_source_candidates as m

class SourceExtensionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.packet=m.build()

    def test_emitted_item_catalog_owner_closure(self):
        out=m.ROOT/'docs/reference/spells/r61-source-closure'
        owners=0
        for row in self.packet['rows']:
            folder=out/row['registration_key'].split('/')[0]/m.prior.sha(row['registration_key'].encode())[:16]
            receipt=json.loads((folder/'receipt.json').read_bytes())
            catalog=json.loads((folder/'catalog.json').read_bytes())
            self.assertEqual(receipt['item_owner_bindings_required'],catalog['definitions'])
            self.assertEqual(catalog['definitions'],m.prior.base.item_references(row['bundle']))
            m.validator.Draft202012Validator(m.prior.base.receipt_schema(),registry=m.validator.REGISTRY).validate(receipt)
            if catalog['definitions']:owners+=1
        self.assertEqual(owners,4)

    def test_partition_and_equipment_actual_dependencies(self):
        self.assertEqual(self.packet['records'],26)
        rows=[r for r in self.packet['rows'] if r['bundle']['spell']['execution']['native_behavior']['key']=='equipment_attack']
        self.assertEqual(len(rows),1)
        self.assertEqual([len(rows[0]['dependencies'][key]) for key in ('abilities','effects','formulas')],[3,3,3])
        self.assertEqual([e['damage_type'] for e in rows[0]['dependencies']['effects']],['physical','energy','earth'])

    def test_monk_formula_values_and_dependency_refs(self):
        row=next(r for r in self.packet['rows'] if r['dependencies']['formulas'])
        deps=row['dependencies']
        known={(family,d['identity']['key'],d['identity']['revision']) for family,key in [('Ability','abilities'),('Effect','effects'),('Formula','formulas')] for d in deps[key]}
        for _,value in m.validator.walk(row['bundle']):
            if isinstance(value,dict) and value.get('family') in ('Ability','Effect','Formula'):
                self.assertIn((value['family'],value['key'],value['revision']),known)
        for level,skill,weapon in ((125,10,7),(300,100,50),(1000,130,80)):
            env={'level':level,'attack_skill':skill,'attack_value':weapon,'base_power':62}
            attack=m.validator.level_base_damage_healing(level)+math.floor(1.2*weapon)*(skill+4)/28
            total=62*attack/100+attack
            for formula in deps['formulas']:
                self.assertAlmostEqual(m.validator.evaluate(formula['minimum'],env),.9*total)
                self.assertAlmostEqual(m.validator.evaluate(formula['maximum'],env),1.1*total)

    def test_private_spell_and_dependency_schema(self):
        schema=m.private_schema(self.packet)
        m.validator.Draft202012Validator.check_schema(schema)
        check=m.validator.Draft202012Validator(schema,registry=m.validator.REGISTRY)
        for row in self.packet['rows']:
            check.validate(row['bundle'])
            self.assertEqual(m.validator.structural('spell-dependencies.schema.json',row['dependencies']),[])
            archived=json.loads(((m.ROOT/row['source_header_path']).parent/'receipt.json').read_bytes())
            self.assertEqual(row['bundle']['spell']['identity']['key'],archived['candidate_key'])
            self.assertFalse(row['source_alias_to_existing_native_profile'])
            self.assertTrue(row['authoring_contract_extension_pending'])
            self.assertFalse(row['source_consumer_implemented'])

    def test_current_party_membership_cost_and_subid(self):
        rows=[r for r in self.packet['rows'] if r['bundle']['spell']['execution']['native_behavior']['key']=='party_buff']
        self.assertEqual(len(rows),10)
        for r in rows:
            p=r['bundle']['spell']['execution']['native_behavior']['parameters']
            self.assertEqual(p['membership']['distance_metric'],'max_abs_xyz')
            self.assertFalse(p['membership']['same_floor_required'])
            self.assertFalse(p['membership']['deduplicate'])
            self.assertEqual(p['membership']['distance_lte'],36)
            self.assertIn('subid',p['conditions'][0]['parameters'])
            self.assertEqual(r['bundle']['spell']['costs']['mana'],p['mana']['registrar_base'])
            self.assertTrue(p['combat_presentation']['condition_application_independent_of_area'])
            if p['mana']['check_total_before_presentation']:
                self.assertEqual(p['insufficient_mana']['message'],'RETURNVALUE_NOTENOUGHMANA')
                self.assertEqual(p['insufficient_mana']['order'],['cancel_message','caster_poff','return_false'])

    def test_acquire_overwrites_and_clears_attack_target(self):
        rows=[r for r in self.packet['rows'] if r['bundle']['spell']['execution']['native_behavior']['key']=='acquire_summon']
        self.assertEqual(len(rows),4)
        for r in rows:
            p=r['bundle']['spell']['execution']['native_behavior']['parameters']['inherit_master_attack_target']
            self.assertTrue(p['clear_when_master_has_no_target']);self.assertTrue(p['overwrite_existing'])
            self.assertEqual(p['order'],'after_set_master')

    def test_familiar_full_lifecycle_dynamic_inputs(self):
        rows=[r for r in self.packet['rows'] if r['bundle']['spell']['execution']['native_behavior']['key']=='familiar_summon']
        self.assertEqual(len(rows),10)
        for r in rows:
            p=r['bundle']['spell']['execution']['native_behavior']['parameters']
            self.assertEqual(len(p['warnings']),2)
            self.assertEqual(p['dynamic_inputs']['duration'],'config.FAMILIAR_TIME')
            self.assertEqual(p['expiry']['warning_storage_reset'],-1)
            self.assertFalse(p['condition_sharing']['automatic_clone_all_owner_conditions'])
            self.assertEqual(p['reference_spell_id'],p['condition_sharing']['spell_cooldown_subid'])
            self.assertEqual(len(r['source_proofs']),10)

    def test_swift_actual_wheel_branches_and_precombat_commit(self):
        rows=[r for r in self.packet['rows'] if r['bundle']['spell']['execution']['native_behavior']['key']=='companion_haste']
        self.assertEqual(len(rows),1)
        p=rows[0]['bundle']['spell']['execution']['native_behavior']['parameters']
        self.assertEqual(len(p['wheel_conditions']['none']),3)
        self.assertEqual(p['wheel_conditions']['regular'][0]['damage_dealt_percent'],50)
        self.assertEqual(p['wheel_conditions']['other'],[])
        self.assertTrue(p['combat_failure_returns_false_without_reverting_familiar_haste'])

    def test_closed_exact_schema_refuses_omission_and_added_alias(self):
        check=m.validator.Draft202012Validator(m.private_schema(self.packet),registry=m.validator.REGISTRY)
        row=copy.deepcopy(self.packet['rows'][0]['bundle'])
        del row['spell']['execution']['native_behavior']['parameters']['inherit_master_attack_target']
        self.assertTrue(list(check.iter_errors(row)))
        row=copy.deepcopy(self.packet['rows'][0]['bundle'])
        row['spell']['execution']['native_behavior']['parameters']['alias_old_profile']='accepted-profile'
        self.assertTrue(list(check.iter_errors(row)))

if __name__=='__main__':unittest.main()
