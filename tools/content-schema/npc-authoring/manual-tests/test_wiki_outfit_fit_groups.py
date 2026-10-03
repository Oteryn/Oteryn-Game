"""Manual D16 evidence-tool regression tests; uses verified native client assets only."""

import tempfile
import unittest
import importlib.util
import sys
from pathlib import Path
import numpy as np

# The integration copy belongs beside wiki_outfit_fit.py. This external prototype
# is selected explicitly so tests cannot accidentally validate the old helper.
MODULE = Path(__file__).resolve().parents[1] / "wiki_outfit_fit.py"
spec = importlib.util.spec_from_file_location("fit_group_candidate", MODULE)
wf = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = wf
spec.loader.exec_module(wf)


class GroupFitTests(unittest.TestCase):
    def test_walking_reference_selects_walking_group_and_all_four_colors(self):
        colors = (19, 58, 105, 94)
        with tempfile.TemporaryDirectory() as temp:
            image = Path(temp) / "native-generated.png"
            wf.render(129, *colors, 0, frame_group=1).save(image)
            got = wf.fit(129, 0, image, frame_group=1)
            self.assertEqual(tuple(got[r] for r in wf.REGIONS), colors)
            self.assertLess(got["score"], 1.0)
            self.assertTrue(all(px >= 30 for px in got["region_px"].values()))
            # A walking screenshot cannot prove idle colors when the feet move.
            self.assertGreater(wf.fit(129, 0, image)["score"], 35)

    def test_default_group_preserves_native_idle_render(self):
        implicit = np.asarray(wf.render(129, 19, 58, 105, 94, 0))
        explicit = np.asarray(wf.render(129, 19, 58, 105, 94, 0, frame_group=0))
        self.assertTrue(np.array_equal(implicit, explicit))
        self.assertEqual(wf.idle_group(129).fixed_frame_group, 0)

    def test_phase_changes_selected_native_frame(self):
        phase0 = np.asarray(wf.render(129, 19, 58, 105, 94, 0, phase=0, frame_group=1))
        phase1 = np.asarray(wf.render(129, 19, 58, 105, 94, 0, phase=1, frame_group=1))
        self.assertFalse(np.array_equal(phase0, phase1))

    def test_invalid_group_and_phase_fail_closed(self):
        for group in [True, -1, 2, "1"]:
            with self.assertRaises(ValueError):
                wf.idle_group(129, group)
        for phase in [True, -1, 8]:
            with self.assertRaises(ValueError):
                wf.layers_for(129, 0, phase, 1)

    def test_missing_requested_group_has_no_implicit_fallback(self):
        from unittest.mock import patch
        from types import SimpleNamespace

        one = SimpleNamespace(frame_groups=(wf.idle_group(129),))
        with patch.object(wf, "outfits", return_value={129: one}):
            with self.assertRaises(ValueError):
                wf.idle_group(129, 1)


if __name__ == "__main__":
    unittest.main()
