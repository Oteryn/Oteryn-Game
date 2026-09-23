#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
MODULE_PATH = HERE / "item_wiki_first_census.py"
spec = importlib.util.spec_from_file_location("item_wiki_first_census", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("census import failed")
census = importlib.util.module_from_spec(spec)
spec.loader.exec_module(census)


def reject(action, marker: str) -> None:
    try:
        action()
    except census.CensusError as exc:
        assert marker in str(exc), (marker, str(exc))
    else:
        raise AssertionError(f"expected rejection containing {marker}")


class DiscoveryClient:
    def __init__(self, responses):
        self.responses = list(responses)
        self.calls = []

    def get_json(self, params):
        self.calls.append(dict(params))
        if not self.responses:
            raise AssertionError("unexpected discovery request")
        return self.responses.pop(0)


def discovered_response(rows, continuation=None):
    value = {"query": {"categorymembers": rows}}
    if continuation is not None:
        value["continue"] = continuation
    return value


def page(page_id=1, title="Falcon Plate", armor=18):
    return {
        "source": census.SOURCE_ID,
        "source_role": census.SOURCE_ROLE,
        "page_id": page_id,
        "title": title,
        "revision_id": 100 + page_id,
        "revision_timestamp": "2026-09-23T00:00:00Z",
        "retrieval_timestamp": "2026-09-23T20:00:00Z",
        "source_digest": "0" * 64,
        "source_shape": "INFOBOX_ITEM",
        "normalized_fields": {
            "name": {"state": "VALUE", "value": title},
            "armor": {"state": "VALUE", "value": armor},
        },
        "unmapped_infobox_fields": {
            "forgeclass": {"state": "VALUE", "value": "Class 4"},
        },
        "infobox_present": True,
    }


def test_discovery_paginates_and_sorts_deterministically() -> None:
    client = DiscoveryClient(
        [
            discovered_response(
                [{"pageid": 2, "ns": 0, "title": "Zulu"}],
                {"continue": "-||", "cmcontinue": "2|Zulu"},
            ),
            discovered_response([{"pageid": 1, "ns": 0, "title": "Alpha"}]),
        ]
    )
    rows = census.discover_item_pages(client)
    assert rows == [
        {"page_id": 1, "title": "Alpha"},
        {"page_id": 2, "title": "Zulu"},
    ]
    assert len(client.calls) == 2
    assert all(call["list"] == "categorymembers" for call in client.calls)
    assert all(
        call["cmtitle"] == census.ITEM_CATEGORY_TITLE for call in client.calls
    )


def test_discovery_rejects_namespace_drift() -> None:
    client = DiscoveryClient(
        [
            discovered_response(
                [{"pageid": 1, "ns": 14, "title": "Categoria:X"}]
            )
        ]
    )
    reject(
        lambda: census.discover_item_pages(client),
        "DISCOVERY_NAMESPACE_INVALID",
    )


def test_discovery_rejects_page_id_title_conflict() -> None:
    client = DiscoveryClient(
        [
            discovered_response(
                [{"pageid": 1, "ns": 0, "title": "Alpha"}],
                {"continue": "-||", "cmcontinue": "1|Alpha"},
            ),
            discovered_response(
                [{"pageid": 1, "ns": 0, "title": "Beta"}]
            ),
        ]
    )
    reject(
        lambda: census.discover_item_pages(client),
        "DISCOVERY_PAGE_ID_TITLE_CONFLICT",
    )


def test_space_spelled_infobox_is_parsed() -> None:
    value = census._extract_infobox(
        "{{Infobox Item\n"
        "| name = Falcon Plate\n"
        "| armor = 18\n"
        "| forgeclass = Class 4\n"
        "}}"
    )
    assert value["infobox_present"] is True
    assert value["mapped"]["name"]["value"] == "Falcon Plate"
    assert value["mapped"]["armor"]["value"] == 18
    assert value["unmapped"]["forgeclass"]["value"] == "Class 4"


def test_namespace_case_whitespace_infobox_is_parsed() -> None:
    value = census._extract_infobox(
        "{{  Predefinição : INFOBOX_item  \n"
        "| name = Falcon Plate\n"
        "| armor = 18\n"
        "}}"
    )
    assert value["infobox_present"] is True
    assert value["mapped"]["name"]["value"] == "Falcon Plate"
    assert value["mapped"]["armor"]["value"] == 18


def test_subtemplate_name_is_not_misparsed_as_item_infobox() -> None:
    value = census._extract_infobox(
        "{{Infobox Item/Template\n| name = Not A Direct Item Infobox\n}}"
    )
    assert value["infobox_present"] is False


def test_compile_census_keeps_full_infobox_partition_without_prose() -> None:
    discovered = [{"page_id": 1, "title": "Falcon Plate"}]
    full = census.compile_census(
        discovered,
        [page()],
        retrieval_timestamp="2026-09-23T20:00:00Z",
    )
    assert full["counts"]["discovered_pages"] == 1
    assert full["counts"]["pages_with_infobox"] == 1
    assert full["counts"]["distinct_infobox_fields"] == 3
    encoded = json.dumps(full)
    assert "forgeclass" in encoded
    assert "wikitext" not in encoded
    assert '"content"' not in encoded


def test_compile_census_counts_no_infobox_source_shape() -> None:
    no_infobox = {
        **page(page_id=2, title="0152551751 (Book)"),
        "source_shape": "NO_INFOBOX_ITEM",
        "normalized_fields": {},
        "unmapped_infobox_fields": {},
        "infobox_present": False,
    }
    full = census.compile_census(
        [
            {"page_id": 1, "title": "Falcon Plate"},
            {"page_id": 2, "title": "0152551751 (Book)"},
        ],
        [page(), no_infobox],
        retrieval_timestamp="2026-09-23T20:00:00Z",
    )
    assert full["counts"]["pages_with_infobox"] == 1
    assert full["counts"]["pages_without_infobox"] == 1
    assert full["counts"]["source_shapes"] == {
        "INFOBOX_ITEM": 1,
        "NO_INFOBOX_ITEM": 1,
    }


def test_compile_rejects_discovery_fetch_mismatch() -> None:
    reject(
        lambda: census.compile_census(
            [{"page_id": 1, "title": "A"}],
            [page(page_id=2, title="B")],
            retrieval_timestamp="2026-09-23T20:00:00Z",
        ),
        "CENSUS_DISCOVERY_FETCH_SET_MISMATCH",
    )


def test_manifest_is_source_only_and_stable_without_retrieval_timestamp() -> None:
    full = census.compile_census(
        [{"page_id": 1, "title": "Falcon Plate"}],
        [page()],
        retrieval_timestamp="2026-09-23T20:00:00Z",
    )
    manifest = census.build_manifest(
        full,
        collector_sha256="1" * 64,
        predecessor_sha256="2" * 64,
    )
    assert manifest["invariants"]["wiki_first_discovery"] is True
    assert manifest["invariants"]["starts_from_crystal_38157"] is False
    assert manifest["invariants"]["source_shape_partition_complete"] is True
    assert manifest["invariants"]["no_infobox_member_dropped"] is True
    assert manifest["invariants"]["identity_resolution_performed"] is False
    assert manifest["invariants"]["semantic_promotion_performed"] is False
    assert manifest["invariants"]["raw_long_form_prose_collected"] is False
    assert manifest["full_output"]["committed_bulk_corpus"] is False
    assert manifest["next_gate"] == "WIKI_FIRST_ITEM_IDENTITY_CROSSWALK"


def test_normalize_page_retains_category_member_without_infobox() -> None:
    meta = {
        "page_id": 1,
        "title": "0152551751 (Book)",
        "revision_id": 2,
        "revision_timestamp": "2026-09-23T00:00:00Z",
    }
    record = census.normalize_page(
        meta,
        "document page with long-form text but no base Item infobox",
        "2026-09-23T20:00:00Z",
    )
    assert record["infobox_present"] is False
    assert record["source_shape"] == "NO_INFOBOX_ITEM"
    assert record["normalized_fields"] == {}
    assert record["unmapped_infobox_fields"] == {}
    assert "document page" not in json.dumps(record)


def test_cache_round_trip_uses_bounded_predecessor_format() -> None:
    record = page()
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory)
        census.predecessor.write_cached_record(path, record)
        loaded = census.predecessor.load_cached_record(path, 1, 101)
        assert loaded == record


def main() -> int:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"item-wiki-first-census self-test: PASS tests={len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
