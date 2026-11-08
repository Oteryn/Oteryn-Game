"""The selected Ram turnips retain exact source identity and native physical facts."""
import copy
import unittest
from unittest.mock import patch
import questpc5_targuna_material as author


class TargunaMaterial(unittest.TestCase):
    def test_exact_existing_turnips_binding_and_known_weight(self):
        row = author.capture()
        self.assertEqual(row['production_definition']['production_key'], 'oteryn:item.tibia.i53073')
        self.assertEqual(row['production_binding']['target'], row['authoring']['item'])
        self.assertEqual(row['production_binding']['external_id'], '53073')
        self.assertEqual(row['production_binding']['disposition'], 'EXACT')
        self.assertEqual(row['semantics']['physical']['value']['weight'], dict(state='KNOWN', value=200))
        self.assertEqual(row['admission'], dict(materializable=True, stack_class='NonStackable',
                         legal_destinations=['CharacterInventory', 'Ground']))
        self.assertEqual(row['semantics']['charges'], dict(state='UNKNOWN'))
        self.assertEqual(row['semantics']['temporal'], dict(state='UNKNOWN'))

    def test_known_canonical_stack_or_container_facts_cannot_be_overwritten(self):
        original = author.json.loads
        for group, fact in [('container', dict(state='KNOWN', value=dict(capacity=dict(state='KNOWN', value=1)))),
                            ('stack', dict(state='KNOWN', value=dict(stackable=dict(state='KNOWN', value=True), stack_max=dict(state='KNOWN', value=100))))]:
            def changed(raw):
                value = original(raw)
                if isinstance(value, dict):
                    for row in value.get('records', []):
                        definition = row.get('definition', {})
                        if definition.get('identity', {}).get('key') == author.KEY:
                            definition['semantics'][group] = fact
                return value
            with self.subTest(group=group), patch.object(author.json, 'loads', changed):
                with self.assertRaisesRegex(ValueError, 'TARGUNA_CANONICAL_CONFLICT'):
                    author.capture()

    def test_ordinary_turnip_or_container_alias_is_not_the_ram_quest_item(self):
        original = author.shared.source.engine_items.load_appearance_objects
        for field in ('name', 'flags'):
            def changed(raw):
                values = original(raw)
                values[53073] = copy.deepcopy(values[53073])
                if field == 'name':
                    values[53073]['name'] = 'turnip'
                else:
                    values[53073]['flags']['flags.container'] = True
                return values
            with self.subTest(field=field), patch.object(author.shared.source.engine_items,
                    'load_appearance_objects', changed):
                with self.assertRaisesRegex(ValueError, 'TARGUNA_TURNIP_CUSTODY'):
                    author.capture()


if __name__ == '__main__':
    unittest.main()
