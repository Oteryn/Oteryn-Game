import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("trim_cargo_cache.py")
SPEC = importlib.util.spec_from_file_location("trim_cargo_cache", SCRIPT)
trim = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = trim
SPEC.loader.exec_module(trim)


class TrimCargoCacheTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.workspace = self.root / "workspace"
        self.target = self.workspace / "target"
        self.cargo = self.root / "cargo"
        self.target.mkdir(parents=True)
        self.cargo.mkdir()
        self.source = self.workspace / "Cargo.toml"
        self.source.write_text('[package]\nname="app"\nversion="0.1.0"\n')

    def file(self, root, relative, size):
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("wb") as stream:
            stream.truncate(size)
        return path

    def unit(self, package, unit, size, *, build=False):
        fingerprint = self.file(self.target, f"debug/.fingerprint/{package}-{unit}/lib-{package}", 3)
        library = self.file(self.target, f"debug/deps/lib{package.replace('-', '_')}-{unit}.rlib", size)
        if build:
            self.file(self.target, f"debug/build/{package}-{'f' * 16}/out/generated.rs", 5)
            self.file(self.target, f"debug/.fingerprint/{package}-{'f' * 16}/run-build-script-build-script-build", 2)
        return library, fingerprint

    def prepare(self, *, budget=1024, reserve=0, check_only=False):
        return trim.prepare(self.workspace, self.target, self.cargo, max_bytes=budget,
                            reserve_bytes=reserve, check_only=check_only, packages={"app"})

    def test_workspace_incremental_and_final_products_removed_dependency_units_untouched(self):
        dependency, fingerprint = self.unit("third-party", "a" * 16, 80, build=True)
        workspace, workspace_fingerprint = self.unit("app", "b" * 16, 100)
        incremental = self.file(self.target, "debug/incremental/app/work.bin", 200)
        executable = self.file(self.target, "debug/app", 300)
        registry = self.file(self.cargo, "registry/cache/source/archive.crate", 40)
        outside_cache = self.file(self.cargo, "registry/src/untouched/source.rs", 1500)
        snapshot = self.file(self.target, ".oteryn-cargo-source-snapshot-v1.json", 9)
        unchanged = dependency.stat(), fingerprint.stat(), self.source.read_bytes()
        report = self.prepare()
        for removed in (workspace, workspace_fingerprint, incremental, executable):
            self.assertFalse(removed.exists())
        self.assertFalse((self.target / "debug/incremental").exists())
        for kept in (dependency, fingerprint, registry, outside_cache, snapshot):
            self.assertTrue(kept.is_file())
        self.assertEqual((dependency.stat(), fingerprint.stat(), self.source.read_bytes()), unchanged)
        self.assertEqual(report["after"]["registry_bytes"], 40)
        self.assertEqual(report["removed_files"], 4)
        self.assertEqual(report["before"]["total_bytes"] - report["after"]["total_bytes"], report["removed_bytes"])

    def test_largest_dependency_group_evicted_with_all_its_fingerprints_and_build_outputs(self):
        big, big_fingerprint = self.unit("large", "a" * 16, 80, build=True)
        small, small_fingerprint = self.unit("small", "b" * 16, 20)
        self.file(self.cargo, "registry/index/index-data", 60)
        report = self.prepare(budget=100)
        self.assertLessEqual(report["after"]["total_bytes"], 100)
        self.assertEqual(report["evicted_packages"], ["large"])
        self.assertFalse(big.exists())
        self.assertFalse(big_fingerprint.exists())
        self.assertFalse((self.target / f"debug/build/large-{'f' * 16}/out/generated.rs").exists())
        self.assertTrue(small.is_file())
        self.assertTrue(small_fingerprint.is_file())

    def test_real_rust_1_100_package_hash_layout_preserves_dependency_outputs_and_fingerprints(self):
        # Captured from the real offline Cargo proof, not inferred from the
        # older deps/.fingerprint layout: each unit owns out/ and fingerprint/.
        dependency_root = "debug/build/cache-proof-dependency/1ccdd08af7d3c8a7"
        dependency = self.file(self.target, dependency_root + "/out/libcache_proof_dependency-1ccdd08af7d3c8a7.rlib", 80)
        metadata = self.file(self.target, dependency_root + "/out/libcache_proof_dependency-1ccdd08af7d3c8a7.rmeta", 20)
        fingerprint = self.file(self.target, dependency_root + "/fingerprint/lib-cache_proof_dependency", 3)
        depinfo = self.file(self.target, dependency_root + "/fingerprint/dep-lib-cache_proof_dependency", 2)
        workspace_root = "debug/build/oteryn-cache-proof/2560faf127852f74"
        executable = self.file(self.target, workspace_root + "/out/oteryn_cache_proof-2560faf127852f74", 200)
        workspace_fingerprint = self.file(self.target, workspace_root + "/fingerprint/test-lib-oteryn_cache_proof", 3)
        trim.prepare(self.workspace, self.target, self.cargo, max_bytes=1024, reserve_bytes=0,
                     packages={"oteryn-cache-proof"})
        for kept in (dependency, metadata, fingerprint, depinfo):
            self.assertTrue(kept.is_file(), kept)
        self.assertFalse(executable.exists())
        self.assertFalse(workspace_fingerprint.exists())

    def test_sparse_61_gib_output_counts_logical_bytes_and_is_not_read_into_memory(self):
        large = self.file(self.target, "debug/app", 61 << 30)
        self.assertEqual(large.stat().st_size, 61 << 30)
        self.assertLess(large.stat().st_blocks * 512, 1 << 20)
        report = self.prepare(budget=4 << 30, reserve=16 << 20)
        self.assertEqual(report["before"]["total_bytes"], 61 << 30)
        self.assertEqual(report["removed_bytes"], 61 << 30)
        self.assertFalse(large.exists())

    def test_registry_alone_above_budget_fails_before_target_mutation(self):
        output = self.file(self.target, "debug/app", 50)
        registry = self.file(self.cargo, "git/db/data", 101)
        with self.assertRaisesRegex(ValueError, "registry alone"):
            self.prepare(budget=100)
        self.assertEqual(output.stat().st_size, 50)
        self.assertEqual(registry.stat().st_size, 101)

    def test_source_snapshot_reserve_is_left_before_the_final_actual_size_check(self):
        library, _ = self.unit("large", "a" * 16, trim.DEFAULT_BUDGET - (8 << 20))
        report = self.prepare(budget=trim.DEFAULT_BUDGET, reserve=trim.SNAPSHOT_RESERVE)
        self.assertFalse(library.exists())
        self.assertLessEqual(report["after"]["total_bytes"], trim.DEFAULT_BUDGET - trim.SNAPSHOT_RESERVE)
        self.file(self.target, ".oteryn-cargo-source-snapshot-v1.json", 2 << 20)
        final = self.prepare(budget=trim.DEFAULT_BUDGET, check_only=True)
        self.assertEqual(final["after"]["total_bytes"], 2 << 20)
        self.assertEqual(final["removed_files"], 0)

    def test_check_only_rejects_post_record_oversize_without_removing_anything(self):
        library, fingerprint = self.unit("small", "a" * 16, 70)
        snapshot = self.file(self.target, ".oteryn-cargo-source-snapshot-v1.json", 30)
        with patch.object(trim, "workspace_packages", side_effect=AssertionError("check-only must not invoke Cargo")):
            with self.assertRaisesRegex(ValueError, "publication is blocked"):
                self.prepare(budget=100, check_only=True)
        for path in (library, fingerprint, snapshot):
            self.assertTrue(path.is_file())

    def test_unremovable_snapshot_prevents_partial_destructive_plan(self):
        snapshot = self.file(self.target, ".oteryn-cargo-source-snapshot-v1.json", 101)
        executable = self.file(self.target, "debug/app", 50)
        with self.assertRaisesRegex(ValueError, "source snapshot"):
            self.prepare(budget=100)
        self.assertTrue(snapshot.is_file())
        self.assertTrue(executable.is_file())

    def test_target_symlink_and_unmanaged_target_are_rejected(self):
        outside = self.root / "outside"
        protected = self.file(outside, "keep", 30)
        self.target.rmdir()
        self.target.symlink_to(outside, target_is_directory=True)
        with self.assertRaises(OSError):
            self.prepare()
        with self.assertRaisesRegex(ValueError, "workspace target"):
            trim.prepare(self.workspace, outside, self.cargo, packages={"app"})
        self.assertEqual(protected.stat().st_size, 30)

    def test_nested_and_dangling_symlinks_never_follow_or_modify_their_destinations(self):
        outside = self.root / "outside"
        protected = self.file(outside, "keep", 1 << 20)
        (self.target / "outside-link").symlink_to(outside, target_is_directory=True)
        (self.target / "dangling").symlink_to(self.root / "missing")
        hardlink = self.target / "hardlink"
        os.link(protected, hardlink)
        report = self.prepare(budget=100)
        self.assertEqual(protected.stat().st_size, 1 << 20)
        self.assertFalse(os.path.lexists(self.target / "outside-link"))
        self.assertFalse(os.path.lexists(self.target / "dangling"))
        self.assertFalse(hardlink.exists())
        self.assertEqual(report["after"]["target_bytes"], 0)

    def test_directory_replaced_by_symlink_between_scan_and_delete_fails_closed(self):
        self.file(self.target, "debug/product", 10)
        entry = trim.scan(self.target)[0]
        outside = self.root / "outside"
        protected = self.file(outside, "product", 10)
        (self.target / "debug").rename(self.target / "old-debug")
        (self.target / "debug").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(OSError):
            trim.unlink_entry(self.target, entry)
        self.assertEqual(protected.stat().st_size, 10)

    def test_registry_root_symlink_or_overlap_is_rejected_without_touching_target(self):
        product = self.file(self.target, "debug/product", 10)
        (self.cargo / "registry").symlink_to(self.root / "missing", target_is_directory=True)
        with self.assertRaises(OSError): self.prepare()
        with self.assertRaisesRegex(ValueError, "disjoint"):
            trim.prepare(self.workspace, self.target, self.target / "cargo", packages={"app"})
        self.assertTrue(product.is_file())

    def test_cli_check_only_emits_final_budget_metrics_and_guards_source_output(self):
        self.file(self.cargo, "registry/cache/one.crate", 40)
        self.file(self.target, ".rustc_info.json", 10)
        metrics = self.root / "metrics.json"
        command = [sys.executable, str(SCRIPT), "--workspace", str(self.workspace), "--target", "target",
                   "--cargo-home", str(self.cargo), "--check-only", "--max-bytes", "100",
                   "--metrics", str(metrics)]
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        report = json.loads(metrics.read_text())
        self.assertEqual(report["before"]["total_bytes"], 50)
        self.assertEqual(report["after"]["total_bytes"], 50)
        self.assertEqual(report["removed_files"], 0)
        original = self.source.read_bytes()
        command[-1] = str(self.source)
        rejected = subprocess.run(command, capture_output=True, text=True)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertEqual(self.source.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
