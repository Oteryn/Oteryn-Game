"""M03 binds existing source objects and live owner-selected entrance requirements."""
import json
from pathlib import Path
import unittest
from unittest import mock
import qengine_access
import qmech_m03_entry_bindings as entries

ROOT = Path(__file__).resolve().parents[3]


class M03EntryTests(unittest.TestCase):
    def test_committed_access_packet_includes_generated_m03_bindings(self):
        committed = json.loads((ROOT / 'content/quests/shared-access-callbacks.json').read_text())
        self.assertEqual(committed, qengine_access.document())
        rows = {row['key']: row for row in committed['callbacks'] if '/m03/' in row['key']}
        self.assertEqual(len(rows), 19)
        entry = rows['oteryn:quest-callback/m03/time-machine-entry-4']
        guards = entry['character_requirements']
        self.assertTrue(guards['premium'] and guards['promoted'])
        self.assertEqual(len(guards['blessings']), 5)
        self.assertNotIn('oteryn:blessing.twist_of_fate', guards['blessings'])
        self.assertNotIn('oteryn:blessing.heart_of_the_mountain', guards['blessings'])
        self.assertEqual(entry['minimum_level'], 20)

    def test_return_retains_actual_bare_stack_ordinal_and_needs_no_entitlement(self):
        rows = {row['key']: row for row in entries.callbacks(ROOT)}
        corner = rows['oteryn:quest-callback/m03/time-machine-return-0']
        center = rows['oteryn:quest-callback/m03/time-machine-return-4']
        self.assertEqual(corner['selector']['native']['ordinal'], 2)
        self.assertEqual(corner['selector']['native']['appearance'], 25091)
        self.assertEqual(center['selector']['native']['ordinal'], 1)
        for row in rows.values():
            if '/time-machine-return-' in row['key']:
                self.assertNotIn('character_requirements', row)
                self.assertEqual(row['storage'], [])
                self.assertEqual(row['teleport'], {'x': 32430, 'y': 32167, 'floor': -8})

    def test_changed_recipe_requirement_is_not_silently_omitted(self):
        original = Path.read_text
        def changed(path, *args, **kwargs):
            text = original(path, *args, **kwargs)
            if path.name == 'quests-00000-00099.json':
                data = json.loads(text)
                definition = data['records'][89]['definition']
                recipe = definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
                recipe['requirements']['min_level'] = 81
                return json.dumps(data)
            return text
        with mock.patch.object(Path, 'read_text', changed):
            with self.assertRaisesRegex(ValueError, 'chosen entry recipe changed'):
                entries.callbacks(ROOT)


if __name__ == '__main__':
    unittest.main()
