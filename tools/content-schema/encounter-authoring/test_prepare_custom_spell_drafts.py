"""Check literal coverage and fail-closed templates; this never executes a game runtime."""
import json
import copy
import os
import tempfile
import unittest
from pathlib import Path
from lupa.luajit21 import LuaRuntime

import prepare_custom_spell_drafts as prep


@unittest.skipUnless(os.environ.get('OTERYN_CANARY') and os.environ.get('OTERYN_CUSTOM_PACKET'),
                     'set exact Canary and current qualified packet paths')
class TypedDrafts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.canary = Path(os.environ['OTERYN_CANARY'])
        cls.packet = Path(os.environ['OTERYN_CUSTOM_PACKET'])
        cls.result = prep.prepare(cls.canary, cls.packet)
        cls.rows = {row['spell'].lower(): row for row in json.loads(cls.packet.read_text())['spells']}

    def test_all_40_boundaries_stay_explicit_and_no_full_conversion_is_claimed(self):
        self.assertEqual(40, len(self.result['ledger']))
        self.assertEqual(43, sum(row['open_references'] for row in self.result['ledger']))
        self.assertEqual(4, sum(row['preparation'] == 'ACCEPTED_TYPED_CORE_DRAFT' for row in self.result['ledger']))
        self.assertFalse(self.result['full_spell_conversion_claimed'])
        self.assertFalse(self.result['admission_authorized'])
        self.assertFalse(self.result['runtime_qualified'])
        self.assertTrue(all(not row['complete_spell_conversion_claimed'] for row in self.result['ledger']))

    def check_packet_mutation(self, mutate, error):
        packet = json.loads(self.packet.read_text())
        mutate(packet)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'packet.json'
            path.write_text(json.dumps(packet))
            with self.assertRaisesRegex(ValueError, error):
                prep.prepare(self.canary, path)

    def test_declared_open_reference_count_cannot_disagree_with_actual_dispositions(self):
        def mutate(packet):
            row = next(r for r in packet['spells'] if r['spell'] == 'charge vortex')
            row['current_open_references'] = 2
        self.check_packet_mutation(mutate, 'reference count mismatch')

    def test_consistent_extra_reference_must_not_be_reported_as_frozen43(self):
        def mutate(packet):
            row = next(r for r in packet['spells'] if r['spell'] == 'charge vortex')
            row['current_monster_dispositions'].append(copy.deepcopy(row['current_monster_dispositions'][0]))
            row['current_open_references'] = 2
        self.check_packet_mutation(mutate, 'exactly43')

    def test_duplicate_selected_schedule_cannot_hide_inside_consistent43_total(self):
        def mutate(packet):
            generator = next(r for r in packet['spells'] if r['spell'] == 'generator')
            generator['current_monster_dispositions'].append(copy.deepcopy(generator['current_monster_dispositions'][0]))
            generator['current_open_references'] = 2
            ring = next(r for r in packet['spells'] if r['spell'] == 'targetfirering')
            ring['current_monster_dispositions'].pop()
            ring['current_open_references'] = 1
        self.check_packet_mutation(mutate, 'exactly one source schedule')

    def test_maxxen_uses_every_source_tile_with_uniform_weights_and_ordered_visuals(self):
        candidate = self.result['candidates']['professor_maxxen']
        rule = next(row for row in candidate['encounter.json']['rules'] if row['key'] == 'round3_maxxenteleport')
        self.assertEqual(['cast', 'one_of', 'cast'], [row['kind'] for row in rule['actions']])
        x_branches = rule['actions'][1]['branches']
        self.assertEqual(15, len(x_branches))
        self.assertEqual({1}, {row['weight'] for row in x_branches})
        self.assertTrue(all(len(row['actions'][0]['branches']) == 14 for row in x_branches))
        branches = [choice for row in x_branches for choice in row['actions'][0]['branches']]
        self.assertEqual(210, len(branches))
        self.assertEqual({1}, {row['weight'] for row in branches})
        anchors = {row['key']: row['location'] for row in candidate['encounter.json']['anchors']}
        points = {(anchors[row['actions'][0]['to']]['x'], anchors[row['actions'][0]['to']]['y'],
                   anchors[row['actions'][0]['to']]['floor']) for row in branches}
        self.assertEqual({(x, y, 15) for x in range(33704, 33719) for y in range(32040, 32054)}, points)
        self.assertTrue(all(row['actions'][0]['who'] == {'triggering': True} for row in branches))
        self.assertNotIn('after_ms', rule['actions'][1])

    def test_generator_four_positions_and_spawned_speaker_and_zamulosh_literal(self):
        candidate = self.result['candidates']['professor_maxxen']
        rule = next(row for row in candidate['encounter.json']['rules'] if row['key'] == 'round3_generator')
        self.assertEqual(4, len(rule['actions'][0]['branches']))
        for branch in rule['actions'][0]['branches']:
            self.assertEqual(1, branch['weight'])
            self.assertEqual(['spawn', 'say'], [a['kind'] for a in branch['actions']])
            self.assertEqual({'spawned': True}, branch['actions'][1]['subject'])
        zamu = self.result['candidates']['zamulosh']
        rule = zamu['encounter.json']['rules'][-1]
        self.assertEqual(['cast', 'teleport', 'cast'], [row['kind'] for row in rule['actions']])
        anchor = next(a for a in zamu['encounter.json']['anchors'] if a['key'] == rule['actions'][1]['to'])
        self.assertEqual({'x': 33644, 'y': 32757, 'floor': 11}, anchor['location'])

    def test_unknown_side_effect_or_nonliteral_teleport_does_not_create_a_draft(self):
        row = self.rows['zamulosh tp']
        text = prep.pinned(self.canary, row['source']['file']).decode()
        with self.assertRaisesRegex(ValueError, 'outside'):
            prep.destinations('zamulosh tp', text.replace('\treturn\n', '\tcreature:remove()\n\treturn\n'))
        with self.assertRaisesRegex(ValueError, 'source literals'):
            prep.destinations('zamulosh tp', text.replace('33644, 32757, 11', 'Quest.x, 32757, 11'))

    def test_compiled_source_constant_oracle_reports_undefined_damage_without_adopting_fire(self):
        oracle = self.result['targetfirering_source_bug']
        self.assertEqual('0 4 4', oracle['compiled_enum_values'])
        self.assertIn('UNDEFINEDDAMAGE4', oracle['mechanism'])
        self.assertEqual('UNKNOWN_NOT_ADOPTED', oracle['D25_intended_wiki_behaviour'])
        self.assertFalse(oracle['admission_authorized'])
        self.assertEqual(6, len(oracle['sources']))
        self.assertEqual(3, oracle['exact_source_lua_parameter_calls'])
        self.assertEqual(4, oracle['recorded_final_combat_type'])

    def test_literal_source_schedules_survive_without_invented_exhaustion(self):
        rows = {row['spell'].lower(): row for row in self.result['source_schedule_bindings']}
        self.assertEqual((30000, 50), (rows['generator']['typed_schedule']['interval_ms'], rows['generator']['typed_schedule']['chance_percent']))
        self.assertEqual((2000, 5), (rows['maxxenteleport']['typed_schedule']['interval_ms'], rows['maxxenteleport']['typed_schedule']['chance_percent']))
        self.assertEqual((2000, 15), (rows['zamulosh tp']['typed_schedule']['interval_ms'], rows['zamulosh tp']['typed_schedule']['chance_percent']))
        self.assertTrue(all('cooldown' not in [c['method'] for c in row['registered_source_calls']] for row in rows.values()))

    def test_lost_time_schedules_bind_both_real_forms_and_direction_flag(self):
        rows = [r for r in self.result['source_schedule_bindings'] if r['spell'] == 'time guardian lost time']
        self.assertEqual(2, len(rows))
        self.assertEqual(set(prep.LOST_TIME_ROLES), {Path(r['source_file']).stem for r in rows})
        self.assertTrue(all(r['source_field'] == 'defenses[1]' for r in rows))
        ability = self.result['candidates']['the_time_guardian']['dependencies.json']['abilities'][0]
        self.assertTrue(ability['needs_direction'])
        self.assertFalse(ability['needs_target'])

    def test_duplicate_lost_time_form_cannot_hide_in_two_bindings(self):
        def mutate(packet):
            row = next(r for r in packet['spells'] if r['spell'] == 'time guardian lost time')
            row['current_monster_dispositions'][1] = copy.deepcopy(row['current_monster_dispositions'][0])
        self.check_packet_mutation(mutate, 'two distinct source form schedules')

    def test_lost_time_template_keeps_literal_spaces_and_rejects_changed_semantics(self):
        row = self.rows['time guardian lost time']
        text = prep.pinned(self.canary, row['source']['file']).decode()
        position = prep.pinned(self.canary, 'src/game/movement/position.hpp').decode()
        for old, new in [('pos.z ~= 15', 'pos.z ~= 14'), ('"lost time"', '"losttime"'),
                         ('"the freezing time guardian"', '"thefreezingtimeguardian"'),
                         ('math.random(-2, 2)', 'math.random(-3, 2)'),
                         ('spell:needDirection(true)', 'spell:needDirection(false)')]:
            with self.assertRaisesRegex(ValueError, 'whole registered source template'):
                prep.lost_time_rules(text.replace(old, new, 1), position)
        with self.assertRaisesRegex(ValueError, 'coordinate domain'):
            prep.lost_time_rules(text, position.replace('uint16_t x', 'uint32_t x'))

    def test_lost_time_nested_draws_match_actual_source_all_fifty_offsets(self):
        text = prep.pinned(self.canary, self.rows['time guardian lost time']['source']['file']).decode()
        candidate = self.result['candidates']['the_time_guardian']
        for role, actor in prep.LOST_TIME_ROLES.items():
            rule = next(r for r in candidate['encounter.json']['rules'] if r['key'] == role + '_casts_' + actor)
            for x in range(-2, 3):
                for y in range(-2, 3):
                    lua = LuaRuntime(unpack_returned_tuples=True)
                    lua.execute('''calls={}; cursor=0
                      function Spell() instance=setmetatable({}, {__index=function() return function() end end}); return instance end
                      Game={createMonster=function(name,pos) calls[#calls+1]={name=name,x=pos.x,y=pos.y,z=pos.z} end}
                      function math.random(low,high) assert(low==-2 and high==2); cursor=cursor+1; return draws[cursor] end''')
                    lua.globals().draws = lua.table_from([x, y])
                    lua.execute(text)
                    position = lua.table_from({'x': 32977, 'y': 31662, 'z': 15})
                    creature = lua.table()
                    creature.getPosition = lambda self: position
                    creature.getName = lambda self: role.replace('_', ' ').title()
                    lua.globals().instance.onCastSpell(creature, lua.table())
                    self.assertEqual(2, lua.globals().cursor)
                    actual = dict(lua.globals().calls[1].items())
                    self.assertEqual({'name': actor.replace('_', ' '), 'x': 32977+x, 'y': 31662+y, 'z': 15}, actual)
                    action = rule['actions'][0]['branches'][x+2]['actions'][0]['branches'][y+2]['actions'][0]
                    self.assertEqual({'relative': {'x': x, 'y': y}}, action['at'])
                    self.assertEqual('none', action['owner'])
                    self.assertEqual(1, action['count'])
        original = json.loads((prep.ROOT / 'samples/the_time_guardian/manifest.json').read_text())
        self.assertEqual(original['covers'], candidate['manifest.json']['covers'])
        self.assertEqual({'x': [0, 65535], 'y': [0, 65535], 'floor': 15},
                         candidate['encounter.json']['anchors'][-1]['location']['boxes'][0])

    def test_lost_time_actual_source_rejects_other_floors_and_unlisted_caster(self):
        text = prep.pinned(self.canary, self.rows['time guardian lost time']['source']['file']).decode()
        for name, floor in [('The Freezing Time Guardian', 14), ('The Blazing Time Guardian', 13),
                            ('The Time Guardian', 15)]:
            lua = LuaRuntime(unpack_returned_tuples=True)
            lua.execute('''function Spell() instance=setmetatable({}, {__index=function() return function() end end}); return instance end
              Game={createMonster=function() error("no spawn is allowed in this scenario") end}
              function math.random() error("no RNG draw is allowed in this scenario") end''')
            lua.execute(text)
            position = lua.table_from({'x': 32977, 'y': 31662, 'z': floor})
            creature = lua.table()
            creature.getPosition = lambda self: position
            creature.getName = lambda self: name
            self.assertTrue(lua.globals().instance.onCastSpell(creature, lua.table()))


if __name__ == '__main__':
    unittest.main()
