import copy
import json
from pathlib import Path
import unittest

from questpc2_ladder_composition import compose, stationary_tool
import source_gate_item_profiles


class StationaryComposition(unittest.TestCase):
    def test_ladder_retains_type_without_mint_authority_and_refuses_other_changes(self):
        floor = json.loads(Path('tools/content-schema/native-gameplay/floor-assembly-item-profiles.json').read_bytes())
        rows = {r['production_definition']['production_key']: r for r in floor['records']}
        source = next(r for r in source_gate_item_profiles.capture()['records']
                      if r['production_definition']['production_key'].endswith('.i1948'))
        existing = rows['oteryn:item.tibia.i1948']
        result = compose(existing, source)
        self.assertIsNotNone(result)
        self.assertFalse(result['admission']['materializable'])
        self.assertEqual(result['semantics']['classification']['value']['item_type'],
                         {'state': 'KNOWN', 'value': 'LADDER'})
        wrong = copy.deepcopy(existing)
        wrong['attributes']['blocks_movement'] = True
        self.assertIsNone(compose(wrong, source))
        self.assertIsNone(compose(existing, rows['oteryn:item.tibia.i1947']))

    def test_tool_only_accepts_exact_stationary_overlap(self):
        root = Path('tools/content-schema/native-gameplay')
        floor = json.loads((root/'floor-assembly-item-profiles.json').read_bytes())
        tools = json.loads((root/'tool-item-profiles.json').read_bytes())
        existing = next(r for r in floor['records'] if r['production_definition']['production_key'].endswith('.i1791'))
        source = next(r for r in tools['records'] if r['production_definition']['production_key'].endswith('.i1791'))
        self.assertEqual(stationary_tool(existing, source), existing)
        wrong = copy.deepcopy(existing)
        wrong['admission']['legal_destinations'] = ['Backpack']
        self.assertIsNone(stationary_tool(wrong, source))
        self.assertIsNone(stationary_tool(source, source))
