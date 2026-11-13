"""Validate thin evidence and optionally reproduce it from pinned public snapshots."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load(root, name):
    return json.loads((root / name).read_text(encoding="utf8"))


def validate(a, c, q, p, e):
    rows = a["rows"]
    assert len(rows) == 6470 and len({r[0] for r in rows}) == 6470
    assert sum(r[1] is None for r in rows) == 1288
    assert a["field_state_rules"]["authoritative_UI_to_appearance_or_server_bindings_added"] == 0
    assert all(len(r) == 8 and 0 <= r[2] < len(a["known_flag_presence_sets"])
               and 0 <= r[3] < 32 and r[4] >= 1 and r[5] >= 1
               and 0 <= r[7] < len(a["retained_candidate_UI_property_sets"]) for r in rows)
    assert sum(r[4] for r in rows) == c["summary"]["frame_groups"] == 6470
    assert sum(r[5] for r in rows) == c["summary"]["sprite_reference_occurrences"] == 16398
    assert sum(v["appearance_rows"] for v in c["categories"].values()) == 6470
    assert c["categories"]["NO_MARKET_CATEGORY"]["appearance_rows"] == 1288
    assert len(q["ui_rows"]) == 2623 and len(q["legacy_rows"]) == 314
    assert all(r[0] == n and "Weight" in r[1] for n, r in enumerate(q["ui_rows"]))
    assert q["no_6470_minus_2623_subtraction"] and not q["GUI_complete"]
    assert q["property_observation_counts"]["Weight"] == 2623
    assert q["next_work_in_order"][0]["files"] == 90
    assert q["clipboard_capability_receipt"]["already_collected_structured_fields"]["Weight"] == "30.00 oz"
    assert p["summary"] == {"profiles":444,"profile_canonical_hashes_unchanged":443,
        "perks":3680,"perk_canonical_hashes_unchanged":3678,"changed_perks":2,
        "added_or_removed_perk_keys":0}
    expected = [(2, 1, 14, .015, 2, .02), (4, 1, 14, .02, 18, .025)]
    for d, (level, perk, oldid, oldvalue, newid, newvalue) in zip(p["deltas"], expected):
        assert d["client_proficiency_id"] == 499 and d["level1"] == level and d["perk_index0"] == perk
        assert d["previous"]["Type"] == d["current"]["Type"] == 6
        assert (d["previous"]["BestiaryId"], d["previous"]["Value"]) == (oldid, oldvalue)
        assert (d["current"]["BestiaryId"], d["current"]["Value"]) == (newid, newvalue)
        assert "BestiaryName" not in d["current"]
    assert p["held_type33_profile499_level3_perk0"]["raw_Value"] == .05
    es = {x["name"]:x["values"] for x in e["enums"]}
    assert {k:len(v) for k,v in es.items()} == {"ITEM_CATEGORY":27,"WEAPON_TYPE":9,"VOCATION":8}
    assert next(v["number"] for v in es["VOCATION"] if v["name"] == "VOCATION_ANY") == -1
    assert next(v["number"] for v in es["WEAPON_TYPE"] if v["name"] == "WEAPON_TYPE_NOWEAPON") == 0


def sources(root, a, q):
    receipts = load(HERE, "source-receipts.json")
    for r in receipts["consumed_snapshot_files"]:
        b = (root / r["file"]).read_bytes()
        assert len(b) == r["bytes"] and hashlib.sha256(b).hexdigest() == r["sha256"]
    f = load(root, "3173-client6470-flags.json")
    t = load(root, "3173-client6470-five-tags.json")
    u = load(root, "3173-ui2623-observations.json")
    i = load(root, "3173-catalogue-index.json")
    cv = {r[0]:r for r in load(root, "3170-client-cyclopedia-coverage.json")["record_arrays"]}
    tr = {r[0]:r for r in t["rows"]}
    ir = {r[0]:r[1] for r in i["rows"]}
    assert [r[0] for r in a["rows"]] == [r[0] for r in f["rows"]]
    for row, old in zip(a["rows"], f["rows"]):
        aid = row[0]
        keys = sorted(f["known_flag_field_dictionary"][k] for k, value in f["known_flag_sets"][old[4]])
        assert keys == a["known_flag_presence_sets"][row[2]]
        assert row[1] == cv[aid][8] and row[3] == tr[aid][3]
        props = sorted({k for rel in ir[aid] for k in u["rows"][rel[0]][4]})
        assert props == a["retained_candidate_UI_property_sets"][row[7]]
        assert bool(ir[aid]) == row[6].startswith("REUSE_")
    for row, old in zip(q["ui_rows"],u["rows"]):
        assert row[1] == sorted(old[4])
    assert len({r[1] for r in f["rows"]}) == 6228
    # Presence is independent of truthiness: all raw zero values remain upstream.
    assert any(v == 0 and not isinstance(v, bool) for pairs in f["known_flag_sets"] for k,v in pairs)
    rel = load(root, "3173-client-ui-relations.json")
    accepted = {r[3][0]:r for r in rel["rows"] if rel["category_codes"][r[0]] == "Helmets and Hats" and r[2] == 0}
    overlay = load(HERE, "accepted-client-binding-overlay.json")
    assert {r["appearance_id"] for r in overlay["rows"]} == set(accepted)
    physical = load(HERE, "physical-card-completeness.json")
    for r in overlay["rows"]:
        old = accepted[r["appearance_id"]]
        assert old[1] == r["physical_ordinal1"] and old[4] == r["icon_proof_sha256"]
    for r in physical["current_card_rows"]:
        if r[0] == 1:
            assert r[5] == 0
            continue
        old = u["rows"][r[1]]
        states = old[5]
        closed = bool(states) and all(v in ["OBSERVED_VALUE","ABSENT_IN_ALL_OBSERVED_BASIC_PAGES"] for v in states.values())
        if r[1] == 2354 or u["category_dictionary"][old[0]] == "Gold":
            closed = True
        assert (r[5] == 0) == closed
    recovery = load(HERE,"npc1992-recovery.json")
    for p in recovery["proofs"]:
        assert p[3] in {u["proof_dictionary"][x[1]][1] for x in u["rows"][p[0]][7]}


def reconcile(r):
    proofs = r["image_receipts"]
    assert len(proofs) == 89 and [p["private_image_index0"] for p in proofs] == list(range(89))
    assert len({p["sha256"] for p in proofs}) == 89
    assert sum(p["disposition"] == "SELECTED_BASIC_CARD" for p in proofs) == 66
    assert r["summary"]["new_potion_cards_beyond_pinned36"] == 8
    assert r["summary"]["new_ID_bindings"] == 0 and not r["summary"]["all_UI_complete"]
    assert len(r["new_potion_observations"]) == 8
    assert sum(len(x["visible_complete_quote_rows"]) for x in r["new_potion_observations"]) == 19
    assert [x["continuation_ordinal_if_existing_strong_health_anchor36"] for x in r["new_potion_observations"]] == list(range(37,45))
    assert all(x["physical_ordinal_independently_verified"] is None and x["ID_binding"] == "CANDIDATE_ONLY" for x in r["new_potion_observations"])
    assert r["new_scoped_basic_observation"]["observed_properties"]["Weight"] == "64.00 oz"
    assert r["existing_card_reuse"]["unchanged_properties"]["Weight"] == "30.00 oz"
    assert r["existing_card_reuse"]["existing_UI_row_index0"] == 2354
    assert r["retained_selection_proof"]["all43_regions_identical"]
    assert all(proofs[i]["selected_card_name"] == "wealth duplex" for i in range(22,65))
    assert proofs[84]["selected_card_name"] == "rat claws"


def completeness():
    s = load(HERE, "completeness-summary.json")
    p = load(HERE, "physical-card-completeness.json")
    i = load(HERE, "item-completeness.json")
    b = load(HERE, "accepted-client-binding-overlay.json")
    inv = load(HERE, "all-open-PR-inventory.json")
    assert len(p["current_card_rows"]) == 2632
    assert sum(r[5] == 0 for r in p["current_card_rows"]) == 701
    assert sum(r[5] == 1 for r in p["current_card_rows"]) == 1931
    assert len(p["legacy_rows"]) == 314 and len(p["historical_pilot_label_groups"]) == 22
    assert len(b["rows"]) == b["accepted_client_namespace_binding_count"] == 126
    assert sum(r["client_Basic_status"] == "COMPLETE" for r in b["rows"]) == 49
    assert b["canonical_server_binding_count"] == 0
    assert len(i["appearance_rows"]) == 6470 and len(i["type_rows"]) == 6228
    all_variants = [a for r in i["type_rows"] for a in r[1]]
    assert len(all_variants) == len(set(all_variants)) == 6470
    assert set(all_variants) == {r[0] for r in i["appearance_rows"]}
    for scope, rows in [("appearance_disjoint_partitions",i["appearance_rows"]), ("type_disjoint_partitions",i["type_rows"])]:
        for key, col in [("Basic",3),("all_applicable_panels",4),("Oteryn_import_ready",5)]:
            part = s["counts"][scope][key]
            assert part["total"] == len(rows) == sum(part[k] for k in ["complete","incomplete_after_resolved_identity","identity_unresolved"])
            assert part["complete"] == sum(r[col] == "COMPLETE" for r in rows)
            assert part["incomplete_after_resolved_identity"] == sum(r[col] == "INCOMPLETE" for r in rows)
    assert s["counts"]["type_disjoint_partitions"]["Basic"] == {"complete":49,"incomplete_after_resolved_identity":77,"identity_unresolved":6102,"total":6228}
    assert inv["open_PR_count"] == len(inv["PRs"]) == 31
    assert sum(len(r["complete_file_paths"]) for r in inv["PRs"]) == 6804
    assert inv["all_paths_complete_and_head_revalidated"]
    # A server admission hold must not erase the126 accepted current-client joins.
    assert sum(r[9] for r in i["appearance_rows"]) == 126
    assert not any(r[10] for r in i["appearance_rows"])
    assert all(r[3] == "IDENTITY_UNRESOLVED" for r in i["appearance_rows"] if not r[9])


def night_collection():
    n = load(HERE,"night-all-visible-items.json")
    s = load(HERE,"night-all-visible-summary.json")
    a = load(HERE,"night-Armors33-visible-data.json")
    ledger = load(HERE,"night-collection-inventory.json")
    rows = n["records"]
    assert len(rows) == len({r["observation_key"] for r in rows}) == 2907
    assert sum(r["ALL_VISIBLE_status"] == "COMPLETE" for r in rows) == 340
    assert sum(r["ALL_VISIBLE_status"] == "PARTIAL" for r in rows) == 2540
    assert sum(r["ALL_VISIBLE_status"] == "UNRECONCILED" for r in rows) == 27
    assert sum(s["complete_components"].values()) == 340
    assert sum(s["partial_primary_reason_groups"].values()) == 2540
    assert len(a["records"]) == 33 and a["NPC_quote_rows"] == 218
    assert sum(len(r[p]["entries"]) for r in a["records"] for p in ["sell_to","buy_from"]) == 218
    assert sum(not r[p]["entries"] for r in a["records"] for p in ["sell_to","buy_from"]) == 37
    assert ledger["Windows_metadata_archive"]["present_byte_identical_in_workspace"] == 531
    assert ledger["Armors33_reference_hash_checks"]["unique_referenced_files"] == 402
    assert not ledger["Armors33_reference_hash_checks"]["failures"]
    assert all(r["state"] == "HASH_MATCH" for r in ledger["Armors33_reference_hash_checks"]["receipts"].values())
    assert all(not r["canonical_binding_required"] for r in rows)
    assert all(not r["missing_data_or_recovery"] for r in rows if r["ALL_VISIBLE_status"] == "COMPLETE")
    assert all(r["missing_data_or_recovery"] for r in rows if r["ALL_VISIBLE_status"] != "COMPLETE")
    assert s["UI_actions"] == s["runtime_changes"] == 0


def check_npc_recovery(d):
    states = {"RECOVERED_COMPLETE":0,"RECOVERED_PARTIAL":0,"UNRESOLVED":0}
    assert len(d["rows"]) == len({r[0] for r in d["rows"]}) == 1992
    for r in d["rows"]:
        assert d["proofs"][r[4]][0] == r[0] and d["proofs"][r[4]][6]
        panels = [d["panels"][r[5]],d["panels"][r[6]]]
        for name,p in zip(["Sell To","Buy From"],panels):
            controls = d["controls"][name]
            if p[0] == "COMPLETE_OBSERVED_EMPTY":
                assert p[3] in controls["empty_hashes"] and not p[1]
            if p[0] == "COMPLETE_NONEMPTY_NO_OVERFLOW":
                assert p[4] in controls["full_scrollbar_hashes"] and p[1]
                assert not d["unresolved_reason_sets"][p[2]]
            for qi,y in p[1]:
                q = d["quote_facts"][qi]
                assert q["panel"] == name and q["currency_literal"] == "gp" and q["native_gp_bitmap_proved"]
                assert isinstance(q["price"],int) and q["price"] >= 0
                if q["location_display"] == "Various Locations":
                    assert q["native_Various_Locations_control_match"]
        expected = "RECOVERED_COMPLETE" if all(p[0].startswith("COMPLETE_") for p in panels) else ("RECOVERED_PARTIAL" if any(p[1] or p[0] == "COMPLETE_OBSERVED_EMPTY" for p in panels) else "UNRESOLVED")
        assert r[3] == expected
        states[r[3]] += 1
    assert states == {"RECOVERED_COMPLETE":1307,"RECOVERED_PARTIAL":678,"UNRESOLVED":7}
    return states


def npc_recovery(source_path=None):
    d = load(HERE,"npc1992-recovery.json")
    check_npc_recovery(d)
    s = load(HERE,"npc1992-recovery-summary.json")
    assert s["current_ALL_counts"] == {"COMPLETE":1647,"PARTIAL":1226,"UNRECONCILED":34,"total":2907}
    assert s["qualified_visible_quote_occurrences"] == sum(len(p[1]) for p in d["panels"]) == 1120
    for mutation in ["empty_pixels","overflow"]:
        bad = copy.deepcopy(d)
        if mutation == "empty_pixels":
            next(p for p in bad["panels"] if p[0] == "COMPLETE_OBSERVED_EMPTY")[3] = "0"*64
        else:
            next(p for p in bad["panels"] if p[0] == "COMPLETE_NONEMPTY_NO_OVERFLOW")[4] = "0"*64
        try:
            check_npc_recovery(bad)
        except AssertionError:
            pass
        else:
            raise AssertionError("NPC proof negative control accepted: "+mutation)
    if source_path:
        b = source_path.read_bytes()
        assert hashlib.sha256(b).hexdigest() == d["raw_fact_source"]["sha256"]
        raw = json.loads(b)
        cols = [c["values"] if c["encoding"] == "LITERAL" else [c["dictionary"][i] for i in c["indices"]] for c in raw["columns"]]
        norm = lambda s:"".join(c for c in s.lower() if c.isalnum())
        for q in d["quote_facts"]:
            i = q["paired_raw_example_index0"]
            assert cols[5][i] == q["NPC_name"]
            assert cols[7 if q["panel"] == "Buy From" else 8][i] == q["price"]
            if not q["native_Various_Locations_control_match"]:
                assert norm(cols[6][i]) == norm(q["location_display"])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source-dir", type=Path)
    ap.add_argument("--npc-source", type=Path)
    args = ap.parse_args()
    for r in load(HERE, "publication-members.json")["members"]:
        b = (HERE / r["file"]).read_bytes()
        assert len(b) == r["bytes"] and hashlib.sha256(b).hexdigest() == r["sha256"]
    names = ["appearance-field-coverage.json", "category-field-coverage.json",
             "ui-field-reuse-and-queue.json", "current-proficiency-delta.json", "compiled-enum-delta.json"]
    data = [load(HERE,n) for n in names]
    validate(*data)
    rec = load(HERE, "saved1853-proof-reconciliation.json")
    reconcile(rec)
    completeness()
    night_collection()
    npc_recovery(args.npc_source)
    stale = copy.deepcopy(rec)
    stale["image_receipts"][24]["selected_card_name"] = "empty potion flask"
    try:
        reconcile(stale)
    except AssertionError:
        pass
    else:
        raise AssertionError("Stale-selection substitution was accepted")
    controls = []
    bad = copy.deepcopy(data); bad[0]["rows"] = [r for r in bad[0]["rows"] if r[1] is not None]
    controls.append(bad)
    bad = copy.deepcopy(data); bad[0]["field_state_rules"]["authoritative_UI_to_appearance_or_server_bindings_added"] = 1
    controls.append(bad)
    bad = copy.deepcopy(data); bad[3]["deltas"][0]["current"]["Value"] = .015
    controls.append(bad)
    for n, bad in enumerate(controls):
        try:
            validate(*bad)
        except AssertionError:
            continue
        raise AssertionError(f"Negative control {n} was accepted")
    if args.source_dir:
        sources(args.source_dir, data[0], data[2])
    print("PASS:6470 appearances/6228 types;1992 NPC recovery1307/678/7;ALL1647/1226/34;1120 quotes;6 negative controls"
          + ("; pinned-source reproduction" if args.source_dir else ""))


if __name__ == "__main__":
    main()
