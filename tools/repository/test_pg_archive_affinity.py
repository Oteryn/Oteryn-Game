#!/usr/bin/env python3
"""Exercise archive-affinity coverage and same-run artifact custody."""

from collections import Counter, defaultdict
import contextlib
import hashlib
import io
from itertools import combinations
import json
from pathlib import Path
import runpy
import shutil
import tempfile
import unittest


TOOLS = Path(__file__).resolve().parents[1] / "ci"
AFFINITY = runpy.run_path(str(TOOLS / "pg_archive_affinity.py"))
PLANNER = runpy.run_path(str(TOOLS / "plan_pg_partitions.py"))
ARCHIVES = tuple(AFFINITY["SLOTS"])
TOTAL = AFFINITY["TOTAL"]
HEAD = "1" * 40
WORKSPACE = "/home/runner/work/Oteryn-Game/Oteryn-Game"


class ArchiveAffinityTests(unittest.TestCase):
    def archive(self, archive, suites, *, head=HEAD, workspace=WORKSPACE):
        inventory = self.temp / "pg-inventories" / f"pr-postgres-test-inventory-{archive}"
        inventory.mkdir(parents=True)
        layout = {"rust-suites": {
            binary: {"binary-id": binary, "testcases": {
                case: {"filter-match": {"status": "matches"}} for case in cases
            }} for binary, cases in suites.items()
        }}
        listing = f"pg-{archive}.json"
        payload = f"pg-{archive}.tar.zst"
        entries = {
            "head-sha": head + "\n", "workspace": workspace + "\n",
            payload: f"archive bytes for {archive}", listing: json.dumps(layout),
        }
        sums = "".join(f"{hashlib.sha256(data.encode()).hexdigest()}  {name}\n"
                       for name, data in entries.items())
        for name, data in entries.items():
            if name != payload:
                (inventory / name).write_text(data)
        (inventory / "SHA256SUMS").write_text(sums)
        return inventory, entries | {"SHA256SUMS": sums}, hashlib.sha256(sums.encode()).hexdigest()

    def staged(self, entries):
        selected = self.temp / "pg-selected"
        selected.mkdir()
        for name, data in entries.items():
            (selected / name).write_text(data)
        return selected

    def verify(self, names, selected, digests):
        AFFINITY["verify"](self.temp, "pr", names, selected, HEAD, WORKSPACE, digests,
                           PLANNER["read_cases"])

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.temp = Path(temporary.name)

    def test_every_scope_allocates_all_thirty_six_workers_once(self):
        for length in range(1, len(ARCHIVES) + 1):
            for scope in combinations(ARCHIVES, length):
                with self.subTest(scope=scope):
                    names = AFFINITY["archives"](json.dumps(scope[::-1]))
                    routed = defaultdict(list)
                    for index in range(TOTAL):
                        name, local, count = AFFINITY["route"](names, index, TOTAL)
                        routed[name].append((local, count))
                    self.assertEqual(set(routed), set(scope))
                    for workers in routed.values():
                        self.assertEqual(workers, list(enumerate([len(workers)] * len(workers))))
                    if length == len(ARCHIVES):
                        self.assertEqual({name: len(workers) for name, workers in routed.items()},
                                         AFFINITY["SLOTS"])
        for raw in ('[]', '["lib","lib"]', '["typo"]', '[1]'):
            with self.assertRaises(ValueError):
                AFFINITY["archives"](raw)
        with self.assertRaises(ValueError):
            AFFINITY["route"](ARCHIVES, 0, TOTAL - 1)

    def test_selected_library_payload_is_enough_for_every_exact_filter(self):
        mounted = PLANNER["DISPATCH_MOUNT"]
        dispatch = "oteryn-gameplay-dispatch"
        suites = {
            "e2e": {"game-server::e2e": [f"test_{i}" for i in range(17)]},
            "lib": {"durability": [f"case_{i}" for i in range(23)]},
            "lib-1": {dispatch: ["unit::ordinary", *(f"{mounted}one_{i}" for i in range(31))]},
            "lib-2": {dispatch: ["unit::ordinary", *(f"{mounted}two_{i}" for i in range(29))]},
            "lib-3": {dispatch: ["unit::ordinary", *(f"{mounted}three_{i}" for i in range(37))]},
        }
        fixtures = {name: self.archive(name, suites[name]) for name in ARCHIVES}
        expected = Counter(
            f"--filterset=binary_id(={binary}) & test(={case})"
            for archive, archives in suites.items() for binary, cases in archives.items()
            for case in cases if archive not in ("lib-2", "lib-3") or case.startswith(mounted)
        )
        actual = Counter()
        for index in range(TOTAL):
            name, local, count = AFFINITY["route"](ARCHIVES, index, TOTAL)
            _, artifact, _ = fixtures[name]
            self.staged(artifact)
            self.verify(ARCHIVES, name, {key: fixture[2] for key, fixture in fixtures.items()})
            plan = self.temp / "pg-plan"
            payloads = self.temp / "pg-archives"
            with contextlib.redirect_stdout(io.StringIO()):
                PLANNER["write_partition"](plan, local, count, {}, optional=True)
            self.assertEqual([path.name for path in payloads.iterdir()], [f"pg-{name}.tar.zst"])
            for path in plan.glob("*.filters"):
                actual.update(path.read_text().splitlines())
            for directory in (plan, payloads, self.temp / "pg-selected"):
                shutil.rmtree(directory)
        self.assertEqual(actual, expected)

    def test_corrupt_or_stale_inventory_and_payload_fail_closed(self):
        inventory, first, first_digest = self.archive("lib", {"durability": ["case"]})
        _, second, second_digest = self.archive("e2e", {"server::e2e": ["other"]})
        selected = self.staged(second)
        digests = {"lib": first_digest, "e2e": second_digest}
        for path, bad in ((inventory / "pg-lib.json", "{}"),
                          (selected / "pg-e2e.tar.zst", "stale attempt's payload"),
                          (selected / "SHA256SUMS", first["SHA256SUMS"])):
            with self.subTest(path=path):
                original = path.read_text()
                path.write_text(bad)
                with self.assertRaises(ValueError):
                    self.verify(("lib", "e2e"), "e2e", digests)
                self.assertFalse((self.temp / "pg-plan").exists())
                path.write_text(original)
        shutil.rmtree(inventory)
        with self.assertRaises(FileNotFoundError):
            self.verify(("lib", "e2e"), "e2e", digests)

    def test_duplicate_ids_across_unselected_archives_are_rejected(self):
        _, e2e, e2e_digest = self.archive("e2e", {"server::e2e": ["other"]})
        _, _, lib_digest = self.archive("lib", {"dispatch": ["case"]})
        _, _, dispatch_digest = self.archive("lib-1", {"dispatch": ["case"]})
        self.staged(e2e)
        with self.assertRaisesRegex(ValueError, "duplicate test cases"):
            self.verify(("e2e", "lib", "lib-1"), "e2e",
                        {"e2e": e2e_digest, "lib": lib_digest, "lib-1": dispatch_digest})

    def test_single_inventory_download_layout_and_fenced_identity(self):
        inventory, artifact, digest = self.archive("lib", {"durability": ["case"]})
        download = inventory.parent
        for file in inventory.iterdir():
            file.rename(download / file.name)
        inventory.rmdir()
        self.staged(artifact)
        with self.assertRaisesRegex(ValueError, "archive head mismatch"):
            AFFINITY["verify"](self.temp, "pr", ("lib",), "lib", "2" * 40, WORKSPACE,
                               {"lib": digest}, PLANNER["read_cases"])
        with self.assertRaisesRegex(ValueError, "archive workspace mismatch"):
            AFFINITY["verify"](self.temp, "pr", ("lib",), "lib", HEAD, "/another/workspace",
                               {"lib": digest}, PLANNER["read_cases"])
        self.verify(("lib",), "lib", {"lib": digest})


if __name__ == "__main__":
    unittest.main()
