#!/usr/bin/env python3
"""Regeneration marker evidence must retain its condition namespace and integer rules."""
import importlib.util
from pathlib import Path
import unittest

class MarkerAuditTests(unittest.TestCase):
    def module(self):
        path=Path(__file__).with_name('build_regeneration_marker_audit.py')
        spec=importlib.util.spec_from_file_location('marker_audit',path)
        module=importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_literal_shared_regeneration_condition_retains_full_subid_and_zero_integer_gain(self):
        body=b'''local condition2 = Condition(CONDITION_REGENERATION, CONDITIONID_DEFAULT)
condition2:setParameter(CONDITION_PARAM_SUBID, 88888)
condition2:setParameter(CONDITION_PARAM_TICKS, 15 * 60 * 1000)
condition2:setParameter(CONDITION_PARAM_HEALTHGAIN, 0.01)
'''
        self.assertEqual(self.module().classify(body),{'sub_id':88888,'duration_ms':900000,'health_gain':0,'declaration_line':1,'sub_id_line':2})

    def test_storage_and_damage_literals_do_not_become_condition_markers(self):
        self.assertIsNone(self.module().classify(b'monster:setStorageValue(88888, essenceCount)'))
        self.assertIsNone(self.module().classify(b'local damage = math.random(66666, 88888)'))

    def test_cache_corruption_refuses_pinned_git_identity(self):
        with self.assertRaisesRegex(ValueError,'Git blob'):
            self.module().verify_blob('0'*40,b'changed source')

    def test_spell_registration_and_speech_do_not_invent_monster_vector_callers(self):
        body = b'''spell:name("unused heal")
creature:say("Lisa healed up!", TALKTYPE_MONSTER_SAY)
monster.defenses = {
    { name = "Lisa Heal", interval = 1000, chance = 100 },
}
'''
        self.assertEqual(self.module().spell_reference_lines(body, ["lisa heal", "unused heal"]),
                         [{"name":"lisa heal", "line":4, "text":'{ name = "Lisa Heal", interval = 1000, chance = 100 },'}])

if __name__=='__main__': unittest.main()
