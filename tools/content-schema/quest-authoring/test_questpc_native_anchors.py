"""M01b must reject native selectors that cannot exist in the retained map."""
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore
import questpc_station_approaches as stations
from questpc_native_anchors import NativeAnchors

ROOT = Path(__file__).resolve().parents[3]


class NativeAnchorTests(unittest.TestCase):
    def test_generation_rejects_a_changed_native_ground(self):
        binding = ((stations.QUEST, 's1', stations.REFERENCE,
                    'chosen-source/horse_station_world_change'),)
        changed = (dict(x=32442, y=32229, floor=-7), 4515, ['source'])
        with mock.patch.object(explore, 'BINDINGS', binding), \
                mock.patch.object(stations, 'resolve', return_value=changed):
            with self.assertRaisesRegex(ValueError, 'native EXPLORE ground'):
                explore.callbacks(ROOT)

    def test_generation_rejects_a_missing_native_tile(self):
        binding = ((stations.QUEST, 's1', stations.REFERENCE,
                    'chosen-source/horse_station_world_change'),)
        changed = (dict(x=0, y=0, floor=-7), 4521, ['source'])
        with mock.patch.object(explore, 'BINDINGS', binding), \
                mock.patch.object(stations, 'resolve', return_value=changed):
            with self.assertRaisesRegex(ValueError, 'native EXPLORE'):
                explore.callbacks(ROOT)

    def test_exact_public_ground_is_accepted(self):
        NativeAnchors(ROOT).validate(dict(x=32442, y=32229, floor=-7), 4521)

    def test_house_ground_cannot_be_an_explore_anchor(self):
        anchors = NativeAnchors(ROOT)
        decode = anchors.codec.decode_sector

        def private_sector(*args):
            for x, y, flags, house, extra, items in decode(*args):
                yield x, y, flags, 1, extra, items

        with mock.patch.object(anchors.codec, 'decode_sector', side_effect=private_sector):
            with self.assertRaisesRegex(ValueError, 'private.*native EXPLORE ground'):
                anchors.validate(dict(x=32442, y=32229, floor=-7), 4521)


if __name__ == '__main__':
    unittest.main()
