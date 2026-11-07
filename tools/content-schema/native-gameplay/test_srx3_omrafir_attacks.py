#!/usr/bin/env python3
"""Frozen actual body and pinned callback evidence must qualify both Omrafir slots."""
import copy
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent

class OmrafirSourceTests(unittest.TestCase):
    def module(self):
        spec = importlib.util.spec_from_file_location('omrafir_attacks', HERE / 'build_srx3_omrafir_attacks.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_actual_complete_vectors_and_pinned_sources(self):
        result = self.module().build()
        self.assertEqual(len(result['callbacks']), 3)
        self.assertEqual([(s['list'],s['index'],s['schedule']['interval_ms'],s['schedule']['chance_ppm'])
                          for s in result['schedules']], [('attacks',2,2000,150000),('defenses',2,2000,200000)])
        self.assertEqual(len(result['abilities']),2)

    def test_exact_beam_matrices_and_literal_item_heal_rules(self):
        result = self.module().build()
        self.assertEqual([sum(row.count('x') for row in rows)
                          for rows in result['beam_matrices']], [8,8,9,8])
        self.assertTrue(all(len(rows) == 11 and all(len(row) == 11 for row in rows)
                            for rows in result['beam_matrices']))
        self.assertEqual(result['healing']['exact_ids'], [1487,1492,1500])
        self.assertEqual(result['beam_delay_ms'], 4000)
        self.assertEqual(result['beam_damage'], {'minimum':7000,'maximum':10000,'element':'fire'})
        self.assertEqual(result['healing'], {'minimum':7500,'maximum':9000,'health_below_ppm':999900,'source_name':'Omrafir','require_tile':True,'exact_ids':[1487,1492,1500]})
        self.assertEqual(result['regeneration_marker'], {'sub_id':88888,'duration_ms':5000,'health_gain':0})

    def test_event_closure_retains_all_existing_overlay_fields(self):
        module = self.module()
        result = module.build()
        before = __import__('json').loads(module.OUTPUT.read_bytes())
        # Native11 extends this one witness; existing native stats, both source
        # vectors, matrices and spell rules remain byte-for-byte semantic data.
        for key, value in before.items():
            if key == 'healing_fire_items':
                self.assertEqual(result['healing']['exact_ids'],value)
            elif key == 'healing':
                for field, old in value.items():
                    self.assertEqual(result[key][field],old,field)
            else:
                self.assertEqual(result[key], value, key)
        event = result['creature_events']
        self.assertEqual(event['callback']['sha256'], 'bad7511af294bebc5a9d94345252d0ba03784172a73463d772be6350a467cbcf')
        self.assertEqual(event['boom_delay_ms'], 10)
        self.assertEqual(event['reform_delay_ms'], 26764)
        self.assertEqual(event['storage'], {'key':112416,'initial':-1,'normal_reform_below':2})
        self.assertEqual(event['summon'], {'name':'Hellfire Fighter','amount':9,'center':[33589,32379,12],'offset_x':[-8,8],'offset_y':[-9,9],'extended':True,'forced':True})
        self.assertEqual(event['reform'], {'name':'Omrafir','position':[33586,32379,12],'extended':False,'forced':True})
        self.assertEqual(event['terminal_reform'], {'name':'Omrafir2','position':[33586,32379,12],'extended':False,'forced':True,'result':'unknown-monster-nil-then-lua-error'})
        self.assertEqual([r['native']['profile']['target']['key'] for r in result['event_children']], ['oteryn:creature.hellfire_fighter'])

    def test_changed_event_child_body_cannot_be_blessed_by_new_root(self):
        module = self.module()
        native = copy.deepcopy(module.read_native())
        native['source_definitions_sha256'] = 'f' * 64
        record = next(r for r in native['records'] if r['profile']['target']['key'] == 'oteryn:creature.hellfire_fighter')
        record['profile']['data']['profile']['health'] += 1
        with self.assertRaisesRegex(ValueError, 'event child body changed'):
            module.build(native)

    def test_changed_native_body_cannot_be_blessed_by_new_root(self):
        module = self.module()
        native = module.read_native()
        changed = copy.deepcopy(native)
        changed['source_definitions_sha256'] = 'f' * 64
        record = next(r for r in changed['records'] if r['profile']['target']['key'] == 'oteryn:creature.omrafir')
        record['behavior']['data']['profile']['attacks'][0]['chance_ppm'] = 999999
        with self.assertRaisesRegex(ValueError, 'body changed'):
            module.build(changed)

if __name__ == '__main__':
    unittest.main()
