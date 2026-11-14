"""Validate the fifth offline recovery and reject source-semantic substitution."""
import copy
import csv
import hashlib
import json


def validate_batch5(root, source_path=None):
    def load(name):
        return json.loads((root/name).read_text(encoding="utf8"))

    published=load("npc1992-recovery.json")
    pubrows={r[0]:r for r in published["rows"]}
    seeds={r["observation_key"]:{"values":r["closed_fields"],"sha256":r["source_bitmap"]["sha256"]} for r in load("offline408-batch1.json")["records"]}
    for r in load("offline408-batch3.json")["manual_records"]:
        for q in r["native_read_all_visible_quotes"]:
            seeds[r["observation_key"]+":quote"+str(q["UI_quote_row1"])]={"values":q,"sha256":r["source_bitmap"]["sha256"]}
    source=None
    if source_path:
        raw=json.loads(source_path.read_text(encoding="utf8"))
        cols=[c["values"] if c["encoding"]=="LITERAL" else [c["dictionary"][i] for i in c["indices"]] for c in raw["columns"]]
        source=[dict(zip(raw["field_order"],v)) for v in zip(*cols)]

    def quote(q,panel,locfield):
        assert q["currency_literal"]=="gp" and q["paired_raw_tuple_indices0"]
        if source:
            for i in q["paired_raw_tuple_indices0"]:
                s=source[i]
                assert s["npc_name"]==q["NPC"] and s["buy_price_raw" if panel=="Sell To" else "sale_price_raw"]==q["price"]
                assert q[locfield]=="Various Locations" or s["location"]==q[locfield]

    def check(p):
        assert p["current_counts"]==dict(COMPLETE=2470,PARTIAL=410,UNRECONCILED=27,total=2907)
        assert len(p["manual_records"])==32 and len(p["exact_bitmap_records"])==22
        assert not p["UI_inputs"] and not p["new_screenshots"] and not p["fuzzy_OCR_promotion"]
        assert p["source_packet_sha256"]==hashlib.sha256((root/"npc1992-recovery.json").read_bytes()).hexdigest()
        controls=dict(seeds)
        for r in p["manual_records"]:
            assert r["geometry_evidence"]=="CALIBRATED_NO_OVERFLOW" and r["current_state"]=="COMPLETE" and not r["remaining_reasons"]
            q=r["newly_qualified_quote"]
            quote(q,r["panel"],"UI_location_literal")
            controls[r["observation_key"]+":quote"+str(q["UI_quote_row1"])]={"values":q,"sha256":r["source_bitmap"]["sha256"]}
        for r in p["exact_bitmap_records"]:
            assert r["current_state"]=="COMPLETE" and not r["remaining_reasons"]
            for panel in r["panels"]:
                assert panel["geometry_evidence"]=="CALIBRATED_NO_OVERFLOW"
                for q in panel["quotes"]:
                    quote(q,panel["panel"],"location_literal")
                    for kind,field,outfield,pubfield in [("price_gp_bitmap","price","price","price"),("NPC_bitmap","NPC","NPC","NPC_name"),("location_bitmap","UI_location_literal","location_literal","location_display")]:
                        s=q["native_seed_receipts"][kind]
                        if s["seed_key"].startswith("published-npc1992:"):
                            receipt=p["published_field_seed_receipts"][s["seed_key"]]
                            qi=receipt["public_quote_fact_index"]
                            fact=published["quote_facts"][qi]
                            row=pubrows[receipt["UI_row_index0"]]
                            pi=row[5] if receipt["panel"]=="Sell To" else row[6]
                            assert qi in [x[0] for x in published["panels"][pi][1]] and fact["native_gp_bitmap_proved"] and fact["currency_literal"]=="gp"
                            assert receipt["values"][kind]==fact[pubfield]==s["value"]==q[outfield]
                            assert receipt["private_bitmap_sha256"]==s["seed_sha256"] and receipt["original_png_sha256"]==published["proofs"][row[4]][3]
                            assert receipt["native_bitmap_keys"][kind]==q["native_bitmap_keys"][kind]
                        else:
                            assert s["seed_sha256"]==controls[s["seed_key"]]["sha256"] and s["value"]==controls[s["seed_key"]]["values"][field]==q[outfield]
        before={r["observation_key"]:r for r in csv.DictReader((root/"all-visible-after-offline-batch4.csv").open(encoding="utf8",newline=""))}
        current=list(csv.DictReader((root/"all-visible-after-offline-batch5.csv").open(encoding="utf8",newline="")))
        keys={r["observation_key"] for r in p["manual_records"]+p["exact_bitmap_records"]}
        assert len(keys)==54 and len(current)==len(before)==2907
        assert {r["observation_key"] for r in current if r!=before[r["observation_key"]]}==keys
        assert {k:sum(r["ALL_VISIBLE_state"]==k for r in current) for k in ["COMPLETE","PARTIAL","UNRECONCILED"]}==dict(COMPLETE=2470,PARTIAL=410,UNRECONCILED=27)
        assert set(load("remaining410-summary.json")["remaining_keys"])=={r["observation_key"] for r in current if r["ALL_VISIBLE_state"]=="PARTIAL"}

    packet=load("offline408-batch5.json")
    check(packet)
    bad=copy.deepcopy(packet)
    seedkey=next(q["native_seed_receipts"]["NPC_bitmap"]["seed_key"] for r in bad["exact_bitmap_records"] for pan in r["panels"] for q in pan["quotes"] if q["native_seed_receipts"]["NPC_bitmap"]["seed_key"].startswith("published-npc1992:"))
    bad["published_field_seed_receipts"][seedkey]["values"]["NPC_bitmap"]="Unproved NPC substitution"
    try:
        check(bad)
    except AssertionError:
        pass
    else:
        raise AssertionError("Published NPC semantic substitution was accepted")
