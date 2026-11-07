#!/usr/bin/env python3
"""Build the Achievement catalogue in content/achievements/ (OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1 §2.2).

Inputs are the committed source observations: the 15.30 client staticdata records and the TibiaWiki facts,
plus owner_resolutions.json. name, description and grade come from staticdata where a record joins, else
from the wiki; points, secret and premium from the wiki; overrides and exclusions from the owner resolutions.
Keys are allocated here once (allocate_key); an existing catalogue record keeps its key and revision.

Usage: python build_catalogue.py [--check]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import validate_achievements as v

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
STATICDATA = (
    ROOT / "imports/cipsoft-staticdata/achievements/achievements-00000-00367.json"
)
WIKI = (
    ROOT / "imports/tibiawiki/achievements/2026-09-29/tibiawiki-achievements-facts.json"
)
RESOLUTIONS = HERE / "owner_resolutions.json"
OUT = ROOT / "content/achievements"
SHARD = 500
REVISION = "1"
CLIENT_VERSION = "15.30"


def canonical(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n").encode()


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def existing_keys() -> dict[tuple[str, int], tuple[str, str]]:
    """(provenance kind, id) -> (key, revision) of the committed catalogue, so a rebuild never re-derives a key."""
    keys = {}
    for path in sorted(OUT.glob("achievements-*.json")):
        for record in load(path)["records"]:
            prov, identity = record["provenance"], record["identity"]
            keys[("wiki", prov["tibiawiki"]["pageid"])] = (
                identity["key"],
                identity["revision"],
            )
    return keys


def build() -> list[dict]:
    static = {r["source_id"]: r for r in load(STATICDATA)["records"]}
    resolutions = load(RESOLUTIONS)
    overrides, excluded = resolutions["overrides"], resolutions["excluded"]
    kept = existing_keys()
    records, used = [], set()
    for page in load(WIKI)["pages"]:
        title, fields = page["title"], page["fields"]
        if title in excluded:
            continue
        override = overrides.get(title, {})
        source_id = int(fields["achievementid"])
        client = static.get(source_id)
        name = client["name"] if client else fields.get("actualname", fields["name"])
        record = {
            "name": name,
            "description": client["description"] if client else fields["description"],
            "grade": override.get(
                "grade", client["grade"] if client else int(fields["grade"])
            ),
            "points": override.get("points", int(fields.get("points", "-1") or -1)),
            "secret": fields["secret"] == "yes",
            "premium": override.get("premium", fields.get("premium") == "yes"),
            "provenance": {
                "tibiawiki": {"pageid": page["pageid"], "revid": page["revid"]}
            },
        }
        if override.get("retired"):
            record["retired"] = True
        if client:
            record["provenance"]["staticdata"] = {
                "source_id": source_id,
                "client_version": CLIENT_VERSION,
            }
        key, revision = kept.get(
            ("wiki", page["pageid"]), (v.allocate_key(name), REVISION)
        )
        record["identity"] = {"family": "Achievement", "key": key, "revision": revision}
        used.add(title)
        records.append(record)
    unused = (set(overrides) | set(excluded)) - used - set(excluded)
    if unused:
        raise SystemExit(f"owner resolutions name unknown pages: {sorted(unused)}")
    return sorted(records, key=lambda r: r["identity"]["key"])


def outputs(records: list[dict]) -> dict[str, bytes]:
    files = {}
    for start in range(0, len(records), SHARD):
        chunk = records[start : start + SHARD]
        name = f"achievements-{start:05d}-{start + len(chunk) - 1:05d}.json"
        files[name] = canonical({"family": "Achievement", "records": chunk})
    return files


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    records = build()
    report = v.validate(records)
    if report["invalid"]:
        print(json.dumps(report["invalid"][:10], indent=1), file=sys.stderr)
        return 1
    files = outputs(records)
    committed = {p.name: p.read_bytes() for p in OUT.glob("achievements-*.json")}
    if args.check:
        if committed != files:
            print("stale or tampered catalogue", file=sys.stderr)
            return 1
        print(f"ok ({len(records)} records, {len(files)} files)")
        return 0
    for name in set(committed) - set(files):
        (OUT / name).unlink()
    for name, data in files.items():
        (OUT / name).write_bytes(data)
    print(f"wrote {len(records)} records in {len(files)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
