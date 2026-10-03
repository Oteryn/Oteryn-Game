"""Static converter enrichment: synthetic fixtures, no reference dialogue text."""
import json
import tempfile
import unittest
from unittest import mock
from pathlib import Path

import ots_interactions as converter


class ConverterEnrichmentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        (self.repo / 'data/items').mkdir(parents=True)
        (self.repo / 'data/items/items.xml').write_text(
            '<items><item id="300" name="test reward"/><item id="301" name="duplicate"/>'
            '<item id="302" name="duplicate"/><item fromid="310" toid="320" name="range"/></items>')

    def script(self, globals='', body=''):
        text = globals + '\nlocal action = Action()\nfunction action.onUse(player, item, fromPosition, target, toPosition)\n' + body + '\nend\naction:id(100)\n'
        (self.repo / 'fixture.lua').write_text(text)
        script = converter.Script('canary', self.repo, 'fixture.lua', {}, 'fixture')
        number = next(n for n, line in enumerate(script.lines, 1) if line.startswith('function action'))
        script.bind(number, 'onUse')
        script.current_line = len(script.lines)
        return script

    def test_complete_call_respects_quotes(self):
        text = '"name, (x)", Position(1, 2, 7)) trailing'
        args = converter.call_complete(text, 0)
        self.assertEqual(converter.split_args(args), ['"name, (x)"', 'Position(1, 2, 7)'])
        self.assertIsNone(converter.call_complete('Position(1, 2, 7)', 0))

    def test_static_position_alias_and_table(self):
        s = self.script('local config = { destination = Position(10, 20, 7) }',
                        'player:teleportTo(config.destination)')
        self.assertEqual(s.position('config.destination'), (10, 20, 7))
        self.assertEqual(s.position('Position(config.destination)'), (10, 20, 7))
        result = s.interactions()[0]
        self.assertEqual(list(converter.walk(result['rules']))[0]['target']['kind'], 'anchor')
        self.assertEqual(result['anchors'][0]['source_position'], {'x': 10, 'y': 20, 'z': 7})

    def test_dynamic_mutated_shadowed_positions_stay_blocked(self):
        cases = ['local pos = Position(1, 2, 7)\npos.x = pos.x + 1',
                 'local pos = Position(1, 2, 7)\nlocal pos = Position(3, 4, 7)',
                 'local pos = Position(1, 2, 7)\npos:moveUpstairs()',
                 'local pos = config[player:getId()]']
        for declarations in cases:
            with self.subTest(declarations=declarations):
                s = self.script(declarations, 'player:teleportTo(pos)')
                child = list(converter.walk(s.interactions()[0]['rules']))[-1]
                self.assertEqual(child['status'], 'blocked')
        s = self.script('local config = { {x=1,y=2,z=7} }')
        self.assertIsNone(s.position('config[index]'))
        self.assertEqual(s.position('Position(1 + 2, 3, 7)'), (3, 3, 7))

    def test_unsupported_table_does_not_erase_sibling(self):
        s = self.script('local broken = { otherCall() }\nlocal good = { stage = 3, storage = Storage.Test.Stage }')
        self.assertEqual(s.literal('good.stage'), 3)
        self.assertEqual(s.resolve_storage('good.storage'), 'Storage.Test.Stage')
        self.assertIsNone(s.literal('broken'))

    def test_item_names_and_constants_resolve_exactly(self):
        s = self.script('local ITEM = "test reward"\nlocal COUNT = 2', 'player:addItem(ITEM, COUNT)\nplayer:removeItem(ITEM, COUNT)')
        children = list(converter.walk(s.interactions()[0]['rules']))
        self.assertEqual([c['request'] for c in children], ['hand_out', 'consume'])
        self.assertTrue(all(c['item']['key'] == 'canary:item/300' and c['count'] == 2 for c in children))
        self.assertIsNone(s.item_id('"duplicate"'))
        self.assertIsNone(s.item_id('"range"'))
        self.assertIsNone(s.item_id('"missing"'))
        self.assertEqual(s.item_id('300 + 1'), 301)
        self.assertEqual(s.item_id('0x12c'), 300)

    def test_boolean_precedence_parentheses_and_negation(self):
        s = self.script('local threshold = 3')
        c = s.condition('(player:getLevel() >= threshold or item:getId() == 300) and not (player:getLevel() < 2)', 999)
        self.assertIn('all', c)
        self.assertIn('any', c['all'][0])
        self.assertTrue(c['all'][1]['negate'])
        self.assertEqual(c['all'][0]['any'][1]['object']['field'], 'item_type')
        c = s.condition('player:getLevel() >= 3 or item:getId() == 300 and player:getLevel() < 10', 999)
        self.assertIn('all', c['any'][1])

    def test_getid_does_not_confuse_actor_or_use_target(self):
        s = self.script()
        for receiver in ('player', 'target'):
            self.assertIn('unresolved', s.condition(receiver + ':getId() == 300', 999))
        self.assertIn('actor_item_count', s.condition('player:getItemCount("test reward") >= 1', 999))
        self.assertIn('unresolved', s.condition('player:getItemCount("duplicate") >= 1', 999))

    def test_runtime_and_sibling_aliases_stay_unknown(self):
        s = self.script('', 'player:teleportTo(pos)\nlocal pos = Position(1, 2, 7)')
        first = list(converter.walk(s.interactions()[0]['rules']))[0]
        self.assertEqual(first['status'], 'blocked')
        self.assertIn('unresolved', s.condition('player:getLevel() >= math.random(10)', 999))
        self.assertIn('unresolved', s.condition('player:getLevel() >= 1 and unknownCall()', 999))
        s = self.script('', 'player:teleportTo(config.pos)\nlocal config = {pos={x=1,y=2,z=7}}')
        self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_branch_local_does_not_leak_to_sibling(self):
        s = self.script('', 'if player:getLevel() >= 1 then\nlocal pos = Position(1, 2, 7)\n'
                        'else\nplayer:teleportTo(pos)\nend')
        self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_alias_escape_and_inline_else_mutation_are_held(self):
        mutations = ['local other = config\nother.pos.x = 9',
                     'mutate(config)', 'mutate(config.pos)',
                     'local other = config.pos\nother.x = 9',
                     'if unknown then print(1) else config.pos.x = 9 end',
                     'print("--") ; config.pos.x = 9']
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                s = self.script('local config = {pos={x=1,y=2,z=7}}\n' + mutation,
                                'player:teleportTo(config.pos)')
                self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_forward_scalar_and_global_branch_aliases_are_held(self):
        cases = ['local COUNT = LATE\nlocal LATE = 2',
                 'if unknown then\nlocal COUNT = 2\nend',
                 'local COUNT=2\nif unknown then print(1) else COUNT=3 end']
        for declarations in cases:
            s = self.script(declarations, 'player:addItem(300, COUNT)')
            child = list(converter.walk(s.interactions()[0]['rules']))[0]
            self.assertIn('value_source_line', child)
            self.assertNotIn('count', child)

    def test_primitive_table_leaf_escape_is_safe_and_forward_leaf_is_held(self):
        s = self.script('local config={item=300,count=2}\nlocal COUNT=config.count',
                        'local reward=player:addItem(config.item, COUNT)')
        child = list(converter.walk(s.interactions()[0]['rules']))[0]
        self.assertEqual(child['count'], 2)
        s = self.script('local config={item=300,count=LATE}\nlocal LATE=2',
                        'player:addItem(config.item, config.count)')
        self.assertIn('value_source_line', list(converter.walk(s.interactions()[0]['rules']))[0])

    def test_multiline_table_and_position_alias_escapes_are_held(self):
        for declarations, target in [
            ('local config={pos={x=1,y=2,z=7}}\nlocal other=\nconfig', 'config.pos'),
            ('local pos=Position(1,2,7)\nlocal other=pos\nother.x=9', 'pos')]:
            s = self.script(declarations, 'player:teleportTo(' + target + ')')
            self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_table_iteration_can_escape_mutable_values(self):
        for iterate in ('pairs', 'ipairs'):
            s = self.script('local config={pos={x=1,y=2,z=7}}\n'
                            'for _,p in ' + iterate + '(config) do p.x=9 end',
                            'player:teleportTo(config.pos)')
            self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_branch_local_literals_are_only_visible_inside_their_branch(self):
        s = self.script('', 'if player:getLevel() >= 1 then\nlocal destination = Position(1,2,7)\n'
                        'player:teleportTo(destination)\nelse\nplayer:teleportTo(destination)\nend\n'
                        'player:teleportTo(destination)')
        children = list(converter.walk(s.interactions()[0]['rules']))
        self.assertEqual(children[0]['target']['kind'], 'anchor')
        self.assertEqual([c.get('status') for c in children[1:]], ['blocked', 'blocked'])

    def test_nested_branch_can_read_parent_local_but_not_sibling(self):
        s = self.script('', 'if player:getLevel() >= 1 then\nlocal COUNT=2\n'
                        'if item.itemid == 100 then\nplayer:addItem(300,COUNT)\nend\n'
                        'else\nplayer:addItem(300,COUNT)\nend')
        children = list(converter.walk(s.interactions()[0]['rules']))
        self.assertEqual(children[0]['count'], 2)
        self.assertIn('value_source_line', children[1])

    def test_integer_folding_and_static_index_are_bounded(self):
        s = self.script('local COUNT=2 * (3 + 1)\nlocal INDEX=2\nlocal config={{x=1,y=2,z=7},{x=3,y=4,z=7}}')
        self.assertEqual(s.literal('COUNT'), 8)
        self.assertEqual(s.position('config[INDEX]'), (3,4,7))
        self.assertEqual(s.literal('-2 * (3-1)'), -4)
        self.assertEqual(s.literal('3-1'),2)
        for expr in ['1/2','1.5*2','math.random(2)+1','runtime+2','2**100',
                     '9007199254740991*2', '(' * 40+'1+2'+')'*40,
                     '1+'*200+'1', '1+1#not_lua', '-'*40+'1']:
            with self.subTest(expr=expr):
                self.assertIsNone(s.literal(expr))
        self.assertIsNone(s.position('Position(65535+1,2,7)'))
        self.assertIsNone(s.position('Position(1,2,15+1)'))
        self.assertIsNone(s.position('Position(fromPosition.x + 1,2,7)'))

    def test_literal_membership_uses_existing_predicates_and_negation(self):
        s = self.script()
        condition = s.condition('table.contains({300,301}, item.itemid)', 999)
        self.assertEqual(len(condition['any']), 2)
        condition = s.condition('not table.contains({300,301}, item:getId())', 999)
        self.assertTrue(all(c['negate'] for c in condition['all']))
        self.assertIn('unresolved', s.condition('table.contains({300,runtime}, item.itemid)', 999))
        self.assertIn('unresolved', s.condition('table.contains({300,301}, player:getId())', 999))
        self.assertIn('unresolved', s.condition('table.contains({}, item.itemid)', 999))

    def test_getplayer_and_storage_snapshot_aliases_cannot_leak(self):
        s = self.script('', 'if item.itemid == 100 then\nlocal stage=player:getStorageValue(Storage.Test.Stage)\n'
                        'if stage == 1 then\nplayer:addItem(300,1)\nend\n'
                        'else\nif stage == 1 then\nplayer:addItem(300,1)\nend\nend')
        result=s.interactions()[0]
        conditions=list(converter.conditions(result['rules']))
        self.assertTrue(any('quest_stage' in c for c in conditions))
        self.assertTrue(any('unresolved' in c for c in conditions))
        self.assertIn('unresolved', s.condition('later == 1', 1))

    def test_storage_snapshot_is_not_a_live_read_after_mutation(self):
        s=self.script('', 'local stage=player:getStorageValue(Storage.Test.Stage)\n'
                      'player:setStorageValue(Storage.Test.Stage,2)\nif stage == 1 then\nplayer:addItem(300,1)\nend')
        conditions=list(converter.conditions(s.interactions()[0]['rules']))
        self.assertIn('unresolved',conditions[0])

    def test_callback_player_alias_is_local_and_declared_before_use(self):
        text=('local move=MoveEvent()\nfunction move.onStepIn(creature,item,position,fromPosition)\n'
              'if player then\nend\nlocal player=creature:getPlayer()\n'
              'if player then\nend\nend\n'
              'local other=MoveEvent()\nfunction other.onStepIn(creature,item,position,fromPosition)\n'
              'local player=creature:getPlayer()\nif player then\nend\nend\n')
        (self.repo/'fixture.lua').write_text(text)
        script=converter.Script('canary',self.repo,'fixture.lua',{},'fixture')
        results=script.interactions()
        first=list(converter.conditions(results[0]['rules']))
        second=list(converter.conditions(results[1]['rules']))
        self.assertIn('unresolved',first[0])
        self.assertIn('actor_is_player',first[1])
        self.assertIn('actor_is_player',second[0])

    def test_copying_engine_calls_admit_position_without_alias_escape(self):
        s = self.script('local config={position=Position(10,20,7)}',
                        'local boss=Game.createMonster("fixture boss",config.position)\n'
                        'Game.createItem(300,1,config.position)\nplayer:teleportTo(config.position)')
        children=list(converter.walk(s.interactions()[0]['rules']))
        self.assertEqual(children[0]['owner'],'Ability')
        self.assertIn('anchor',children[0])
        self.assertEqual(children[1]['operation'],'CREATE')
        self.assertIn('anchor',children[1])
        self.assertEqual(children[2]['target']['kind'],'anchor')
        for call in ['mutator.createMonster(config.position)', 'createMonster(config.position)',
                     'Game.customCreate(config.position)']:
            s = self.script('local config={position=Position(10,20,7)}',call+'\nplayer:teleportTo(config.position)')
            self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[-1]['status'],'blocked')

    def test_engine_copy_call_whitelist_rejects_shadowed_game(self):
        for declaration in ['local Game=custom', 'Game.createMonster=custom',
                            'function helper(Game)\nend', 'local alternate=Game\nalternate.createMonster=custom', 'mutate(Game)']:
            s=self.script('local config={position=Position(1,2,7)}\n'+declaration,
                          'Game.createMonster("fixture boss",config.position)\nplayer:teleportTo(config.position)')
            self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[-1]['status'],'blocked')

    def test_summon_does_not_find_literal_inside_computed_argument(self):
        s=self.script('', 'Game.createMonster("fixture boss", compute(Position(1,2,7)))')
        child=list(converter.walk(s.interactions()[0]['rules']))[0]
        self.assertIn('anchor_source_line',child)
        self.assertNotIn('anchor',child)
        self.assertEqual(s.interactions()[0]['anchors'],[])

    def test_source_complete_storage_rhs_does_not_clip_nested_expression(self):
        s=self.script('local STAGE=1+2', 'player:setStorageValue(Storage.Test.Stage, STAGE)\n'
                      'player:setStorageValue(Storage.Test.Stage, compute(1))')
        children=list(converter.walk(s.interactions()[0]['rules']))
        self.assertEqual(children[0]['to'],3)
        self.assertIn('value_source_line',children[1])

    def test_h1_all_effect_receivers_require_scope_and_declaration_proof(self):
        calls=['p:addItem(300,1)', 'local reward=p:addItem(300,1)', 'p:removeItem(300,1)',
               'p:addAchievement("fixture")', 'p:addOutfitAddon(100,1)', 'p:addOutfit(100)',
               'p:addMount(1)', 'p:addExperience(10)', 'p:setStorageValue(Storage.Test.Stage,1)',
               'p:remove()', 'p:teleportTo(Position(1,2,7))']
        for call in calls:
            with self.subTest(call=call):
                script=self.script('', 'if item.itemid==100 then\nlocal p=player:getPlayer()\nelse\n'+call+'\nend')
                result=script.interactions()[0]
                self.assertEqual(list(converter.walk(result['rules'])),[])
                self.assertTrue(result['unresolved'])
                script=self.script('', call+'\nlocal p=player:getPlayer()')
                self.assertEqual(list(converter.walk(script.interactions()[0]['rules'])),[])
                self.assertTrue(script.interactions()[0]['unresolved'])
        script=self.script('', 'local p=player:getPlayer()\np:addItem(300,1)\np:removeItem(300,1)')
        self.assertEqual([c['request'] for c in converter.walk(script.interactions()[0]['rules'])], ['hand_out','consume'])

    def movement_script(self, rhs):
        text=('local move=MoveEvent()\nfunction move.onStepIn(creature,item,position,fromPosition)\n'
              'local p='+rhs+'\nif p:getLevel()>=1 then\np:addItem(300,1)\nend\nend\n')
        (self.repo/'fixture.lua').write_text(text)
        return converter.Script('canary',self.repo,'fixture.lua',{},'fixture')

    def test_h2_getplayer_rhs_is_complete_and_comments_are_lexical(self):
        for rhs in ['creature:getPlayer() or otherPlayer', 'creature:getPlayer().owner']:
            result=self.movement_script(rhs).interactions()[0]
            self.assertIn('unresolved',list(converter.conditions(result['rules']))[0])
            self.assertEqual(list(converter.walk(result['rules'])),[])
            self.assertTrue(result['unresolved'])
        result=self.movement_script('creature:getPlayer() -- fixture comment').interactions()[0]
        self.assertIn('actor_level',list(converter.conditions(result['rules']))[0])
        self.assertEqual(list(converter.walk(result['rules']))[0]['request'],'hand_out')

    def test_h3_table_membership_requires_pristine_library_binding(self):
        declarations=['local table={contains=function() return false end}',
                      'table.contains=function() return false end',
                      'table.contains, other=custom,nil',
                      'local other=table\nother.contains=custom', 'mutate(table)',
                      'table["contains"]=custom', 'function helper(table)\nend']
        for declaration in declarations:
            with self.subTest(declaration=declaration):
                script=self.script(declaration,'if table.contains({100},item.itemid) then\nplayer:addItem(300,1)\nend')
                self.assertIn('unresolved',list(converter.conditions(script.interactions()[0]['rules']))[0])
        script=self.script('', 'if table.contains({100},item.itemid) then\nplayer:addItem(300,1)\nend')
        self.assertIn('object',list(converter.conditions(script.interactions()[0]['rules']))[0])

    def test_h4_tuple_assignment_disables_engine_copy_and_effect_assumptions(self):
        mutations=['Game.createMonster, extra=custom,nil', 'extra, Game.createMonster=nil,custom',
                   'Game["createMonster"], extra=custom,nil',
                   'Game.createItem, extra=custom,nil', 'local other=Game\nother.createMonster=custom']
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                script=self.script('local pos=Position(1,2,7)\n'+mutation,
                                   'Game.createMonster("fixture boss",pos)\nGame.createItem(300,1,pos)\nplayer:teleportTo(pos)')
                result=script.interactions()[0]
                children=list(converter.walk(result['rules']))
                self.assertFalse(any(c['owner']=='Ability' or c.get('operation')=='CREATE' for c in children))
                self.assertEqual(children[-1]['status'],'blocked')
                self.assertEqual(len(result['unresolved']),2)

    def test_h5_call_spans_skip_preceding_short_and_long_quoted_lookalikes(self):
        for quoted in ["'Game.createMonster(\"ghost\",Position(1,2,7))'",
                       '[=[Game.createMonster("ghost",Position(1,2,7))]=]']:
            script=self.script('', 'local monster=select(2,'+quoted+',Game.createMonster("real",Position(3,4,7)))')
            result=script.interactions()[0]
            child=list(converter.walk(result['rules']))[0]
            self.assertEqual(child['creature']['key'],'canary:creature/real')
            self.assertEqual(result['anchors'][0]['source_position'],{'x':3,'y':4,'z':7})
        script=self.script('', "local wall=select(2,'Game.createItem(301,1,Position(1,2,7))',Game.createItem(300,1,Position(3,4,7)))")
        result=script.interactions()[0]
        self.assertEqual(list(converter.walk(result['rules']))[0]['def']['key'],'canary:item/300')
        self.assertEqual(result['anchors'][0]['source_position'],{'x':3,'y':4,'z':7})
        script=self.script('', "local reward=select(2,'player:addItem(301,1)',player:addItem(300,1))")
        self.assertEqual(list(converter.walk(script.interactions()[0]['rules']))[0]['item']['key'],'canary:item/300')

    def test_h5_multiline_literal_calls_and_headers_are_never_callbacks_or_effects(self):
        script=self.script('', 'local text=[=[\nfunction fake.onUse(player,item)\n'
                           'Game.createMonster("ghost",Position(1,2,7))\nend\n]=]\n'
                           'Game.createMonster("real",Position(3,4,7))')
        results=script.interactions()
        self.assertEqual(len(results),1)
        children=list(converter.walk(results[0]['rules']))
        self.assertEqual([c['creature']['key'] for c in children if c['owner']=='Ability'],['canary:creature/real'])
        self.assertEqual(results[0]['anchors'][0]['source_position'],{'x':3,'y':4,'z':7})

    def test_h6_createitem_parameters_stay_in_documented_slots(self):
        for call in ['Game.createItem(300,POS)', 'Game.createItem(300,1,2,POS)',
                     'Game.createItem(300,1,2)', 'Game.createItem(300,POS,1)',
                     'Game.createItem(300,1,POS,1)', 'local x=Game.createItem(300,POS)']:
            script=self.script('local POS=Position(1,2,7)',call)
            children=list(converter.walk(script.interactions()[0]['rules']))
            self.assertEqual(children[0]['status'],'blocked')
            self.assertNotIn('anchor',children[0])
        script=self.script('local POS=Position(1,2,7)','Game.createItem(300,1,POS)')
        self.assertIn('anchor',list(converter.walk(script.interactions()[0]['rules']))[0])

    def test_h7_condition_override_does_not_resolve_reward_conflict(self):
        repos={}
        for server,ident in [('canary',300),('crystalserver',301)]:
            repo=self.repo/server
            path=repo/converter.SOURCES[server]['datapack']/'scripts/quests/fixture.lua'
            path.parent.mkdir(parents=True)
            path.write_text('local action=Action()\nfunction action.onUse(player,item,fromPosition,target)\n'
                            'if unknownPredicate(player) then\nplayer:addItem('+str(ident)+',1)\nend\nend\naction:id(100)\n')
            repos[server]=repo
        questlog=self.repo/'questlog';questlog.mkdir()
        (questlog/'quests.json').write_text('{"quests":[]}')
        (questlog/'progress.json').write_text('{"progress":[]}')
        key='canary:interaction/fixture'
        overrides={key:{'3':{'condition':{'actor_is_player':True,'negate':False},'basis':'synthetic source proof'}}}
        with mock.patch.object(converter,'OVERRIDES',overrides), \
             mock.patch.object(converter,'CONFLICT_DECISIONS',{'interactions':{}}), \
             mock.patch.object(converter,'unused_decisions'), \
             mock.patch.object(converter,'git_blob',return_value='a'*40):
            result=converter.build(repos,'scripts/quests',questlog)
        entry=result['manifest.json']['entries'][0]
        self.assertEqual(entry['status'],'conflict')
        self.assertEqual(len(entry['conflict_alternatives']),2)
        ids=[list(converter.walk(v['interaction']['rules']))[0]['item']['key'] for v in entry['conflict_alternatives']]
        self.assertEqual(ids,['canary:item/300','crystalserver:item/301'])

    def test_h9_ast_folding_rejects_python_only_numerals(self):
        script=self.script('local A_1000=3')
        for expr in ['0b11+1','0o10+1','1_000+1','0B11-1','0O10*2','0xF_F+1']:
            self.assertIsNone(script.literal(expr))
        self.assertEqual(script.literal('0x10+2'),18)
        self.assertEqual(script.literal('A_1000+1'),4)

    def test_enriched_output_matches_existing_schema(self):
        import jsonschema
        s = self.script('local pos = Position(1, 2, 7)',
                        'if player:getItemCount("test reward") >= 1 and item:getId() == 100 then\n'
                        'player:teleportTo(pos)\nplayer:addItem("test reward", 2)\nend')
        schema = json.loads((Path(converter.ROOT) / 'interaction.schema.json').read_text())
        output = s.interactions()[0]
        for key in ('script', 'callback_line'):
            output.pop(key)
        jsonschema.validate({'interactions': [output]}, schema)




class StorageAliasEnrichmentTests(unittest.TestCase):
    def test_numeric_storage_alias_records_original_line(self):
        import lua_writers
        text = 'local ObedienceStorage = 44502\nlocal a = Action()\nfunction a.onUse(player)\nplayer:setStorageValue(ObedienceStorage, 1)\nend\n'
        self.assertEqual(lua_writers.expand_aliases('local ObedienceStorage = 44502', {'ObedienceStorage': '44502'}),
                         'local ObedienceStorage = 44502')
        result = lua_writers.scan(text, 'scripts/quests/test.lua')
        self.assertEqual(result[0]['target'], '44502')
        self.assertEqual(result[0]['line'], 4)
        self.assertEqual(result[0]['to'], 1)

    def test_mutated_shadowed_or_late_alias_is_not_resolved(self):
        import lua_writers
        cases = ['local Key=44502\nKey=44503\nplayer:setStorageValue(Key,1)',
                 'local Key=44502\nlocal Key=44503\nplayer:setStorageValue(Key,1)',
                 'player:setStorageValue(Key,1)\nlocal Key=44502']
        for text in cases:
            self.assertNotIn('Key', lua_writers.storage_aliases(text.splitlines()))

    def test_config_storage_alias_preserves_named_key(self):
        import lua_writers
        text = ('local config = { storageKey = Storage.Quest.Test.Obedience }\n'
                'local a = Action()\nfunction a.onUse(player)\n'
                'player:setStorageValue(config.storageKey, 1)\nend\n')
        self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua')[0]['target'], 'Storage.Quest.Test.Obedience')
        self.assertNotIn('config.storageKey', lua_writers.storage_aliases(
            (text + 'config.storageKey = Storage.Quest.Other\n').splitlines()))

    def test_storage_alias_sibling_branch_and_escape_are_held(self):
        import lua_writers
        cases = [
            'local a=Action()\nfunction a.first(player)\nlocal Key=44502\nend\n'
            'function a.onUse(player)\nplayer:setStorageValue(Key,1)\nend',
            'if condition then\nlocal Key=44502\nend\n'
            'function a.onUse(player)\nplayer:setStorageValue(Key,1)\nend',
            'local config={storageKey=Storage.Quest.Test.A}\nlocal other=config\n'
            'other.storageKey=Storage.Quest.Test.B\nfunction a.onUse(player)\n'
            'player:setStorageValue(config.storageKey,1)\nend',
            'local config={storageKey=Storage.Quest.Test.A}\nmutate(config)\n'
            'function a.onUse(player)\nplayer:setStorageValue(config.storageKey,1)\nend',
            'local config={storageKey=Storage.Quest.Test.A}\n'
            'if unknown then print(1) else config.storageKey=Storage.Quest.Test.B end\n'
            'function a.onUse(player)\nplayer:setStorageValue(config.storageKey,1)\nend',
            'if condition then\nlocal config={storageKey=Storage.Quest.Test.A}\nend\n'
            'function a.onUse(player)\nplayer:setStorageValue(config.storageKey,1)\nend']
        for text in cases:
            with self.subTest(text=text):
                self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua'), [])

    def test_storage_alias_parameter_and_loop_shadows_are_held(self):
        import lua_writers
        cases = [
            'local Key=44502\nfunction a.onUse(player,Key)\nplayer:setStorageValue(Key,1)\nend',
            'local Key=44502\nfunction a.onUse(player)\nfor _,Key in pairs(config) do\n'
            'player:setStorageValue(Key,1)\nend\nend',
            'local config={storageKey=Storage.Quest.Test.A}\nfunction a.onUse(player,config)\n'
            'player:setStorageValue(config.storageKey,1)\nend']
        for text in cases:
            self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua'), [])

    def test_storage_scalar_table_leaf_is_not_parent_alias(self):
        import lua_writers
        text = ('local config={item=300,storageKey=Storage.Quest.Test.A}\n'
                'function a.onUse(player)\nlocal found=lookup(config.item)\n'
                'player:setStorageValue(config.storageKey,1)\nend')
        self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua')[0]['target'], 'Storage.Quest.Test.A')

    def test_storage_alias_never_expands_quoted_text(self):
        import lua_writers
        line = 'player:say("Key ) StorageAlias.Stage") player:setStorageValue(Key, 1)'
        expanded = lua_writers.expand_aliases(line, {'Key': '44502', 'StorageAlias': 'Storage.Quest.Test'})
        self.assertEqual(expanded, 'player:say("Key ) StorageAlias.Stage") player:setStorageValue(44502, 1)')


if __name__ == '__main__':
    unittest.main()
