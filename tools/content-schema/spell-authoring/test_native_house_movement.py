"""Semantic and fail-closed qualification tests for the nine world/house adapters."""
import copy
import unittest
from unittest.mock import patch

from jsonschema import Draft202012Validator
import native_house_movement as native


class HouseMovementTests(unittest.TestCase):
    def test_nine_adapters_have_strict_discriminated_parameters(self):
        schemas = native.schemas()
        for name, behaviour in native.BEHAVIOURS.items():
            with self.subTest(name=name):
                validator = Draft202012Validator(schemas[behaviour['key']])
                validator.validate(behaviour['parameters'])
                modified = {**behaviour['parameters'], 'lua_body': 'return true'}
                self.assertTrue(list(validator.iter_errors(modified)))
                missing = copy.deepcopy(behaviour['parameters'])
                missing.pop(next(iter(missing)))
                self.assertTrue(list(validator.iter_errors(missing)))
        self.assertEqual(9, len(native.BEHAVIOURS))

    def test_unrelated_or_wrong_carrier_is_not_captured(self):
        self.assertIsNone(native.build('magic rope', 'rune', {}, {}))
        self.assertIsNone(native.build('energy wave', 'instant', {}, {}))

    def test_missing_changed_or_wrong_source_fails_closed(self):
        with self.assertRaises(ValueError):
            native.build('magic rope', 'instant', {}, {})
        path = native.FILES['magic rope']
        record = {'file': path, 'blob': '0' * 40, 'spell_type': 'instant'}
        for text in (None, 'return true', 'return false'):
            with self.subTest(text=text), self.assertRaises(ValueError):
                native.build('magic rope', 'instant', {'canary': record}, {('canary', path): text})
        with self.assertRaises(ValueError):
            native.build('magic rope', 'instant', {'unknown': record}, {('unknown', path): 'return true'})

    def test_explicit_wrong_revision_fails_before_source_acceptance(self):
        name = 'creature illusion'
        path = native.FILES[name]
        record = {'file': path, 'source': 'canary', 'name': name, 'spell_type': 'instant',
                  'revision': 'wrong-pin', 'blob': '0' * 40}
        with self.assertRaisesRegex(ValueError, 'pinned revision mismatch'):
            native.build(name, 'instant', {'canary': record}, {('canary', path): 'source'})

    def test_build_returns_an_independent_payload(self):
        with patch.object(native, 'evidence', return_value=[]):
            payload = native.build('House Door List', 'instant', {}, {})
        payload['parameters']['door_lookup'].clear()
        self.assertEqual(['front', 'own_tile'], native.BEHAVIOURS['house door list']['parameters']['door_lookup'])

    def test_location_edges_and_vertical_phrases(self):
        cases = [((0, 4, 0), 'is standing next to you'),
                 ((0, 4, 1), 'is below you'), ((0, 4, -1), 'is above you'),
                 ((0, 5, 0), 'is to the south'), ((0, 100, 0), 'is to the south'),
                 ((0, 101, 0), 'is far to the south'), ((0, 250, 0), 'is far to the south'),
                 ((0, 251, 0), 'is very far to the south'),
                 ((-10, 10, 0), 'is to the south-west'), ((10, -10, 0), 'is to the north-east')]
        for target, expected in cases:
            with self.subTest(target=target):
                self.assertEqual(expected, native.location_phrase((0, 0, 0), target, native.PERSON))

    def test_bestiary_visibility_and_boundary_difficulties(self):
        self.assertEqual('Unknown', native.fiend_difficulty(25, False))
        for kills, label in [(4, 'Trivial'), (5, 'Harmless'), (25, 'Harmless'), (26, 'Trivial'),
                             (250, 'Trivial'), (251, 'Easy'), (500, 'Easy'), (501, 'Medium'),
                             (1001, 'Hard'), (2501, 'Challenging'), (5001, 'Unknown')]:
            self.assertEqual(label, native.fiend_difficulty(kills, True))

    def test_fiend_selection_crosses_floors_and_keeps_source_tie_order(self):
        a = {'alive': True, 'fiendish': True, 'position': (3, 2, 0), 'id': 50}
        b = {'alive': True, 'fiendish': True, 'position': (1, 1, 3), 'id': 20}
        c = {'alive': False, 'fiendish': True, 'position': (0, 0, 0), 'id': 10}
        self.assertIs(a, native.nearest_fiend((0, 0, 0), [a, b, c]))
        self.assertIs(b, native.nearest_fiend((0, 0, 0), [b, a, c]))
        self.assertIsNone(native.nearest_fiend((0, 0, 0), [c]))

    def test_house_denial_success_boundary_and_door_rights(self):
        self.assertEqual({'succeeds': True, 'open_editor': True}, native.house_list_cast('subowner', 'guest'))
        self.assertEqual({'succeeds': True, 'open_editor': False}, native.house_list_cast('subowner', 'subowner'))
        self.assertEqual({'succeeds': False, 'open_editor': False}, native.house_list_cast('subowner', 'door'))
        self.assertFalse(native.house_list_cast('owner', 'door', has_door=False)['succeeds'])
        self.assertFalse(native.house_list_cast('owner', 'guest', has_house=False)['succeeds'])

    def test_self_and_cross_house_kick(self):
        self.assertTrue(native.house_kick_allowed(1, 1, True, False, 'guest', 'guest'))
        self.assertFalse(native.house_kick_allowed(None, None, True, False, 'guest', 'guest'))
        self.assertTrue(native.house_kick_allowed(1, 2, False, True, 'owner', 'guest'))
        self.assertFalse(native.house_kick_allowed(1, 2, False, True, 'subowner', 'owner'))
        self.assertFalse(native.house_kick_allowed(1, 2, False, True, 'owner', 'guest', True))

    def test_levitate_probes_and_surface_boundary(self):
        destination = (11, 10, 8)
        tiles = {destination: {'ground': True, 'block_solid': True, 'creature': True}}
        self.assertEqual(destination, native.levitate_destination((10, 10, 9), (1, 0), 'UP', tiles))
        tiles[(10, 10, 8)] = {'ground': True}
        self.assertIsNone(native.levitate_destination((10, 10, 9), (1, 0), 'up', tiles))
        self.assertIsNone(native.levitate_destination((10, 10, 8), (1, 0), 'up', tiles))
        self.assertIsNone(native.levitate_destination((10, 10, 7), (1, 0), 'down', tiles))
        self.assertIsNone(native.levitate_destination((10, 10, 9), (1, 0), 'sideways', tiles))

    def test_rope_rejects_immovable_solid_before_fallback(self):
        origin = (10, 10, 9)
        south, north = (10, 11, 8), (10, 9, 8)
        tiles = {origin: {'ground': True, 'ground_item': 386},
                 south: {'ground': True, 'immovable_block_solid': True},
                 north: {'ground': True}}
        # Both qualified Tile:isWalkable helpers reject immovable solid even
        # when block=false. A valid north cell wins before the south fallback.
        self.assertEqual(north, native.rope_destination(origin, tiles))

    def test_rope_uses_north_when_south_blocked_and_preserves_fallback(self):
        origin = (10, 10, 9)
        south, north = (10, 11, 8), (10, 9, 8)
        tiles = {origin: {'ground': True, 'ground_item': 386},
                 south: {'ground': True, 'block_solid': True},
                 north: {'ground': True, 'creature': True, 'floor_change': True, 'protection_zone': True}}
        self.assertEqual(north, native.rope_destination(origin, tiles))
        del tiles[north]
        self.assertEqual(south, native.rope_destination(origin, tiles))
        del tiles[south]
        self.assertIsNone(native.rope_destination(origin, tiles))
        tiles[south] = {'ground': True}
        tiles[origin]['ground_item'] = 0
        tiles[origin]['top_items'] = [12935]
        self.assertEqual(south, native.rope_destination(origin, tiles))


if __name__ == '__main__':
    unittest.main()
