"""Cover rerun custody and the PR harness's exact nextest selections."""

from collections import Counter
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from pg_test_rerun import artifact_catalog, check_listing, plan


HEAD = "1" * 40
RUN = 123
REPO = 45
PREFIX = "pr-postgres-test-archives-"
WORKSPACE = "/home/runner/work/Oteryn-Game/Oteryn-Game"
DISPATCH = "oteryn-gameplay-dispatch"
MOUNT = "gameplay_transport::monk_save::postgres_tests::"
ORDINARY = (DISPATCH, "unit::ordinary")


def filter_line(case):
    binary, test = case
    return f"--filterset=binary_id(={binary}) & test(={test})\n"


def write_listing(path, cases):
    suites = {}
    for binary, name in cases:
        suite = suites.setdefault(binary, {"binary-id": binary, "testcases": {}})
        suite["testcases"][name] = {"filter-match": {"status": "matches"}}
    path.write_text(json.dumps({"rust-suites": suites}))


def artifact(name, artifact_id=55, *, run=RUN, head=HEAD, repo=REPO, expired=False):
    return {"name": PREFIX + name, "id": artifact_id, "expired": expired,
            "workflow_run": {"id": run, "head_sha": head, "repository_id": REPO,
                             "head_repository_id": repo}}


class RerunArchiveTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.download = self.root / "download"
        self.filters = self.root / "filters"
        self.output = self.root / "output"

    def bundle(self, name, cases, *, flat=False, head=HEAD, workspace=WORKSPACE):
        folder = self.download if flat else self.download / (PREFIX + name)
        folder.mkdir(parents=True)
        payload = f"pg-{name}.tar.zst"
        inventory = f"pg-{name}.json"
        (folder / "head-sha").write_text(head + "\n")
        (folder / "workspace").write_text(workspace + "\n")
        (folder / payload).write_bytes(b"archive payload")
        write_listing(folder / inventory, cases)
        # Match the producer command and its bare, ordered filenames.
        manifest = subprocess.check_output(["sha256sum", "head-sha", "workspace", payload, inventory],
                                           cwd=folder)
        (folder / "SHA256SUMS").write_bytes(manifest)
        return folder

    def run_plan(self, names, cases):
        self.filters.write_text("".join(map(filter_line, cases)))
        catalog = artifact_catalog([{"artifacts": [artifact(name, index)
                                                    for index, name in enumerate(names, 1)]}],
                                   RUN, HEAD, REPO)
        plan(self.download, catalog, HEAD, WORKSPACE, self.filters, self.output)

    def test_producer_bundles_plan_every_requested_test_once(self):
        cases = {
            "e2e": ("oteryn-game-server::durability_postgres", "round_trip"),
            "lib": ("oteryn-gameplay-durability", "postgres::lookup"),
            "lib-1": (DISPATCH, MOUNT + "one"),
            "lib-2": (DISPATCH, MOUNT + "two"),
            "lib-3": (DISPATCH, MOUNT + "three"),
        }
        for name, case in cases.items():
            self.bundle(name, [case, ORDINARY] if name.startswith("lib-") else [case])
        wanted = [*cases.values(), ORDINARY]
        self.run_plan(cases, wanted + [ORDINARY])
        actual = Counter()
        for name, case in cases.items():
            selected = self.output / f"pg-{name}.filters"
            expected = [case, ORDINARY] if name == "lib-1" else [case]
            self.assertEqual(set(selected.read_text().splitlines(keepends=True)), set(map(filter_line, expected)))
            self.assertTrue((self.output / f"pg-{name}.tar.zst").is_file())
            actual.update(selected.read_text().splitlines(keepends=True))
            listing = self.root / f"listed-{name}.json"
            write_listing(listing, expected)
            self.assertEqual(check_listing(selected, listing), len(expected))
        self.assertEqual(actual, Counter(map(filter_line, wanted)))

    def test_single_flat_combined_library_supports_unsharded_source(self):
        self.bundle("lib", [ORDINARY], flat=True)
        self.run_plan(["lib"], [ORDINARY])
        self.assertEqual((self.output / "pg-lib.filters").read_text(), filter_line(ORDINARY))

    def test_catalog_rejects_wrong_source_run_and_invalid_archive_inventory(self):
        valid = artifact("lib")
        self.assertEqual(artifact_catalog([{"artifacts": [valid, {"name": "other"}]}], RUN, HEAD, REPO),
                         {"pg-lib": 55})
        for entries in ([artifact("lib", run=RUN + 1)], [artifact("lib", head="2" * 40)],
                        [artifact("lib", repo=REPO + 1)], [artifact("lib", expired=True)],
                        [artifact("lib-4")], [valid, valid], [valid, artifact("lib-1")], []):
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                artifact_catalog([{"artifacts": entries}], RUN, HEAD, REPO)

    def test_rejects_missing_and_unexpected_downloads(self):
        self.bundle("lib", [ORDINARY])
        for names in (["lib", "lib-1"], ["lib-1"]):
            with self.subTest(names=names), self.assertRaisesRegex(ValueError, "archive downloads"):
                self.run_plan(names, [ORDINARY])
        self.assertFalse(self.output.exists())

    def test_rejects_invalid_manifest_names_files_and_checksums(self):
        folder = self.bundle("lib", [ORDINARY])
        manifest = folder / "SHA256SUMS"
        original = manifest.read_bytes()
        for contents in (original.replace(b"  pg-lib.tar.zst\n", b"  ../pg-lib.tar.zst\n"),
                         original.replace(b"  pg-lib.tar.zst\n", b"  pg-e2e.tar.zst\n"),
                         original + original.splitlines(keepends=True)[0],
                         original.replace(b"  workspace\n", b"  head-sha\n"),
                         original.replace(b"  pg-lib.tar.zst\n", b"  ./pg-lib.tar.zst\n")):
            with self.subTest(contents=contents), self.assertRaisesRegex(ValueError, "checksum manifest"):
                manifest.write_bytes(contents)
                self.run_plan(["lib"], [ORDINARY])
        manifest.write_bytes(original)
        payload = folder / "pg-lib.tar.zst"
        payload.unlink()
        with self.assertRaisesRegex(ValueError, "bundle files"):
            self.run_plan(["lib"], [ORDINARY])
        payload.symlink_to(folder / "head-sha")
        with self.assertRaisesRegex(ValueError, "regular file"):
            self.run_plan(["lib"], [ORDINARY])
        payload.unlink()
        payload.write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "bad checksum"):
            self.run_plan(["lib"], [ORDINARY])
        self.assertFalse(self.output.exists())

    def test_rejects_rehashed_wrong_head_and_workspace(self):
        self.bundle("lib", [ORDINARY], head="2" * 40)
        self.bundle("lib-1", [(DISPATCH, MOUNT + "one")], workspace="/wrong/workspace")
        for name, mismatch in (("lib", "head mismatch"), ("lib-1", "workspace mismatch")):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, mismatch):
                plan(self.download / (PREFIX + name), {f"pg-{name}": 55}, HEAD, WORKSPACE,
                     self.filters, self.output)
        self.assertFalse(self.output.exists())

    def test_missing_selection_or_runtime_mismatch_fails(self):
        self.bundle("lib", [ORDINARY])
        missing = (DISPATCH, "unit::missing")
        with self.assertRaisesRegex(ValueError, "unit::missing"):
            self.run_plan(["lib"], [ORDINARY, missing])
        self.assertFalse(self.output.exists())
        self.run_plan(["lib"], [ORDINARY])
        selection = self.output / "pg-lib.filters"
        listing = self.root / "listed.json"
        for actual in ([], [ORDINARY, missing]):
            with self.subTest(actual=actual), self.assertRaisesRegex(ValueError, "nextest selection differs"):
                write_listing(listing, actual)
                check_listing(selection, listing)


if __name__ == "__main__":
    unittest.main()
