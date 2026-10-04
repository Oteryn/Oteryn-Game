import copy
from collections import Counter
import unittest
import jsonschema
import source_monster_slot_semantics as producer


class MonsterSourceSlotTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = producer.build()

    def test_all175_population_and_inert_flags(self):
        producer.validate(self.packet)
        self.assertEqual((175,156,19,21,141), tuple(self.packet[k] for k in
                         ['slot_count','registered_slot_count','inline_slot_count','custom_descriptor_slot_count','source_program_count']))
        self.assertEqual(0,self.packet['candidate_count'])
        self.assertTrue(all(s['source_data_complete'] and not s['full_slot_projection_complete']
                            and not s['native_provider_qualified'] and s['projection_gaps']
                            for s in self.packet['slots']))

    def test_real_parameters_callbacks_effects_conditions(self):
        operations=[op for p in self.packet['source_programs'] for op in p['mechanic_operations']]
        kinds=Counter(op['category'] for op in operations)
        for kind in ['combat_constructor','condition_constructor','parameter_binding','callback_binding',
                     'combat_execute','world_magic_effect','scheduled_callback']:
            self.assertGreater(kinds[kind],0)
        roles={op['parameter_semantic_role'] for op in operations}
        self.assertTrue({'combat_type','impact_effect','condition_duration'} <= roles)
        self.assertTrue(any(op['control_ancestor_refs'] for op in operations))

    def test_exactregistry_slot_program_mapping(self):
        links,_=producer.load_inputs()
        self.assertEqual([x['slot_identity'] for x in links], [x['slot_identity'] for x in self.packet['slots']])
        for link,slot in zip(links,self.packet['slots']):
            self.assertEqual(link['source_parameters'],slot['source_parameters'])
            if link['registered_source']:
                ast=self.packet['source_programs'][slot['source_program_index']]['source_syntax']
                self.assertEqual(link['registered_source']['sha256'],ast['source_sha256'])
                self.assertEqual(link['source'],ast['source'])

    def test_omitted_mechanic_or_wrong_argument_rejected(self):
        for change in ['omit','argument']:
            packet=copy.deepcopy(self.packet)
            operations=packet['source_programs'][0]['mechanic_operations']
            if change=='omit':operations.pop()
            else:operations[0]['arguments']=[{'node_ref':0,'ast_kind':'Name','symbol':'WRONG',
                                             'numeric_literal':None,'string_literal':None,
                                             'boolean_literal':None,'nil_literal':False}]
            with self.assertRaises(ValueError):producer.validate(packet)

    def test_duplicate_slot_rejected(self):
        packet=copy.deepcopy(self.packet)
        packet['slots'][1]=packet['slots'][0]
        with self.assertRaises(ValueError):producer.validate(packet)

    def test_activation_and_unknownfields_rejected(self):
        for key,value in [('native_admission',True),('untyped_extra',{})]:
            packet=copy.deepcopy(self.packet);packet[key]=value
            with self.assertRaises(jsonschema.ValidationError):producer.validate(packet)

    def test_full_ast_controlflow_retained(self):
        _,syntax=producer.load_inputs()
        actual={(p['source_syntax']['source'],p['source_syntax']['path']):p['source_syntax']
                for p in self.packet['source_programs']}
        self.assertEqual(syntax,actual)
        self.assertTrue(all(p['program_chronology']=='full_AST_control_flow_not_call_preorder_execution_order'
                            for p in self.packet['source_programs']))


if __name__=='__main__':unittest.main()
