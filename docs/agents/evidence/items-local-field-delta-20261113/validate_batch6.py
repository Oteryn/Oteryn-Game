"""Verify alternative-currency source custody without defaulting units to gp."""
import collections
import copy
import csv
import hashlib
import json


def validate_batch6(root, source_path=None, source_dir=None):
    load=lambda name:json.loads((root/name).read_text(encoding="utf8"))
    source=None
    if source_path:
        raw=json.loads(source_path.read_text(encoding="utf8"))
        cols=[c["values"] if c["encoding"]=="LITERAL" else [c["dictionary"][i] for i in c["indices"]] for c in raw["columns"]]
        source=[dict(zip(raw["field_order"],v)) for v in zip(*cols)]
    names=None
    if source_dir:
        flags=json.loads((source_dir/"3173-client6470-flags.json").read_text(encoding="utf8"))
        names={r[0]:(r[3],r[2]) for r in flags["rows"]}

    def check(packet):
        assert packet["current_counts"]==dict(COMPLETE=2537,PARTIAL=343,UNRECONCILED=27,total=2907)
        assert len(packet["records"])==67 and not packet["UI_inputs"] and not packet["new_screenshots"] and not packet["fuzzy_OCR_promotion"]
        assert dict(collections.Counter(r["closed_fields"]["currency_literal"] for r in packet["records"]))==packet["currency_counts"]
        for r in packet["records"]:
            q=r["closed_fields"];unit=q["currency_literal"]
            assert r["current_state"]=="COMPLETE" and r["geometry_evidence"]=="CALIBRATED_NO_OVERFLOW" and not r["remaining_reasons"]
            assert len(r["currency_source_checks"])==len(r["paired_raw_tuple_indices0"])==len(r["paired_raw_tuple_sha256"])>0
            for i,h,p in zip(r["paired_raw_tuple_indices0"],r["paired_raw_tuple_sha256"],r["currency_source_checks"]):
                if p["kind"]=="QUEST_LABEL_UTF8_SHA256_EXACT":
                    assert hashlib.sha256(unit.encode()).hexdigest()==p["literal_sha256"] and len(unit)==p["character_count"]
                elif p["kind"]=="UI_LITERAL_CORROBORATED_CLIENT_CURRENCY_REFERENCE_NAME":
                    assert unit.lower()==p["source_name_exact"].lower() and p["server_binding"]=="UNKNOWN"
                    if names:assert names[p["raw_appearance_reference"]]==(p["source_name_exact"],p["source_record_sha256"])
                else:
                    assert p["kind"]=="UI_EXPLICIT_NATIVE_GP" and unit=="gp"
                if source:
                    s=source[i]
                    assert s["raw_tuple_sha256"]==h and s["npc_name"]==q["NPC"]
                    assert s["buy_price_raw" if q["panel"]=="Sell To" else "sale_price_raw"]==q["quantity"]
                    assert q["location_literal"]=="Various Locations" or s["location"]==q["location_literal"]
                    if p["kind"]=="QUEST_LABEL_UTF8_SHA256_EXACT":assert s["quest_currency_label"]["utf8_sha256"]==p["literal_sha256"]
                    elif p["kind"]=="UI_LITERAL_CORROBORATED_CLIENT_CURRENCY_REFERENCE_NAME":assert s["currency_object_ref"]["value"]==p["raw_appearance_reference"]
                    else:assert s["quest_currency_label"]["state"]==s["currency_object_ref"]["state"]=="FIELD_ABSENT"
        before={r["observation_key"]:r for r in csv.DictReader((root/"all-visible-after-offline-batch5.csv").open(encoding="utf8",newline=""))}
        current=list(csv.DictReader((root/"all-visible-after-offline-batch6.csv").open(encoding="utf8",newline="")))
        keys={r["observation_key"] for r in packet["records"]}
        assert len(keys)==67 and len(current)==len(before)==2907
        assert {r["observation_key"] for r in current if r!=before[r["observation_key"]]}==keys
        assert {k:sum(r["ALL_VISIBLE_state"]==k for r in current) for k in ["COMPLETE","PARTIAL","UNRECONCILED"]}==dict(COMPLETE=2537,PARTIAL=343,UNRECONCILED=27)
        assert set(load("remaining343-summary.json")["remaining_keys"])=={r["observation_key"] for r in current if r["ALL_VISIBLE_state"]=="PARTIAL"}

    packet=load("offline408-batch6.json")
    check(packet)
    bad=copy.deepcopy(packet)
    bad["records"][0]["closed_fields"]["currency_literal"]="gp"
    try:
        check(bad)
    except AssertionError:
        pass
    else:
        raise AssertionError("Alternative currency silently replaced by gp")
