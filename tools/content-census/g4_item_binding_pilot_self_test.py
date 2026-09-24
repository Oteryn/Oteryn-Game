#!/usr/bin/env python3
"""Synthetic fail-closed tests for the bounded G4 Item binding pilot."""
from __future__ import annotations

import importlib.util
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("g4_item_binding_pilot.py")
SPEC = importlib.util.spec_from_file_location("g4_item_binding_pilot", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("G4 Item binding pilot import failed")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def main() -> None:
    disposition = MODULE.disposition
    assert disposition(matched=["attack"], contradicted=[])[0] == "PROBABLE_MATCH"
    assert disposition(matched=["attack", "armor"], contradicted=[])[0] == "EXACT"
    assert disposition(matched=["attack", "armor"], contradicted=["charges"])[0] == "CONFLICT"
    assert disposition(matched=["attack", "armor"], contradicted=[], page_conflict=True)[0] == "CONFLICT"
    assert disposition(matched=["attack", "armor"], contradicted=[], target_ambiguous=True)[0] == "AMBIGUOUS"
    assert disposition(matched=[], contradicted=[])[0] == "NO_MATCH"

    profiles = {
        "p": {
            "candidate_observations": [
                {"native_field": "attack", "nested_values": [], "source_value": "12"},
                {"native_field": "armor", "nested_values": [], "source_value": 5},
                {"native_field": "weight", "nested_values": [], "source_value": "123"},
                {"native_field": "attack", "nested_values": [], "source_value": "13"},
                {"native_field": "charges", "nested_values": ["nested"], "source_value": "8"},
            ]
        }
    }
    assert MODULE.protected_signals({"source_profile_id": "p"}, profiles) == {"armor": 5}
    assert MODULE.integer_signal(True) is None
    assert MODULE.integer_signal("+12") == 12
    source_tuple = {
        "source": "TIBIAWIKI_STRUCTURED",
        "source_role": "STRUCTURED_REFERENCE_DATA",
        "source_namespace": "mediawiki/tibiawiki.com.br",
        "external_id": "12345",
        "page_key": "mediawiki/tibiawiki.com.br/page_id/12345",
        "title": "example title is not identity authority",
        "revision_id": 67890,
        "revision_timestamp": "2026-09-24T12:34:56Z",
        "source_digest": "a" * 64,
    }
    retained = MODULE.captured_identity_tuple(source_tuple)
    assert retained == source_tuple
    assert retained["source_digest"] == "a" * 64
    assert retained["revision_timestamp"] == "2026-09-24T12:34:56Z"
    assert MODULE.PROJECT_SOURCE_KEY == "oteryn:source.tibiawiki"
    assert MODULE.SOURCE_KEY_PATTERN.fullmatch(MODULE.PROJECT_SOURCE_KEY)
    try:
        MODULE.captured_identity_tuple({key: value for key, value in source_tuple.items() if key != "source_digest"})
    except MODULE.PilotError as exc:
        assert str(exc) == "CAPTURE_IDENTITY_TUPLE_INCOMPLETE"
    else:
        raise AssertionError("incomplete namespaced source tuple was accepted")
    stable_a = {"retrieval_timestamp": "first", "records": [{"source_item_id": 1, "native_key": "oteryn:item.example", "page_id": 12345}], "pages": []}
    stable_b = {"retrieval_timestamp": "later", "records": [{"source_item_id": 1, "native_key": "oteryn:item.example", "page_id": 12345}], "pages": []}
    expected = MODULE.stable_current_digest(stable_a)
    assert MODULE.stable_current_digest(stable_b) == expected
    manifest = {"full_output": {"stable_without_retrieval_timestamp_sha256": expected}}
    protected = {"full_output": {"stable_without_retrieval_timestamp_sha256": expected}}
    MODULE.verify_stable_current_digest(expected, manifest, protected)
    drifted = {"retrieval_timestamp": "later", "records": [{"source_item_id": 1, "native_key": "oteryn:item.example", "page_id": 54321}], "pages": []}
    try:
        MODULE.verify_stable_current_digest(MODULE.stable_current_digest(drifted), manifest, protected)
    except MODULE.PilotError as exc:
        assert str(exc) == "CURRENT_SOURCE_STABLE_OUTPUT_DRIFT"
    else:
        raise AssertionError("stable candidate mapping drift was accepted")
    print("G4 Item binding pilot synthetic self-test: PASS")


if __name__ == "__main__":
    main()
