#!/usr/bin/env python3
"""Synthetic tests for item_target_continuity.py."""

from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("item_target_continuity.py")
spec = importlib.util.spec_from_file_location("item_target_continuity", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("unable to load item_target_continuity.py")
continuity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(continuity)


class FakeCollector:
    MAX_WIKITEXT_BYTES = 4096
    CurrentSourceError = RuntimeError

    @staticmethod
    def bounded_text(value, *, label, max_bytes):
        if not isinstance(value, str):
            raise AssertionError(f"{label} must be string")
        if len(value.encode("utf-8")) > max_bytes:
            raise AssertionError(f"{label} too large")
        return value

    @staticmethod
    def extract_infobox_item(content):
        payload = json.loads(content)
        mapped = {}
        for key, value in payload.items():
            mapped[key] = {"state": "VALUE", "value": value}
        return {"mapped": mapped, "infobox_present": True}


class FakeClient:
    def __init__(self):
        self.calls = []

    def get_json(self, params):
        self.calls.append(dict(params))
        page_id = int(params["pageids"])
        start = params["rvstart"]
        if start == "2026-07-27T23:59:59Z":
            value = 42 if page_id == 101 else 4
            field = "attack" if page_id == 101 else "defense"
            return {
                "batchcomplete": True,
                "query": {
                    "pages": [
                        {
                            "pageid": page_id,
                            "revisions": [
                                {
                                    "revid": page_id * 10,
                                    "timestamp": "2026-07-27T12:00:00Z",
                                    "slots": {
                                        "main": {
                                            "content": json.dumps({field: value})
                                        }
                                    },
                                }
                            ],
                        }
                    ]
                },
            }
        if start == continuity.TARGET_DAY_START:
            if page_id == 101:
                revisions = [
                    {
                        "revid": 10101,
                        "timestamp": "2026-07-28T12:00:00Z",
                        "slots": {
                            "main": {
                                "content": json.dumps({"attack": 42})
                            }
                        },
                    }
                ]
            else:
                revisions = []
            return {
                "batchcomplete": True,
                "query": {"pages": [{"pageid": page_id, "revisions": revisions}]},
            }
        raise AssertionError(f"unexpected query: {params}")


def current_inputs():
    verification = {
        "schema": continuity.FIELD_VERIFICATION_SCHEMA,
        "target_cut": continuity.TARGET_DATE,
        "records": [
            {
                "source_item_id": 1,
                "native_key": "oteryn:item.1",
                "field_overrides": {
                    "weapon.attack": {
                        "field_state": "CORROBORATED_CURRENT",
                        "continuity_to_target": "UNKNOWN",
                        "observations": [
                            {
                                "source": continuity.CURRENT_SOURCE_ID,
                                "authority": "STRUCTURED_REFERENCE_DATA",
                                "page_id": 101,
                                "source_field": "attack",
                                "value": 42,
                                "revision_id": 5001,
                                "revision_timestamp": "2026-09-22T00:00:00Z",
                            }
                        ],
                    }
                },
            },
            {
                "source_item_id": 2,
                "native_key": "oteryn:item.2",
                "field_overrides": {
                    "weapon.defense": {
                        "field_state": "CORROBORATED_CURRENT",
                        "continuity_to_target": "UNKNOWN",
                        "observations": [
                            {
                                "source": continuity.CURRENT_SOURCE_ID,
                                "authority": "STRUCTURED_REFERENCE_DATA",
                                "page_id": 102,
                                "source_field": "defense",
                                "value": 5,
                                "revision_id": 5002,
                                "revision_timestamp": "2026-09-22T00:00:00Z",
                            }
                        ],
                    }
                },
            },
            {
                "source_item_id": 3,
                "native_key": "oteryn:item.3",
                "field_overrides": {
                    "weapon.attack": {
                        "field_state": "UNKNOWN",
                        "continuity_to_target": "UNKNOWN",
                        "observations": [],
                    }
                },
            },
        ],
    }
    current = {
        "schema": continuity.CURRENT_SOURCE_SCHEMA,
        "collector_profile": continuity.CURRENT_SOURCE_PROFILE,
        "target_cut": continuity.TARGET_DATE,
        "source": {
            "id": continuity.CURRENT_SOURCE_ID,
            "role": "STRUCTURED_REFERENCE_DATA",
        },
        "records": [
            {
                "source_item_id": 1,
                "native_key": "oteryn:item.1",
                "current_source": {
                    "disposition": "WIKI_MATCHED",
                    "candidate_page_ids": [101],
                },
            },
            {
                "source_item_id": 2,
                "native_key": "oteryn:item.2",
                "current_source": {
                    "disposition": "WIKI_CONFLICT",
                    "candidate_page_ids": [102],
                },
            },
            {
                "source_item_id": 3,
                "native_key": "oteryn:item.3",
                "current_source": {
                    "disposition": "WIKI_AMBIGUOUS",
                    "candidate_page_ids": [103],
                },
            },
        ],
        "pages": [
            {"page_id": 101},
            {"page_id": 102},
            {"page_id": 103},
        ],
    }
    return verification, current


def test_candidate_selection_is_atomic_and_bounded():
    continuity.TARGET_COUNT = 3
    verification, current = current_inputs()
    candidates = continuity.candidate_fields(verification, current)
    assert [(row["native_key"], row["field_path"]) for row in candidates] == [
        ("oteryn:item.1", "weapon.attack"),
        ("oteryn:item.2", "weapon.defense"),
    ]


def test_history_fetch_and_cache():
    client = FakeClient()
    collector = FakeCollector()
    with tempfile.TemporaryDirectory() as directory:
        cache = Path(directory)
        first = continuity.collect_histories(
            [101, 102],
            cache_dir=cache,
            client=client,
            collector=collector,
        )
        assert first[101]["pre_target_revision"]["normalized_fields"]["attack"]["value"] == 42
        assert len(first[101]["target_day_revisions"]) == 1
        calls_after_first = len(client.calls)
        second = continuity.collect_histories(
            [102, 101],
            cache_dir=cache,
            client=client,
            collector=collector,
        )
        assert continuity.canonical_bytes(first) == continuity.canonical_bytes(second)
        assert len(client.calls) == calls_after_first


def test_history_continuation_is_pre_target_only():
    class ContinuationClient(FakeClient):
        def __init__(self, continuation_start):
            super().__init__()
            self.continuation_start = continuation_start

        def get_json(self, params):
            value = super().get_json(params)
            if params["rvstart"] == self.continuation_start:
                value.pop("batchcomplete", None)
                value["continue"] = {
                    "continue": "||",
                    "rvcontinue": "synthetic-bounded-continuation",
                }
            return value

    collector = FakeCollector()

    pre_client = ContinuationClient("2026-07-27T23:59:59Z")
    history = continuity.fetch_page_history(pre_client, collector, 101)
    assert history["pre_target_revision"]["revision_id"] == 1010
    assert pre_client.calls[0]["rvlimit"] == "1"

    day_client = ContinuationClient(continuity.TARGET_DAY_START)
    try:
        continuity.fetch_page_history(day_client, collector, 101)
    except continuity.ContinuityError as exc:
        assert str(exc) == "HISTORY_QUERY_CONTINUATION_EXCEEDS_BOUND"
    else:
        raise AssertionError("target-day continuation must fail closed")
    assert day_client.calls[1]["rvlimit"] == str(continuity.MAX_TARGET_DAY_REVISIONS)


def test_derived_conflict_and_unknown_rules():
    derived = continuity.classify_candidate(
        {
            "current_value": 42,
            "source_field": "attack",
        },
        {
            "pre_target_revision": {
                "revision_id": 1,
                "revision_timestamp": "2026-07-27T00:00:00Z",
                "source_digest": "a" * 64,
                "normalized_fields": {"attack": {"state": "VALUE", "value": 42}},
            },
            "target_day_revisions": [
                {
                    "revision_id": 2,
                    "revision_timestamp": "2026-07-28T12:00:00Z",
                    "source_digest": "b" * 64,
                    "normalized_fields": {"attack": {"state": "VALUE", "value": 42}},
                }
            ],
        },
    )
    assert derived["continuity_to_target"] == "DERIVED"
    assert derived["promotion_bridge"] == "ELIGIBLE_FOR_SEMANTIC_PROMOTION_GENERATION"

    conflict = continuity.classify_candidate(
        {"current_value": 42, "source_field": "attack"},
        {
            "pre_target_revision": {
                "revision_id": 1,
                "revision_timestamp": "2026-07-27T00:00:00Z",
                "source_digest": "a" * 64,
                "normalized_fields": {"attack": {"state": "VALUE", "value": 41}},
            },
            "target_day_revisions": [],
        },
    )
    assert conflict["continuity_to_target"] == "CONFLICT"
    assert conflict["promotion_bridge"] == "BLOCKED"

    unknown = continuity.classify_candidate(
        {"current_value": 42, "source_field": "attack"},
        {"pre_target_revision": None, "target_day_revisions": []},
    )
    assert unknown["continuity_to_target"] == "UNKNOWN"


def test_full_compile_partition_and_no_promotion():
    continuity.TARGET_COUNT = 3
    verification, current = current_inputs()
    histories = {
        101: {
            "pre_target_revision": {
                "revision_id": 1,
                "revision_timestamp": "2026-07-27T00:00:00Z",
                "source_digest": "a" * 64,
                "normalized_fields": {"attack": {"state": "VALUE", "value": 42}},
            },
            "target_day_revisions": [],
        },
        102: {
            "pre_target_revision": {
                "revision_id": 2,
                "revision_timestamp": "2026-07-27T00:00:00Z",
                "source_digest": "b" * 64,
                "normalized_fields": {"defense": {"state": "VALUE", "value": 4}},
            },
            "target_day_revisions": [],
        },
    }
    first = continuity.compile_continuity(
        verification,
        current,
        histories,
        protected_inputs={"synthetic": "1"},
    )
    second = continuity.compile_continuity(
        copy.deepcopy(verification),
        copy.deepcopy(current),
        copy.deepcopy(histories),
        protected_inputs={"synthetic": "1"},
    )
    assert continuity.canonical_bytes(first) == continuity.canonical_bytes(second)
    assert first["counts"]["candidate_fields"] == 2
    assert first["counts"]["continuity"] == {
        "DERIVED": 1,
        "UNKNOWN": 0,
        "CONFLICT": 1,
    }
    assert first["invariants"]["semantic_promotion_performed"] is False
    assert first["invariants"]["proven_continuity_emitted"] is False



def test_unparseable_history_becomes_unknown_not_batch_failure():
    candidate = {"current_value": 42, "source_field": "attack"}
    result = continuity.classify_candidate(
        candidate,
        {
            "pre_target_revision": {
                "revision_id": 1,
                "revision_timestamp": "2026-07-27T00:00:00Z",
                "source_digest": "a" * 64,
                "parse_state": "UNPARSED",
                "normalized_fields": {},
            },
            "target_day_revisions": [],
        },
    )
    assert result["continuity_to_target"] == "UNKNOWN"
    assert result["promotion_bridge"] == "BLOCKED"

def main() -> int:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"item-target-continuity self-test: PASS tests={len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
