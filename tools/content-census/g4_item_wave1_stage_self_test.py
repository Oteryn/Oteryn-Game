#!/usr/bin/env python3
"""Offline synthetic tests for the Item Wave 1 staging rules."""
from __future__ import annotations

import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("g4_item_wave1_stage", HERE / "g4_item_wave1_stage.py")
if spec is None or spec.loader is None:
    raise RuntimeError("stage import failed")
stage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage)

TARGET = {"family": "Item", "key": "oteryn:item.registry.i00000001", "revision": "definition-r1"}
CENSUS = {"family_assignments": {"Espadas": "weapon_melee"}}


def value(item):
    return {"state": "VALUE", "value": item}


def snapshot(fields, *, timestamp="2024-01-01T00:00:00Z", title="Test Sword"):
    return {"source": {"source_key": "oteryn:source.tibiawiki", "source_revision": "r", "identity_namespace": "mediawiki/page_id"},
            "rows": [{"external_id": "42", "revision_id": 7, "revision_timestamp": timestamp, "source_digest": "0" * 64,
                      "title": title, "target": TARGET, "fields": {"name": value(title), **fields}}]}


def legacy(semantics=None, stack_class="Unknown"):
    return {"records": [{"identity": TARGET, "stack_class": stack_class, "semantics": semantics or {}}]}


def main() -> int:
    fields = {"type": value("Espada"), "imbuement": value(2), "stackable": value(True), "primarytype": value("Espadas"),
              "secondarytype": value(""), "classificacao": value("2"), "max_tier": value("2"), "enchantable": value("não"),
              "weight": value("42"), "hands": value("Uma")}
    staged = stage.stage(snapshot(fields), CENSUS, legacy())
    assert stage.canonical_bytes(staged) == stage.canonical_bytes(stage.stage(snapshot(fields), CENSUS, legacy())), "not deterministic"
    item = staged["items"][0]
    assert [fact["field_path"] for fact in item["facts"]] == ["imbuement.slot_count", "weapon.weapon_type"], item["facts"]
    assert item["authoring"]["taxonomy"] == {"primary": "Espadas"}
    assert item["authoring"]["forge"] == {"classification": 2, "max_tier": 2}
    assert item["family_profile"] == "weapon_melee"
    assert item["capability_relations"] == ["rulesets/items/exaltation-forge/", "rulesets/items/imbuements/"]
    reasons = {(entry["source_parameter"], entry["reason"]) for entry in staged["unknown_report"]}
    assert ("stackable", "STACK_MAX") in reasons and ("weight", "TYPED_WEIGHT_UNIT") in reasons and ("hands", "SLOT_SEMANTICS") in reasons

    stacked = stage.stage(snapshot({"stackable": value(False)}), CENSUS, legacy(stack_class="StackCapable"))
    assert stacked["items"][0]["facts"] == [] and stacked["counts"]["unknown_preserved"] >= 1

    late = stage.stage(snapshot(fields, timestamp="2026-08-01T00:00:00Z"), CENSUS, legacy())
    assert late["items"] == [] and late["rejected"][0]["reason"] == "REVISION_AFTER_TARGET_CUT"
    renamed = stage.stage(snapshot({**fields, "name": value("Other")}), CENSUS, legacy())
    assert renamed["rejected"][0]["reason"] == "NAME_TITLE_DISAGREE"

    unknown_family = stage.stage(snapshot({"primarytype": value("Armas de Arremesso")}), CENSUS, legacy())
    assert unknown_family["items"][0]["family_profile"] is None

    conflicting = legacy({"imbuement": {"state": "KNOWN", "value": {"slot_count": {"state": "KNOWN", "value": 3}}}})
    try:
        stage.stage(snapshot(fields), CENSUS, conflicting)
    except stage.StageError as exc:
        assert str(exc).startswith("CONFLICTS:"), exc
    else:
        raise AssertionError("conflict was not rejected")
    print("PASS g4 item wave1 stage self-test")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
