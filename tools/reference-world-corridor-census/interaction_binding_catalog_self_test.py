#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import py_compile
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
PATH = HERE / "interaction_binding_catalog.py"

spec = importlib.util.spec_from_file_location("cw2_b6_catalog", PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load B6 generator")
catalog = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = catalog
spec.loader.exec_module(catalog)


def observation(family, x, y, floor, item_order, source_item_id, source_value):
    return {
        "structural_kind": family,
        "position": {"x": x, "y": y, "floor": floor},
        "item_order": item_order,
        "source_item_id": source_item_id,
        "source_value": source_value,
    }


def expect_error(fragment, fn):
    try:
        fn()
    except catalog.CatalogError as exc:
        assert fragment in str(exc), exc
    else:
        raise AssertionError(f"expected CatalogError containing {fragment!r}")


def git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ("git", "-C", str(repo), *args),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise RuntimeError(f"git {' '.join(args)} failed: {detail}")
    return completed.stdout.strip()


def create_game_input_repo(root: Path) -> tuple[str, dict[str, str]]:
    git(root, "init")
    git(root, "config", "user.email", "cw2-b6-self-test@example.invalid")
    git(root, "config", "user.name", "CW2 B6 self test")
    git(root, "config", "commit.gpgsign", "false")
    git(root, "config", "core.autocrlf", "false")

    payloads = {
        "tools/game-atlas-fullworld-source/producer.py": b"VALUE = 'producer'\n",
        "tools/reference-world-corridor-census/census.py": b"VALUE = 'census'\n",
        "tools/game-atlas-thais-fixture/export.py": b"VALUE = 'export'\n",
    }
    for relative_path, payload in payloads.items():
        path = root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)

    git(root, "add", ".")
    git(root, "commit", "-m", "fixture")
    head = git(root, "rev-parse", "HEAD")
    blobs = {
        relative_path: git(root, "rev-parse", f"HEAD:{relative_path}")
        for relative_path in payloads
    }
    return head, blobs


def relative_evidence_path(tracked_path: str) -> str:
    prefix = "docs/agents/evidence/"
    assert tracked_path.startswith(prefix), tracked_path
    return tracked_path[len(prefix):]


def verify_storage(index, family_docs, payloads):
    family_counts = {}
    for entry in index["storage"]["family_files"]:
        family = entry["family"]
        relative = relative_evidence_path(entry["tracked_path"])
        payload = payloads[relative]
        assert len(payload) == entry["bytes"]
        assert hashlib.sha256(payload).hexdigest() == entry["sha256"]
        document = json.loads(payload.decode("utf-8"))
        assert document["family"] == family
        assert document["logical_product_digest_sha256"] == index["logical_product_digest_sha256"]
        assert document["records_digest_sha256"] == entry["records_digest_sha256"]
        assert document["counts"]["occurrences"] == entry["records"]

        if entry["storage_mode"] == "INLINE":
            assert entry["shard_count"] == 0
            records = document["records"]
            assert len(records) == entry["records"]
            assert catalog.sha256_bytes(catalog.canonical_bytes(records)) == entry["records_digest_sha256"]
            family_counts[family] = len(records)
            continue

        assert entry["storage_mode"] == "SHARDED"
        storage = document["storage"]
        assert storage["mode"] == "SHARDED"
        assert storage["max_shard_bytes"] == catalog.MAX_SHARD_BYTES
        assert storage["shard_count"] == entry["shard_count"]

        reconstructed = []
        for ordinal, shard_meta in enumerate(storage["shards"], start=1):
            assert shard_meta["ordinal"] == ordinal
            shard_relative = relative_evidence_path(shard_meta["tracked_path"])
            shard_payload = payloads[shard_relative]
            assert len(shard_payload) == shard_meta["bytes"] <= catalog.MAX_SHARD_BYTES
            assert hashlib.sha256(shard_payload).hexdigest() == shard_meta["sha256"]
            shard = json.loads(shard_payload.decode("utf-8"))
            assert shard["schema"] == catalog.SHARD_SCHEMA
            assert shard["family"] == family
            assert shard["ordinal"] == ordinal
            assert shard["shard_count"] == entry["shard_count"]
            assert shard["logical_product_digest_sha256"] == index["logical_product_digest_sha256"]
            assert shard["family_records_digest_sha256"] == entry["records_digest_sha256"]
            assert shard["record_count"] == shard_meta["record_count"] == len(shard["records"])
            assert catalog.sha256_bytes(catalog.canonical_bytes(shard["records"])) == shard["records_digest_sha256"]
            assert shard["records_digest_sha256"] == shard_meta["records_digest_sha256"]
            reconstructed.extend(shard["records"])

        assert len(reconstructed) == entry["records"]
        assert catalog.sha256_bytes(catalog.canonical_bytes(reconstructed)) == entry["records_digest_sha256"]
        assert catalog.sha256_bytes(catalog.canonical_bytes(family_docs[family]["records"])) == entry["records_digest_sha256"]
        family_counts[family] = len(reconstructed)

    assert set(family_counts) == set(catalog.FAMILIES)
    return family_counts


def build(records, counts):
    index, families = catalog.build_catalog(records, counts)
    payloads = catalog.storage_payloads(index, families)
    return index, families, payloads


def exercise_game_input_provenance() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        head, blobs = create_game_input_repo(root)
        original_main = catalog.ADMISSION_MAIN
        original_blobs = catalog.GAME_INPUT_BLOBS
        module_name = "cw2_b6_game_input_self_test"
        producer_rel = "tools/game-atlas-fullworld-source/producer.py"
        census_rel = "tools/reference-world-corridor-census/census.py"
        export_rel = "tools/game-atlas-thais-fixture/export.py"
        try:
            catalog.ADMISSION_MAIN = head
            catalog.GAME_INPUT_BLOBS = blobs
            verified = catalog.verify_game_inputs(root)
            assert set(verified) == set(blobs)

            producer_path = root / producer_rel
            census_path = root / census_rel
            export_path = root / export_rel

            producer_cache = Path(importlib.util.cache_from_source(str(producer_path)))
            producer_cache.parent.mkdir(parents=True, exist_ok=True)
            producer_cache.write_bytes(b"unchecked producer bytecode")
            expect_error(
                "PROTECTED_GAME_INPUT_BYTECODE_CACHE",
                lambda: catalog.verify_game_inputs(root),
            )
            producer_cache.unlink()
            producer_cache.parent.rmdir()

            census_cache = census_path.with_suffix(".pyc")
            census_cache.write_bytes(b"unchecked census bytecode")
            expect_error(
                "PROTECTED_GAME_INPUT_BYTECODE_CACHE",
                lambda: catalog.verify_game_inputs(root),
            )
            census_cache.unlink()

            export_cache = export_path.with_suffix(".pyo")
            export_cache.write_bytes(b"unchecked export bytecode")
            expect_error(
                "PROTECTED_GAME_INPUT_BYTECODE_CACHE",
                lambda: catalog.verify_game_inputs(root),
            )
            export_cache.unlink()

            active_prefix = root / "hostile-pycache-prefix"
            previous_prefix = sys.pycache_prefix
            sys.pycache_prefix = str(active_prefix)
            try:
                for relative_path in (producer_rel, census_rel, export_rel):
                    protected_path = root / relative_path
                    active_cache = Path(
                        importlib.util.cache_from_source(str(protected_path))
                    )
                    active_cache.parent.mkdir(parents=True, exist_ok=True)
                    marker = root / f"{protected_path.stem}-hostile-executed"
                    hostile_source = root / f"{protected_path.stem}-hostile.py"
                    hostile_source.write_text(
                        "from pathlib import Path\n"
                        f"Path({str(marker)!r}).write_text('executed', encoding='utf-8')\n"
                        "VALUE = 'hostile bytecode'\n",
                        encoding="utf-8",
                    )
                    py_compile.compile(
                        str(hostile_source),
                        cfile=str(active_cache),
                        doraise=True,
                        invalidation_mode=py_compile.PycInvalidationMode.UNCHECKED_HASH,
                    )
                    expect_error(
                        "PROTECTED_GAME_INPUT_BYTECODE_CACHE",
                        lambda: catalog._load_game_module(
                            root, relative_path, f"{module_name}_{protected_path.stem}"
                        ),
                    )
                    assert not marker.exists()
                    active_cache.unlink()
                    hostile_source.unlink()
            finally:
                sys.pycache_prefix = previous_prefix

            previous_dont_write = sys.dont_write_bytecode
            sys.dont_write_bytecode = False
            try:
                module = catalog._load_game_module(root, producer_rel, module_name)
            finally:
                sys.dont_write_bytecode = previous_dont_write
            assert not Path(importlib.util.cache_from_source(str(producer_path))).exists()
            original_origin = module.__file__
            wrong_origin = root / "wrong-origin.py"
            wrong_origin.write_bytes(b"VALUE = 'wrong origin'\n")
            module.__file__ = str(wrong_origin)
            expect_error(
                "PROTECTED_GAME_INPUT_MODULE_ORIGIN_MISMATCH",
                lambda: catalog._verify_loaded_game_module(
                    root, producer_rel, blobs[producer_rel], module
                ),
            )
            module.__file__ = original_origin
            wrong_origin.unlink()

            git(root, "update-index", "--assume-unchanged", producer_rel)
            producer_path.write_bytes(producer_path.read_bytes() + b"# stealth replacement\n")
            expect_error(
                "PROTECTED_GAME_INPUT_WORKTREE_BLOB_MISMATCH",
                lambda: catalog._verify_loaded_game_module(
                    root, producer_rel, blobs[producer_rel], module
                ),
            )
            git(root, "update-index", "--no-assume-unchanged", producer_rel)
            git(root, "checkout", "--", producer_rel)

            census_path.write_bytes(census_path.read_bytes() + b"# dirty\n")
            expect_error(
                "PROTECTED_GAME_INPUT_DIRTY",
                lambda: catalog.verify_game_inputs(root),
            )
            git(root, "checkout", "--", census_rel)

            export_path.unlink()
            export_path.mkdir()
            expect_error(
                "PROTECTED_GAME_INPUT_NOT_REGULAR",
                lambda: catalog.verify_game_inputs(root),
            )
            export_path.rmdir()
            git(root, "checkout", "--", export_rel)

            hardlink_target = root / "hardlink-target.py"
            hardlink_target.write_bytes(export_path.read_bytes())
            export_path.unlink()
            os.link(hardlink_target, export_path)
            expect_error(
                "PROTECTED_GAME_INPUT_HARDLINK",
                lambda: catalog.verify_game_inputs(root),
            )
            export_path.unlink()
            hardlink_target.unlink()
            git(root, "checkout", "--", export_rel)

            target = root / "symlink-target.py"
            target.write_bytes(b"VALUE = 'replacement'\n")
            export_path.unlink()
            try:
                os.symlink(target, export_path)
            except OSError:
                pass
            else:
                expect_error(
                    "PROTECTED_GAME_INPUT_SYMLINK",
                    lambda: catalog.verify_game_inputs(root),
                )
                export_path.unlink()
            finally:
                if export_path.is_symlink():
                    export_path.unlink()
                if target.exists():
                    target.unlink()
            if not export_path.exists():
                git(root, "checkout", "--", export_rel)

            catalog.verify_game_inputs(root)
        finally:
            sys.modules.pop(module_name, None)
            catalog.ADMISSION_MAIN = original_main
            catalog.GAME_INPUT_BLOBS = original_blobs


def exercise_parser_active_prefix_provenance() -> None:
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        active_prefix = root / "hostile-parser-pycache-prefix"
        previous_prefix = sys.pycache_prefix
        sys.pycache_prefix = str(active_prefix)
        try:
            for relative_path in catalog.PARSER_BLOBS:
                parser_path = root / relative_path
                parser_path.parent.mkdir(parents=True, exist_ok=True)
                parser_path.write_text("VALUE = 'verified source'\n", encoding="utf-8")

                marker = root / f"{parser_path.stem}-parser-hostile-executed"
                hostile_source = root / f"{parser_path.stem}-parser-hostile.py"
                hostile_body = (
                    "from pathlib import Path\n"
                    f"Path({str(marker)!r}).write_text('executed', encoding='utf-8')\n"
                    "VALUE = 'hostile bytecode'\n"
                )
                if relative_path.endswith("/assets.py"):
                    hostile_body += (
                        "class ObjectAppearances:\n"
                        "    hostile_parser_cache = True\n"
                    )
                hostile_source.write_text(hostile_body, encoding="utf-8")

                active_cache = Path(importlib.util.cache_from_source(str(parser_path)))
                active_cache.parent.mkdir(parents=True, exist_ok=True)
                py_compile.compile(
                    str(hostile_source),
                    cfile=str(active_cache),
                    doraise=True,
                    invalidation_mode=py_compile.PycInvalidationMode.UNCHECKED_HASH,
                )

                expect_error(
                    "PARSER_ACTIVE_BYTECODE_CACHE",
                    lambda: catalog._reject_parser_active_bytecode_caches(root),
                )
                assert not marker.exists()
                active_cache.unlink()
                hostile_source.unlink()
        finally:
            sys.pycache_prefix = previous_prefix


def main() -> int:
    exercise_parser_active_prefix_provenance()
    records = [
        observation("UNIQUE_ID", 10, 20, -7, 0, 100, 500),
        observation("ACTION_ID", 10, 20, -7, 1, 101, 600),
        observation("TELEPORT_DESTINATION", 11, 20, -7, 0, 102, {"x": 1, "y": 2, "floor": -8}),
        observation("HOUSE_DOOR_ID", 12, 20, -7, 0, 103, 7),
        observation("ACTION_ID", 13, 20, -7, 0, 104, 600),
    ]
    counts = {"map_header": 1, "tile": 3, "town": 0, "waypoint": 0}

    first_index, first_families, first_payloads = build(records, counts)
    second_index, second_families, second_payloads = build(list(reversed(records)), counts)

    assert catalog.canonical_bytes(first_index) == catalog.canonical_bytes(second_index)
    assert first_payloads == second_payloads
    for family in catalog.FAMILIES:
        assert catalog.canonical_bytes(first_families[family]) == catalog.canonical_bytes(second_families[family])

    assert first_index["total_occurrences"] == 5
    assert first_index["game_inputs"] == catalog.GAME_INPUT_BLOBS
    assert first_index["dispositions"] == {
        "UNKNOWN": 5, "UNSUPPORTED": 0, "AMBIGUOUS": 0, "CONFLICT": 0, "LOSS": 0
    }
    assert first_index["silent_drop"] == 0
    assert first_index["unclassified"] == 0
    assert first_families["ACTION_ID"]["counts"]["occurrences"] == 2
    assert first_families["ACTION_ID"]["counts"]["unique_source_values"] == 1
    assert first_families["ACTION_ID"]["counts"]["source_value_reuse_groups"] == 1
    assert first_families["ACTION_ID"]["counts"]["max_source_value_reuse"] == 2

    entries = {entry["family"]: entry for entry in first_index["storage"]["family_files"]}
    assert entries["ACTION_ID"]["storage_mode"] == "INLINE"
    assert entries["UNIQUE_ID"]["storage_mode"] == "INLINE"
    assert entries["TELEPORT_DESTINATION"]["storage_mode"] == "SHARDED"
    assert entries["HOUSE_DOOR_ID"]["storage_mode"] == "SHARDED"
    assert entries["TELEPORT_DESTINATION"]["shard_count"] == 1
    assert entries["HOUSE_DOOR_ID"]["shard_count"] == 1
    assert verify_storage(first_index, first_families, first_payloads) == {
        "ACTION_ID": 2,
        "UNIQUE_ID": 1,
        "TELEPORT_DESTINATION": 1,
        "HOUSE_DOOR_ID": 1,
    }

    bulk = []
    for i in range(1_600):
        bulk.append(
            observation(
                "TELEPORT_DESTINATION",
                20_000 + i,
                30_000 + (i // 1000),
                -7,
                i % 7,
                500 + (i % 300),
                {"x": 40_000 + i, "y": 50_000 + (i % 2000), "floor": -8},
            )
        )
    for i in range(2_500):
        bulk.append(
            observation(
                "HOUSE_DOOR_ID",
                40_000 + i,
                41_000 + (i // 1000),
                -7,
                i % 5,
                900 + (i % 200),
                i % 255,
            )
        )

    bulk_index, bulk_families, bulk_payloads = build(
        bulk,
        {"map_header": 1, "tile": 4_100, "town": 0, "waypoint": 0},
    )
    bulk_entries = {entry["family"]: entry for entry in bulk_index["storage"]["family_files"]}
    assert bulk_entries["TELEPORT_DESTINATION"]["shard_count"] == 3
    assert bulk_entries["HOUSE_DOOR_ID"]["shard_count"] == 5
    assert all(
        shard_meta["bytes"] <= catalog.MAX_SHARD_BYTES
        for family in ("TELEPORT_DESTINATION", "HOUSE_DOOR_ID")
        for shard_meta in json.loads(
            bulk_payloads[
                relative_evidence_path(bulk_entries[family]["tracked_path"])
            ].decode("utf-8")
        )["storage"]["shards"]
    )
    verified_bulk = verify_storage(bulk_index, bulk_families, bulk_payloads)
    assert verified_bulk["TELEPORT_DESTINATION"] == 1_600
    assert verified_bulk["HOUSE_DOOR_ID"] == 2_500

    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        catalog.write_catalog(root, first_payloads)
        assert (root / catalog.INDEX_NAME).is_file()
        assert (root / catalog.FAMILY_DIR_NAME / "action-id.json").is_file()
        assert (root / catalog.FAMILY_DIR_NAME / "teleport-destination-0001.json").is_file()

    expect_error(
        "UNSUPPORTED_STRUCTURAL_FAMILY",
        lambda: catalog.build_catalog([observation("OTHER", 1, 2, -7, 0, 1, 2)], counts),
    )
    expect_error(
        "MALFORMED_TELEPORT_DESTINATION",
        lambda: catalog.build_catalog(
            [observation("TELEPORT_DESTINATION", 1, 2, -7, 0, 1, {"x": 1, "y": 2})], counts
        ),
    )
    duplicate = observation("ACTION_ID", 1, 2, -7, 0, 1, 100)
    expect_error(
        "DUPLICATE_RECORD_ID",
        lambda: catalog.build_catalog([duplicate, dict(duplicate)], counts),
    )

    try:
        catalog.validate_real_source_counts(counts)
    except catalog.CatalogError as exc:
        assert "SOURCE_STREAM_COUNT_MISMATCH" in str(exc)
    else:
        raise AssertionError("synthetic counts must not pass real-source validation")

    exercise_game_input_provenance()

    print("interaction_binding_catalog_self_test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
