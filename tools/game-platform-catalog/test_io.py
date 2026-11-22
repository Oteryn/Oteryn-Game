from __future__ import annotations

import hashlib
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from producer import (
    CatalogValidationError,
    build_snapshot,
    canonical_json_bytes,
    load_json_file,
    verify_snapshot,
    write_snapshot_files,
)
from test_producer import valid_source


class ProducerIoTests(unittest.TestCase):
    def test_public_payload_fixture_and_fail_closed_validation(self) -> None:
        source = load_json_file(HERE / "fixtures/supported-native-input.json")
        snapshot = build_snapshot(source)
        self.assertEqual(
            {row["state"] for row in snapshot["completeness_manifest"]},
            {"partial", "unknown"},
        )
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "public-catalog.json"
            write_snapshot_files(snapshot, output)
            verify_snapshot(load_json_file(output))

        item = next(row for row in source["entities"] if row["capability_id"] == "item")
        item["data"]["payload_revision"] = 2
        with self.assertRaisesRegex(CatalogValidationError, "payload_revision"):
            build_snapshot(source)
        item["data"]["payload_revision"] = 1
        item["data"]["source_fields"] = {}
        with self.assertRaisesRegex(CatalogValidationError, "source_fields"):
            build_snapshot(source)
        del item["data"]["source_fields"]

        for manifest, field, value in (
            ("capability_manifest", "support", "unsupported"),
            ("completeness_manifest", "state", "unknown"),
        ):
            next(row for row in source[manifest] if row["capability_id"] == "item")[field] = value
        with self.assertRaisesRegex(CatalogValidationError, "references unsupported capability: item"):
            build_snapshot(source)
        source["required_capabilities"] = ["item"]
        with self.assertRaisesRegex(CatalogValidationError, "required capability item is unsupported"):
            build_snapshot(source)

    def test_public_payload_verifier_checks_endpoints_after_valid_digest(self) -> None:
        source = load_json_file(HERE / "fixtures/supported-native-input.json")
        snapshot = build_snapshot(source)
        shop = next(row for row in snapshot["relations"] if row["capability_id"] == "npc_shop")
        shop["target"] = shop["source"]
        integrity = {key: value for key, value in snapshot.items()
                     if key not in {"snapshot_id", "payload_digest"}}
        digest = "sha256:" + hashlib.sha256(canonical_json_bytes(integrity)).hexdigest()
        snapshot["snapshot_id"] = snapshot["payload_digest"] = digest
        with self.assertRaisesRegex(CatalogValidationError, "endpoint type"):
            verify_snapshot(snapshot)

    def test_loader_rejects_duplicate_json_object_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text(
                '{"authority_epoch":"a","authority_epoch":"b"}', encoding="utf-8"
            )
            with self.assertRaisesRegex(
                CatalogValidationError, "duplicate JSON object key"
            ):
                load_json_file(path)

    def test_writer_emits_canonical_snapshot_and_sha256_sidecar(self) -> None:
        snapshot = build_snapshot(valid_source())
        expected_bytes = canonical_json_bytes(snapshot) + b"\n"
        expected_digest = hashlib.sha256(expected_bytes).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "game-platform-catalog.json"
            write_snapshot_files(snapshot, output)
            self.assertEqual(output.read_bytes(), expected_bytes)
            self.assertEqual(
                output.with_suffix(output.suffix + ".sha256").read_text(
                    encoding="ascii"
                ),
                expected_digest + "\n",
            )

    def test_loader_rejects_file_above_hard_limit_before_json_parse(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "oversized.json"
            from unittest.mock import patch

            with patch("producer.MAX_FILE_BYTES", 8):
                path.write_bytes(b"{" + b"x" * 8 + b"}")
                with self.assertRaisesRegex(CatalogValidationError, "file size limit"):
                    load_json_file(path)

    def test_builder_rejects_oversized_snapshot_before_publication(self) -> None:
        source = valid_source()
        source["entities"][0]["data"]["description"] = "x" * 1500
        from unittest.mock import patch

        with (
            patch("producer.MAX_FILE_BYTES", 512),
            self.assertRaisesRegex(
                CatalogValidationError, "snapshot exceeds file size limit"
            ),
        ):
            build_snapshot(source)


if __name__ == "__main__":
    unittest.main(verbosity=2)
