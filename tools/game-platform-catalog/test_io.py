from __future__ import annotations

import hashlib
import sys
import tempfile
import unittest
from concurrent.futures import ThreadPoolExecutor, TimeoutError
from pathlib import Path
from threading import Event
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from producer import (
    CatalogValidationError,
    build_snapshot,
    canonical_json_bytes,
    load_json_file,
    main,
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
            next(row for row in source[manifest] if row["capability_id"] == "item")[
                field
            ] = value
        with self.assertRaisesRegex(
            CatalogValidationError, "references unsupported capability: item"
        ):
            build_snapshot(source)
        source["required_capabilities"] = ["item"]
        with self.assertRaisesRegex(
            CatalogValidationError, "required capability item is unsupported"
        ):
            build_snapshot(source)

    def test_public_payload_verifier_checks_endpoints_after_valid_digest(self) -> None:
        source = load_json_file(HERE / "fixtures/supported-native-input.json")
        snapshot = build_snapshot(source)
        shop = next(
            row for row in snapshot["relations"] if row["capability_id"] == "npc_shop"
        )
        shop["target"] = shop["source"]
        integrity = {
            key: value
            for key, value in snapshot.items()
            if key not in {"snapshot_id", "payload_digest"}
        }
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

    def test_concurrent_public_profiles_publish_matching_sidecar(self) -> None:
        fixtures = HERE / "fixtures"
        default = fixtures / "unsupported-native-input.json"
        preproduction = fixtures / "supported-native-input.json"
        published_json = Event()
        resume_default = Event()
        replace = Path.replace

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "catalog.json"

            def pause_after_first_json(path, destination):
                result = replace(path, destination)
                if destination == output and not published_json.is_set():
                    published_json.set()
                    if not resume_default.wait(10):
                        raise TimeoutError("default publication did not resume")
                return result

            def produce(source):
                return main(["produce", str(source), str(output)])

            with (
                patch.object(Path, "replace", pause_after_first_json),
                ThreadPoolExecutor(2) as pool,
            ):
                first = pool.submit(produce, default)
                try:
                    self.assertTrue(published_json.wait(10))
                    second = pool.submit(produce, preproduction)
                    try:
                        second.result(timeout=1)
                    except TimeoutError:
                        pass  # A serialized writer waits for the first publication.
                finally:
                    resume_default.set()
                self.assertEqual(first.result(timeout=10), 0)
                self.assertEqual(second.result(timeout=10), 0)

            verify_snapshot(load_json_file(output))
            self.assertEqual(
                output.with_suffix(".json.sha256").read_text(encoding="ascii"),
                hashlib.sha256(output.read_bytes()).hexdigest() + "\n",
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


class PublicProfileRejectionTests(unittest.TestCase):
    def assert_cli_rejects(self, change, diagnostic):
        from contextlib import redirect_stderr, redirect_stdout
        from io import StringIO

        from producer import main

        source = load_json_file(HERE / "fixtures/supported-native-input.json")
        snapshot = build_snapshot(source)
        change(source)
        change(snapshot)
        snapshot["relations"].sort(key=lambda row: (row["type"], row["relation_key"]))
        integrity = {
            key: value
            for key, value in snapshot.items()
            if key not in {"snapshot_id", "payload_digest"}
        }
        digest = "sha256:" + hashlib.sha256(canonical_json_bytes(integrity)).hexdigest()
        snapshot["snapshot_id"] = snapshot["payload_digest"] = digest
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source_path = root / "input.json"
            artifact_path = root / "artifact.json"
            source_path.write_bytes(canonical_json_bytes(source))
            artifact_path.write_bytes(canonical_json_bytes(snapshot))
            for command in (
                ["produce", str(source_path), str(root / "output.json")],
                ["verify", str(artifact_path)],
            ):
                with (
                    self.subTest(command=command[0]),
                    redirect_stdout(StringIO()),
                    redirect_stderr(StringIO()) as errors,
                ):
                    self.assertEqual(main(command), 2)
                    self.assertIn(diagnostic, errors.getvalue())

    def assert_field_rejected(self, capability, field, value, diagnostic=None):
        def change(document):
            row = next(
                row
                for row in document["entities"] + document["relations"]
                if row["capability_id"] == capability
            )
            row["data"][field] = value

        with self.subTest(capability=capability, field=field, value=value):
            self.assert_cli_rejects(change, diagnostic or field)

    def test_identity_only_items_cannot_be_materialized(self):
        self.assert_field_rejected("item", "materializable", True)

    def test_native_numeric_bounds(self):
        for cap, field, values in (
            ("creature", "health", [0]),
            ("creature", "speed", [1 << 32]),
            ("creature", "armor", [1 << 32]),
            ("creature_loot", "min_count", [0]),
            ("creature_loot", "max_count", [1 << 32]),
            ("npc_shop", "count", [0, 1 << 32]),
            ("npc_shop", "sub_type", [1 << 16]),
        ):
            for value in values:
                self.assert_field_rejected(cap, field, value)

        for count in (0, 1 << 32):

            def change(document, count=count):
                loot = next(
                    row
                    for row in document["relations"]
                    if row["capability_id"] == "creature_loot"
                )
                loot["data"].update(min_count=count, max_count=count)

            with self.subTest(loot_quantity=count):
                self.assert_cli_rejects(change, "min_count")

    def test_creature_arrays_are_canonical(self):
        def resistance(damage_type, numerator=15, denominator=1):
            return {
                "damage_type": damage_type,
                "percent": {"numerator": numerator, "denominator": denominator},
            }

        for field, value in (
            ("resistances", [resistance("death"), resistance("death")]),
            ("resistances", [resistance("fire"), resistance("death")]),
            ("resistances", [resistance("death", 30, 2)]),
            ("immunities", ["death", "death"]),
            ("immunities", ["fire", "death"]),
        ):
            self.assert_field_rejected("creature", field, value)

    def test_native_text_constraints(self):
        for capability in ("item", "creature", "npc"):
            for revision in ("\n", "definition r1", "x" * 513, "fixture-r1"):
                self.assert_field_rejected(capability, "definition_revision", revision)
        self.assert_field_rejected("item", "name", "é" * 23 + "x")
        for text in (" fire", "fire ", "fi\nre", "fi\x85re"):
            self.assert_field_rejected("creature", "name", text)
            self.assert_field_rejected("creature", "immunities", [text], "immunity")
            self.assert_field_rejected(
                "creature",
                "resistances",
                [
                    {
                        "damage_type": text,
                        "percent": {"numerator": 15, "denominator": 1},
                    },
                ],
                "damage_type",
            )

    def test_relation_keys_bind_source_and_ordinal(self):
        for cap, key in (
            ("creature_loot", "oteryn:creature.another_monster.loot.0"),
            (
                "npc_shop",
                "oteryn:npc.another_vendor.shop.oteryn.service.trade.cledwyn.row.0",
            ),
            ("creature_loot", "oteryn:creature.abyssador.loot.00"),
            ("npc_shop", "oteryn:npc.cledwyn.shop.oteryn.service.trade.cledwyn.row.00"),
            ("creature_loot", "oteryn:relation.alias"),
            ("npc_shop", "oteryn:relation.alias"),
        ):

            def change(document, cap=cap, key=key):
                row = next(
                    row for row in document["relations"] if row["capability_id"] == cap
                )
                if key == "oteryn:relation.alias":
                    document["relations"].append(dict(row, relation_key=key))
                else:
                    row["relation_key"] = key

            with self.subTest(capability=cap, key=key):
                self.assert_cli_rejects(change, "relation_key")

    def test_public_profile_epoch_is_fixed(self):
        self.assert_cli_rejects(
            lambda document: document.update(authority_epoch="epoch-42"),
            "authority_epoch",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
