#!/usr/bin/env python3
"""Retain bounded seasonal facts from an already obtained MediaWiki API response.

This is offline evidence tooling; it does not retry blocked endpoints or retain
whole page bodies. The existing BR capture parser supplies bounded infobox facts.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = "docs/reference/content-prep/season-20261107/seasonal-calendar-source.json"
EXPECTED = {"Santa Claus": 443236, "Christmas": 431702}


def project(response):
    spec = importlib.util.spec_from_file_location("wiki_br_capture", ROOT / "tools/content-schema/monster-authoring/wiki_br_capture.py")
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    records = []
    for page in response["query"]["pages"].values():
        if page["title"] not in EXPECTED:
            continue
        revision, = page["revisions"]
        if revision["revid"] != EXPECTED[page["title"]]:
            raise ValueError("SEASONAL_WIKI_REVISION")
        text = revision["slots"]["main"].get("*", revision["slots"]["main"].get("content"))
        infobox, fields = parser.infobox(text)
        lines = parser.field_lines(text)
        record = {"title": page["title"], "page_id": page["pageid"], "revision_id": revision["revid"],
                  "revision_timestamp": revision["timestamp"], "content_sha256": hashlib.sha256(text.encode()).hexdigest(),
                  "source_url": f"https://www.tibiawiki.com.br/index.php?oldid={revision['revid']}",
                  "infobox": infobox, "fields": fields, "field_lines": {key: lines[key] for key in fields if key in lines}}
        if page["title"] == "Santa Claus":
            placements = []
            for number, line in enumerate(text.splitlines(), 1):
                match = re.match(r"\* '''\[\[(.*?)\]\]:'''", line)
                coordinate = re.search(r"\{\{mapa\|(\d+),(\d+),(\d+):", line)
                if match and coordinate:
                    x, y, z = map(int, coordinate.groups())
                    placements.append({"city": match[1], "x": x, "y": y, "z": z, "source_line": number})
            if len(placements) != 15 or len({(p["x"], p["y"], p["z"]) for p in placements}) != 15:
                raise ValueError("SEASONAL_SANTA_PLACEMENTS")
            record["placements"] = placements
        else:
            record["availability_observations"] = [{"source_line": number, "text": text.splitlines()[number-1][:300]} for number in (27,83)]
        records.append(record)
    if {record["title"] for record in records} != set(EXPECTED):
        raise ValueError("SEASONAL_WIKI_PAGE_SET")
    return {"schema": "OTERYN_SEASONAL_STRUCTURED_WIKI_EVIDENCE/v1", "evidence_level": "TIBIAWIKI_STRUCTURED",
            "api": parser.API, "capture_method": "offline structured extraction of retained successful API response",
            "generator": "tools/content-migration/seasonal_wiki_evidence.py", "raw_page_text_retained": False,
            "records": sorted(records, key=lambda record: record["title"])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cached-response", required=True, type=Path)
    parser.add_argument("--out", type=Path, default=ROOT / OUTPUT)
    args = parser.parse_args()
    result = project(json.loads(args.cached_response.read_bytes()))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    from compact_record_json import encode
    args.out.write_bytes(encode(result))
    print("seasonal wiki evidence: two pinned pages,15 exact Santa positions")

if __name__ == "__main__":
    main()
