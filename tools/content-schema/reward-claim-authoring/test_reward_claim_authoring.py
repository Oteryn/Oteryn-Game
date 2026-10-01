#!/usr/bin/env python3
"""No-network tests for reward_claim_authoring.py. Run with `python test_reward_claim_authoring.py`."""

from __future__ import annotations

import copy
import json
from collections import Counter

import reward_claim_authoring as rca

COIN = "oteryn:item.tibia.i3031"
STONE = "oteryn:item.tibia.i1781"


def items() -> dict:
    return {
        COIN: {"materializable": True, "stack_class": "StackCapable"},
        STONE: {"materializable": True, "stack_class": "NonStackable"},
        "oteryn:item.tibia.i9": {"materializable": False, "stack_class": "Unknown"},
    }


def pilot_claim(marker: str, reward: str, count: int, x: int = 100) -> dict:
    return {
        "identity": {"key": f"canary:reward-claim/{marker}", "revision": "pilot-r1"},
        "claim": {"per": "character", "repeat": {"kind": "once"}},
        "quest": {"family": "Quest", "key": "canary:quest/q", "revision": "pilot-r1"},
        "placements": [
            {
                "position": {"x": x, "y": 200, "z": 7},
                "appearance": {"family": "Item", "key": "canary:item/28828"},
                "reward": {
                    "items": [
                        {"item": {"family": "Item", "key": reward}, "count": count}
                    ]
                },
            }
        ],
    }


def manifest_for(*claims: dict) -> dict:
    return {
        "entries": [
            {
                "destination": c["identity"]["key"],
                "position": [
                    p["position"]["x"],
                    p["position"]["y"],
                    p["position"]["z"],
                ],
                "sources": [{"source": "canary", "uid": 6000 + i}],
            }
            for i, c in enumerate(claims)
            for p in c["placements"]
        ]
    }


def build(*claims: dict) -> tuple[list, list]:
    return rca.build_records(list(claims), manifest_for(*claims), items())


def test_plain_once_claims_become_records() -> None:
    records, checks = build(pilot_claim("a/one", "canary:item/3031", 7))
    assert checks == []
    record = records[0]["definition"]
    assert record["identity"] == {
        "key": "oteryn:reward-claim.a.one",
        "revision": rca.REVISION,
    }
    placement = record["placements"][0]
    assert placement["appearance_tibia_id"] == 28828
    assert placement["reward"]["items"] == [{"count": 7, "item": rca.item_ref(COIN)}]
    assert placement["source_binding"] == {
        "legacy_unique_ids": [{"server": "canary", "unique_id": 6000}],
        "project_position": {"x": 100, "y": 200, "z": 7},
    }
    assert record["readiness"] == "ready"
    assert rca.validate(records, items()) == []


def test_out_of_scope_claims_are_skipped() -> None:
    cooldown = pilot_claim("b", "canary:item/3031", 1)
    cooldown["claim"]["repeat"] = {"kind": "cooldown", "hours": 24}
    container = pilot_claim("c", "canary:item/3031", 1)
    container["placements"][0]["reward"]["container"] = {
        "item": {"key": "canary:item/2854"}
    }
    achievement = pilot_claim("d", "canary:item/3031", 1)
    achievement["placements"][0]["achievement"] = "Explorer"
    records, _ = build(cooldown, container, achievement)
    assert records == []


def test_readiness_and_source_checks() -> None:
    records, checks = build(
        pilot_claim("w", "canary:item/9", 1, x=1),
        pilot_claim("s", "crystalserver:item/1781", 5, x=2),
    )
    by_key = {r["definition"]["identity"]["key"]: r["definition"] for r in records}
    assert by_key["oteryn:reward-claim.w"]["readiness"] == "waiting_item_semantics"
    assert by_key["oteryn:reward-claim.s"]["readiness"] == "waiting_item_semantics"
    assert checks == [
        {
            "claim": "oteryn:reward-claim.s",
            "count": 5,
            "item": STONE,
            "reason": "NonStackable reward with count > 1",
        }
    ]


def test_validation_fails_closed() -> None:
    records, _ = build(pilot_claim("a", "canary:item/3031", 1))
    cases = {
        "not an A12 Item key": lambda r: r["placements"][0]["reward"]["items"][0][
            "item"
        ].update(key="oteryn:item.test.coin"),
        "has no Item record": lambda r: r["placements"][0]["reward"]["items"][0][
            "item"
        ].update(key="oteryn:item.tibia.i424242"),
        "reward count must be positive": lambda r: r["placements"][0]["reward"][
            "items"
        ][0].update(count=0),
        "exactly one item": lambda r: r["placements"][0]["reward"]["items"].append(
            copy.deepcopy(r["placements"][0]["reward"]["items"][0])
        ),
        "no legacy unique id": lambda r: r["placements"][0]["source_binding"].update(
            legacy_unique_ids=[]
        ),
        "only per-character once": lambda r: r["claim"]["repeat"].update(
            kind="cooldown"
        ),
        "unknown readiness": lambda r: r.update(readiness="maybe"),
        "claim key": lambda r: r["identity"].update(key="canary:reward-claim/a"),
        "appearance_tibia_id": lambda r: r["placements"][0].update(
            appearance_tibia_id=0
        ),
    }
    for fragment, mutate in cases.items():
        broken = copy.deepcopy(records)
        mutate(broken[0]["definition"])
        errors = rca.validate(broken, items())
        assert any(fragment in e for e in errors), (fragment, errors)


def test_only_a_false_ready_is_an_error() -> None:
    records, _ = build(pilot_claim("w", "canary:item/9", 1))
    false_ready = copy.deepcopy(records)
    false_ready[0]["definition"]["readiness"] = "ready"
    assert any("marked ready" in e for e in rca.validate(false_ready, items()))
    # A stale "waiting" (the Item gained semantics later) stays valid until the next rebuild.
    now_ready = items()
    now_ready["oteryn:item.tibia.i9"] = {
        "materializable": True,
        "stack_class": "StackCapable",
    }
    assert rca.validate(records, now_ready) == []


def test_duplicate_key_and_shared_chest_fail() -> None:
    records, _ = build(pilot_claim("a", "canary:item/3031", 1))
    assert any("duplicate claim key" in e for e in rca.validate(records * 2, items()))
    other, _ = build(pilot_claim("b", "canary:item/3031", 1))
    errors = rca.validate(records + other, items())
    assert any("bound by another claim" in e for e in errors), errors


def test_stack_count_at_proven_maximum_and_above() -> None:
    known = items()
    known[COIN]["semantics"] = {"stack": {"state": "KNOWN", "value": {
        "stackable": {"state": "KNOWN", "value": True},
        "stack_max": {"state": "KNOWN", "value": 5}}}}
    for count, ready in [(5, True), (6, False)]:
        claim = pilot_claim("stack", "canary:item/3031", count)
        records, checks = rca.build_records([claim], manifest_for(claim), known)
        assert (records[0]["definition"]["readiness"] == "ready") == ready
        assert bool(checks) == (not ready)
        assert rca.validate(records, known, checks) == []
        if not ready:
            records[0]["definition"]["readiness"] = "ready"
            assert any("marked ready" in e for e in rca.validate(records, known, checks))


def test_stack_default_maximum_and_invalid_proven_maximum() -> None:
    assert rca.stack_problem(items()[COIN], 100) is None
    assert "stack maximum" in rca.stack_problem(items()[COIN], 101)
    for maximum in [0, 101, True]:
        coin = copy.deepcopy(items()[COIN])
        coin["semantics"] = {"stack": {"state": "KNOWN", "value": {
            "stackable": {"state": "KNOWN", "value": True},
            "stack_max": {"state": "KNOWN", "value": maximum}}}}
        assert "unsupported stack maximum" in rca.stack_problem(coin, 1)


def test_legacy_uid_collision_is_server_scoped_and_must_be_reported() -> None:
    claims = [pilot_claim("a", "canary:item/3031", 1, x=1),
              pilot_claim("b", "canary:item/3031", 1, x=2)]
    manifest = manifest_for(*claims)
    for entry in manifest["entries"]:
        entry["sources"][0]["uid"] = 6117
    records, checks = rca.build_records(claims, manifest, items())
    assert checks[0]["reason"] == "duplicate legacy unique id"
    assert checks[0]["unique_id"] == 6117
    assert len(checks[0]["bindings"]) == 2
    assert any("duplicate legacy unique id" in e for e in rca.validate(records, items()))
    assert rca.validate(records, items(), checks) == []
    assert any("missing or stale" in e for e in rca.validate(records, items(), []))
    stale = copy.deepcopy(checks)
    stale[0]["bindings"].pop()
    assert any("missing or stale" in e for e in rca.validate(records, items(), stale))
    manifest["entries"][1]["sources"][0]["source"] = "crystalserver"
    records, checks = rca.build_records(claims, manifest, items())
    assert checks == []
    assert rca.validate(records, items()) == []


def test_malformed_stack_fields_never_default_or_crash() -> None:
    invalid = [{"state": "KNOWN"}, {"state": "KNOWN", "value": None},
               {"state": "KNOWN", "value": "bad"},
               {"state": "UNKNOWN", "value": {}},
               {"state": "KNOWN", "value": {"stack_max": None}},
               {"state": "KNOWN", "value": {"stack_max": {"state": "KNOWN"}}}]
    claim = pilot_claim("bad", "canary:item/3031", 1)
    for field in invalid:
        known = items()
        known[COIN]["semantics"] = {"stack": field}
        assert "unsupported" in rca.stack_problem(known[COIN], 1)
        records, checks = rca.build_records([claim], manifest_for(claim), known)
        assert records[0]["definition"]["readiness"] == "waiting_item_semantics"
        records[0]["definition"]["readiness"] = "ready"
        assert any("marked ready" in e for e in rca.validate(records, known, checks))


def test_known_stack_payload_is_closed_and_class_consistent() -> None:
    maximum = {"state": "KNOWN", "value": 5}
    invalid = [{"stack_max": maximum},
               {"stack_max": maximum, "stackable": None},
               {"stack_max": maximum, "stackable": {"state": "KNOWN", "value": False}},
               {"stack_max": maximum, "stackable": {"state": "KNOWN", "value": True}, "extra": 1}]
    for payload in invalid:
        coin = copy.deepcopy(items()[COIN])
        coin["semantics"] = {"stack": {"state": "KNOWN", "value": payload}}
        assert "unsupported" in rca.stack_problem(coin, 1)


def test_nonstack_charge_subtype_is_held_in_build_and_validate() -> None:
    admitted = items()
    admitted[STONE]["semantics"] = {
        "charges": {"state": "KNOWN", "value": {
            "count": {"state": "KNOWN", "value": 20}}}}
    claim = pilot_claim("charged/one", "canary:item/1781", 1)
    records, checks = rca.build_records([claim], manifest_for(claim), admitted)
    assert records[0]["definition"]["readiness"] == "waiting_item_semantics"
    assert any("SOURCE_CHARGE_ARGUMENT_UNKNOWN" == check["reason"] for check in checks)
    assert rca.validate(records, admitted, checks) == []
    records[0]["definition"]["readiness"] = "ready"
    assert any("marked ready" in error for error in rca.validate(records, admitted, checks))


def test_stackable_charge_definition_does_not_turn_quantity_into_subtype() -> None:
    coin = copy.deepcopy(items()[COIN])
    coin["semantics"] = {"charges": {"state": "KNOWN", "value": {
        "count": {"state": "KNOWN", "value": 3}}}}
    assert rca.source_subtype_problem(coin) is None
    assert rca.stack_problem(coin, 3) is None
    stone = copy.deepcopy(items()[STONE])
    stone["semantics"] = {"charges": {"state": "KNOWN", "value": {
        "count": {"state": "KNOWN", "value": 1}}}}
    assert rca.source_subtype_problem(stone) is not None


AMULET='oteryn:item.tibia.i3081'

def charged_fixture(raw=5,default=5):
    claim=pilot_claim('charged/proved','canary:item/3081',raw)
    admitted=items();admitted[AMULET]={'materializable':True,'stack_class':'NonStackable','semantics':{'charges':{'state':'KNOWN','value':{'count':{'state':'KNOWN','value':default}}}}}
    proof=[{'pilot_key':claim['identity']['key'],'project_position':claim['placements'][0]['position'],'item':AMULET,'source_count_argument':raw,'source_default_charges':default}]
    return claim,admitted,proof

def build_charged(claim,items,proof):return rca.build_records([claim],manifest_for(claim),items,proof)

def test_definition_default_normalizes_one_instance_and_preserves_raw_source():
    claim,items,proof=charged_fixture();records,checks=build_charged(claim,items,proof)
    reward=records[0]['definition']['placements'][0]['reward']['items'][0]
    assert reward['count']==1
    assert checks[0]['source_count_argument']==5 and checks[0]['definition_charges']==5
    assert checks[0]['reason']=='NATIVE_INSTANCE_LOWERING_NOT_IMPLEMENTED'
    assert records[0]['definition']['readiness']=='waiting_item_semantics'
    assert rca.validate(records,items,checks,[claim],proof)==[]
    assert claim['placements'][0]['reward']['items'][0]['count']==5
    records[0]['definition']['readiness']='ready'
    assert any('marked ready' in e for e in rca.validate(records,items,checks,[claim],proof))

def test_banshee_one_charge_is_a_source_definition_conflict():
    claim,items,proof=charged_fixture(raw=1,default=5);records,checks=build_charged(claim,items,proof)
    assert checks[0]['reason']=='SOURCE_CHARGE_MISMATCH'
    assert checks[0]['classification']=='CONFLICT'
    assert 'normalization' not in checks[0]
    assert rca.validate(records,items,checks,[claim],proof)==[]

def test_forged_quantity_or_source_diagnostics_are_rejected():
    claim,items,proof=charged_fixture();records,checks=build_charged(claim,items,proof)
    changed=copy.deepcopy(records);changed[0]['definition']['placements'][0]['reward']['items'][0]['count']=5
    assert any('quantity disagrees' in e for e in rca.validate(changed,items,checks,[claim],proof))
    assert any('missing or stale' in e for e in rca.validate(records,items,[],[claim],proof))
    changed=copy.deepcopy(checks);changed[0]['source_count_argument']=1
    assert any('missing or stale' in e for e in rca.validate(records,items,changed,[claim],proof))

def test_same_item_and_number_without_exact_source_proof_are_not_normalized():
    claim,items,proof=charged_fixture();records,checks=build_charged(claim,items,[])
    assert records[0]['definition']['placements'][0]['reward']['items'][0]['count']==5
    assert checks[0]['reason']=='SOURCE_CHARGE_ARGUMENT_UNKNOWN'
    assert rca.validate(records,items,checks,[claim],[])==[]
    proof[0]['project_position']={**proof[0]['project_position'],'x':101}
    records,checks=build_charged(claim,items,proof)
    assert checks[0]['reason']=='SOURCE_CHARGE_ARGUMENT_UNKNOWN'

def test_source_edits_cannot_reuse_the_old_normalization_proof():
    claim,items,proof=charged_fixture();claim['placements'][0]['reward']['items'][0]['count']=2
    records,checks=build_charged(claim,items,proof)
    assert checks[0]['reason']=='SOURCE_CHARGE_ARGUMENT_UNKNOWN'
    assert records[0]['definition']['placements'][0]['reward']['items'][0]['count']==2
    assert rca.validate(records,items,checks,[claim],proof)==[]

def test_changed_definition_charge_fact_never_uses_reward_as_its_replacement():
    claim,items,proof=charged_fixture();items[AMULET]['semantics']['charges']['value']['count']['value']=10
    records,checks=build_charged(claim,items,proof)
    assert checks[0]['reason']=='SOURCE_CHARGE_MISMATCH'
    assert records[0]['definition']['placements'][0]['reward']['items'][0]['count']==5
    assert checks[0]['definition_charges']==10

def test_exact_charged_source_without_a_definition_charge_fact_stays_held():
    claim,items,proof=charged_fixture();items[AMULET]['semantics']['charges']={'state':'UNKNOWN'}
    records,checks=build_charged(claim,items,proof)
    assert checks[0]['reason']=='ITEM_CHARGE_SEMANTICS_UNKNOWN'
    assert records[0]['definition']['placements'][0]['reward']['items'][0]['count']==5
    assert rca.validate(records,items,checks,[claim],proof)==[]
    records[0]['definition']['readiness']='ready'
    assert any('marked ready' in e for e in rca.validate(records,items,checks,[claim],proof))

def test_explicit_empty_original_source_cannot_fall_back_to_evidence():
    claim,items,proof=charged_fixture();records,checks=build_charged(claim,items,proof)
    assert rca.validate(records,items,checks,[claim],proof)==[]
    assert any('original source reward binding is missing' in e
        for e in rca.validate(records,items,checks,[],proof))


def test_removed_original_reward_cannot_keep_a_stale_normalization():
    claim,items,proof=charged_fixture();records,checks=build_charged(claim,items,proof)
    removed=copy.deepcopy(claim);removed['placements'][0]['reward']['items']=[]
    assert any('original source reward binding is missing' in e
        for e in rca.validate(records,items,checks,[removed],proof))


def test_moved_original_reward_cannot_keep_the_old_binding():
    claim,items,proof=charged_fixture();records,checks=build_charged(claim,items,proof)
    moved=copy.deepcopy(claim);moved['placements'][0]['position']['x']+=1
    assert any('original source reward binding is missing' in e
        for e in rca.validate(records,items,checks,[moved],proof))


def test_stackable_rune_charge_definition_keeps_piece_quantity():
    claim=pilot_claim('rune/quantity','canary:item/3155',5)
    rune='oteryn:item.tibia.i3155'
    admitted={rune:{'materializable':True,'stack_class':'StackCapable','semantics':{
        'charges':{'state':'KNOWN','value':{'count':{'state':'KNOWN','value':3}}}}}}
    records,checks=rca.build_records([claim],manifest_for(claim),admitted,[])
    assert records[0]['definition']['placements'][0]['reward']['items'][0]['count']==5
    assert checks==[]
    assert records[0]['definition']['readiness']=='ready'




def test_committed_content_is_valid() -> None:
    assert rca.committed_errors() == []
    index = json.loads((rca.ROOT / rca.INDEX_PATH).read_text(encoding="utf-8"))
    assert index["record_count"] == 231
    records = [r["definition"] for p in index["shards"]
               for r in json.loads((rca.ROOT / p).read_text())["records"]]
    assert index["readiness"] == dict(Counter(r["readiness"] for r in records))
    collisions = [c for c in index["source_checks"] if c.get("reason") == "duplicate legacy unique id"]
    assert any(c["server"] == "canary" and c["unique_id"] == 6117 for c in collisions)


if __name__ == "__main__":
    for name, test in sorted(globals().items()):
        if name.startswith("test_") and callable(test):
            test()
            print(f"ok {name}")
