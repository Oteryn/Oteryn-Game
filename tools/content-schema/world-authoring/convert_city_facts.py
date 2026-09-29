#!/usr/bin/env python3
"""Enrich the committed City Areas with the pinned English TibiaWiki city snapshot.

Reads `imports/tibiawiki/cities/fandom-snapshot-v1.json` (offline; captured by
`fandom_city_snapshot.py`), the committed City Areas and the NPC definitions, and rewrites
content/world/areas/cities/ plus `samples/cities-capture-v1.json`. Existing keys, names,
temples and CrystalServer bindings stay unchanged. Only facts that parse unambiguously become
normalized fields; the raw wiki values stay in `source_facts`. The conversion is idempotent,
so it also re-applies after `convert_world_metadata.py` regenerated the plain City records.

    python convert_city_facts.py [--snapshot PATH] [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import convert_hunting_places as hunting
import convert_world_metadata as base

ROOT = base.ROOT
SNAPSHOT = "imports/tibiawiki/cities/fandom-snapshot-v1.json"
SUMMARY = "tools/content-schema/world-authoring/samples/cities-capture-v1.json"
GENERATOR = "tools/content-schema/world-authoring/convert_city_facts.py"
SNAPSHOT_SCHEMA = "OTERYN_TIBIAWIKI_FANDOM_CITIES_SNAPSHOT/v1"
NAMESPACE = hunting.NAMESPACE
SOURCE_KEY = hunting.SOURCE_KEY
NPC_INDEX = "content/npcs/definitions/index.json"
NPC_PREFIX = "oteryn:npc."
VERSION = re.compile(r"^(Pre-)?\d{1,2}\.\d{1,3}(\.\d{1,6})?$")
ENRICHED = ("implemented", "npcs", "source_facts")


class ConvertError(base.ConvertError):
    pass


def npc_names(root: Path) -> dict[str, str]:
    """Lower-cased NPC name -> key. Definitions carry no display name, so the key's slug is
    the name with spaces written as underscores; a name matches only when that is exact."""
    names: dict[str, str] = {}
    for shard in json.loads((root / NPC_INDEX).read_text(encoding="utf-8"))["shards"]:
        for record in json.loads((root / shard).read_text(encoding="utf-8"))["records"]:
            key = record["declaration"]["identity"]["key"]
            names[key.removeprefix(NPC_PREFIX).replace("_", " ")] = key
    return names


def load_snapshot(data: bytes) -> dict:
    snapshot = json.loads(data)
    if snapshot.get("schema") != SNAPSHOT_SCHEMA:
        raise ConvertError("snapshot schema differs")
    rows = snapshot["pages"]
    titles = [row["title"] for row in rows] + [
        row["name"] for row in snapshot["unmatched"]
    ]
    ids = [row["pageid"] for row in rows]
    if not rows or len(set(ids)) != len(ids) or len(set(titles)) != len(titles):
        raise ConvertError("snapshot pages are empty or repeat a page or city")
    if any(row["pageid"] < 1 or row["revid"] < 1 for row in rows):
        raise ConvertError("snapshot page or revision id is not positive")
    return snapshot


def source(snapshot: dict, data: bytes) -> dict:
    return {
        "evidence": "Derived",
        "fetched_at": snapshot["fetched_at"],
        "generator": GENERATOR,
        "license": snapshot["license"],
        "site": snapshot["source_url"],
        "snapshot": {"path": SNAPSHOT, "sha256": hashlib.sha256(data).hexdigest()},
        "source_key": SOURCE_KEY,
    }


def enrich(record: dict, row: dict, npcs: dict[str, str], counts: dict) -> dict:
    """The record without earlier enrichment, then with the facts of snapshot row `row`."""
    declaration = {k: v for k, v in record["declaration"].items() if k not in ENRICHED}
    key = declaration["identity"]["key"]
    facts = row["facts"]
    source_facts: dict = {}
    implemented = facts.get("implemented", "").strip()
    if implemented:
        source_facts["implemented"] = implemented
        if VERSION.fullmatch(implemented):
            declaration["implemented"] = implemented
            counts["implemented"] += 1
        else:
            counts["implemented_unparsed"] += 1
    else:
        counts["implemented_absent"] += 1
    for field in ("ruler", "near"):
        text = hunting.clean(facts.get(field, ""))
        if text:
            source_facts[field] = text
    linked, unmatched = set(), []
    for name in facts["npc_names"]:
        if name.lower() in npcs:
            linked.add(npcs[name.lower()])
        else:
            unmatched.append(name)
    if linked:
        declaration["npcs"] = [base.ref("NPC", k) for k in sorted(linked)]
        counts["npcs"] += 1
    if unmatched:
        source_facts["npc_names_unmatched"] = sorted(unmatched)
    counts["npc_linked"] += len(linked)
    counts["npc_unmatched"] += len(unmatched)
    if source_facts:
        declaration["source_facts"] = source_facts
        counts["source_facts"] += 1
    bindings = [
        b for b in record["source_bindings"] if b["identity_namespace"] != NAMESPACE
    ]
    bindings.append(
        {
            "disposition": "EXACT",
            "external_id": str(row["pageid"]),
            "identity_namespace": NAMESPACE,
            "source_key": SOURCE_KEY,
            "source_revision": str(row["revid"]),
            "target": base.ref("Area", key),
        }
    )
    return {"declaration": declaration, "source_bindings": bindings}


def build(snapshot_bytes: bytes, root: Path = ROOT) -> dict[str, bytes]:
    snapshot = load_snapshot(snapshot_bytes)
    directory = base.FAMILIES["Area.City"]["dir"]
    index = json.loads((root / f"{directory}/index.json").read_text(encoding="utf-8"))
    rows = {row["title"]: row for row in snapshot["pages"]}
    unmatched = {row["name"]: row["reason"] for row in snapshot["unmatched"]}
    npcs = npc_names(root)
    counts = dict.fromkeys(
        (
            "implemented",
            "implemented_absent",
            "implemented_unparsed",
            "npc_linked",
            "npc_unmatched",
            "npcs",
            "source_facts",
        ),
        0,
    )
    out, seen = {}, set()
    for path in index["shards"]:
        shard = json.loads((root / path).read_text(encoding="utf-8"))
        records = []
        for record in shard["records"]:
            name = record["declaration"]["name"]
            seen.add(name)
            if name in rows:
                record = enrich(record, rows[name], npcs, counts)
            elif name not in unmatched:
                raise ConvertError(
                    f"{name}: City is neither in the snapshot nor listed apart"
                )
            records.append(record)
        out[path] = base.canonical({**shard, "records": records})
    if seen != set(rows) | set(unmatched) or set(rows) & set(unmatched):
        raise ConvertError("snapshot cities differ from the committed City Areas")
    pinned = source(snapshot, snapshot_bytes)
    out[f"{directory}/index.json"] = base.canonical({**index, "enrichment": pinned})
    summary = {
        "families": {"Area.City": index["record_count"]},
        "not_imported": {
            "cities_without_wiki_page": len(unmatched),
            "implemented_absent": counts["implemented_absent"],
            "implemented_unparsed": counts["implemented_unparsed"],
            "npc_names_unmatched": counts["npc_unmatched"],
        },
        "npc_names_linked": counts["npc_linked"],
        "records_with": {
            "implemented": counts["implemented"],
            "npcs": counts["npcs"],
            "source_facts": counts["source_facts"],
            "wiki_binding": len(rows),
        },
        "schema": "OTERYN_CITIES_SOURCE_CAPTURE/v1",
        "source": pinned,
    }
    out[SUMMARY] = base.canonical(summary)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--snapshot", type=Path, default=ROOT / SNAPSHOT)
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    args = parser.parse_args()
    try:
        out = build(args.snapshot.read_bytes())
    except (base.ConvertError, OSError, KeyError, json.JSONDecodeError) as error:
        print(f"FAIL {error!r}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        return 1 if stale else 0
    for path in stale:
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
