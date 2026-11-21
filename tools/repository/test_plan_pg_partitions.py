#!/usr/bin/env python3
"""Check that PostgreSQL archives produce one complete, balanced test plan."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / "ci" / "plan_pg_partitions.py"
SPEC = importlib.util.spec_from_file_location("plan_pg_partitions", MODULE)
planner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(planner)


class PartitionPlanTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.plan = Path(temp.name)

    def listing(self, archive, binary, *names):
        tests = {name: {"filter-match": {"status": "matches"}} for name in names}
        listing = {"rust-suites": {"suite": {"binary-id": binary, "testcases": tests}}}
        (self.plan / f"{archive}.json").write_text(json.dumps(listing))

    def test_dedicated_dispatch_reuses_library_estimates_without_changing_filters(self):
        binary = "oteryn-gameplay-dispatch::oteryn-gameplay-dispatch"
        name = planner.DISPATCH_MOUNT + "srx3::long_postgres"
        legacy_key = f"oteryn-gameplay-dispatch {name}"
        self.listing("pg-lib-2", binary, name)
        self.assertEqual(planner.estimate((binary, name), {legacy_key: 243}), 243)
        with contextlib.redirect_stdout(io.StringIO()):
            planner.write_partition(self.plan, 0, 1, {legacy_key: 243}, optional=True)
        self.assertEqual((self.plan / "pg-lib-2.filters").read_text(),
                         f"--filterset=binary_id(={binary}) & test(={name})\n")

    def test_dedicated_dispatch_measurement_overrides_the_legacy_estimate(self):
        binary = "oteryn-gameplay-dispatch::oteryn-gameplay-dispatch"
        name = planner.DISPATCH_MOUNT + "srx3::long_postgres"
        measured = {f"oteryn-gameplay-dispatch {name}": 243, f"{binary} {name}": 229}
        self.assertEqual(planner.estimate((binary, name), measured), 229)
        self.assertEqual(planner.estimate(("unrelated::oteryn-gameplay-dispatch", name), measured), 5)

    def test_matches_between_archives_are_balanced_without_repeating_primary_tests(self):
        dispatch = "oteryn-gameplay-dispatch"
        first, second, third = (planner.DISPATCH_MOUNT + name for name in ("a", "b", "c"))
        self.listing("pg-e2e", "server::e2e", "postgres_long")
        self.listing("pg-lib", "server", "postgres_medium")
        self.listing("pg-lib-1", dispatch, first, "non_postgres")
        self.listing("pg-lib-2", dispatch, second, "non_postgres")
        self.listing("pg-lib-3", dispatch, third, "non_postgres")
        durations = {"server::e2e postgres_long": 100, f"{dispatch} {second}": 90,
                     "server postgres_medium": 10}
        parts = []
        for index in range(2):
            with contextlib.redirect_stdout(io.StringIO()):
                planner.write_partition(self.plan, index, 2, durations, optional=False)
            parts.append({line for file in self.plan.glob("*.filters")
                          for line in file.read_text().splitlines()})
        self.assertFalse(parts[0] & parts[1])
        self.assertEqual(len(parts[0] | parts[1]), 6)
        self.assertTrue(any("postgres_long" in line for line in parts[0]))
        self.assertTrue(any(f"test(={second})" in line for line in parts[1]))
        self.assertIn(f"test(={second})", (self.plan / "priority.toml").read_text())

    def test_partial_scope_and_duplicate_case_handling(self):
        self.listing("pg-lib", "durability", "postgres_case")
        cases = planner.read_cases(self.plan, optional=True)
        self.assertEqual(cases["pg-lib"], [("durability", "postgres_case")])
        with self.assertRaises(FileNotFoundError):
            planner.read_cases(self.plan, optional=False)
        self.listing("pg-e2e", "durability", "postgres_case")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            planner.read_cases(self.plan, optional=True)

    def test_new_rune_cases_use_historical_budget_until_measured(self):
        binary, old = planner.OLD_RUNE_CASE.split(" ", 1)
        name = old.removesuffix("real_dispatch_casts_once_and_reloads_one_debit") + "case"
        durations = {planner.OLD_RUNE_CASE: 1514}
        self.assertEqual(planner.estimate((binary, name), durations), 152)
        durations[f"{binary} {name}"] = 90
        self.assertEqual(planner.estimate((binary, name), durations), 90)


    def test_split_exiva_cases_keep_their_world_specific_estimates(self):
        durations = dict(zip(planner.OLD_EXIVA_CASES, (1007, 1338, 1293), strict=True))
        for old, estimate in zip(planner.OLD_EXIVA_CASES, (112, 149, 144), strict=True):
            binary, case = old.split(" ", 1)
            future = case + "_public"
            self.assertEqual(planner.estimate((binary, future), durations), estimate)
        durations[f"{binary} {future}"] = 30
        self.assertEqual(planner.estimate((binary, future), durations), 30)


if __name__ == "__main__":
    unittest.main()
