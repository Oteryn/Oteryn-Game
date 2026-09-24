#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parent
MODULE_PATH = HERE / "source_universe.py"
spec = importlib.util.spec_from_file_location("source_universe", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("source universe import failed")
census = importlib.util.module_from_spec(spec)
spec.loader.exec_module(census)


def reject(action, marker: str) -> None:
    try:
        action()
    except census.CensusError as exc:
        assert marker in str(exc), (marker, str(exc))
    else:
        raise AssertionError(f"expected rejection containing {marker}")


class FakeClient:
    def __init__(self, responses):
        self.responses = list(responses)
        self.calls = []

    def get_json(self, params):
        self.calls.append(dict(params))
        if not self.responses:
            raise AssertionError("unexpected API call")
        return self.responses.pop(0)


def page_raw(
    *,
    page_id=1,
    title="Alpha",
    revision_id=101,
    timestamp="2026-09-24T00:00:00Z",
    categories=None,
    templates=None,
    redirect=False,
):
    value = {
        "pageid": page_id,
        "ns": 0,
        "title": title,
        "revisions": [{"revid": revision_id, "timestamp": timestamp}],
        "categories": categories or [],
        "templates": templates or [],
    }
    if redirect:
        value["redirect"] = True
    return value


def test_real_registry_is_complete_and_excludes_hard_sections() -> None:
    raw = census.load_registry(census.DEFAULT_REGISTRY)
    registry = census.validate_registry(raw)
    assert len(registry["surfaces"]) == 29
    assert len(registry["roots"]) == 22
    names = {row["name"] for row in registry["surfaces"]}
    assert "Kalkulatory" not in names
    assert "Narzędzie do nasycania" not in names
    assert "Dostawca" not in names
    assert registry["roots"]["items-protected"]["kind"] == "protected_manifest"


def test_registry_rejects_hard_exclusion_root() -> None:
    raw = census.load_registry(census.DEFAULT_REGISTRY)
    bad = copy.deepcopy(raw)
    bad["roots"].append(
        {"id": "bad", "kind": "page_only", "title": "Imbuement Tool"}
    )
    bad["surfaces"][0]["root_ids"].append("bad")
    reject(lambda: census.validate_registry(bad), "HARD_EXCLUSION_ROOT")


def test_continuation_loop_is_rejected() -> None:
    client = FakeClient(
        [
            {"query": {}, "continue": {"continue": "-||", "x": "same"}},
            {"query": {}, "continue": {"continue": "-||", "x": "same"}},
        ]
    )
    budget = census.RequestBudget(3)
    reject(
        lambda: list(
            census.iter_continued(
                client,
                budget,
                {"action": "query", "format": "json"},
            )
        ),
        "API_CONTINUATION_LOOP",
    )


def test_malformed_continuation_is_rejected() -> None:
    client = FakeClient([{"query": {}, "continue": ["bad"]}])
    budget = census.RequestBudget(1)
    reject(
        lambda: list(
            census.iter_continued(
                client,
                budget,
                {"action": "query", "format": "json"},
            )
        ),
        "API_CONTINUATION_INVALID",
    )


def test_request_budget_fails_closed() -> None:
    budget = census.RequestBudget(1)
    budget.take()
    reject(budget.take, "REQUEST_BUDGET_MAX_PLUS_ONE")


def test_discovery_index_deduplicates_and_rejects_conflicts() -> None:
    index = census.DiscoveryIndex(max_pages=2, exclusion_aliases=set())
    assert index.add(1, "Alpha", root_id="r1", discovery_kind="PAGE_LINK")
    assert index.add(1, "Alpha", root_id="r2", discovery_kind="CATEGORY_MEMBER")
    assert len(index.by_id) == 1
    assert index.by_id[1]["root_ids"] == {"r1", "r2"}
    reject(
        lambda: index.add(
            1, "Beta", root_id="r3", discovery_kind="PAGE_LINK"
        ),
        "DISCOVERY_PAGE_ID_TITLE_CONFLICT",
    )
    reject(
        lambda: index.add(
            2, "Alpha", root_id="r3", discovery_kind="PAGE_LINK"
        ),
        "DISCOVERY_TITLE_PAGE_ID_CONFLICT",
    )


def test_discovery_index_drops_exact_hard_exclusion() -> None:
    excluded = {census.normalized_title("Imbuement Tool")}
    index = census.DiscoveryIndex(max_pages=1, exclusion_aliases=excluded)
    assert (
        index.add(
            1,
            "Imbuement Tool",
            root_id="r",
            discovery_kind="PAGE_LINK",
        )
        is False
    )
    assert index.by_id == {}


def test_discovery_page_limit_fails_closed() -> None:
    index = census.DiscoveryIndex(max_pages=1, exclusion_aliases=set())
    index.add(1, "Alpha", root_id="r", discovery_kind="PAGE_LINK")
    reject(
        lambda: index.add(
            2, "Beta", root_id="r", discovery_kind="PAGE_LINK"
        ),
        "DISCOVERY_PAGE_COUNT_MAX_PLUS_ONE",
    )


def test_metadata_revision_drift_is_rejected() -> None:
    aggregate = {}
    limits = census.validate_registry(
        census.load_registry(census.DEFAULT_REGISTRY)
    )["limits"]
    census._merge_page_metadata(
        aggregate,
        page_raw(),
        requested={1},
        limits=limits,
        exclusion_aliases=set(),
    )
    reject(
        lambda: census._merge_page_metadata(
            aggregate,
            page_raw(revision_id=102),
            requested={1},
            limits=limits,
            exclusion_aliases=set(),
        ),
        "SOURCE_SNAPSHOT_DRIFT",
    )


def test_reverify_detects_revision_drift() -> None:
    metadata = {
        1: {
            "page_id": 1,
            "title": "Alpha",
            "revision_id": 101,
            "revision_timestamp": "2026-09-24T00:00:00Z",
            "redirect": False,
            "categories": set(),
            "templates": set(),
        }
    }
    client = FakeClient(
        [
            {
                "query": {
                    "pages": [
                        {
                            "pageid": 1,
                            "title": "Alpha",
                            "revisions": [
                                {
                                    "revid": 102,
                                    "timestamp": "2026-09-24T00:01:00Z",
                                }
                            ],
                        }
                    ]
                }
            }
        ]
    )
    reject(
        lambda: census.verify_revisions(
            client,
            census.RequestBudget(2),
            metadata,
            batch_size=20,
        ),
        "SOURCE_SNAPSHOT_DRIFT",
    )


def test_source_shape_and_family_state_are_explicit() -> None:
    primary = {
        "redirect": False,
        "categories": set(),
        "templates": {"Predefinição:Infobox Creature"},
    }
    assert (
        census.classify_source_shape(primary, {"CATEGORY_MEMBER"})
        == "STRUCTURED_PRIMARY"
    )
    assert census.classification_state([]) == "SOURCE_NAVIGATION_ONLY"
    assert census.classification_state(["Creature"]) == "EXACT_FAMILY"
    assert (
        census.classification_state(["Creature", "Encounter"])
        == "MULTI_FAMILY_RELATION"
    )


def test_protected_item_manifest_is_reused_and_digest_bound() -> None:
    registry = census.validate_registry(
        census.load_registry(census.DEFAULT_REGISTRY)
    )
    lane = census.load_protected_manifest(registry["roots"]["items-protected"])
    assert lane["source_pages"] == 6918
    assert (
        lane["stable_digest"]
        == "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a"
    )


def test_compile_is_deterministic_and_source_only() -> None:
    raw = census.load_registry(census.DEFAULT_REGISTRY)
    registry = census.validate_registry(raw)
    index = census.DiscoveryIndex(
        max_pages=10, exclusion_aliases=registry["exclusion_aliases"]
    )
    index.add(
        7,
        "Sample Creature",
        root_id="creatures",
        discovery_kind="CATEGORY_MEMBER",
    )
    metadata = {
        7: {
            "page_id": 7,
            "title": "Sample Creature",
            "revision_id": 700,
            "revision_timestamp": "2026-09-24T00:00:00Z",
            "redirect": False,
            "categories": {"Categoria:Criaturas"},
            "templates": {"Predefinição:Infobox Creature"},
        }
    }
    lane = census.load_protected_manifest(registry["roots"]["items-protected"])
    full_a = census.compile_full_output(
        registry,
        index,
        metadata,
        [{"root_id": "creatures", "kind": "category_tree", "page_ids": [7]}],
        [lane],
        request_count=4,
        retrieval_timestamp="2026-09-24T00:00:00Z",
        registry_sha256="1" * 64,
    )
    full_b = census.compile_full_output(
        registry,
        index,
        metadata,
        [{"root_id": "creatures", "kind": "category_tree", "page_ids": [7]}],
        [lane],
        request_count=4,
        retrieval_timestamp="2026-09-24T00:01:00Z",
        registry_sha256="1" * 64,
    )
    manifest_a = census.build_manifest(
        full_a, registry, collector_sha256="2" * 64
    )
    manifest_b = census.build_manifest(
        full_b, registry, collector_sha256="2" * 64
    )
    assert manifest_a["counts"]["protected_item_pages"] == 6918
    assert manifest_a["counts"]["live_unique_pages"] == 1
    assert manifest_a["invariants"]["protected_item_full_census_refetched"] is False
    assert manifest_a["invariants"]["raw_long_form_prose_collected"] is False
    assert manifest_a["next_gate"] == "GLOBAL_SOURCE_OVERLAP_AND_DEDUPLICATION"
    assert (
        manifest_a["full_output"]["stable_without_retrieval_timestamp_sha256"]
        == manifest_b["full_output"]["stable_without_retrieval_timestamp_sha256"]
    )
    assert "wikitext" not in census.canonical_bytes(full_a).decode("utf-8")



def test_page_link_discovery_counts_missing_redlinks_without_retaining_them() -> None:
    client = FakeClient(
        [
            {
                "query": {
                    "pages": [
                        {
                            "pageid": 10,
                            "title": "Root",
                            "revisions": [
                                {
                                    "revid": 110,
                                    "timestamp": "2026-09-24T00:00:00Z",
                                }
                            ],
                        }
                    ]
                }
            },
            {
                "query": {
                    "pages": [
                        {"ns": 0, "title": "Ghost", "missing": True},
                        {"pageid": 1, "ns": 0, "title": "Alpha"},
                    ]
                }
            },
        ]
    )
    index = census.DiscoveryIndex(max_pages=10, exclusion_aliases=set())
    result = census.discover_page_links(
        client,
        census.RequestBudget(4),
        index,
        root_id="root",
        title="Root",
        include_root=False,
        max_links=10,
    )
    assert result["page_ids"] == [1]
    assert result["discovered_links"] == 1
    assert result["missing_links"] == 1
    assert set(index.by_id) == {1}


def main() -> int:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"full-content-source-discovery self-test: PASS tests={len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
