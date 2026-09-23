#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
MODULE_PATH = HERE / "item_current_source_tibiawiki.py"
spec = importlib.util.spec_from_file_location("item_current_source_tibiawiki", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("collector import failed")
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)


def reject(action, marker: str) -> None:
    try:
        action()
    except collector.CurrentSourceError as exc:
        assert marker in str(exc), (marker, str(exc))
    else:
        raise AssertionError(f"expected rejection containing {marker}")


def profile(observations=None):
    return {"p": {"profile_id": "p", "candidate_observations": observations or []}}


def protected():
    return {"source_item_id": 1, "native_key": "oteryn:item.registry.i00000001", "source_profile_id": "p"}


def page(page_id=10, **fields):
    return {
        "page_id": page_id,
        "title": "Falcon Plate",
        "revision_id": 395428,
        "revision_timestamp": "2023-06-13T03:16:49Z",
        "target_continuity": "UNKNOWN",
        "normalized_fields": {key: {"state": "VALUE", "value": value} for key, value in fields.items()},
    }


def test_canonical_json_determinism() -> None:
    left = {"b": 2, "a": [3, 1]}
    right = {"a": [3, 1], "b": 2}
    assert collector.canonical_bytes(left) == collector.canonical_bytes(right)


def test_falcon_plate_infobox() -> None:
    fixture = """{{Infobox_Item|List={{{1|}}}|GetValue={{{GetValue|}}}
| name = Falcon Plate
| stackable = não
| levelrequired = 300
| vocrequired = [[Knight]]s
| itemclass = Equipamentos de Corpo
| primarytype = Armaduras
| skillboost = [[Shielding]] +4
| imbuement = 2
| armor = 18
| resist = [[Physical Damage|Physical]] +12%
| weight = 188.00
}}"""
    value = collector.extract_infobox_item(fixture)
    assert value["infobox_present"] is True
    assert value["mapped"]["name"]["value"] == "Falcon Plate"
    assert value["mapped"]["stackable"]["value"] is False
    assert value["mapped"]["levelrequired"]["value"] == 300
    assert value["mapped"]["imbuement"]["value"] == 2
    assert value["mapped"]["armor"]["value"] == 18
    assert value["mapped"]["weight"]["value"] == "188"


def test_stackable_normalization() -> None:
    assert collector.parse_bool_pt("sim") is True
    assert collector.parse_bool_pt("não") is False
    assert collector.parse_bool_pt("nao") is False
    assert collector.parse_bool_pt("talvez") is None


def test_bounded_field_rejection() -> None:
    reject(lambda: collector.bounded_text("x" * (collector.MAX_FIELD_VALUE_BYTES + 1), label="FIELD_VALUE"), "FIELD_VALUE_MAX_PLUS_ONE")
    too_many = "{{Infobox_Item\n" + "\n".join(f"| k{i} = v" for i in range(collector.MAX_INFOBOX_FIELDS + 1)) + "\n}}"
    reject(lambda: collector.extract_infobox_item(too_many), "INFOBOX_FIELD_COUNT_MAX_PLUS_ONE")



def test_oversized_unmapped_field_is_digest_only() -> None:
    raw = "x" * (collector.MAX_FIELD_VALUE_BYTES + 1)
    value = collector.extract_infobox_item("{{Infobox_Item\n| notes = " + raw + "\n}}")
    field = value["unmapped"]["notes"]
    assert field["state"] == "OVERSIZED_DIGEST_ONLY"
    assert field["utf8_bytes"] == len(raw)
    assert len(field["sha256"]) == 64


def test_infobox_balanced_nested_parameters_and_templates() -> None:
    fixture = "{{Infobox_Item|List={{{1|}}}|GetValue={{{GetValue|}}}\n| name = Nested\n| notes = {{Something|x={{Inner|y}}}}\n| armor = 3\n}} trailing"
    value = collector.extract_infobox_item(fixture)
    assert value["mapped"]["name"]["value"] == "Nested"
    assert value["mapped"]["armor"]["value"] == 3
    assert "notes" in value["unmapped"]

def test_malformed_api_rejected() -> None:
    reject(lambda: collector.validate_api_query({"query": {"pages": []}}), "API_ROOT_INVALID")
    reject(lambda: collector.validate_page_metadata({"pageid": 1, "title": "x", "revisions": []}), "API_REVISION_CARDINALITY_INVALID")


def test_name_only_is_ambiguous() -> None:
    result = collector.classify_wiki_candidates(protected(), [page()], profile())
    assert result["disposition"] == "WIKI_AMBIGUOUS"
    assert result["reason"] == "NAME_ONLY_INSUFFICIENT"


def test_correlated_single_is_matched() -> None:
    profiles = profile([{"native_field": "armor", "source_value": "18", "nested_values": []}])
    result = collector.classify_wiki_candidates(protected(), [page(armor=18)], profiles)
    assert result["disposition"] == "WIKI_MATCHED"
    assert result["matched_non_name_signals"] == ["armor"]


def test_contradiction_is_conflict() -> None:
    profiles = profile([{"native_field": "armor", "source_value": "17", "nested_values": []}])
    result = collector.classify_wiki_candidates(protected(), [page(armor=18)], profiles)
    assert result["disposition"] == "WIKI_CONFLICT"
    assert result["contradicted_non_name_signals"] == ["armor"]


def test_zero_is_not_found() -> None:
    result = collector.classify_wiki_candidates(protected(), [], profile())
    assert result["disposition"] == "WIKI_NOT_FOUND"


def test_no_discovery_signal_is_explicit_residual() -> None:
    result = collector.classify_wiki_candidates(protected(), [], profile(), had_discovery_signal=False)
    assert result == {"disposition": "WIKI_AMBIGUOUS", "reason": "NO_ADMITTED_DISCOVERY_SIGNAL", "candidate_page_ids": []}


def test_multiple_plausible_is_ambiguous() -> None:
    result = collector.classify_wiki_candidates(protected(), [page(10), page(11)], profile())
    assert result["disposition"] == "WIKI_AMBIGUOUS"
    assert result["reason"] == "MULTIPLE_PLAUSIBLE_CANDIDATES"


def test_post_target_revision_retained_unknown() -> None:
    meta = {"page_id": 10, "title": "Future Item", "revision_id": 99, "revision_timestamp": "2026-09-01T12:00:00Z"}
    value = collector.normalize_page(meta, "{{Infobox_Item\n| name = Future Item\n}}", "2026-09-22T16:00:00Z")
    assert value["revision_timestamp"] == "2026-09-01T12:00:00Z"
    assert value["target_continuity"] == "UNKNOWN"


def test_cache_exact_revision_avoids_content_fetch() -> None:
    class FakeClient:
        def __init__(self):
            self.calls = []
        def get_json(self, params):
            self.calls.append(dict(params))
            if "pageids" in params:
                raise AssertionError("content fetch must not occur on exact cache hit")
            return {"batchcomplete": True, "query": {"pages": [{"pageid": 10, "ns": 0, "title": "Falcon Plate", "revisions": [{"revid": 395428, "timestamp": "2023-06-13T03:16:49Z"}]}]}}
    cached = {
        "source": collector.SOURCE_ID,
        "source_role": collector.SOURCE_ROLE,
        "page_id": 10,
        "title": "Falcon Plate",
        "revision_id": 395428,
        "revision_timestamp": "2023-06-13T03:16:49Z",
        "retrieval_timestamp": "old",
        "target_cut": collector.TARGET_CUT,
        "target_continuity": "UNKNOWN",
        "source_digest": "0" * 64,
        "normalized_fields": {},
        "unmapped_infobox_fields": {},
        "infobox_present": True,
    }
    with tempfile.TemporaryDirectory() as directory:
        cache = Path(directory)
        collector.write_cached_record(cache, cached)
        client = FakeClient()
        result = collector.collect_pages(["Falcon Plate"], cache_dir=cache, client=client, retrieval_timestamp="new")
        assert result["Falcon Plate"]["retrieval_timestamp"] == "new"
        assert len(client.calls) == 1


def test_manifest_excludes_raw_wikitext() -> None:
    full = {
        "schema": collector.SCHEMA,
        "source": {"id": collector.SOURCE_ID, "role": collector.SOURCE_ROLE, "api": collector.API_BASE},
        "retrieval_timestamp": "2026-09-22T16:00:00Z",
        "pages": [{"page_id": 10, "title": "Falcon Plate", "normalized_fields": {}}],
        "counts": {"records": collector.TARGET_COUNT, "dispositions": {name: 0 for name in collector.DISPOSITIONS}},
    }
    manifest = collector.build_manifest(full, collector_sha256="1" * 64)
    encoded = json.dumps(manifest)
    assert "pages" not in manifest
    assert "normalized_fields" not in encoded
    assert "source_digest" not in encoded


def test_response_max_plus_one_rejected() -> None:
    calls = 0
    def fetch(_request):
        nonlocal calls
        calls += 1
        return b"x" * (collector.MAX_API_RESPONSE_BYTES + 1), {}
    client = collector.ApiClient(fetch=fetch, sleep=lambda _: None, monotonic=lambda: 100.0, min_interval=0.0)
    reject(lambda: client.get_json({"action": "query"}), "API_RETRIES_EXHAUSTED")
    assert calls == collector.MAX_RETRIES


def main() -> int:
    tests = [value for name, value in sorted(globals().items()) if name.startswith("test_") and callable(value)]
    for test in tests:
        test()
    print(f"item-current-source-tibiawiki self-test: PASS tests={len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
