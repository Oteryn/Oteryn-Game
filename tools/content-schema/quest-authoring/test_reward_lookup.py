"""Finite reward selections and partial SOURCE facts remain fail closed."""
import json
from pathlib import Path
import tempfile
import unittest

import ots_interactions as c
ROOT = c.ROOT


class RewardLookupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        (self.repo / 'data/items').mkdir(parents=True)
        (self.repo / 'data/items/items.xml').write_text(
            '<items><item id="300" name="reward"/><item id="301" name="duplicate"/>'
            '<item id="302" name="duplicate"/></items>')

    def script(self, prefix='local rewards = {[1300]=300, [1301]=301}',
               body='player:addItem(rewards[item.uid], 1)', suffix=''):
        text = prefix + '\nlocal action = Action()\nfunction action.onUse(player, item, fromPosition, target, toPosition)\n' + body + '\nend\n' + suffix
        (self.repo / 'fixture.lua').write_text(text)
        s = c.Script('canary', self.repo, 'fixture.lua', {}, 'fixture')
        result = s.interactions()[0]
        return s, result, list(c.walk(result['rules']))

    def test_exact_selection_is_guarded_not_all_rewards(self):
        _, doc, children = self.script()
        branches = doc['rules'][0]['branch']
        self.assertEqual([b['when']['object']['value'] for b in branches], [1300, 1301])
        self.assertEqual([b['then'][0]['item']['key'] for b in branches], ['canary:item/300', 'canary:item/301'])
        self.assertEqual(doc['rules'][0]['otherwise'], [{'owner': 'Item', 'request': 'hand_out', 'value_source_line': 4}])
        self.assertEqual(len(children), 3)

    def test_registration_pairs_and_scalar_copy_do_not_escape(self):
        _, doc, _ = self.script(body='player:addItem(rewards[item.uid], 1)\nlocal name = ItemType(rewards[item.uid]):getName()',
                                suffix='for uniqueId, reward in pairs(rewards) do\naction:uid(uniqueId)\nend')
        self.assertIn('branch', doc['rules'][0])

    def test_unknown_tables_and_mutations_stay_opaque(self):
        cases = [
            ('local rewards = {[1300]=300}', 'rewards[1300]=301'),
            ('local rewards = {[1300]=300}', 'rewards[1300], other = 301,4'),
            ('local rewards = {[1300]=300}', 'first, rewards[1300], other = 0,301,4'),
            ('local rewards = {[1300]=300}', 'local alias = rewards\nalias[1300]=301'),
            ('local rewards = {[1300]=300}', 'local iterator,pool,key=pairs(rewards)\npool[1300]=301'),
            ('local rewards = {[1300]=300}', 'local iterator,pool,key=pairs((rewards))\npool[1300]=301'),
            ('local rewards = {[1300]={id=300}}', ''),
            ('local rewards = {[1300]=false}', ''),
            ('local rewards = {[1300]=300, [1301]=getReward()}', ''),
            ('local rewards = {[1300]=300}', 'evil(rewards)'),
            ('local rewards = {[1300]=300}', 'evil.pairs(rewards)'),
            ('local rewards = {[1300]=300}', 'evil:pairs(rewards)'),
            ('local rewards = {[1300]=300}', 'setmetatable(rewards, anything)'),
            ('local rewards = {[1300]=300}', 'local rewards = {[1300]=301}'),
            ('local rewards = {[1300]=300}', 'local pairs=evil\nfor x in pairs(rewards) do end'),
        ]
        for prefix, extra in cases:
            with self.subTest(extra=extra, prefix=prefix):
                _, _, children = self.script(prefix, extra + '\nplayer:addItem(rewards[item.uid], 1)')
                items = [child for child in children if child.get('owner') == 'Item']
                self.assertTrue(items)
                self.assertTrue(all('item' not in child for child in items))

    def test_source_selector_reassignment_shadow_and_escape_stay_opaque(self):
        for extra in ['item=target', 'local item=target', 'item.uid=1300', 'evil(item)',
                      'local x=item', 'item, another = target, player', 'local function f(item) return item end']:
            with self.subTest(extra=extra):
                _, _, children = self.script(body=extra + '\nplayer:addItem(rewards[item.uid], 1)')
                self.assertTrue(all('item' not in x for x in children if x.get('owner') == 'Item'))

    def test_receiver_reassignment_shadow_and_escape_stay_opaque(self):
        for extra in ['player=target', 'local player=target', 'local alias=player',
                      'player, other=target, item', 'evil(player)']:
            with self.subTest(extra=extra):
                _, _, children = self.script(body=extra + '\nplayer:addItem(rewards[item.uid], 1)')
                self.assertTrue(all('item' not in x for x in children if x.get('owner') == 'Item'))

    def test_pairs_global_environment_mutation_stays_opaque(self):
        for extra in ['_G["pairs"]=evil', '_ENV.pairs=evil', 'rawset(_G,"pairs",evil)',
                      'local env=getfenv()', 'setfenv(1,env)', 'load("anything")()',
                      'loadstring("anything")()', 'loadfile("anything")()', 'dofile("anything")', 'require("anything")']:
            with self.subTest(extra=extra):
                _, _, children = self.script(prefix='local rewards={[1300]=300}\n'+extra,
                    suffix='for uid, value in pairs(rewards) do\naction:uid(uid)\nend')
                self.assertTrue(all('item' not in x for x in children if x.get('owner') == 'Item'))

    def test_reflection_without_pairs_registration_keeps_table_unknown(self):
        for suffix in ['debug.setupvalue(action.onUse,1,{[1300]=301})\naction:uid(1300)',
                       'local name,pool=debug.getupvalue(action.onUse,1)\npool[1300]=301\naction:uid(1300)']:
            with self.subTest(suffix=suffix):
                _, _, children = self.script(prefix='local rewards={[1300]=300}', suffix=suffix)
                self.assertTrue(all('item' not in x for x in children if x.get('owner') == 'Item'))

    def test_other_role_random_and_runtime_indices_are_not_uid_choices(self):
        for expression in ['rewards[target.uid]', 'rewards[math.random(1300,1301)]', 'rewards[index]']:
            with self.subTest(expression=expression):
                _, _, children = self.script(body=f'player:addItem({expression},1)')
                self.assertEqual(children[0], {'owner': 'Item', 'request': 'hand_out', 'value_source_line': 4})

    def test_literal_names_and_unknown_count(self):
        _, doc, _ = self.script(prefix='local rewards = {[1300]="reward"}')
        self.assertEqual(doc['rules'][0]['branch'][0]['then'][0]['item']['key'], 'canary:item/300')
        _, _, children = self.script(prefix='local rewards = {[1300]="duplicate"}')
        self.assertNotIn('item', children[0])
        _, _, children = self.script(body='player:addItem(rewards[item.uid], count)')
        self.assertNotIn('item', children[0])

    def test_third_argument_keeps_exact_item_but_holds_semantics(self):
        s, _, children = self.script(body='player:addItem(300,2,true)')
        self.assertEqual(children[0]['item']['key'], 'canary:item/300')
        self.assertNotIn('count', children[0])
        self.assertIn('value_source_line', children[0])
        self.assertEqual(s.unresolved[0]['reason'], 'Item hand-out additional arguments outside the transcribed vocabulary')
        _, _, children = self.script(body='player:addItem(rewards[item.uid],1,true)')
        self.assertNotIn('item', children[0])

    def test_partial_count_and_boolean_are_not_inventory_or_charge_guesses(self):
        for body in ['player:addItem(300,unknown)', 'player:addItem(300,true,true)', 'player:addItem(300,0)']:
            with self.subTest(body=body):
                _, _, children = self.script(body=body)
                self.assertIn('item', children[0])
                self.assertNotIn('count', children[0])
                self.assertIn('value_source_line', children[0])

    def test_partial_parent_never_enables_published_container_fill(self):
        _, _, children = self.script(body='local bag=player:addItem(300,1,true)\nbag:addItem(301,2)')
        self.assertNotIn('contents', children[0])

    def test_existing_typed_calls_and_repeated_opaque_selection_preserved(self):
        _, _, children = self.script(body='player:addItem(300,2)')
        self.assertEqual(children[0]['count'], 2)
        _, doc, children = self.script(body='for x=1,3 do\nplayer:addItem(rewards[item.uid],1)\nend')
        self.assertNotIn('branch', doc['rules'][0])
        self.assertTrue(children[0]['repeated'])

    def test_current_source_schema_accepts_both_enrichments(self):
        import jsonschema
        schema = json.loads((ROOT / 'interaction.schema.json').read_text())
        for body in ['player:addItem(rewards[item.uid],1)', 'player:addItem(300,1,true)']:
            _, doc, _ = self.script(body=body)
            doc.pop('script'); doc.pop('callback_line')
            jsonschema.Draft202012Validator(schema).validate({'interactions': [doc]})


if __name__ == '__main__':
    unittest.main()
