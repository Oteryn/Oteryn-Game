#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import copy
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
GAME_ROOT = HERE.parent.parent
MODULE_PATH = HERE / "npc_service_binding_catalog.py"
EVIDENCE_PATH = GAME_ROOT / "docs/agents/evidence/OTV2-20260920-content-world-cw2-b5-npc-service-bindings.json"


def _load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


catalog = _load(MODULE_PATH, "cw2_b5_catalog_selftest")
gameplay = _load(
    GAME_ROOT / "tools/game-atlas-creature-gameplay/export.py",
    "cw2_b5_gameplay_selftest",
)


def _raises(code: str, function, *args, **kwargs) -> None:
    try:
        function(*args, **kwargs)
    except catalog.CatalogError as exc:
        assert code in str(exc), exc
    else:
        raise AssertionError(f"expected {code}")


def _committed_product() -> tuple[dict, dict[str, dict], dict]:
    raw = EVIDENCE_PATH.read_bytes()
    index = json.loads(raw.decode("utf-8"))
    assert raw == catalog.canonical_bytes(index)
    manifest = index["storage_layout"]["shards"]
    expected_paths = {str(value["path"]) for value in manifest}
    actual_paths = {
        value.relative_to(GAME_ROOT).as_posix()
        for value in EVIDENCE_PATH.with_suffix("").glob("*.json")
    }
    assert actual_paths == expected_paths
    shards = {}
    for path in sorted(expected_paths):
        shard_path = GAME_ROOT / path
        shard_raw = shard_path.read_bytes()
        shard = json.loads(shard_raw.decode("utf-8"))
        assert shard_raw == catalog.canonical_bytes(shard)
        assert len(shard_raw) <= catalog.MAX_SHARD_BYTES
        shards[path] = shard
    return index, shards, catalog.reconstruct_evidence(index, shards)


def test_duplicate_identity_and_conflicting_profile_fail_closed() -> None:
    base = {
        "entity_id": "npc-entity:00000000000000000000000000000000",
        "kind": "npc",
        "name": "Same",
        "shop": {"state": "UNKNOWN", "sells": [], "buys": [], "reason_codes": ["NO_STATIC_SHOP_EVIDENCE"]},
        "services": {"state": "UNKNOWN", "values": [], "reason_codes": ["NO_EXHAUSTIVE_STATIC_SERVICE_EVIDENCE"]},
        "travel": {"state": "UNKNOWN", "destinations": [], "reason_codes": ["NO_STATIC_TRAVEL_EVIDENCE"]},
    }
    assert gameplay._merge_duplicate_profile(base, dict(base)) == base
    changed = json.loads(json.dumps(base))
    changed["shop"] = {"state": "COMPLETE", "sells": [], "buys": [], "reason_codes": []}
    merged = gameplay._merge_duplicate_profile(base, changed)
    assert merged["shop"]["state"] == "AMBIGUOUS"
    assert merged["shop"]["reason_codes"] == ["DUPLICATE_PROFILE_CONFLICT"]


def test_blob_change_and_enumeration_order_are_detected_deterministically() -> None:
    records = [
        {"path": "b", "blob": "b" * 40},
        {"path": "a", "blob": "a" * 40},
    ]
    payloads = {"a": b"one", "b": b"two"}
    forward = catalog._aggregate(records, payloads)
    assert forward == catalog._aggregate(list(reversed(records)), payloads)
    assert forward != catalog._aggregate(records, {"a": b"changed", "b": b"two"})


def test_changed_protected_product_digest_is_rejected() -> None:
    basis = {"closure": "CANDIDATE_ONLY", "value": 1}
    digest = catalog.sha256_bytes(catalog.canonical_bytes(basis))
    protected = {**basis, "product_digest_sha256": digest, "product_digest_scope": "canonical JSON with product_digest fields omitted"}
    assert catalog._verify_product(catalog.canonical_bytes(protected), digest, "TEST")["value"] == 1
    protected["value"] = 2
    _raises("PRODUCT_DIGEST_MISMATCH", catalog._verify_product, catalog.canonical_bytes(protected), digest, "TEST")


def test_dynamic_and_conditional_shop_constructs_remain_partial() -> None:
    payload = b'''local internalNpcName = "Trader"
local npcConfig = {}
npcConfig.shop = {
 { itemName = "rope", clientId = 3003, buy = dynamicPrice() },
 { itemName = "torch", clientId = 2920, buy = 2, premium = true },
}
table.insert(npcConfig.shop, buildDynamicOffer())
'''
    result = catalog.parse_static_sections(payload, gameplay)["shop"]
    assert result["state"] == "PARTIAL"
    assert "DYNAMIC_SHOP_PRICE_UNSUPPORTED" in result["reason_codes"]
    assert "CONDITIONAL_OR_EXTENDED_SHOP_ROW_UNSUPPORTED" in result["reason_codes"]
    assert "DYNAMIC_SHOP_MUTATION_UNSUPPORTED" in result["reason_codes"]


def test_dynamic_and_conditional_travel_constructs_remain_partial() -> None:
    conditional = b'''local internalNpcName = "Boat"
local npcConfig = {}
local route = keywordHandler:addKeyword({ "Thais" }, StdModule.say, { npcHandler = npcHandler })
route:addChildKeyword({ "yes" }, StdModule.travel, { npcHandler = npcHandler, cost = 10, destination = Position(1, 2, 7), condition = premiumOnly })
'''
    parsed = catalog.parse_static_sections(conditional, gameplay)["travel"]
    assert parsed["state"] == "PARTIAL"
    assert "CONDITIONAL_TRAVEL_UNSUPPORTED" in parsed["reason_codes"]
    dynamic = b'''local internalNpcName = "Boat"
local npcConfig = {}
local route = keywordHandler:addKeyword({ "There" }, StdModule.say, { npcHandler = npcHandler })
route:addChildKeyword({ "yes" }, StdModule.travel, { npcHandler = npcHandler, cost = dynamicCost(), destination = routePosition })
'''
    parsed = catalog.parse_static_sections(dynamic, gameplay)["travel"]
    assert parsed["state"] == "PARTIAL"
    assert "DYNAMIC_TRAVEL_UNSUPPORTED" in parsed["reason_codes"]


def test_native_identity_cannot_be_minted_from_legacy_values() -> None:
    for identity_inputs in (
        {"source_name": "Sam"}, {"numeric_id": 42}, {"display_id": 42},
        {"client_id": 42}, {"atlas_entity_id": "npc-entity:fake"},
    ):
        _raises(
            "NATIVE_NPC_IDENTITY_PROMOTION_FORBIDDEN",
            catalog.unresolved_native_npc,
            source="source:test",
            identity_inputs=identity_inputs,
        )
    _raises(
        "NATIVE_NPC_IDENTITY_PROMOTION_FORBIDDEN",
        catalog.unresolved_native_npc,
        source="source:test",
        proposed_key="oteryn:npc.fake",
    )


def test_dialogue_runtime_transliteration_is_forbidden() -> None:
    record = catalog.dialogue_state("npc-entity:test", ["source:test"])
    assert record["state"] == "UNKNOWN" and record["support_state"] == "UNSUPPORTED"
    _raises(
        "DIALOGUE_RUNTIME_TRANSLITERATION_FORBIDDEN",
        catalog.dialogue_state,
        "npc-entity:test",
        ["source:test"],
        translate_runtime=True,
    )


def test_shard_payload_and_manifest_tampering_fail_closed() -> None:
    index, shards, _ = _committed_product()
    first_path = sorted(shards)[0]
    changed = copy.deepcopy(shards)
    changed[first_path]["records"][0]["source_blob"] = "0" * 40
    _raises("SHARD_FILE_DIGEST_MISMATCH", catalog.reconstruct_evidence, index, changed)
    changed_index = copy.deepcopy(index)
    changed_index["storage_layout"]["shards"][0]["record_count"] += 1
    _raises("INDEX_DIGEST_MISMATCH", catalog.reconstruct_evidence, changed_index, shards)


def test_committed_evidence_is_canonical_partition_complete_and_candidate_only() -> None:
    index, shards, evidence = _committed_product()
    assert index["storage_layout"]["shard_count"] == 35
    assert len(shards) == 35
    assert max(len(catalog.canonical_bytes(value)) for value in shards.values()) == 559818
    basis = dict(evidence)
    digest = basis.pop("product_digest_sha256")
    basis.pop("product_digest_scope")
    assert digest == catalog.sha256_bytes(catalog.canonical_bytes(basis))
    assert evidence["closure"] == "CANDIDATE_ONLY"
    assert evidence["classification"] == "OTS_HYPOTHESIS_ONLY"
    assert evidence["authority"]["reference_claims_proven"] == 0
    assert evidence["source_snapshot"]["aggregate_source_bytes_sha256"] == catalog.SOURCE_AGGREGATE
    assert evidence["source_snapshot"]["npc_world_aggregate_source_bytes_sha256"] == catalog.NPC_WORLD_AGGREGATE
    counts = evidence["counts"]
    for key, expected in catalog.EXPECTED_COUNTS.items():
        assert counts[key] == expected, (key, counts[key], expected)
    assert sum(evidence["definition_identity"]["partition"].values()) == counts["definition_files"]
    assert sum(evidence["placement_occurrences"]["partition"].values()) == counts["placements"]
    assert sum(evidence["service_types"]["partition"].values()) == counts["service_records"]
    assert sum(evidence["shop_catalogues"]["state_partition"].values()) == counts["profile_identities"]
    assert sum(evidence["shop_catalogues"]["source_item_partition"].values()) == counts["shop_rows"]
    assert sum(evidence["shop_catalogues"]["native_item_partition"].values()) == counts["shop_rows"]
    assert sum(evidence["travel_destinations"]["state_partition"].values()) == counts["profile_identities"]
    assert evidence["dialogue"]["partition"] == {"UNKNOWN_UNSUPPORTED": 1049}
    assert evidence["travel_conditions"]["partition"] == {"UNKNOWN_UNSUPPORTED": 1049}
    assert evidence["shop_catalogues"]["native_item_partition"]["RESOLVED"] == 0
    assert all(record["reason_codes"] for record in evidence["dialogue"]["records"])
    assert all(record["reason_codes"] for record in evidence["travel_conditions"]["records"])


def main() -> int:
    tests = [value for name, value in sorted(globals().items()) if name.startswith("test_") and callable(value)]
    for test in tests:
        test()
    print(f"npc_service_binding_catalog_self_test: ok ({len(tests)} tests)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
