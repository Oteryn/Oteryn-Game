#!/usr/bin/env python3
"""Offline synthetic tests for the Wave 1 exact-revision capture."""
from __future__ import annotations

import hashlib
import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("g4_item_wave1_capture", HERE / "g4_item_wave1_capture.py")
if spec is None or spec.loader is None:
    raise RuntimeError("capture import failed")
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)

WIKITEXT = """{{Infobox Item|List={{{1|}}}|GetValue={{{GetValue|}}}
| name = Test Shield
| primarytype = Escudos
| armor = 3
| defense = 20
| weight = 40.00
| imbuement = 1
| notes = Long prose that must never be retained.
| flavortext = More prose.
}}"""


def page(content: str, *, revid: int = 777, page_id: int = 42) -> dict:
    return {
        "external_id": str(page_id),
        "revision_id": revid,
        "revision_timestamp": "2024-01-01T00:00:00Z",
        "source_digest": hashlib.sha256(content.encode("utf-8")).hexdigest(),
        "title": "Test Shield",
        "target": {"family": "Item", "key": "oteryn:item.registry.i00000001", "revision": "definition-r1"},
    }


def api(content: str, *, page_id: int = 42, revid: int = 777, timestamp: str = "2024-01-01T00:00:00Z"):
    calls = []

    def get_json(params: dict[str, str]) -> dict:
        calls.append(params)
        assert params["revids"] == str(revid) and "rvprop" in params
        return {"query": {"pages": [{"pageid": page_id, "revisions": [
            {"revid": revid, "timestamp": timestamp, "slots": {"main": {"content": content}}}]}]}}
    return get_json, calls


def expect_error(code: str, action) -> None:
    try:
        action()
    except capture.CaptureError as exc:
        assert str(exc).startswith(code), exc
        return
    raise AssertionError(f"expected {code}")


def main() -> int:
    get_json, calls = api(WIKITEXT)
    first = capture.canonical_bytes(capture.capture([page(WIKITEXT)], get_json))
    second = capture.canonical_bytes(capture.capture([page(WIKITEXT)], api(WIKITEXT)[0]))
    assert first == second, "capture is not deterministic"
    snapshot = capture.capture([page(WIKITEXT)], api(WIKITEXT)[0])
    fields = snapshot["rows"][0]["fields"]
    assert fields["armor"] == {"state": "VALUE", "value": 3}, fields
    assert fields["primarytype"] == {"state": "VALUE", "value": "Escudos"}, fields
    assert "notes" not in fields and "flavortext" not in fields and "defense" not in fields
    assert b"Long prose" not in first and b"More prose" not in first
    assert snapshot["prose_retained"] is False
    assert calls and calls[0]["revids"] == "777"

    tampered = page(WIKITEXT)
    tampered["source_digest"] = "0" * 64
    expect_error("SOURCE_DIGEST_MISMATCH", lambda: capture.capture([tampered], api(WIKITEXT)[0]))
    expect_error("REVISION_PAGE_MISMATCH", lambda: capture.capture([page(WIKITEXT)], api(WIKITEXT, page_id=43)[0]))
    expect_error("REVISION_TIMESTAMP_MISMATCH", lambda: capture.capture(
        [page(WIKITEXT)], api(WIKITEXT, timestamp="2025-01-01T00:00:00Z")[0]))
    expect_error("REVID_PARTITION_MISMATCH", lambda: capture.capture(
        [page(WIKITEXT)], lambda params: {"query": {"pages": []}}))
    no_box = "plain page"
    expect_error("INFOBOX_ABSENT", lambda: capture.capture([page(no_box)], api(no_box)[0]))
    print("PASS g4 item wave1 capture self-test")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
