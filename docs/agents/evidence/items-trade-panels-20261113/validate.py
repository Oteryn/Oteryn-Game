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
        require({v[0] for v in unknown_currency} == {8, 27, 46, 49}
                and len(unknown_currency) == 6, "bare-price currency policy drift")
        require(any(q["npc_literal"] == "Ramina"
                    for p in doc["items"][20]["panels"].values()
                    for q in p["visible_entries"]), "Ramina overlay missing")
    return {"category": category, "cards": count, "panels": count * 2,
            "visible_quotes": quoted, "currency_unknown_quotes": len(unknown_currency)}


def validate(root=HERE, private_root=None):
    receipts = json.loads((root / "source-receipts.json").read_text())
    keys(receipts, ("schema", "base_sha", "categories", "correction_overlays", "adapter"))
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
    require(collections.Counter(p[0] for p in pairs) == {"Decoration": 43, "Food": 34},
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
    return {"categories": results, "cards": 631, "initial_panels": 1262,
            "source_proofs": 631, "retake_pairs": len(pairs),
            "private_sources_and_proofs_checked": private_root is not None,
            "full_npc_lists": "PARTIAL"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--private-root", type=Path)
    args = parser.parse_args()
    print(json.dumps(validate(private_root=args.private_root), sort_keys=True))
