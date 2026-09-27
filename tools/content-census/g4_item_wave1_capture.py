#!/usr/bin/env python3
"""Capture the frozen TibiaWiki revisions behind the 165 EXACT Item bindings.

Every page is fetched by its recorded MediaWiki ``revision_id`` (never the live
head), and the UTF-8 wikitext SHA-256 must equal the ``source_digest`` recorded
by the protected census. Only whitelisted structured Infobox Item parameters are
retained; prose (notes, flavour text, attribute text) is never copied.

The output has no retrieval timestamp, so a rerun against the same selection
must reproduce the snapshot byte for byte.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
from typing import Any, Callable

HERE = Path(__file__).resolve().parent
CENSUS_PATH = HERE.parent / "reference-world-corridor-census" / "item_wiki_first_census.py"
_spec = importlib.util.spec_from_file_location("item_wiki_first_census", CENSUS_PATH)
if _spec is None or _spec.loader is None:
    raise RuntimeError("protected TibiaWiki census import failed")
census = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(census)
source = census.predecessor

SCHEMA = "OTERYN_G4_ITEM_WAVE1_SOURCE_SNAPSHOT/v1"
PROFILE = "OTERYN_G4_ITEM_WAVE1_CAPTURE/v1"
SELECTED_SCHEMA = "oteryn-item-g4-selected-input/v1"
SELECTED_SHA256 = "c624a978bfc83d2dc57865126c6cda63dc21eb4134ee6eb7dc91a6b198ec945c"
SOURCE_REVISION = "tibiawiki-item-census:389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a"
EXPECTED_PAGES = 165
MAX_BATCH_REVIDS = 20

# Structured Infobox Item parameters retained as source observations. Each has a
# disposition in the protected field census; free prose parameters are excluded.
RETAINED_FIELDS = (
    "armor", "charges", "classificacao", "edible", "enchantable", "hands", "imbuement",
    "implemented", "itemclass", "levelrequired", "max_tier", "mercado", "name",
    "primarytype", "regenseconds", "removed", "secondarytype", "slottype", "stackable",
    "tertiarytype", "type", "vocrequired", "volume", "weight",
)


class CaptureError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_hex(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def load_selection(path: Path) -> list[dict[str, Any]]:
    payload = path.read_bytes()
    if sha256_hex(payload) != SELECTED_SHA256:
        raise CaptureError("SELECTED_DIGEST_DRIFT")
    packet = json.loads(payload)
    if packet.get("schema") != SELECTED_SCHEMA or packet["source"]["source_revision"] != SOURCE_REVISION:
        raise CaptureError("SELECTED_SOURCE_DRIFT")
    pages: list[dict[str, Any]] = []
    seen: set[str] = set()
    for row in packet["selected"]:
        binding, page = row["binding"], row["page"]
        external_id = binding["external_id"]
        if binding["disposition"] != "EXACT" or external_id in seen:
            raise CaptureError(f"SELECTED_BINDING_INVALID:{external_id}")
        seen.add(external_id)
        pages.append({
            "external_id": external_id,
            "revision_id": int(page["revision_id"]),
            "revision_timestamp": page["revision_timestamp"],
            "source_digest": page["source_digest"],
            "title": page["title"],
            "target": binding["target"],
        })
    if len(pages) != EXPECTED_PAGES:
        raise CaptureError(f"SELECTED_COUNT_DRIFT:{len(pages)}")
    return sorted(pages, key=lambda row: int(row["external_id"]))


def fetch_revisions(get_json: Callable[[dict[str, str]], dict[str, Any]], revids: list[int]) -> dict[int, dict[str, Any]]:
    if not revids or len(revids) > MAX_BATCH_REVIDS:
        raise CaptureError("REVID_BATCH_INVALID")
    response = get_json({
        "action": "query", "prop": "revisions", "revids": "|".join(str(value) for value in revids),
        "rvprop": "ids|timestamp|content", "rvslots": "main", "format": "json", "formatversion": "2",
    })
    query = response.get("query")
    if not isinstance(query, dict) or query.get("badrevids"):
        raise CaptureError("REVID_QUERY_INVALID")
    out: dict[int, dict[str, Any]] = {}
    for page in query.get("pages", []):
        for revision in page.get("revisions", []):
            content = revision.get("slots", {}).get("main", {}).get("content")
            if not isinstance(content, str):
                raise CaptureError(f"REVISION_CONTENT_MISSING:{revision.get('revid')}")
            out[int(revision["revid"])] = {
                "page_id": int(page["pageid"]),
                "timestamp": revision["timestamp"],
                "content": content,
            }
    if set(out) != set(revids):
        raise CaptureError("REVID_PARTITION_MISMATCH")
    return out


def retained_fields(wikitext: str) -> dict[str, Any]:
    extracted = census._extract_infobox(wikitext)
    if extracted["infobox_present"] is not True:
        raise CaptureError("INFOBOX_ABSENT")
    observations = {**extracted["unmapped"], **extracted["mapped"]}
    return {key: observations[key] for key in RETAINED_FIELDS if key in observations}


def capture(pages: list[dict[str, Any]], get_json: Callable[[dict[str, str]], dict[str, Any]]) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    for start in range(0, len(pages), MAX_BATCH_REVIDS):
        batch = pages[start:start + MAX_BATCH_REVIDS]
        fetched = fetch_revisions(get_json, [page["revision_id"] for page in batch])
        for page in batch:
            revision = fetched[page["revision_id"]]
            if revision["page_id"] != int(page["external_id"]):
                raise CaptureError(f"REVISION_PAGE_MISMATCH:{page['external_id']}")
            if revision["timestamp"] != page["revision_timestamp"]:
                raise CaptureError(f"REVISION_TIMESTAMP_MISMATCH:{page['external_id']}")
            if sha256_hex(revision["content"].encode("utf-8")) != page["source_digest"]:
                raise CaptureError(f"SOURCE_DIGEST_MISMATCH:{page['external_id']}")
            rows.append({**{key: page[key] for key in ("external_id", "revision_id", "revision_timestamp", "source_digest", "title", "target")},
                         "fields": retained_fields(revision["content"])})
    return {
        "schema": SCHEMA,
        "profile": PROFILE,
        "source": {
            "source_key": "oteryn:source.tibiawiki",
            "source_namespace": "mediawiki/tibiawiki.com.br",
            "identity_namespace": "mediawiki/page_id",
            "source_revision": SOURCE_REVISION,
            "selected_input_sha256": SELECTED_SHA256,
            "revision_pinning": "EXACT_REVISION_ID_AND_WIKITEXT_SHA256",
        },
        "retained_fields": list(RETAINED_FIELDS),
        "prose_retained": False,
        "row_count": len(rows),
        "rows": rows,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selected", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expect", type=Path, help="committed snapshot that the capture must reproduce byte for byte")
    args = parser.parse_args()
    client = source.ApiClient()
    snapshot = canonical_bytes(capture(load_selection(args.selected), client.get_json))
    args.output.write_bytes(snapshot)
    print(f"PASS rows={EXPECTED_PAGES} sha256={sha256_hex(snapshot)}")
    if args.expect is not None:
        if args.expect.read_bytes() != snapshot:
            raise CaptureError("COMMITTED_SNAPSHOT_NOT_REPRODUCED")
        print("PASS committed snapshot reproduced byte for byte")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
