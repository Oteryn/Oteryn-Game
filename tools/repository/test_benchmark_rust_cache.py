#!/usr/bin/env python3
"""Prove benchmark isolation and honest sample grouping without compiling Rust."""
from pathlib import Path
import unittest

from benchmark_rust_cache import cargo_command, local_environment, summarize


class RustCacheBenchmarkTests(unittest.TestCase):
    def test_external_backends_and_wrappers_cannot_contaminate_plain_build(self):
        parent = {"PATH": "/bin", "RUSTFLAGS": "-Cdebuginfo=0", "CARGO_INCREMENTAL": "1",
                  "RUSTC_WRAPPER": "foreign", "RUSTC_WORKSPACE_WRAPPER": "other",
                  "SCCACHE_GHA_ENABLED": "on", "SCCACHE_BUCKET": "foreign",
                  "SCCACHE_REDIS": "foreign", "SCCACHE_SERVER_PORT": "123"}
        env = local_environment(parent, Path("/tmp/owned/target"), Path("/tmp/owned/cache"), Path("/tmp/owned/sccache.sock"))
        for name in ("SCCACHE_GHA_ENABLED", "SCCACHE_BUCKET", "SCCACHE_REDIS"):
            self.assertNotIn(name, env)
        self.assertEqual(env["RUSTC_WRAPPER"], "")
        self.assertEqual(env["RUSTC_WORKSPACE_WRAPPER"], "")
        self.assertEqual(env["CARGO_INCREMENTAL"], "0")
        self.assertNotIn("SCCACHE_SERVER_PORT", env)
        self.assertEqual(env["SCCACHE_SERVER_UDS"], "/tmp/owned/sccache.sock")
        self.assertEqual(env["SCCACHE_DIR"], "/tmp/owned/cache")
        self.assertEqual(env["RUSTFLAGS"], parent["RUSTFLAGS"])
        self.assertEqual(parent["CARGO_INCREMENTAL"], "1")

    def test_actual_workspace_build_remains_locked_and_offline(self):
        workspace = cargo_command("cargo", None)
        self.assertIn("--locked", workspace)
        self.assertIn("--offline", workspace)
        self.assertEqual(workspace[-2:], ["--workspace", "--all-targets"])
        package = cargo_command("cargo", "oteryn-game-server")
        self.assertEqual(package[-3:], ["-p", "oteryn-game-server", "--lib"])

    def test_cold_and_warm_samples_are_not_pooled(self):
        rows = [{"case": "cold", "build_seconds": 20},
                {"case": "warm", "build_seconds": 2},
                {"case": "warm", "build_seconds": 4}]
        grouped = summarize(rows)
        self.assertEqual(grouped["cold"]["samples"], 1)
        self.assertEqual(grouped["cold"]["median_seconds"], 20)
        self.assertEqual(grouped["warm"], {"samples": 2, "median_seconds": 3,
                                           "min_seconds": 2, "max_seconds": 4})


if __name__ == "__main__":
    unittest.main()
