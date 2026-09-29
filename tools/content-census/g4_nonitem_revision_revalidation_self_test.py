#!/usr/bin/env python3
"""Focused fail-closed tests for the two-row G4 non-Item revalidation."""
from __future__ import annotations

import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("g4_nonitem_revision_revalidation", HERE / "g4_nonitem_revision_revalidation.py")
if spec is None or spec.loader is None:
    raise RuntimeError("G4 non-Item revision revalidation import failed")
tool = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tool)


def pinned(page_id: int) -> dict:
    family = tool.EXPECTED[page_id]
    signature = tool.classifier.G1_SIGNATURES[family]
    categories = [signature["category"]] if "category" in signature else []
    return {
        "family": family,
        "signature_id": signature["id"],
        "signature": signature,
        "g3_shape": {
            "source_shape": "STRUCTURED_PRIMARY", "redirect": False,
            "discovery_roots": [signature["root"]], "source_surfaces": [signature["surface"]],
            "templates": [signature["template"]], "categories": categories,
        },
    }


def current(page_id: int, *, templates: list[str] | None = None, categories: list[str] | None = None) -> dict:
    signature = tool.classifier.G1_SIGNATURES[tool.EXPECTED[page_id]]
    return {
        "page_id": page_id, "namespace_id": 0, "revision_id": 900000 + page_id,
        "revision_timestamp": "2026-09-24T00:00:00Z", "raw_utf8_sha256": "a" * 64,
        "templates": templates if templates is not None else [signature["template"]],
        "categories": categories if categories is not None else ([signature["category"]] if "category" in signature else []),
    }


def expect_error(code: str, fn) -> None:
    try:
        fn()
    except tool.RevalidationError as exc:
        assert code in str(exc), (code, str(exc))
    else:
        raise AssertionError(f"expected {code}")


def expect_api_error(code: str, payload: dict) -> None:
    try:
        tool.validate_api_page(payload, 63947)
    except tool.RevalidationError as exc:
        assert code in str(exc), (code, str(exc))
    else:
        raise AssertionError(f"expected {code}")


def run() -> None:
    for page_id in (63947, 46925):
        result = tool.validate_live_shape(page_id, pinned(page_id), current(page_id))
        assert result["state"] == "DIRECT_FAMILY_SOURCE_SHAPE_SUPPORTED"
        assert result["direct_family_signature"] == tool.classifier.G1_SIGNATURES[tool.EXPECTED[page_id]]["id"]

    expect_error("CURRENT_PAGE_ID_MISMATCH", lambda: tool.validate_live_shape(63947, pinned(63947), current(46925)))
    bad_namespace = current(63947); bad_namespace["namespace_id"] = 1
    expect_error("CURRENT_NAMESPACE_INVALID", lambda: tool.validate_live_shape(63947, pinned(63947), bad_namespace))
    missing_template = current(63947, templates=[])
    expect_error("CURRENT_FAMILY_SHAPE_UNSUPPORTED_OR_AMBIGUOUS", lambda: tool.validate_live_shape(63947, pinned(63947), missing_template))
    missing_category = current(63947, categories=[])
    expect_error("CURRENT_FAMILY_SHAPE_UNSUPPORTED_OR_AMBIGUOUS", lambda: tool.validate_live_shape(63947, pinned(63947), missing_category))
    # A future classifier-registry collision is rejected rather than guessed.
    ambiguous = current(63947, templates=["Predefinição:Infobox Criatura", "Predefinição:Infobox NPC"], categories=["Categoria:Criaturas"])
    npc_signature = tool.classifier.G1_SIGNATURES["NPC"]
    old_root, old_surface = npc_signature["root"], npc_signature["surface"]
    npc_signature["root"], npc_signature["surface"] = "creatures", "Stworzenia"
    try:
        expect_error("CURRENT_FAMILY_SHAPE_UNSUPPORTED_OR_AMBIGUOUS", lambda: tool.validate_live_shape(63947, pinned(63947), ambiguous))
    finally:
        npc_signature["root"], npc_signature["surface"] = old_root, old_surface

    good = {
        "query": {"pages": [{"pageid": 63947, "ns": 0, "revisions": [{
            "revid": 443993, "timestamp": "2026-09-24T09:09:13Z", "slots": {"main": {"content": "{{Infobox Criatura}}"}},
        }], "templates": [{"title": "Predefinição:Infobox Criatura"}], "categories": [{"title": "Categoria:Criaturas"}]}]},
    }
    parsed, templates, categories, content, continuation = tool.validate_api_page(good, 63947)
    assert parsed["raw_utf8_sha256"] == tool.sha256_bytes(b"{{Infobox Criatura}}")
    assert templates == ["Predefinição:Infobox Criatura"] and categories == ["Categoria:Criaturas"]
    assert content == "{{Infobox Criatura}}" and continuation is None
    expect_api_error("CURRENT_PAGE_MISSING", {"query": {"pages": [{"pageid": 63947, "missing": True}]}})
    redirect = {"query": {"pages": [{**good["query"]["pages"][0], "redirect": True}]}}
    expect_api_error("CURRENT_PAGE_REDIRECT", redirect)
    mismatch = {"query": {"pages": [{**good["query"]["pages"][0], "pageid": 46925}]}}
    expect_api_error("CURRENT_PAGE_ID_MISMATCH", mismatch)
    no_revision = {"query": {"pages": [{"pageid": 63947, "ns": 0, "revisions": []}]}}
    expect_api_error("CURRENT_REVISION_CARDINALITY_INVALID", no_revision)

    bad_timestamp = {"query": {"pages": [{**good["query"]["pages"][0], "revisions": [{"revid": 443993, "timestamp": "bad", "slots": {"main": {"content": "x"}}}]}]}}
    expect_api_error("CURRENT_REVISION_TIMESTAMP_INVALID", bad_timestamp)
    no_content = {"query": {"pages": [{**good["query"]["pages"][0], "revisions": [{"revid": 443993, "timestamp": "2026-09-24T09:09:13Z", "slots": {"main": {}}}]}]}}
    expect_api_error("CURRENT_REVISION_CONTENT_MISSING", no_content)
    print("PASS: 14 focused G4 non-Item revision-revalidation cases")


if __name__ == "__main__":
    run()
