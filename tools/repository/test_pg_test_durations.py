import importlib.util
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

MODULE = Path(__file__).resolve().parents[1] / "ci" / "pg_test_durations.py"
SPEC = importlib.util.spec_from_file_location("pg_test_durations", MODULE)
generator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(generator)


class PgTestDurationsTests(unittest.TestCase):
    def test_result_parser_keeps_the_longest_observation(self):
        with TemporaryDirectory() as tmp:
            first, second = (Path(tmp) / name for name in ("first.log", "second.log"))
            first.write_text(
                "timestamp PASS [ 243.299s] (102/105) dispatch first\n"
                "timestamp PASS [   4.123s] (103/105) dispatch second\n"
                "timestamp Summary [ 879.371s] 105 tests run: 105 passed\n"
            )
            second.write_text("timestamp FAIL [ 250.123s] (1/1) dispatch first\n")
            self.assertEqual(generator.durations([first, second]),
                             {"dispatch first": 250.123, "dispatch second": 4.123})

    def run_update(self, directory, *options):
        root = Path(directory)
        log = root / "partition.log"
        log.write_text("PASS [ 120.321s] (1/3) dispatch retained\n"
                       "PASS [  80.123s] (2/3) dispatch new\n"
                       "PASS [   2.000s] (3/3) dispatch short\n")
        out = root / "durations.json"
        out.write_text(json.dumps({"dispatch retained": 200, "dispatch other": 70,
                                   "dispatch short": 17}))
        result = subprocess.run(
            [sys.executable, str(MODULE),
             str(log), "--out", str(out), *options], capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(out.read_text())

    def test_partial_refresh_preserves_other_cases_and_previous_maxima(self):
        with TemporaryDirectory() as tmp:
            self.assertEqual(self.run_update(tmp, "--merge-existing"), {
                "dispatch retained": 200, "dispatch other": 70,
                "dispatch short": 17, "dispatch new": 80,
            })

    def test_default_refresh_still_replaces_the_table(self):
        with TemporaryDirectory() as tmp:
            self.assertEqual(self.run_update(tmp), {"dispatch retained": 120, "dispatch new": 80})

    def test_missing_results_do_not_replace_existing_measurements(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            log, out = root / "empty.log", root / "durations.json"
            log.write_text("no test results\n")
            original = '{"dispatch retained": 200}\n'
            out.write_text(original)
            result = subprocess.run(
                [sys.executable, str(MODULE),
                 str(log), "--out", str(out), "--merge-existing"],
                capture_output=True, text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(out.read_text(), original)


if __name__ == "__main__":
    unittest.main()
