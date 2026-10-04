"""Registrar display names and independent engine inspection descriptions."""
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import canary_batch as cb


class DisplayName(unittest.TestCase):
    def convert(self, registration_name, fields):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / cb.MONSTER_DIR / 'example.lua'
            path.parent.mkdir(parents=True)
            assignments = '\n'.join('monster.' + key + ' = ' + json.dumps(value)
                                    for key, value in fields.items())
            path.write_text('local mType = Game.createMonsterType(' + json.dumps(registration_name) + ')\n'
                            'local monster = {race="none", outfit={lookType=42}}\n' + assignments
                            + '\nmType:register(monster)\n', encoding='utf-8')
            with patch.object(cb, 'load_effect_constants', return_value=({}, {})):
                converter = cb.Converter(root, {}, {}, {}, {})
            converter.pending_definitions = set()
            return converter.convert('example')

    def test_override_changes_display_without_changing_identity(self):
        slug, monster, _, _, manifest, _ = self.convert('Brown Horse', {'name': 'Horse', 'description': 'a horse'})
        self.assertEqual('brown_horse', slug)
        self.assertEqual('canary:creature/brown_horse', monster['creature']['identity']['key'])
        self.assertEqual('Horse', monster['creature']['display_name'])
        self.assertEqual({'article': 'a'}, monster['creature']['name_forms'])
        self.assertTrue(any(row['source_field'] == 'name' and row['status'] == 'mapped'
                            and row['destination'] == '/monster/creature/display_name'
                            for row in manifest['entries']))

    def test_registration_name_and_default_description_are_independent(self):
        _, monster, _, _, manifest, _ = self.convert('Brown Horse', {'name': 'Horse'})
        self.assertEqual('Horse', monster['creature']['display_name'])
        self.assertEqual('a Brown Horse', monster['creature']['inspection']['description'])
        self.assertNotIn('name_forms', monster['creature'])
        self.assertTrue(any(row['source_field'] == 'description' and 'Game.createMonsterType' in row['resolution']
                            for row in manifest['entries']))

    def test_no_override_keeps_registration_name_and_engine_article(self):
        _, monster, _, _, _, _ = self.convert('Rat', {})
        self.assertEqual('Rat', monster['creature']['display_name'])
        self.assertEqual('a Rat', monster['creature']['inspection']['description'])
        self.assertEqual({'article': 'a'}, monster['creature']['name_forms'])

    def test_explicit_description_and_article_override_the_default(self):
        _, monster, _, _, _, _ = self.convert('Invulnerable Eye', {'name': 'Eye', 'description': 'an eye'})
        self.assertEqual('an eye', monster['creature']['inspection']['description'])
        self.assertEqual({'article': 'an'}, monster['creature']['name_forms'])
        _, monster, _, _, _, _ = self.convert('Invulnerable Eye', {'name': 'Eye', 'description': 'the watching eye'})
        self.assertEqual('the watching eye', monster['creature']['inspection']['description'])
        self.assertNotIn('name_forms', monster['creature'])

    @unittest.skipUnless(os.environ.get('OTERYN_CANARY'), 'set OTERYN_CANARY to the pinned checkout')
    def test_pinned_brown_horse_keeps_source_identity_and_display(self):
        root = Path(os.environ['OTERYN_CANARY'])
        objects = cb.load_appearance_objects(root / 'data/items/appearances.dat')
        items = cb.load_items_xml(root / 'data/items/items.xml')
        names, index = cb.name_index(objects, items)
        converter = cb.Converter(root, objects, items, names, index)
        converter.pending_definitions = set()
        slug, monster, _, _, manifest, _ = converter.convert('mammals/brown_horse')
        self.assertEqual('brown_horse', slug)
        self.assertEqual('Horse', monster['creature']['display_name'])
        self.assertEqual('a horse', monster['creature']['inspection']['description'])
        self.assertEqual({'article': 'a'}, monster['creature']['name_forms'])
        self.assertTrue(any(row['source_field'] == 'name' and row['source_line'] == 4 for row in manifest['entries']))


if __name__ == '__main__':
    unittest.main()
