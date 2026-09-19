#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
GAME_ROOT = HERE.parent.parent
MODULE_PATH = HERE / "loot_item_binding_catalog.py"

spec = importlib.util.spec_from_file_location("cw2_b3_catalog", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
catalog = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = catalog
spec.loader.exec_module(catalog)


def _b1(
    item_id: int,
    digest: str,
    disposition: str = "UNRESOLVED",
    detail: dict | None = None,
) -> dict:
    value = {
        "source_item_id": item_id,
        "source_node_digest": digest,
        "native_disposition": disposition,
    }
    if detail is not None:
        value["native_mapping_detail"] = detail
    return value


def _registry() -> dict:
    return {
        "items": {
            100: {
                "server_item_id": 100,
                "source_node_digest": "digest-a",
                "runtime_name": "same",
            },
            101: {
                "server_item_id": 101,
                "source_node_digest": "digest-b",
                "runtime_name": "same",
            },
        },
        "name_to_server_item_ids": {
            "same": [100, 101],
            "one": [100],
            "two": [101],
        },
        "counts": {"runtime_name_collision_groups": 1},
    }


def _varint(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def _field_varint(field: int, value: int) -> bytes:
    return _varint((field << 3) | 0) + _varint(value)


def _field_bytes(field: int, value: bytes) -> bytes:
    return _varint((field << 3) | 2) + _varint(len(value)) + value


def _appearance_object(item_id: int, name: str) -> bytes:
    message = (
        _field_varint(1, item_id)
        + _field_bytes(3, b"")
        + _field_bytes(4, name.encode("utf-8"))
    )
    return _field_bytes(1, message)


def test_runtime_registry_applies_xml_name_override() -> None:
    b1_module = catalog._load_module(
        GAME_ROOT / catalog.B1_TOOL_PATH,
        "cw2_b3_selftest_b1",
    )
    appearances = _appearance_object(100, "old name")
    items_xml = b'<items><item id="100" name="new name"/></items>'
    registry = catalog.build_runtime_item_registry(
        appearances,
        items_xml,
        b1_module,
    )
    assert registry["items"][100]["runtime_name"] == "new name"
    assert registry["name_to_server_item_ids"]["new name"] == [100]
    assert "old name" not in registry["name_to_server_item_ids"]
    assert registry["items"][100]["source_node_digest"]


def test_duplicate_display_name_is_ambiguous_not_first_wins() -> None:
    result = catalog.resolve_source_item(
        item_name="same",
        server_item_id=None,
        client_id=None,
        runtime_registry=_registry(),
        b1_index={
            100: _b1(100, "digest-a"),
            101: _b1(101, "digest-b"),
        },
    )
    assert result["disposition"] == "AMBIGUOUS"
    assert result["candidate_server_item_ids"] == [100, 101]
    assert "b1_source_item_id" not in result


def test_numeric_equality_without_exact_semantics_stays_unresolved() -> None:
    result = catalog.resolve_source_item(
        item_name=None,
        server_item_id=100,
        client_id=None,
        runtime_registry=_registry(),
        b1_index={100: _b1(100, "different-digest")},
    )
    assert result["disposition"] == "UNRESOLVED"
    assert result["reason_code"] == "EXACT_SOURCE_NODE_MISMATCH"


def test_name_selector_precedes_raw_id() -> None:
    result = catalog.resolve_source_item(
        item_name="one",
        server_item_id=101,
        client_id=None,
        runtime_registry=_registry(),
        b1_index={
            100: _b1(100, "digest-a"),
            101: _b1(101, "digest-b"),
        },
    )
    assert result["disposition"] == "RESOLVED"
    assert result["b1_source_item_id"] == 100
    assert result["raw_server_item_id"] == 101
    assert "ignored" in result["raw_server_item_id_semantics"]


def test_client_id_never_becomes_server_item_identity() -> None:
    unresolved = catalog.resolve_source_item(
        item_name=None,
        server_item_id=None,
        client_id=100,
        runtime_registry=_registry(),
        b1_index={100: _b1(100, "digest-a")},
    )
    assert unresolved["disposition"] == "UNRESOLVED"
    assert unresolved["reason_code"] == (
        "CLIENT_ID_HAS_NO_ADMITTED_SERVER_ID_CROSSWALK"
    )

    first = catalog.resolve_source_item(
        item_name=None,
        server_item_id=100,
        client_id=777,
        runtime_registry=_registry(),
        b1_index={
            100: _b1(100, "digest-a"),
            101: _b1(101, "digest-b"),
        },
    )
    second = catalog.resolve_source_item(
        item_name=None,
        server_item_id=101,
        client_id=777,
        runtime_registry=_registry(),
        b1_index={
            100: _b1(100, "digest-a"),
            101: _b1(101, "digest-b"),
        },
    )
    assert first["b1_source_item_id"] == 100
    assert second["b1_source_item_id"] == 101
    assert first["raw_client_id"] == second["raw_client_id"] == 777


def test_exact_crosswalk_can_resolve_source_identity() -> None:
    result = catalog.resolve_source_item(
        item_name=None,
        server_item_id=100,
        client_id=None,
        runtime_registry=_registry(),
        b1_index={100: _b1(100, "digest-a")},
    )
    assert result["disposition"] == "RESOLVED"
    assert result["reason_code"] == "EXACT_CANONICAL_SOURCE_NODE_CROSSWALK"
    assert result["b1_source_item_id"] == 100


def test_native_join_consumes_b1_disposition_only() -> None:
    source = {
        "disposition": "RESOLVED",
        "b1_source_item_id": 100,
    }
    unresolved = catalog.resolve_native_item(
        source,
        {100: _b1(100, "digest-a", "UNRESOLVED")},
    )
    assert unresolved["disposition"] == "UNRESOLVED"
    assert "content_key" not in unresolved

    resolved = catalog.resolve_native_item(
        source,
        {
            100: _b1(
                100,
                "digest-a",
                "RESOLVED",
                {
                    "content_key": "oteryn:item.example",
                    "evidence_refs": ["protected-binding"],
                },
            )
        },
    )
    assert resolved["disposition"] == "RESOLVED"
    assert resolved["content_key"] == "oteryn:item.example"

    conflict = catalog.resolve_native_item(
        source,
        {
            100: _b1(
                100,
                "digest-a",
                "CONFLICT",
                {
                    "candidate_content_keys": [
                        "oteryn:item.a",
                        "oteryn:item.b",
                    ]
                },
            )
        },
    )
    assert conflict["disposition"] == "CONFLICT"
    assert conflict["candidate_content_keys"] == [
        "oteryn:item.a",
        "oteryn:item.b",
    ]


def test_fake_native_key_minting_fails_closed() -> None:
    source = {
        "disposition": "RESOLVED",
        "b1_source_item_id": 100,
    }
    for detail, expected in (
        ({"content_key": "item:100"}, "B1_RESOLVED_NATIVE_TARGET_INVALID"),
        ({"content_key": "oteryn:vsl.item.100"}, "FAKE_NATIVE_VSL_KEY_FORBIDDEN"),
    ):
        try:
            catalog.resolve_native_item(
                source,
                {100: _b1(100, "digest-a", "RESOLVED", detail)},
            )
        except catalog.CatalogError as exc:
            assert expected in str(exc)
        else:
            raise AssertionError(f"forbidden native target accepted: {detail}")


def test_b1_unresolved_cannot_be_promoted() -> None:
    source = {
        "disposition": "RESOLVED",
        "b1_source_item_id": 100,
    }
    try:
        catalog.resolve_native_item(
            source,
            {
                100: _b1(
                    100,
                    "digest-a",
                    "UNRESOLVED",
                    {"content_key": "oteryn:item.illicit"},
                )
            },
        )
    except catalog.CatalogError as exc:
        assert "B1_UNRESOLVED_CARRIES_NATIVE_TARGET" in str(exc)
    else:
        raise AssertionError("B1 UNRESOLVED was promoted to native RESOLVED")


def test_ambiguous_source_never_reaches_native_resolution() -> None:
    native = catalog.resolve_native_item(
        {
            "disposition": "AMBIGUOUS",
            "candidate_server_item_ids": [100, 101],
        },
        {
            100: _b1(
                100,
                "digest-a",
                "RESOLVED",
                {"content_key": "oteryn:item.a"},
            )
        },
    )
    assert native["disposition"] == "UNRESOLVED"
    assert native["reason_code"] == "NO_SINGLE_RESOLVED_B1_SOURCE_IDENTITY"


def test_mapper_fingerprint_is_checkout_line_ending_invariant() -> None:
    lf = b"line one\nline two\n"
    crlf = b"line one\r\nline two\r\n"
    assert catalog.canonical_repository_text_bytes(lf) == lf
    assert catalog.canonical_repository_text_bytes(crlf) == lf
    assert catalog.sha256_bytes(catalog.canonical_repository_text_bytes(lf)) == catalog.sha256_bytes(
        catalog.canonical_repository_text_bytes(crlf)
    )
    try:
        catalog.canonical_repository_text_bytes(b"line one\rline two\n")
    except catalog.CatalogError as exc:
        assert "UNSUPPORTED_MAPPER_LINE_ENDING" in str(exc)
    else:
        raise AssertionError("lone CR line ending was accepted")


def test_input_order_does_not_change_canonical_output() -> None:
    rows = [
        {"loot_row_identity": "b", "value": 2},
        {"loot_row_identity": "a", "value": 1},
    ]
    forward = catalog.canonical_record_list(rows)
    reverse = catalog.canonical_record_list(reversed(rows))
    assert catalog.canonical_bytes(forward) == catalog.canonical_bytes(reverse)


def test_protected_b1_b2_products_are_exact() -> None:
    protected, b1, b2 = catalog.verify_game_products(GAME_ROOT)
    assert protected["admission_main"] == catalog.ADMISSION_MAIN
    assert protected["b1"]["product_digest_sha256"] == catalog.B1_PRODUCT_DIGEST
    assert protected["b2"]["product_digest_sha256"] == catalog.B2_PRODUCT_DIGEST
    assert b1["semantic_catalog"]["counts"]["native_RESOLVED"] == 0
    assert b2["counts"]["deferred_b3_loot_rows"] == 17086


def main() -> int:
    test_runtime_registry_applies_xml_name_override()
    test_duplicate_display_name_is_ambiguous_not_first_wins()
    test_numeric_equality_without_exact_semantics_stays_unresolved()
    test_name_selector_precedes_raw_id()
    test_client_id_never_becomes_server_item_identity()
    test_exact_crosswalk_can_resolve_source_identity()
    test_native_join_consumes_b1_disposition_only()
    test_fake_native_key_minting_fails_closed()
    test_b1_unresolved_cannot_be_promoted()
    test_ambiguous_source_never_reaches_native_resolution()
    test_mapper_fingerprint_is_checkout_line_ending_invariant()
    test_input_order_does_not_change_canonical_output()
    test_protected_b1_b2_products_are_exact()
    print("loot-item-binding-catalog self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
