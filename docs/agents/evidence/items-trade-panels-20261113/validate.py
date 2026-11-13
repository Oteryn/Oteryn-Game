#!/usr/bin/env python3
"""Validate the bounded export; --private-root additionally checks Ubuntu originals."""
import argparse
import collections
import hashlib
import json
from pathlib import Path, PurePosixPath
import re

HERE = Path(__file__).resolve().parent
EXPECTED = {"Decoration": (462, 669, 255, 410), "Food": (169, 245, 93, 227)}
QUOTE_KEYS = {
    "price_literal", "npc_literal", "first_line_literal", "currency_state",
    "currency_literal", "location_state", "location_literal",
    "location_display_line_literal", "location_unknown_reason",
}
PANELS = {"Sell To", "Buy From"}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def keys(value, required, optional=()):
    require(isinstance(value, dict), "expected object")
    require(set(required) <= value.keys() <= set(required) | set(optional),
            "unexpected or missing fields: " + str(sorted(value)))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def safe_key(value):
    path = PurePosixPath(value)
    require(not path.is_absolute() and ".." not in path.parts
            and "\\" not in value and path.as_posix() == value, "unsafe proof key")


def currency(price):
    if re.fullmatch(r"[0-9][0-9,]*(?:\.[0-9]+)? gp", price):
        return "OBSERVED_EXPLICIT", "gp"
    match = re.fullmatch(r"[0-9][0-9,]*(?:\.[0-9]+)? x (.+)", price)
    return ("OBSERVED_EXPLICIT", match.group(1)) if match else ("UNKNOWN", None)


def projected_item(item, category):
    panels = {}
    for label, panel in item["panels"].items():
        entries = []
        for entry in panel["visible_entries"]:
            state, literal = currency(entry["price_literal"])
            location = entry["location"]
            quote = {key: entry[key] for key in
                     ("price_literal", "npc_literal", "first_line_literal")}
            quote.update(currency_state=state, currency_literal=literal,
                         location_state=location["state"])
            for source, target in (("literal", "location_literal"),
                                   ("display_line_literal", "location_display_line_literal"),
                                   ("reason", "location_unknown_reason")):
                if source in location:
                    quote[target] = location[source]
            entries.append(quote)
        panels[label] = {key: panel[key] for key in
                         ("state", "vertical_coverage", "horizontal_clipping",
                          "additional_offscreen_entries")}
        panels[label]["visible_entries"] = entries
    return {
        "category": category, "physical_ui_ordinal": item["ui_entry_ordinal"],
        "capture_key": "items-sequence-" + category.lower() + ":" + str(item["capture_id"]),
        "fullName": item["fullName"], "panels": panels,
        "proof": {"key": item["proof"]["private_source_key"],
                  "sha256": item["proof"]["sha256"]},
    }


def validate_document(doc):
    keys(doc, ("schema", "classification", "category", "coverage", "items"))
    require(doc["schema"] == "OTERYN_INITIAL_TRADE_PANEL_EVIDENCE/v1"
            and doc["classification"] == "OBSERVED", "schema/classification")
    category = doc["category"]
    require(category in EXPECTED, "unexpected category")
    count, empty, nonempty, offers = EXPECTED[category]
    keys(doc["coverage"], ("physical_ui_ordinals", "cards", "initial_panels",
                          "initial_view_complete", "full_npc_lists"))
    require(doc["coverage"] == {
        "physical_ui_ordinals": [1, count], "cards": count, "initial_panels": count * 2,
        "initial_view_complete": True, "full_npc_lists": "PARTIAL",
    }, "coverage must distinguish initial view from complete NPC lists")
    require(len(doc["items"]) == count, "card count")
    states = collections.Counter()
    quoted = 0
    proofs = set()
    unknown_currency = []
    for ordinal, item in enumerate(doc["items"], 1):
        keys(item, ("category", "physical_ui_ordinal", "capture_key", "fullName",
                    "panels", "proof"))
        require(item["category"] == category and item["physical_ui_ordinal"] == ordinal,
                "ordinal/category gap or duplicate")
        require(item["capture_key"] == f"items-sequence-{category.lower()}:{ordinal}",
                "capture key is not an ID binding")
        require(isinstance(item["fullName"], str) and 0 < len(item["fullName"]) <= 200
                and "\n" not in item["fullName"], "invalid name")
        keys(item["proof"], ("key", "sha256"))
        proof = item["proof"]
        safe_key(proof["key"])
        require(re.fullmatch(r"items-sequence-" + category.lower()
                            + r"/cards/[0-9]{5}-basic-top\.png", proof["key"]),
                "unexpected proof namespace")
        require(re.fullmatch(r"[0-9a-f]{64}", proof["sha256"]), "proof SHA256")
        proofs.add(proof["key"])
        require(set(item["panels"]) == PANELS, "two named panels required")
        for label, panel in item["panels"].items():
            keys(panel, ("state", "visible_entries", "vertical_coverage",
                         "horizontal_clipping", "additional_offscreen_entries"))
            state = panel["state"]
            require(state in {"OBSERVED_EMPTY_PANEL", "OBSERVED_NONEMPTY_INITIAL_PANEL"},
                    "panel state")
            require(bool(panel["visible_entries"]) ==
                    (state == "OBSERVED_NONEMPTY_INITIAL_PANEL"), "empty-panel inconsistency")
            states[state] += 1
            for quote in panel["visible_entries"]:
                keys(quote, ("price_literal", "npc_literal", "first_line_literal",
                             "currency_state", "currency_literal", "location_state"),
                     QUOTE_KEYS)
                for field in ("price_literal", "npc_literal", "first_line_literal"):
                    require(isinstance(quote[field], str) and len(quote[field]) <= 400
                            and "\n" not in quote[field], "invalid literal")
                expected_currency = currency(quote["price_literal"])
                require((quote["currency_state"], quote["currency_literal"]) ==
                        expected_currency, "currency inferred or corrupted")
                if expected_currency[0] == "UNKNOWN":
                    unknown_currency.append((ordinal, label, quote["price_literal"]))
                quoted += 1
    require(len(proofs) == count, "proof keys reused across distinct captures")
    require((states["OBSERVED_EMPTY_PANEL"], states["OBSERVED_NONEMPTY_INITIAL_PANEL"],
             quoted) == (empty, nonempty, offers), "panel/quote census mismatch")
    if category == "Decoration":
        first = doc["items"][309]["panels"]["Sell To"]["visible_entries"][0]
        require(first["npc_literal"] == "Yasir" and
                first["first_line_literal"] == "10 gp, Yasir", "Yasir overlay missing")
    else:
        require(not unknown_currency, "native gp correction missing")
        require(any(q["npc_literal"] == "Ramina"
                    for p in doc["items"][20]["panels"].values()
                    for q in p["visible_entries"]), "Ramina overlay missing")
    return {"category": category, "cards": count, "panels": count * 2,
            "visible_quotes": quoted, "currency_unknown_quotes": len(unknown_currency)}


def validate_material(root, receipts, documents, pairs, private_root=None):
    correction = receipts["material_corrections"]
    keys(correction, ("file", "sha256", "private_overlay_key", "private_overlay_sha256"))
    require(correction["file"] == "material-corrections.json", "material filename")
    require(digest(root / correction["file"]) == correction["sha256"], "material hash")
    data = json.loads((root / correction["file"]).read_text())
    keys(data, ("schema", "operations", "source_unions", "qc_inputs", "semantic_qc"))
    require(data["schema"] == "OTERYN_TRADE_NATIVE_CORRECTIONS/v1"
            and data["semantic_qc"] == "PARTIAL", "semantic QC remains partial")
    require(len(data["operations"]) == 20 and len(data["qc_inputs"]) == 4,
            "material operation/input census")
    for op in data["operations"]:
        keys(op, ("category", "ui_entry_ordinal", "localrow", "field", "old_literal",
                  "new_literal", "panel", "proof"))
        item = documents[op["category"]]["items"][op["ui_entry_ordinal"] - 1]
        require(op["proof"] == item["proof"], "correction source proof")
        if op["field"] == "panel":
            require(op["ui_entry_ordinal"] == 42 and op["old_literal"] == "Sell To"
                    and op["new_literal"] == "Buy From"
                    and not item["panels"]["Sell To"]["visible_entries"]
                    and item["panels"]["Buy From"]["visible_entries"][0]["npc_literal"]
                    == "Black Bert", "panel correction missing")
        else:
            quote = item["panels"][op["panel"]]["visible_entries"][op["localrow"] - 1]
            require(op["field"] in {"price_literal", "npc_literal", "location_literal"}
                    and quote[op["field"]] == op["new_literal"], "native correction missing")
            require(quote["first_line_literal"] == quote["price_literal"] + ", "
                    + quote["npc_literal"], "corrected composite literal")
    for ref in data["source_unions"] + data["qc_inputs"]:
        keys(ref, ("key", "sha256"), ("category",))
        safe_key(ref["key"])
        if private_root is not None:
            require(digest(private_root / ref["key"]) == ref["sha256"], "QC source hash")
    safe_key(correction["private_overlay_key"])
    if private_root is not None:
        require(digest(private_root / correction["private_overlay_key"])
                == correction["private_overlay_sha256"], "private material overlay hash")
    group = receipts["full_panel_retakes"]
    keys(group, ("panels", "pending_pairs_before", "pending_pairs_after"))
    require(group["pending_pairs_before"] == 77 and group["pending_pairs_after"] == 74
            and len(group["panels"]) == 3, "resolved retake census")
    for receipt, ordinal, rows, pages, price, location in zip(
            group["panels"], (46, 52, 59), (18, 18, 19), (24, 24, 25),
            ("25 gp", "25 gp", "5,000 gp"), ("Cormaya", "Cormaya", "Darashia City")):
        validate_full_panel(root, receipt, documents, pairs, private_root,
                            ordinal, rows, pages, price, location)


def validate_full_panel(root, receipt, documents, pairs, private_root,
                        ordinal, row_count, page_count, expected_price, fourth_location):
    keys(receipt, ("file", "sha256", "initial_quotes_not_added_to_full_quote_count"))
    require(receipt["file"] == f"decoration-full-panel-{ordinal}-buy-from.json"
            and receipt["initial_quotes_not_added_to_full_quote_count"] is True,
            "retake scope")
    require(digest(root / receipt["file"]) == receipt["sha256"], "full-panel hash")
    full = json.loads((root / receipt["file"]).read_text())
    keys(full, ("schema", "category", "physical_ui_ordinal", "panel", "source_receipt",
                "coverage", "rows", "viewports", "initial_resolution"))
    require(full["schema"] == "OTERYN_OBSERVED_FULL_TRADE_PANEL/v1"
            and (full["category"], full["physical_ui_ordinal"], full["panel"])
            == ("Decoration", ordinal, "Buy From"), "full-panel identity")
    require(full["coverage"] == {
        "observed_panel_literals_complete": True, "global_npc_lists": "PARTIAL",
        "distinct_physical_rows": row_count, "viewports": page_count,
        "identity": "Measured contiguous pixel ancestry; no name deduplication"},
        "full-panel coverage")
    require(len(full["rows"]) == row_count and len(full["viewports"]) == page_count, "full-panel census")
    offsets = list(range(0, 401, 20)) + [406, 406, 406] if ordinal != 59 else list(range(0, 421, 20)) + [434, 434, 434]
    for index, page in enumerate(full["viewports"]):
        keys(page, ("page_index0", "viewport_offset_pixels", "proof"))
        require(page["page_index0"] == index and page["viewport_offset_pixels"] == offsets[index],
                "contiguous viewport offsets")
    def check_proof(proof):
        keys(proof, ("key", "sha256")); safe_key(proof["key"])
        require(proof["key"].startswith("items-sequence-decoration/trade-retake-"),
                "retake proof namespace")
        if private_root is not None:
            require(digest(private_root / proof["key"]) == proof["sha256"], "retake proof hash")
    for page in full["viewports"]:
        check_proof(page["proof"])
    for index, row in enumerate(full["rows"]):
        keys(row, ("physical_panel_row_ordinal", "quote", "content_first_line_y_pixels",
                   "content_residence_line_y_pixels", "qualified_page_indices0", "proofs"))
        require(row["physical_panel_row_ordinal"] == index + 1
                and row["content_first_line_y_pixels"] == 3 + 28 * index
                and row["content_residence_line_y_pixels"] == 17 + 28 * index,
                "physical row ancestry")
        quote = row["quote"]
        keys(quote, ("price_literal", "npc_literal", "first_line_literal", "currency_state",
                     "currency_literal", "location_state", "location_literal",
                     "location_display_line_literal"))
        require(quote["price_literal"] == expected_price and quote["currency_state"] == "OBSERVED_EXPLICIT"
                and quote["currency_literal"] == "gp" and quote["location_state"] == "OBSERVED_VALUE",
                "retake literals")
        require(row["qualified_page_indices0"] and
                row["proofs"] == [full["viewports"][i]["proof"]
                                  for i in row["qualified_page_indices0"]], "row page ancestry")
        for proof in row["proofs"]:
            check_proof(proof)
    resolution = full["initial_resolution"]
    keys(resolution, ("resolved_pending_pair", "initial_quote_rows1", "full_panel_rows1",
                      "initial_snapshot_preserved", "new_disjoint_quotes_not_claimed",
                      "fourth_location_previous_state", "fourth_location_now_literal"))
    require(resolution["initial_quote_rows1"] == [1, 2, 3, 4]
            and resolution["full_panel_rows1"] == [1, 2, 3, 4]
            and resolution["initial_snapshot_preserved"] is True
            and resolution["new_disjoint_quotes_not_claimed"] is True
            and resolution["fourth_location_previous_state"] ==
            ("UNKNOWN" if ordinal == 46 else "UNKNOWN_CLIPPED_BELOW_PANEL")
            and resolution["fourth_location_now_literal"] == fourth_location, "initial overlap boundary")
    initial = documents["Decoration"]["items"][ordinal - 1]["panels"]["Buy From"]["visible_entries"]
    # Initial packets use two observed-state vocabularies; compare literal fields
    # without rewriting either snapshot's original state labels.
    literal_keys = ("price_literal", "npc_literal", "first_line_literal",
                    "location_literal", "location_display_line_literal")
    require(len(initial) == 4 and all(
        all(initial[i][k] == full["rows"][i]["quote"][k] for k in literal_keys)
        for i in range(3))
            and initial[3]["location_state"] == resolution["fourth_location_previous_state"]
            and full["rows"][3]["quote"]["location_literal"] == fourth_location, "initial history lost")
    require(("Decoration", ordinal, "Buy From") not in pairs, "resolved retake still pending")
    ref = full["source_receipt"]; keys(ref, ("key", "sha256")); safe_key(ref["key"])
    if private_root is not None:
        require(digest(private_root / ref["key"]) == ref["sha256"], "full-panel source hash")
        source = json.loads((private_root / ref["key"]).read_text())
        require(len(source["rows"]) == len(full["rows"]), "private full-row count")
        for raw, row in zip(source["rows"], full["rows"]):
            quote = row["quote"]
            require(all(quote[k] == raw[k] for k in
                        ("price_literal", "npc_literal", "first_line_literal"))
                    and quote["location_state"] == raw["location"]["state"]
                    and quote["location_literal"] == raw["location"]["literal"]
                    and quote["location_display_line_literal"] ==
                    raw["location"]["display_line_literal"], "private full-row projection")


def validate(root=HERE, private_root=None):
    receipts = json.loads((root / "source-receipts.json").read_text())
    keys(receipts, ("schema", "base_sha", "categories", "correction_overlays", "adapter",
                    "material_corrections", "full_panel_retakes"))
    require(receipts["schema"] == "OTERYN_INITIAL_TRADE_SOURCE_RECEIPTS/v1",
            "receipt schema")
    require(receipts["base_sha"] == "5efd67da164ac9dc33c8c066434b228cebb86c2b",
            "base pin")
    results = []
    documents = {}
    for receipt in receipts["categories"]:
        keys(receipt, ("category", "source_union_key", "source_union_sha256", "export_file",
                       "export_sha256", "cards", "panels", "source_Basic_proofs_freshly_verified",
                       "states", "visible_quotes", "currency_UNKNOWN"))
        safe_key(receipt["source_union_key"])
        require(receipt["export_file"] == receipt["category"].lower() + "-initial-panels.json",
                "unexpected export filename")
        path = root / receipt["export_file"]
        require(digest(path) == receipt["export_sha256"], "export receipt mismatch")
        doc = json.loads(path.read_text())
        results.append(validate_document(doc))
        documents[doc["category"]] = doc
        require(receipt["category"] == doc["category"] and
                receipt["cards"] == len(doc["items"]) and
                receipt["panels"] == len(doc["items"]) * 2 and
                receipt["source_Basic_proofs_freshly_verified"] == len(doc["items"]) and
                receipt["visible_quotes"] == results[-1]["visible_quotes"],
                "source receipt census mismatch")
        if private_root is not None:
            source = private_root / receipt["source_union_key"]
            require(digest(source) == receipt["source_union_sha256"], "private union mismatch")
            original = json.loads(source.read_text())
            require([projected_item(i, doc["category"]) for i in original["items"]] ==
                    doc["items"], "whitelist differs from private source union")
            for item in doc["items"]:
                require(digest(private_root / item["proof"]["key"]) ==
                        item["proof"]["sha256"], "private Basic proof mismatch")
    require(set(documents) == set(EXPECTED) and len(results) == 2, "category set")
    backlog = json.loads((root / "retake-backlog.json").read_text())
    keys(backlog, ("schema", "global_npc_lists", "retakes"))
    require(backlog["global_npc_lists"] == "PARTIAL", "retakes are not full coverage")
    pairs = set()
    for row in backlog["retakes"]:
        keys(row, ("category", "physical_ui_ordinal", "panel", "unknown_reason",
                   "location_state", "additional_entries_state"))
        triple = row["category"], row["physical_ui_ordinal"], row["panel"]
        require(triple not in pairs and row["panel"] in PANELS, "retake duplicate/panel")
        pairs.add(triple)
        item = documents[row["category"]]["items"][row["physical_ui_ordinal"] - 1]
        require(item["panels"][row["panel"]]["state"] == "OBSERVED_NONEMPTY_INITIAL_PANEL",
                "retake references empty panel")
    require(collections.Counter(p[0] for p in pairs) == {"Decoration": 40, "Food": 34},
            "retake census")
    require({p[1] for p in pairs if p[0] == "Decoration" and p[2] == "Sell To"} ==
            {310, 382}, "Decoration Sell To retakes lost")
    require(len(receipts["correction_overlays"]) == 2, "correction receipt count")
    for correction in receipts["correction_overlays"]:
        common = ("category", "key", "sha256", "physical_ui_ordinal")
        if correction["category"] == "Decoration":
            keys(correction, common + ("panel", "entry_index1", "fields"))
            require(correction["physical_ui_ordinal"] == 310 and
                    correction["panel"] == "Sell To" and correction["entry_index1"] == 1 and
                    correction["fields"] == {"npc_literal": "Yasir",
                                              "first_line_literal": "10 gp, Yasir"},
                    "unexpected Decoration correction")
        else:
            keys(correction, common + ("npc_literal",))
            require(correction["category"] == "Food" and
                    correction["physical_ui_ordinal"] == 21 and
                    correction["npc_literal"] == "Ramina", "unexpected Food correction")
        safe_key(correction["key"])
        require(re.fullmatch(r"[0-9a-f]{64}", correction["sha256"]), "overlay hash")
        if private_root is not None:
            require(digest(private_root / correction["key"]) == correction["sha256"],
                    "private overlay receipt mismatch")
    validate_material(root, receipts, documents, pairs, private_root)
    return {"categories": results, "cards": 631, "initial_panels": 1262,
            "source_proofs": 631, "retake_pairs": len(pairs),
            "full_panels": 3, "full_panel_rows": 55, "full_panel_viewports": 73,
            "private_sources_and_proofs_checked": private_root is not None,
            "full_npc_lists": "PARTIAL"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--private-root", type=Path)
    args = parser.parse_args()
    print(json.dumps(validate(private_root=args.private_root), sort_keys=True))
