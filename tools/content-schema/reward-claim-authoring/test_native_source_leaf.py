"""New source leaves preserve literal donor identity, text and once progress."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import native_source_leaf as tool


class NativeSourceLeaf(unittest.TestCase):
    def test_skeleton_reward_and_progress_come_from_the_pinned_source(self):
        records, bindings = tool.build()
        record, = [r['definition'] for r in records
                   if r['definition']['identity']['key'].endswith('formorgar_hoist_skeleton')]
        recipe, = record['native_contents']['placements']
        placement, = record['placements']
        self.assertEqual(recipe['root']['key'], 'oteryn:item.tibia.i2822')
        self.assertEqual(recipe['unique_id'], 20003)
        self.assertEqual(placement['appearance_tibia_id'], 4285)
        self.assertEqual(placement['source_binding']['project_position'], {'x':32211,'y':31273,'z':10})
        self.assertTrue(recipe['root_text'].startswith('The map shows the original floor plan'))
        self.assertEqual(recipe['completion']['key'],
                         'Storage.Quest.U8_0.TheIceIslands.FormorgarMinesHoistSkeleton')
        binding, = [b for b in bindings if b['claim'] == record['identity']['key']]
        self.assertEqual((binding['initial'],binding['minimum'],binding['maximum'],binding['value']),
                         (-1,-1,1,1))
        self.assertEqual(placement['quest_transition'], binding['transition'])
        self.assertEqual(tool.validate(records, tool.ROOT), [])

    def test_unknown_or_duplicate_authoring_inputs_are_refused(self):
        original = json.loads((tool.ROOT / tool.INPUT).read_text())
        for case in ('unknown','duplicate','no_rule','invalid_uid'):
            value = copy.deepcopy(original)
            if case == 'unknown': value['unknown'] = True
            elif case == 'duplicate': value['leaves'].append(copy.deepcopy(value['leaves'][0]))
            elif case == 'no_rule': value['leaves'][0]['rule'] = ''
            else: value['leaves'][0]['source']['unique_id'] = 0
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); path = root / tool.INPUT
                path.parent.mkdir(parents=True); path.write_text(json.dumps(value))
                with self.assertRaises(ValueError, msg=case): tool.inputs(root)


if __name__ == '__main__': unittest.main()
