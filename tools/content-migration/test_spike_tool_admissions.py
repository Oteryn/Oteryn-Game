"""Validate the bounded Upper Spike tool promotion against canonical Item facts."""
import copy
import json
from pathlib import Path
import unittest

import spike_tool_admissions


class SpikeToolAdmissions(unittest.TestCase):
    def test_only_spike_tools_are_promoted_and_the_overlay_is_idempotent(self):
        root = Path(__file__).resolve().parents[2]
        shard = root / 'content/items/definitions/items-08000-08499.json'
        definitions = [row['definition'] for row in json.loads(shard.read_bytes())['records']]
        for row in definitions:
            if row['identity']['key'] in spike_tool_admissions.TOOLS:
                row.update(materializable=False, stack_class='Unknown')
        before = copy.deepcopy(definitions)
        spike_tool_admissions.apply_admissions(definitions)
        changed = {after['identity']['key'] for old, after in zip(before, definitions, strict=True)
                   if old != after}
        self.assertEqual(changed, spike_tool_admissions.TOOLS.keys())
        for row in definitions:
            if row['identity']['key'] in changed:
                self.assertTrue(row['materializable'])
                self.assertEqual(row['stack_class'], 'NonStackable')
        once = copy.deepcopy(definitions)
        spike_tool_admissions.apply_admissions(definitions)
        self.assertEqual(definitions, once)


if __name__ == '__main__':
    unittest.main()
