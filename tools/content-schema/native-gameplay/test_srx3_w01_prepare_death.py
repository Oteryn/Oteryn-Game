"""Prepare-death callbacks bind actual native bodies and pinned source events."""
import copy
import importlib.util
from pathlib import Path
import unittest

class PrepareDeathTests(unittest.TestCase):
    def module(self):
        spec=importlib.util.spec_from_file_location('w01_prepare_death',Path(__file__).with_name('build_srx3_w01_prepare_death.py'))
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
    def test_actual_two_source_events_retain_distinct_heals_and_shared_marker(self):
        data=self.module().build()
        self.assertEqual([1000,400],[row['heal'] for row in data['events']])
        self.assertTrue(all(row['cancel_incoming'] and row['marker_sub_id']==88888 and row['marker_duration_ms']==900000 for row in data['events']))
        self.assertEqual(2,len({row['native']['profile']['target']['key'] for row in data['events']}))
    def test_source_root_tag_does_not_bless_changed_native_body(self):
        tool=self.module();native=tool.read_native();changed=copy.deepcopy(native)
        row=next(row for row in changed['records'] if row['profile']['target']['key']=='oteryn:creature.hirintror')
        row['profile']['data']['profile']['health']+=1
        with self.assertRaisesRegex(ValueError,'Native prepare-death body changed'):tool.build(changed)

if __name__=='__main__':unittest.main()
