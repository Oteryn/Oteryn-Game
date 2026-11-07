#!/usr/bin/env python3
"""Capture English TibiaWiki (tibia.fandom.com) Item stat evidence for every Tibia item id.

Network tool, run manually; not part of repository CI (ITEM-SEM-2a). It lists every
main-namespace page embedding `{{Infobox Object` (`list=embeddedin`), fetches the current
revision of each, parses the infobox and keeps, for every integer in its `itemid` field,
the raw value of each admitted stat parameter (`STAT_PARAMS`: requirements, hands and
slot, attack and elemental attacks, defense, armor, range, hit chance, imbuement slots,
upgrade classification, leech and critical hit, the `attrib` and `resist` texts, weight,
charges, duration, stackability and similar). Nothing else is stored: no article text,
notes, prices, drop lists or images, only page and revision identity, digests and the raw
field observations.

Values stay the wiki's own strings (`"+3"`, `"2%"`, `"club fighting +4"`); typing them is
the promotion lowering's job, which records both. Several pages may list the same id; each
is kept as its own observation, ordered by page id, so disagreement stays visible.

A record is keyed by the A12 Tibia Item key `oteryn:item.tibia.i<id>` only when that key
resolves: an Item record in `content/items/definitions` or a target of the Crystal item
bindings (the rule `item_key_references.py` enforces for `imports/`). Every other id is
keyed by its bare decimal Tibia id (`"12345"`), so the evidence is kept without naming an
Item key that does not exist. Records are ordered by item id. `snapshot_sha256` digests the
canonical records, as in the family fallback snapshot.

Usage:
    python item_wiki_stats_capture.py [--cache RAW.json] [--output PATH]
    python item_wiki_stats_capture.py --rekey
`--cache` reads or writes the raw page fetch, so a re-run can re-parse without the network.
`--rekey` re-applies the key rule to the committed snapshot offline, keeping every
observation and the capture time.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

import item_wiki_family_capture as family_capture
from item_key_references import CRYSTAL_BINDINGS, record_keys

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = ROOT / "imports" / "tibiawiki" / "facts" / "items-stats.json"
SCHEMA = "OTERYN_ITEM_WIKI_STATS_SNAPSHOT/v1"
BATCH_ID = "g5-item-stats-tibiawiki-r1"
WIKI = "https://tibia.fandom.com/wiki/"
ATTRIBUTION = {
    "authors": "TibiaWiki contributors",
    "license": "CC BY-SA 3.0",
    "license_url": "https://creativecommons.org/licenses/by-sa/3.0/",
    "page_history": "https://tibia.fandom.com/index.php?curid=<page_id>&oldid=<revision_id>",
}
FETCH_BATCH = 50

# Admitted infobox parameters (English TibiaWiki `Template:Infobox Object`), each a stat
# the Item model has a place for. Anything else on the page is never stored.
STAT_PARAMS = (
    "armor",
    "atk_mod",
    "attack",
    "attrib",
    "augments",
    "charges",
    "critextra_dmg",
    "crithit_ch",
    "damagerange",
    "damagetype",
    "death_attack",
    "defense",
    "defensemod",
    "duration",
    "earth_attack",
    "elementalbond",
    "energy_attack",
    "fire_attack",
    "hands",
    "hit_chance",
    "hit_mod",
    "holy_attack",
    "hpleech_am",
    "hpleech_ch",
    "ice_attack",
    "imbueslots",
    "levelrequired",
    "manacost",
    "manaleech_am",
    "manaleech_ch",
    "mantra",
    "mlrequired",
    "objectclass",
    "primarytype",
    "range",
    "resist",
    "secondarytype",
    "slot",
    "stackable",
    "upgradeclass",
    "vocrequired",
    "volume",
    "walkingspeed",
    "weapontype",
    "weight",
)
INFOBOX_START = re.compile(r"\{\{\s*Infobox[ _]Object\b", re.IGNORECASE)
ITEM_ID = re.compile(r"^\s*([1-9][0-9]*)\s*$")


def infobox_body(content):
    """The `{{Infobox Object ...}}` body up to its own closing braces, or None."""
    match = INFOBOX_START.search(content)
    if match is None:
        return None
    index, depth = match.end(), 1
    while index < len(content):
        pair = content[index : index + 2]
        if pair == "{{":
            depth += 1
            index += 2
        elif pair == "}}":
            depth -= 1
            index += 2
            if depth == 0:
                return content[match.end() : index - 2]
        else:
            index += 1
    return None


def infobox_fields(content):
    """Top-level `name = value` parameters of the page's Infobox Object; None without one."""
    body = infobox_body(content)
    if body is None:
        return None
    fields = {}
    for part in family_capture.split_template_params(body)[1:]:
        if "=" not in part:
            continue
        name, value = part.split("=", 1)
        name = name.strip().lower()
        if name and name not in fields:
            fields[name] = value.strip()
    return fields


def item_ids(value):
    """Every distinct item id listed in an `itemid` field (comma separated); raise on junk."""
    ids = []
    for token in value.split(","):
        if not token.strip():
            continue
        match = ITEM_ID.match(token)
        if match is None:
            raise ValueError(f"itemid entry is not a positive integer: {token!r}")
        ids.append(int(match.group(1)))
    return sorted(set(ids))


def observation(title, page, fields):
    stats = {name: fields[name] for name in STAT_PARAMS if fields.get(name, "").strip()}
    return {
        "content_sha256": hashlib.sha256(page["content"].encode("utf-8")).hexdigest(),
        "fields": stats,
        "page_id": page["page_id"],
        "revision_id": page["revision_id"],
        "revision_sha1": page["sha1"],
        "revision_timestamp": page["timestamp"],
        "url": WIKI + title.replace(" ", "_"),
        "wiki_title": title,
    }


def resolvable_item_keys():
    """Item keys an `imports/` file may name: Item records and Crystal binding targets."""
    bindings = json.loads((ROOT / CRYSTAL_BINDINGS).read_text(encoding="utf-8"))
    return record_keys() | {row["target"]["key"] for row in bindings["bindings"]}


def record_key(item_id, known):
    key = f"oteryn:item.tibia.i{item_id}"
    return key if key in known else str(item_id)


def rekey(records, known):
    """The records under the key rule, ordered by item id."""
    rows = sorted(records.values(), key=lambda record: record["item_id"])
    return {record_key(record["item_id"], known): record for record in rows}


def build_records(pages, known):
    """Return ({record key: record}, report) from {title: raw page}."""
    records = {}
    report = {"pages": len(pages), "no_infobox": 0, "no_itemid": 0, "bad_itemid": []}
    for title in sorted(pages):
        page = pages[title]
        fields = infobox_fields(page["content"])
        if fields is None:
            report["no_infobox"] += 1
            continue
        raw_ids = fields.get("itemid", "")
        if not raw_ids.strip():
            report["no_itemid"] += 1
            continue
        try:
            ids = item_ids(raw_ids)
        except ValueError:
            report["bad_itemid"].append(title)
            continue
        seen = observation(title, page, fields)
        if not seen["fields"]:
            continue
        for item_id in ids:
            records.setdefault(item_id, {"item_id": item_id, "observations": []})
            records[item_id]["observations"].append(seen)
    for record in records.values():
        record["observations"].sort(key=lambda row: row["page_id"])
    records = rekey(records, known)
    report["records"] = len(records)
    report["unbound_ids"] = sum(1 for key in records if not key.startswith("oteryn:"))
    report["multi_page_ids"] = sum(
        1 for record in records.values() if len(record["observations"]) > 1
    )
    return records, report


def canonical_records_bytes(records):
    return family_capture.canonical_records_bytes(records)


def snapshot_document(records, captured_at):
    return {
        "batch_id": BATCH_ID,
        "captured_at": captured_at,
        "family": "Item",
        "records": records,
        "schema": SCHEMA,
        "snapshot_sha256": hashlib.sha256(canonical_records_bytes(records)).hexdigest(),
        "source": {
            "api": family_capture.API_URL,
            "attribution": ATTRIBUTION,
            "provider": "tibia_fandom",
            "source_key": "oteryn:source.tibiawiki",
            "stat_params": list(STAT_PARAMS),
        },
    }


def fetch_pages():
    titles = sorted(family_capture.fetch_infobox_object_titles())
    pages = {}
    for start in range(0, len(titles), FETCH_BATCH):
        data = family_capture.fetch_json(
            {
                "action": "query",
                "prop": "revisions",
                "rvprop": "content|ids|timestamp|sha1",
                "rvslots": "main",
                "titles": "|".join(titles[start : start + FETCH_BATCH]),
                "format": "json",
                "formatversion": "2",
            }
        )
        for page in data["query"]["pages"]:
            if "revisions" not in page:
                continue
            revision = page["revisions"][0]
            pages[page["title"]] = {
                "content": revision["slots"]["main"]["content"],
                "page_id": page["pageid"],
                "revision_id": revision["revid"],
                "sha1": revision.get("sha1"),
                "timestamp": revision["timestamp"],
            }
    return pages


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--cache", type=Path, help="raw page fetch to read or write")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--captured-at", help="UTC timestamp to record (default: now, or the cache's)"
    )
    parser.add_argument(
        "--rekey",
        action="store_true",
        help="re-key the committed snapshot offline instead of capturing",
    )
    args = parser.parse_args(argv)
    known = resolvable_item_keys()

    if args.rekey:
        committed = json.loads(DEFAULT_OUTPUT.read_text(encoding="utf-8"))
        records = rekey(committed["records"], known)
        report = {
            "records": len(records),
            "unbound_ids": sum(1 for key in records if not key.startswith("oteryn:")),
        }
        captured_at = committed["captured_at"]
    elif args.cache is not None and args.cache.is_file():
        cached = json.loads(args.cache.read_text(encoding="utf-8"))
        pages, captured_at = cached["pages"], cached["captured_at"]
    else:
        print("fetching every Infobox Object page...", file=sys.stderr)
        pages = fetch_pages()
        captured_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        if args.cache is not None:
            args.cache.write_text(
                json.dumps({"captured_at": captured_at, "pages": pages}),
                encoding="utf-8",
            )
    if not args.rekey:
        records, report = build_records(pages, known)
    document = snapshot_document(records, args.captured_at or captured_at)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    report["snapshot_sha256"] = document["snapshot_sha256"]
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
