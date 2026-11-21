from collections import Counter
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

import check_dispatch_pg_shards as checker
from check_dispatch_pg_shards import DISPATCH, check, inventory, verify_partition


class CheckDispatchPgShardsTests(unittest.TestCase):
    def test_static_mount_is_primary_only(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            entry = root / "entry.rs"
            entry.write_text('#[path = "case.rs"]\n#[cfg(oteryn_pg_shard_primary)]\nmod case;\n')
            (root / "case.rs").write_text("#[test]\nfn only_once() {}\n")
            self.assertEqual(inventory([entry], True)[1], {("entry", "case", "only_once"): 1})
            self.assertFalse(inventory([entry], False)[1])

    def test_same_line_path_attribute(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            entry = root / "entry.rs"
            entry.write_text('#[path="case.rs"]mod alias;\n')
            (root / "case.rs").write_text("#[test]\nfn nested() {}\n")
            self.assertEqual(inventory([entry], True)[1], {("entry", "alias", "nested"): 1})

    def test_partition_requires_every_case_once(self):
        expected = Counter(["first", "second"])
        verify_partition(expected, [Counter(["first"]), Counter(["second"])], "tests")
        for observed in (["first"], ["first", "first", "second"], ["first", "other"]):
            with self.subTest(observed=observed), self.assertRaises(ValueError):
                verify_partition(expected, [Counter(observed)], "tests")

    def test_inventory_counts_macro_generated_cases(self):
        postgres = DISPATCH / "src/gameplay_transport/postgres_tests"
        for module, count in (
            ("all_charms_outgoing", 37),  # 9 literal tests and 14 pairs
            ("source_quest_rewards", 28),
            ("srvenc_source_deaths", 14),
        ):
            with self.subTest(module=module):
                _, cases = inventory([postgres / f"{module}.rs"], True)
                self.assertEqual(sum(cases.values()), count)
                self.assertTrue(all(multiplicity == 1 for multiplicity in cases.values()))

    def test_repository_shards_cover_the_unsharded_inventory(self):
        check(3)

    def test_exact_test_resolver_follows_current_assignment(self):
        name = ("boss_activity", "retained_retry")
        exact = "gameplay_transport::monk_save::postgres_tests::boss_activity::retained_retry"
        for index in range(3):
            shards = [Counter() for _ in range(3)]
            shards[index][name] = 1
            self.assertEqual(checker.resolve_test_shard(exact, shards), f"{index + 1}/3")

    def test_exact_test_resolver_preserves_static_owner_module_paths(self):
        exact = "gameplay_transport::monk_save::postgres_tests::first_vocation_vitals::case"
        shards = [Counter({("monk_save_postgres_tests", "first_vocation_vitals", "case"): 1}),
                  Counter()]
        self.assertEqual(checker.resolve_test_shard(exact, shards), "1/2")

    def test_exact_test_resolver_rejects_missing_or_ambiguous_targets(self):
        name = ("boss_activity", "retained_retry")
        exact = "gameplay_transport::monk_save::postgres_tests::boss_activity::retained_retry"
        for shards in ([Counter()], [Counter({name: 2})],
                       [Counter({name: 1}), Counter({name: 1})]):
            message = "missing" if not any(shards) else "ambiguous"
            with self.subTest(shards=shards), self.assertRaisesRegex(ValueError, message):
                checker.resolve_test_shard(exact, shards)
        for invalid in ("retained_retry", exact + "_missing", exact.replace("monk_save", "other")):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                checker.resolve_test_shard(invalid, [Counter({name: 1})])

    def test_resolver_cli_emits_only_one_shard_for_the_retained_retry(self):
        exact = ("gameplay_transport::monk_save::postgres_tests::boss_activity::"
                 "boss_activity_postgres_offline_native_death_deadline_retry_keeps_retained_trigger")
        result = subprocess.run(
            [sys.executable, str(Path(checker.__file__)), "3", "--test", exact],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stdout, r"^[1-3]/3\n$")
        self.assertIn("zero duplicates", result.stderr)

    def test_resolver_cli_rejects_unknown_test_without_a_selection(self):
        result = subprocess.run(
            [sys.executable, str(Path(checker.__file__)), "3", "--test",
             "gameplay_transport::monk_save::postgres_tests::missing::case"],
            capture_output=True, text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("missing exact PostgreSQL test", result.stderr)


if __name__ == "__main__":
    unittest.main()
