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


def make_d3_fixture():
    deferred = {
        "canonical_target_spatial_address": "DEFERRED_REQUIRES_PHASE_B",
        "target_ordered_placement_sequence": "DEFERRED_REQUIRES_PHASE_B",
        "target_collision_walkability": "DEFERRED_REQUIRES_PHASE_B",
    }
    placement = {
        "appearance_source_id": 2031,
        "source_role": "ground",
        "source_presentation_order": {"plane": 0, "order": 1},
        "identity_disposition": "UNRESOLVED_SOURCE_IDENTITY",
        "typed_definition_ref": None,
        "placement_key": None,
        "target_sensitive_fields": dict(deferred),
    }
    first = dict(placement, source_occurrence_ref="presentation:aaa")
    second = dict(
        placement,
        source_occurrence_ref="presentation:bbb",
        appearance_source_id=3687,
        source_role="tile_item",
        source_presentation_order={"plane": 0, "order": 2},
    )
    third = dict(placement, source_occurrence_ref="presentation:ccc")
    return {
        "schema_version": 1,
        "world_id": "d3-test-world",
        "critical_features": ["chunk-index-v1", "projection-v1"],
        "provenance": {
            "measurement_profile": spike.D3_REAL_BATCH_PROFILE,
            "classification": spike.D3_SOURCE_CLASSIFICATION,
            "source_generation_profile_id": spike.D3_FRESH_SOURCE_PROFILE,
            "source_generation_profile_revision": 2,
            "selection": {
                "windows": [
                    {"name": "newhaven", "retained_shard": "source-shard-a"}
                ]
            },
        },
        "definitions": {
            "source-appearance:2031": {
                "definition_kind": "SOURCE_APPEARANCE_REFERENCE",
                "appearance_source_id": 2031,
                "identity_disposition": "SOURCE_ID_ONLY_NOT_CANONICAL",
                "production_authority": "NONE",
            },
            "source-appearance:3687": {
                "definition_kind": "SOURCE_APPEARANCE_REFERENCE",
                "appearance_source_id": 3687,
                "identity_disposition": "SOURCE_ID_ONLY_NOT_CANONICAL",
                "production_authority": "NONE",
            },
        },
        "server_only": {
            "typed_batch_schema": "OTERYN_REFERENCE_CONTENT_SOURCE_BATCH/v1",
            "production_authority": "NONE",
            "reference_parity_claim": "NONE",
        },
        "cells": [
            {"x": 16, "y": 16, "z": -7, "source_placements": [second, first]},
            {"x": 25, "y": 25, "z": -7, "source_placements": [third]},
        ],
    }


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


    def test_d3_two_carriers_share_logical_index_and_round_trip(self) -> None:
        fixture = make_d3_fixture()
        baseline = self.root / "d3-none"
        compressed = self.root / "d3-zlib"
        for target, compression in ((baseline, "none"), (compressed, "zlib")):
            spike.write_d3_carrier(
                target, fixture, 8, compression=compression, projection="server"
            )
        none_manifest = spike.read_d3_manifest(
            baseline, expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE
        )
        zlib_manifest = spike.read_d3_manifest(
            compressed, expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE
        )
        self.assertEqual(
            none_manifest["logical_identity_sha256"],
            zlib_manifest["logical_identity_sha256"],
        )
        self.assertEqual(
            spike.d3_index_signature(none_manifest),
            spike.d3_index_signature(zlib_manifest),
        )
        self.assertEqual(
            spike.canonical_json(
                spike.reconstruct_d3_fixture(
                    baseline, expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE
                )
            ),
            spike.canonical_json(spike.d3_normalize_fixture(fixture)),
        )
        cell = spike.read_d3_cell(
            compressed,
            (16, 16, -7),
            expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE,
        )
        self.assertEqual(len(cell["source_placements"]), 2)
        placement = spike.read_d3_placement(
            compressed,
            "presentation:aaa",
            expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE,
        )
        self.assertEqual(placement["appearance_source_id"], 2031)
        definition = spike.read_d3_definition(
            compressed,
            "source-appearance:2031",
            expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE,
        )
        self.assertEqual(
            definition["identity_disposition"], "SOURCE_ID_ONLY_NOT_CANONICAL"
        )

    def test_d3_client_projection_and_negative_boundaries_fail_closed(self) -> None:
        fixture = make_d3_fixture()
        server = self.root / "d3-server"
        client = self.root / "d3-client"
        spike.write_d3_carrier(
            server, fixture, 8, compression="zlib", projection="server"
        )
        spike.write_d3_carrier(
            client,
            spike.d3_client_fixture(fixture),
            8,
            compression="zlib",
            projection="client",
        )
        manifest = spike.read_d3_manifest(
            client, expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE
        )
        self.assertNotIn("server_only", manifest)
        self.assertEqual(
            spike.canonical_json(
                spike.reconstruct_d3_fixture(
                    client, expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE
                )
            ),
            spike.canonical_json(spike.d3_client_fixture(fixture)),
        )
        target_key = (2, 2, -7)
        for mode in ("corrupt", "truncate"):
            self.assertTrue(
                spike._d3_corruption_rejected(
                    server,
                    target_key,
                    self.root,
                    expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE,
                    mode=mode,
                )
            )
        for mutation in ("profile", "version", "critical", "placement-index", "raw-size"):
            self.assertTrue(
                spike._d3_manifest_negative(
                    server,
                    self.root,
                    expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE,
                    mutation=mutation,
                )
            )
        leaky = make_d3_fixture()
        leaky["provenance"]["server_secret"] = "must-not-reach-client"
        with self.assertRaises(spike.SpikeError):
            spike.d3_client_fixture(leaky)


    def test_d3_identity_is_enumeration_and_rechunk_independent(self) -> None:
        fixture = make_d3_fixture()
        reversed_fixture = copy.deepcopy(fixture)
        reversed_fixture["cells"].reverse()
        for cell in reversed_fixture["cells"]:
            cell["source_placements"].reverse()
        self.assertEqual(
            spike.d3_logical_identity(fixture),
            spike.d3_logical_identity(reversed_fixture),
        )
        shard_variant = copy.deepcopy(fixture)
        shard_variant["provenance"]["selection"]["windows"][0][
            "retained_shard"
        ] = "source-shard-b"
        self.assertEqual(
            spike.d3_logical_identity(fixture),
            spike.d3_logical_identity(shard_variant),
        )

        a = self.root / "d3-c8"
        b = self.root / "d3-c16"
        spike.write_d3_carrier(
            a, fixture, 8, compression="none", projection="server"
        )
        spike.write_d3_carrier(
            b, fixture, 16, compression="none", projection="server"
        )
        ma = spike.read_d3_manifest(
            a, expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE
        )
        mb = spike.read_d3_manifest(
            b, expected_source_profile=spike.D3_FRESH_SOURCE_PROFILE
        )
        self.assertEqual(
            ma["logical_identity_sha256"], mb["logical_identity_sha256"]
        )

    def test_d3_one_record_update_changes_manifest_and_one_chunk(self) -> None:
        fixture = make_d3_fixture()
        before = self.root / "d3-before"
        after = self.root / "d3-after"
        spike.write_d3_carrier(
            before, fixture, 8, compression="none", projection="server"
        )
        spike.write_d3_carrier(
            after,
            spike._d3_mutated_fixture(fixture),
            8,
            compression="none",
            projection="server",
        )
        names, patch_bytes = spike._d3_changed_artifact_metrics(before, after)
        self.assertEqual(len(names), 2)
        self.assertIn("manifest.json", names)
        self.assertEqual(sum(name.startswith("chunks/") for name in names), 1)
        self.assertGreater(patch_bytes, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
