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
    write_snapshot_files,
)
from test_producer import valid_source


class ProducerIoTests(unittest.TestCase):
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
