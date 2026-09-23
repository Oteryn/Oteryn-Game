#!/usr/bin/env python3
"""Wiki-first TibiaWiki Item census for Oteryn Content/World evidence.

Discovery starts from the public TibiaWiki Item infobox template rather than from
the protected Crystal-derived 38,157 identity closure. The product is source
evidence only: it does not mint Oteryn identities, resolve Crystal/OTS identity,
promote Reference semantics, or copy long-form page/book prose.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import importlib.util
from pathlib import Path
from typing import Any, Iterable

HERE = Path(__file__).resolve().parent
PREDECESSOR_PATH = HERE / "item_current_source_tibiawiki.py"
_spec = importlib.util.spec_from_file_location("item_current_source_tibiawiki", PREDECESSOR_PATH)
if _spec is None or _spec.loader is None:
    raise RuntimeError("protected TibiaWiki collector import failed")
predecessor = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(predecessor)

SCHEMA = "OTERYN_ITEM_WIKI_FIRST_CENSUS/v1"
MANIFEST_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_CENSUS_MANIFEST/v1"
PROFILE = "OTERYN_ITEM_WIKI_FIRST_CENSUS_COLLECTOR/v1"
SOURCE_ROLE = predecessor.SOURCE_ROLE
SOURCE_ID = predecessor.SOURCE_ID
API_BASE = predecessor.API_BASE
ITEM_TEMPLATE_TITLE = "Predefinição:Infobox Item"
MAX_DISCOVERY_REQUESTS = 128
MAX_DISCOVERED_PAGES = 20_000
MAX_CONTINUE_FIELDS = 16

CensusError = predecessor.CurrentSourceError


def canonical_bytes(value: Any) -> bytes:
    return predecessor.canonical_bytes(value)


def sha256_bytes(value: bytes) -> str:
    return predecessor.sha256_bytes(value)


def normalized_name(value: str) -> str:
    return predecessor.normalized_name(value)


def _validate_continue(value: Any) -> dict[str, str] | None:
    if value is None:
        return None
    if not isinstance(value, dict) or len(value) > MAX_CONTINUE_FIELDS:
        raise CensusError("DISCOVERY_CONTINUE_INVALID")
    output: dict[str, str] = {}
    for key, item in value.items():
        if not isinstance(key, str) or not isinstance(item, str):
            raise CensusError("DISCOVERY_CONTINUE_INVALID")
        predecessor.bounded_text(key, label="DISCOVERY_CONTINUE_KEY", max_bytes=64)
        predecessor.bounded_text(item, label="DISCOVERY_CONTINUE_VALUE", max_bytes=512)
        output[key] = item
    return output


def _validate_embeddedin_response(
    value: Any,
) -> tuple[list[dict[str, Any]], dict[str, str] | None]:
    if not isinstance(value, dict):
        raise CensusError("DISCOVERY_ROOT_INVALID")
    query = value.get("query")
    if not isinstance(query, dict):
        raise CensusError("DISCOVERY_QUERY_INVALID")
    rows = query.get("embeddedin")
    if not isinstance(rows, list):
        raise CensusError("DISCOVERY_ROWS_INVALID")
    output: list[dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict):
            raise CensusError("DISCOVERY_ROW_INVALID")
        page_id, namespace, title = row.get("pageid"), row.get("ns"), row.get("title")
        if not isinstance(page_id, int) or isinstance(page_id, bool) or page_id <= 0:
            raise CensusError("DISCOVERY_PAGE_ID_INVALID")
        if namespace != 0:
            raise CensusError("DISCOVERY_NAMESPACE_INVALID")
        predecessor.bounded_text(
            title, label="DISCOVERY_TITLE", max_bytes=predecessor.MAX_TITLE_BYTES
        )
        output.append({"page_id": page_id, "title": title})
    return output, _validate_continue(value.get("continue"))


def discover_item_pages(client: predecessor.ApiClient) -> list[dict[str, Any]]:
    """Return the bounded deterministic set of main-namespace Item infobox transcluders."""
    found: dict[int, str] = {}
    continuation: dict[str, str] | None = {"continue": ""}
    requests = 0
    while continuation is not None:
        requests += 1
        if requests > MAX_DISCOVERY_REQUESTS:
            raise CensusError("DISCOVERY_REQUEST_LIMIT_EXCEEDED")
        params = {
            "action": "query",
            "format": "json",
            "formatversion": "2",
            "list": "embeddedin",
            "eititle": ITEM_TEMPLATE_TITLE,
            "einamespace": "0",
            "eifilterredir": "nonredirects",
            "eilimit": "max",
            **continuation,
        }
        value = client.get_json(params)
        rows, continuation = _validate_embeddedin_response(value)
        for row in rows:
            page_id, title = int(row["page_id"]), str(row["title"])
            existing = found.get(page_id)
            if existing is not None and existing != title:
                raise CensusError("DISCOVERY_PAGE_ID_TITLE_CONFLICT")
            found[page_id] = title
        if len(found) > MAX_DISCOVERED_PAGES:
            raise CensusError("DISCOVERY_PAGE_LIMIT_EXCEEDED")
    if not found:
        raise CensusError("DISCOVERY_EMPTY")
    titles = list(found.values())
    if len(set(titles)) != len(titles):
        raise CensusError("DISCOVERY_DUPLICATE_TITLE")
    return [
        {"page_id": page_id, "title": found[page_id]}
        for page_id in sorted(
            found, key=lambda key: (normalized_name(found[key]), found[key], key)
        )
    ]


def _extract_infobox(wikitext: str) -> dict[str, Any]:
    """Accept MediaWiki-equivalent space/underscore spelling without changing predecessor."""
    candidates = ("{{Infobox_Item", "{{Infobox Item")
    present = [marker for marker in candidates if marker in wikitext]
    if not present:
        return {"mapped": {}, "unmapped": {}, "infobox_present": False}
    normalized = wikitext
    if "{{Infobox_Item" not in normalized:
        normalized = normalized.replace("{{Infobox Item", "{{Infobox_Item", 1)
    return predecessor.extract_infobox_item(normalized)


def normalize_page(
    meta: dict[str, Any], wikitext: str, retrieval_timestamp: str
) -> dict[str, Any]:
    encoded = wikitext.encode("utf-8")
    if len(encoded) > predecessor.MAX_WIKITEXT_BYTES:
        raise CensusError(f"WIKITEXT_MAX_PLUS_ONE:{len(encoded)}")
    extracted = _extract_infobox(wikitext)
    if not extracted["infobox_present"]:
        raise CensusError(f"DISCOVERED_PAGE_WITHOUT_INFOBOX:{meta['page_id']}")
    return {
        "source": SOURCE_ID,
        "source_role": SOURCE_ROLE,
        "page_id": int(meta["page_id"]),
        "title": meta["title"],
        "revision_id": int(meta["revision_id"]),
        "revision_timestamp": meta["revision_timestamp"],
        "retrieval_timestamp": retrieval_timestamp,
        "source_digest": sha256_bytes(encoded),
        "normalized_fields": extracted["mapped"],
        "unmapped_infobox_fields": extracted["unmapped"],
        "infobox_present": True,
    }


def collect_discovered_pages(
    discovered: Iterable[dict[str, Any]],
    *,
    cache_dir: Path,
    client: predecessor.ApiClient,
    retrieval_timestamp: str,
) -> list[dict[str, Any]]:
    discovered_rows = list(discovered)
    expected = {str(row["title"]): int(row["page_id"]) for row in discovered_rows}
    if len(expected) != len(discovered_rows):
        raise CensusError("DISCOVERY_TITLE_NOT_UNIQUE")
    titles = sorted(expected, key=lambda value: (normalized_name(value), value))
    output: list[dict[str, Any]] = []
    for batch in predecessor.chunks(titles, predecessor.MAX_BATCH_TITLES):
        metadata = predecessor.fetch_metadata_for_titles(client, batch)
        need_content: list[int] = []
        cached_by_page: dict[int, dict[str, Any]] = {}
        for title in batch:
            meta = metadata[title]
            if meta["missing"]:
                raise CensusError(f"DISCOVERED_PAGE_BECAME_MISSING:{title}")
            if int(meta["page_id"]) != expected[title]:
                raise CensusError(f"DISCOVERY_METADATA_PAGE_ID_DRIFT:{title}")
            cached = predecessor.load_cached_record(
                cache_dir, int(meta["page_id"]), int(meta["revision_id"])
            )
            if cached is None:
                need_content.append(int(meta["page_id"]))
            else:
                cached_by_page[int(meta["page_id"])] = cached
        content_by_page: dict[int, str] = {}
        for page_batch in predecessor.chunks(
            sorted(set(need_content)), predecessor.MAX_BATCH_PAGEIDS
        ):
            content_by_page.update(
                predecessor.fetch_content_for_pageids(client, page_batch)
            )
        for title in batch:
            meta = metadata[title]
            page_id = int(meta["page_id"])
            if page_id in cached_by_page:
                record = dict(cached_by_page[page_id])
                record["retrieval_timestamp"] = retrieval_timestamp
                if not record.get("infobox_present"):
                    raise CensusError(f"DISCOVERED_CACHE_WITHOUT_INFOBOX:{page_id}")
            else:
                record = normalize_page(
                    meta, content_by_page[page_id], retrieval_timestamp
                )
                predecessor.write_cached_record(cache_dir, record)
            output.append(record)
    output.sort(
        key=lambda row: (
            normalized_name(str(row["title"])),
            str(row["title"]),
            int(row["page_id"]),
        )
    )
    return output


def compile_census(
    discovered: list[dict[str, Any]],
    pages: list[dict[str, Any]],
    *,
    retrieval_timestamp: str,
) -> dict[str, Any]:
    discovered_by_id = {
        int(row["page_id"]): str(row["title"]) for row in discovered
    }
    page_by_id = {int(row["page_id"]): row for row in pages}
    if len(discovered_by_id) != len(discovered) or len(page_by_id) != len(pages):
        raise CensusError("CENSUS_DUPLICATE_PAGE_ID")
    if set(discovered_by_id) != set(page_by_id):
        raise CensusError("CENSUS_DISCOVERY_FETCH_SET_MISMATCH")

    field_keys: Counter[str] = Counter()
    mapped_keys: Counter[str] = Counter()
    unmapped_keys: Counter[str] = Counter()
    stable_pages: list[dict[str, Any]] = []
    for page_id in sorted(
        page_by_id,
        key=lambda key: (
            normalized_name(discovered_by_id[key]),
            discovered_by_id[key],
            key,
        ),
    ):
        page = dict(page_by_id[page_id])
        if str(page.get("title")) != discovered_by_id[page_id]:
            raise CensusError("CENSUS_TITLE_DRIFT")
        if page.get("infobox_present") is not True:
            raise CensusError("CENSUS_INFOBOX_MISSING")
        mapped = page.get("normalized_fields")
        unmapped = page.get("unmapped_infobox_fields")
        if not isinstance(mapped, dict) or not isinstance(unmapped, dict):
            raise CensusError("CENSUS_FIELD_MAP_INVALID")
        overlap = set(mapped) & set(unmapped)
        if overlap:
            raise CensusError(f"CENSUS_FIELD_PARTITION_OVERLAP:{sorted(overlap)[0]}")
        mapped_keys.update(mapped)
        unmapped_keys.update(unmapped)
        field_keys.update(mapped)
        field_keys.update(unmapped)
        page.pop("retrieval_timestamp", None)
        stable_pages.append(page)

    if not stable_pages:
        raise CensusError("CENSUS_EMPTY")
    return {
        "schema": SCHEMA,
        "collector_profile": PROFILE,
        "source": {
            "id": SOURCE_ID,
            "role": SOURCE_ROLE,
            "api": API_BASE,
            "discovery": {
                "kind": "MEDIAWIKI_EMBEDDEDIN",
                "template": ITEM_TEMPLATE_TITLE,
                "namespace": 0,
            },
        },
        "retrieval_timestamp": retrieval_timestamp,
        "authority": {
            "gameplay_truth": "NONE",
            "identity_minting": "FORBIDDEN",
            "crystal_ots_identity_resolution": "NOT_PERFORMED",
            "semantic_promotion": "FORBIDDEN",
            "long_form_prose_collection": "FORBIDDEN",
        },
        "pages": stable_pages,
        "counts": {
            "discovered_pages": len(discovered),
            "fetched_pages": len(stable_pages),
            "pages_with_infobox": sum(
                1 for page in stable_pages if page["infobox_present"]
            ),
            "distinct_infobox_fields": len(field_keys),
            "mapped_field_occurrences": sum(mapped_keys.values()),
            "unmapped_field_occurrences": sum(unmapped_keys.values()),
            "top_fields": [
                {"field": key, "pages": count}
                for key, count in sorted(
                    field_keys.items(), key=lambda item: (-item[1], item[0])
                )[:100]
            ],
        },
    }


def build_manifest(
    full: dict[str, Any], *, collector_sha256: str, predecessor_sha256: str
) -> dict[str, Any]:
    if full.get("schema") != SCHEMA or not isinstance(full.get("pages"), list):
        raise CensusError("FULL_SCHEMA_INVALID")
    if any(
        "content" in page or "wikitext" in page
        for page in full["pages"]
        if isinstance(page, dict)
    ):
        raise CensusError("RAW_LONG_FORM_TEXT_IN_OUTPUT")
    stable = dict(full)
    retrieval_timestamp = stable.pop("retrieval_timestamp", None)
    counts = full["counts"]
    return {
        "schema": MANIFEST_SCHEMA,
        "status": "WIKI_FIRST_SOURCE_EVIDENCE_ONLY_NO_IDENTITY_PROMOTION",
        "collector": {
            "profile": PROFILE,
            "path": (
                "tools/reference-world-corridor-census/"
                "item_wiki_first_census.py"
            ),
            "sha256": collector_sha256,
            "predecessor_parser_path": (
                "tools/reference-world-corridor-census/"
                "item_current_source_tibiawiki.py"
            ),
            "predecessor_parser_sha256": predecessor_sha256,
        },
        "source": full["source"],
        "retrieval_timestamp": retrieval_timestamp,
        "counts": counts,
        "full_output": {
            "schema": SCHEMA,
            "sha256": sha256_bytes(canonical_bytes(full)),
            "stable_without_retrieval_timestamp_sha256": sha256_bytes(
                canonical_bytes(stable)
            ),
            "committed_bulk_corpus": False,
        },
        "invariants": {
            "wiki_first_discovery": True,
            "starts_from_crystal_38157": False,
            "all_discovered_pages_have_infobox": (
                counts["discovered_pages"] == counts["pages_with_infobox"]
            ),
            "identity_resolution_performed": False,
            "semantic_promotion_performed": False,
            "raw_long_form_prose_collected": False,
            "raw_wikitext_committed": False,
        },
        "limitations": [
            (
                "This generation inventories pages that directly transclude the "
                "TibiaWiki Item infobox in namespace 0; redirects and pages "
                "without that template are outside this census."
            ),
            (
                "TibiaWiki is structured source evidence, not Reference "
                "gameplay truth."
            ),
            (
                "The census intentionally does not map pages to "
                "Crystal/OTS/Oteryn identities; multi-signal identity "
                "crosswalk is the next gate."
            ),
            (
                "Long-form article and book bodies are not collected. Only "
                "bounded Infobox Item fields are retained in scratch output."
            ),
        ],
        "next_gate": "WIKI_FIRST_ITEM_IDENTITY_CROSSWALK",
    }


def utc_now_iso() -> str:
    return (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    parser.add_argument("--retrieval-timestamp", default=None)
    args = parser.parse_args()
    retrieval_timestamp = args.retrieval_timestamp or utc_now_iso()
    client = predecessor.ApiClient()
    discovered = discover_item_pages(client)
    pages = collect_discovered_pages(
        discovered,
        cache_dir=args.cache_dir,
        client=client,
        retrieval_timestamp=retrieval_timestamp,
    )
    full = compile_census(
        discovered, pages, retrieval_timestamp=retrieval_timestamp
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(full))
    collector_payload = Path(__file__).read_bytes().replace(b"\r\n", b"\n")
    predecessor_payload = PREDECESSOR_PATH.read_bytes().replace(b"\r\n", b"\n")
    manifest = build_manifest(
        full,
        collector_sha256=sha256_bytes(collector_payload),
        predecessor_sha256=sha256_bytes(predecessor_payload),
    )
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    counts = manifest["counts"]
    print(
        "item-wiki-first-census: PASS "
        f"pages={counts['discovered_pages']} "
        f"fields={counts['distinct_infobox_fields']} "
        f"digest={manifest['full_output']['stable_without_retrieval_timestamp_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
