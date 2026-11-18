from collections import Counter
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

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


if __name__ == "__main__":
    unittest.main()
