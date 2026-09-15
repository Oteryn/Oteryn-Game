#!/usr/bin/env python3
from __future__ import annotations

import copy
import sys
import tempfile
import unittest
import zlib
from pathlib import Path
from unittest import mock

import spike


class ContentFormatSpikeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.fixture = spike.make_fixture(16)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_chunked_json_is_deterministic_and_round_trips_one_chunk(self) -> None:
        a = self.root / "json-a"
        b = self.root / "json-b"
        spike.write_json_project(a, self.fixture, 8)
        spike.write_json_project(b, self.fixture, 8)
        self.assertEqual(spike.artifact_digest(a), spike.artifact_digest(b))
        chunk = spike.read_json_chunk(a, (0, 0, 7))
        self.assertEqual(len(chunk["cells"]), 64)

    def test_json_source_is_multiline_for_reviewable_diff(self) -> None:
        project = self.root / "reviewable-json"
        spike.write_json_project(project, self.fixture, 8)
        manifest = spike.read_json_manifest(project)
        target = project / manifest["chunks"][0]["path"]
        self.assertGreater(target.read_bytes().count(b"\n"), 10)

    def test_malformed_json_manifest_shape_is_rejected_as_spike_error(self) -> None:
        project = self.root / "malformed-json"
        spike.write_json_project(project, self.fixture, 8)
        manifest_path = project / "manifest.json"
        manifest = spike.read_json_manifest(project)
        manifest["chunks"] = {"not": "a-list"}
        manifest_path.write_bytes(spike.canonical_pretty_json(manifest))
        with self.assertRaises(spike.SpikeError):
            spike.read_json_manifest(project)
        manifest["chunks"] = [{"key": [0, 0, 7]}]
        manifest_path.write_bytes(spike.canonical_pretty_json(manifest))
        with self.assertRaises(spike.SpikeError):
            spike.read_json_chunk(project, (0, 0, 7))

    def test_json_chunk_path_cannot_escape_project_root(self) -> None:
        project = self.root / "contained-json"
        spike.write_json_project(project, self.fixture, 8)
        manifest = spike.read_json_manifest(project)
        entry = manifest["chunks"][0]
        key = tuple(entry["key"])
        payload = spike.read_json_chunk(project, key)
        escape = self.root / "escape.json"
        escape.write_bytes(spike.canonical_json(payload))
        entry["path"] = "../escape.json"
        entry["sha256"] = spike.sha256(escape.read_bytes())
        (project / "manifest.json").write_bytes(spike.canonical_json(manifest))
        with self.assertRaises(spike.SpikeError):
            spike.read_json_chunk(project, key)

    def test_sqlite_is_deterministic_and_round_trips_one_chunk(self) -> None:
        a = self.root / "a.sqlite"
        b = self.root / "b.sqlite"
        spike.write_sqlite_project(a, self.fixture, 8)
        spike.write_sqlite_project(b, self.fixture, 8)
        self.assertEqual(spike.artifact_digest(a), spike.artifact_digest(b))
        chunk = spike.read_sqlite_chunk(a, (1, 1, 7))
        self.assertEqual(len(chunk["cells"]), 64)

    def test_binary_bundle_is_deterministic_and_client_safe(self) -> None:
        a = self.root / "a.bundle"
        b = self.root / "b.bundle"
        spike.write_binary_bundle(a, self.fixture, 8, "client")
        spike.write_binary_bundle(b, self.fixture, 8, "client")
        self.assertEqual(spike.artifact_digest(a), spike.artifact_digest(b))
        manifest = spike.read_binary_manifest(a)
        self.assertEqual(manifest["projection"], "client")
        self.assertNotIn("server_only", manifest)
        chunk = spike.read_binary_chunk(a, (0, 1, 7))
        self.assertEqual(len(chunk["cells"]), 64)

    def test_binary_corruption_is_rejected(self) -> None:
        path = self.root / "corrupt.bundle"
        spike.write_binary_bundle(path, self.fixture, 8, "server")
        data = bytearray(path.read_bytes())
        data[-1] ^= 0x01
        path.write_bytes(data)
        with self.assertRaises(spike.SpikeError):
            spike.read_binary_chunk(path, (1, 1, 7))

    def test_decompression_ratio_limit_is_fail_closed(self) -> None:
        raw = b"A" * 4096
        compressed = zlib.compress(raw, 9)
        with self.assertRaises(spike.SpikeError):
            spike.bounded_decompress(
                compressed,
                expected_raw_size=len(raw),
                max_raw_size=len(raw),
                max_ratio=2.0,
            )

    def test_unknown_critical_feature_is_rejected(self) -> None:
        fixture = copy.deepcopy(self.fixture)
        fixture["critical_features"] = ["unknown-critical"]
        with self.assertRaises(spike.SpikeError):
            spike.validate_fixture(fixture)

    def test_small_benchmark_emits_three_candidates_and_invariant(self) -> None:
        result = spike.run_benchmarks(
            self.root / "bench",
            scales=[(16, 8)],
            load_iterations=3,
        )
        self.assertEqual(
            result["spike_invariant"], "SPIKE_RESULT != OWNER_FORMAT_DECISION"
        )
        self.assertEqual(
            {entry["candidate"] for entry in result["measurements"]},
            {"chunked-json-tree", "sqlite-project", "indexed-zlib-bundle"},
        )
        self.assertTrue(
            all(entry["corruption_rejected"] for entry in result["measurements"])
        )
        json_row = next(
            entry
            for entry in result["measurements"]
            if entry["candidate"] == "chunked-json-tree"
        )
        self.assertGreater(json_row["review_diff_lines_after_one_cell_edit"], 0)
        self.assertTrue(all(result["negative_evidence"].values()))
        for key in (
            "path_traversal_rejected",
            "nesting_depth_rejected",
            "collection_count_rejected",
        ):
            self.assertTrue(result["negative_evidence"][key])
        self.assertTrue(
            all(
                row["server_only_absent"]
                for row in result["client_projection_evidence"]
            )
        )

    def test_cli_can_write_results_and_dossier_together(self) -> None:
        results = self.root / "cli-results.json"
        dossier = self.root / "cli-dossier.md"
        work = self.root / "cli-work"
        original_scales = spike.default_scales
        spike.default_scales = lambda: [(8, 8)]
        try:
            argv = [
                "spike.py",
                "--work-dir",
                str(work),
                "--results",
                str(results),
                "--dossier",
                str(dossier),
                "--base-sha",
                "deadbeef",
                "--iterations",
                "1",
            ]
            with mock.patch.object(sys, "argv", argv):
                self.assertEqual(spike.main(), 0)
        finally:
            spike.default_scales = original_scales
        self.assertTrue(results.is_file())
        self.assertIn("Owner decision required", dossier.read_text(encoding="utf-8"))

    def test_dossier_renders_invariant_candidates_and_owner_gate(self) -> None:
        result = spike.run_benchmarks(
            self.root / "dossier-bench", scales=[(16, 8)], load_iterations=2
        )
        text = spike.render_dossier(result, exact_base_sha="deadbeef")
        self.assertIn("SPIKE_RESULT != OWNER_FORMAT_DECISION", text)
        self.assertIn("chunked-json-tree", text)
        self.assertIn("sqlite-project", text)
        self.assertIn("indexed-zlib-bundle", text)
        self.assertIn("Owner decision required", text)


if __name__ == "__main__":
    unittest.main(verbosity=2)
