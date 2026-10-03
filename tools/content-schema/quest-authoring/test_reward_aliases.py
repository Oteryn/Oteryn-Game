"""Guarded scalar reward aliases retain unknown selectors and lexical fences."""
import tempfile
import unittest
from pathlib import Path

import ots_interactions as c


class RewardAliasTests(unittest.TestCase):
    def convert(self, body, prefix='local rewards = {[897]=300,[898]=301}', suffix=''):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            text = prefix + '\nlocal action=Action()\nfunction action.onUse(player,item,fromPosition,target,toPosition)\n' + body + '\nend\n' + suffix
            (repo / 'fixture.lua').write_text(text)
            script = c.Script('canary', repo, 'fixture.lua', {}, 'fixture')
            doc = script.interactions()[0]
            return script, doc, list(c.walk(doc['rules']))

    def test_item_type_alias_is_guarded_with_exact_references_and_unknown_fallback(self):
        _, doc, _ = self.convert('local selected=rewards[item.itemid]\nplayer:addItem(selected)')
        branches = doc['rules'][0]['branch']
        self.assertEqual([b['when']['object']['item']['key'] for b in branches], ['canary:item/897','canary:item/898'])
        self.assertEqual([b['then'][0]['item']['key'] for b in branches], ['canary:item/300','canary:item/301'])
        self.assertTrue(all(b['then'][0]['count'] == 1 for b in branches))
        self.assertEqual(doc['rules'][0]['otherwise'], [{'owner':'Item','request':'hand_out','value_source_line':5}])

    def test_uid_alias_preserves_quantity_and_direct_selector_behavior(self):
        _, doc, _ = self.convert('local selected=rewards[item.uid]\nplayer:addItem(selected,2)')
        self.assertEqual(doc['rules'][0]['branch'][0]['when']['object']['value'], 897)
        self.assertEqual(doc['rules'][0]['branch'][0]['then'][0]['count'], 2)
        _, direct, _ = self.convert('player:addItem(rewards[item.uid],2)')
        self.assertEqual(direct['rules'][0]['branch'], doc['rules'][0]['branch'])

    def test_branch_sibling_after_use_loop_and_mutation_are_not_bound(self):
        bodies = ['player:addItem(selected)\nlocal selected=rewards[item.uid]',
                  'if unknown then\nlocal selected=rewards[item.uid]\nelse\nplayer:addItem(selected)\nend',
                  'local selected=rewards[item.uid]\nselected=301\nplayer:addItem(selected)',
                  'for i=1,2 do\nlocal selected=rewards[item.uid]\nplayer:addItem(selected)\nend',
                  'local selected=rewards[target.uid]\nplayer:addItem(selected)',
                  'local selected=rewards[math.random(897,898)]\nplayer:addItem(selected)',
                  'local selected=rewards[item.uid]\nplayer:addItem(selected,unknown)',
                  'local selected=rewards[item.uid]\nplayer:addItem(selected,true,true)']
        for body in bodies:
            with self.subTest(body=body):
                _, _, children = self.convert(body)
                items = [x for x in children if x.get('owner') == 'Item']
                self.assertTrue(items); self.assertTrue(all('item' not in x for x in items))

    def test_dominating_alias_in_child_branch_and_schema_are_valid(self):
        import json
        import jsonschema
        _, doc, _ = self.convert('local selected=rewards[item.itemid]\nif unknown then\nplayer:addItem(selected)\nend')
        leaves = list(c.walk(doc['rules']))
        self.assertEqual(len([x for x in leaves if x.get('owner') == 'Item' and 'item' in x]), 2)
        doc.pop('script'); doc.pop('callback_line')
        schema = json.loads((Path(__import__('lua_tables').__file__).parent / 'interaction.schema.json').read_text())
        jsonschema.Draft202012Validator(schema).validate({'interactions': [doc]})

    def test_unknown_duplicate_values_are_checked_before_collapse(self):
        for value in ['getReward()', 'unknown', 'false', '{id=301}']:
            _, _, children = self.convert('local selected=rewards[item.uid]\nplayer:addItem(selected)',
                                         'local rewards = {[897]='+value+', [897]=300}')
            self.assertTrue(all('item' not in x for x in children if x.get('owner') == 'Item'))

    def test_snapshot_selector_methods_between_capture_and_grant_fail_closed(self):
        for method in ['transform(898)', 'remove()', 'setAttribute(1,898)', 'setActionId(898)',
                       'setUniqueId(898)', 'unknownMutation()']:
            _, _, children = self.convert('local selected=rewards[item.itemid]\nitem:'+method+'\nplayer:addItem(selected)')
            self.assertTrue(all('item' not in x for x in children if x.get('owner') == 'Item' and x.get('request') == 'hand_out'))
        _, doc, _ = self.convert('local selected=rewards[item.itemid]\nitem:getId()\nplayer:addItem(selected)\nitem:remove()')
        self.assertTrue(any('item' in x for x in c.walk(doc['rules']) if x.get('owner') == 'Item'))
        _, live, _ = self.convert('item:transform(898)\nplayer:addItem(rewards[item.itemid])')
        self.assertTrue(any('item' in x for x in c.walk(live['rules']) if x.get('owner') == 'Item'))

    def test_prior_closure_body_can_mutate_after_capture_and_stays_unknown(self):
        body = 'local function mutate()\nitem:transform(898)\nend\nlocal selected=rewards[item.itemid]\nmutate()\nplayer:addItem(selected)'
        _, _, children = self.convert(body)
        self.assertTrue(all('item' not in x for x in children if x.get('owner') == 'Item' and x.get('request') == 'hand_out'))

    def test_root_or_source_receiver_escape_and_mutation_fail_closed(self):
        for extra in ['rewards[897]=301', 'evil(rewards)', 'local other=rewards', 'item=target', 'item.itemid=897', 'item.itemid,other=897,0', 'evil(item)', 'player=target']:
            with self.subTest(extra=extra):
                _, _, children = self.convert(extra+'\nlocal selected=rewards[item.uid]\nplayer:addItem(selected)')
                self.assertTrue(all('item' not in x for x in children if x.get('owner')=='Item'))


if __name__ == '__main__':
    unittest.main()
