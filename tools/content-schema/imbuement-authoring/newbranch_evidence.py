#!/usr/bin/env python3
"""Verify normalized evidence from Crystal's pinned imbuements branch, offline."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

HERE = Path(__file__).resolve().parent
PACKET = HERE / "samples/crystal-imbuements-evidence.json"
REVISION = "15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1"


def parse_xml(raw):
    text = raw.decode("utf-8")
    root = ET.fromstring(text)
    starts = [i for i, line in enumerate(text.splitlines(), 1)
              if re.search(r"<imbuement\s", line)]
    records = []
    for node, line in zip(root.findall("imbuement"), starts, strict=True):
        effects = [dict(a.attrib) for a in node.findall("attribute")
                   if a.get("key") == "effect"]
        if len(effects) != 1:
            raise ValueError("exactly one explicit effect is required")
        records.append({
            "name": node.get("name"), "tier": int(node.get("base")),
            "category_id": int(node.get("category")),
            "premium": node.get("premium") == "1",
            "storage": int(node.get("storage")),
            "scroll_item_id": int(node.get("scrollid")),
            "effect": effects[0],
            "materials": [{"source_item_id": int(a.get("value")),
                           "count": int(a.get("count"))}
                          for a in node.findall("attribute") if a.get("key") == "item"],
            "source_line": line,
        })
    records.sort(key=lambda row: (row["name"], row["tier"]))
    return {
        "bases": [dict(n.attrib) for n in root.findall("base")],
        "categories": [dict(n.attrib) for n in root.findall("category")],
        "records": records,
    }


def validate(packet):
    if packet["revision"] != REVISION or packet["role"] != "OTS_HYPOTHESIS_ONLY":
        raise ValueError("Crystal hypothesis must retain its exact provenance")
    records = packet["xml"]["records"]
    names = {r["name"] for r in records}
    if len(records) != 72 or len(names) != 24:
        raise ValueError("expected 24 types with all three tiers")
    if {(r["name"], r["tier"]) for r in records} != {(n, t) for n in names for t in (1, 2, 3)}:
        raise ValueError("duplicate or missing tier")
    for record in records:
        if not record["materials"] or any(m["count"] < 1 for m in record["materials"]):
            raise ValueError("invalid recipe")
    comparison = packet["selected_catalogue_comparison"]
    if len(comparison["records"]) != 72:
        raise ValueError("every record requires a comparison")
    if len(packet["gold_token_bundles"]) != 9:
        raise ValueError("nine token exchange bundles required")
    for fact in packet["engine_facts"]:
        if fact["confidence"] != "OTS_SOURCE_CODE_ONLY":
            raise ValueError("engine code cannot prove official Global behavior")
        for anchor in fact["anchors"]:
            source = packet["sources"][anchor["path"]]
            if not 1 <= anchor["line"] <= source["line_count"]:
                raise ValueError("source anchor is outside pinned file")
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--xml", type=Path, help="Optionally reparse a downloaded pinned XML")
    args = parser.parse_args()
    packet = json.loads(PACKET.read_text())
    validate(packet)
    if args.xml:
        raw = args.xml.read_bytes()
        source = packet["sources"]["data/XML/imbuements.xml"]
        if hashlib.sha256(raw).hexdigest() != source["sha256"]:
            raise ValueError("XML differs from the captured pinned source")
        if parse_xml(raw) != packet["xml"]:
            raise ValueError("normalized XML differs from the pinned source")
    print("Crystal imbuements: 72 records, 9 token bundles, pinned OTS evidence verified")


if __name__ == "__main__":
    main()
