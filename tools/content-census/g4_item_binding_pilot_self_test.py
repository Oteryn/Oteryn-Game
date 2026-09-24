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
    print("G4 Item binding pilot synthetic self-test: PASS")


if __name__ == "__main__":
    main()
