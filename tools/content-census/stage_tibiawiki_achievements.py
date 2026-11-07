#!/usr/bin/env python3
"""Stage English TibiaWiki (tibia.fandom.com) Achievement facts and join them to 15.30 staticdata.

Commands:
  capture RAW   fetch every current page of Category:Achievements into a raw snapshot (not committed)
  refetch RAW   fetch the exact revisions pinned by the committed facts into a raw snapshot
  build RAW     derive the committed facts and staticdata join report from a raw snapshot
  check [RAW]   verify the committed files; with RAW, also that RAW regenerates the facts byte-exactly

The facts keep the Infobox Achievement fields except the wiki's own prose (`spoiler`, `history`, `notes`);
from `spoiler` only the linked page titles are kept. Values stay the wiki's raw strings; the join
report lists every disagreement with staticdata and every value outside the documented grade and
point ranges. Nothing here is an Oteryn identity or gameplay definition.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

API_URL = "https://tibia.fandom.com/api.php"
USER_AGENT = "OterynAchievementCapture/1.0 (+https://github.com/Oteryn/Oteryn-Game)"
CATEGORY = "Category:Achievements"
BATCH_SIZE = 50
REQUEST_SLEEP_SECONDS = 0.3
RETRY_ATTEMPTS = 3

CAPTURE_DATE = "2026-09-29"
OUTPUT_DIR = Path("imports/tibiawiki/achievements") / CAPTURE_DATE
FACTS_FILE = "tibiawiki-achievements-facts.json"
JOIN_FILE = "staticdata-join.json"
STATICDATA = Path("imports/cipsoft-staticdata/achievements/achievements-00000-00367.json")
FACTS_SCHEMA = "OTERYN_TIBIAWIKI_ACHIEVEMENT_FACTS/v1"
JOIN_SCHEMA = "OTERYN_TIBIAWIKI_ACHIEVEMENT_STATICDATA_JOIN/v1"
AUTHORITY = "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion"
LICENSE = ("TibiaWiki (tibia.fandom.com), CC BY-SA; the TibiaWiki contributors. Wiki prose is not stored; "
           "descriptions are the game's own achievement text.")

INFOBOX = "{{Infobox Achievement"
KEPT_FIELDS = ("achievementid", "name", "actualname", "grade", "points", "secret", "premium",
               "implemented", "status", "unknown", "description", "coincideswith")
LINK_FIELDS = ("relatedpages", "spoiler")
DROPPED_FIELDS = ("history", "notes", "List", "GetValue")
KNOWN_FIELDS = frozenset(KEPT_FIELDS + LINK_FIELDS + DROPPED_FIELDS)
REQUIRED_FIELDS = ("achievementid", "name", "grade", "points", "secret", "premium")
# Grade -> point range, from the TibiaWiki "Achievements" page and the tibia.com manual.
GRADE_POINTS = {1: range(1, 4), 2: range(4, 7), 3: range(7, 10), 4: range(10, 11)}
# A trailing "?" is the wiki's mark for an id inferred for a not yet identified achievement.
ID_RE = re.compile(r"([1-9][0-9]*)(\??)")
LINK_RE = re.compile(r"\[\[([^\]|#]+)")
# Extension tags whose content may hold a literal "|" that is not a template parameter separator.
TAG_RE = re.compile(r"<(gallery|ref|nowiki)\b[^>]*>.*?</\1>", re.S)


def canonical(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=1) + "\n").encode()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fetch(params: dict) -> dict:
    query = {**params, "format": "json", "formatversion": "2"}
    request = urllib.request.Request(API_URL + "?" + urllib.parse.urlencode(query),
                                     headers={"User-Agent": USER_AGENT})
    error = None
    for _ in range(RETRY_ATTEMPTS):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except (urllib.error.URLError, TimeoutError) as exc:
            error = exc
            time.sleep(2)
    raise SystemExit(f"MediaWiki API request failed after retries: {error}")


def fetch_revisions(key: str, values: list[str]) -> list[dict]:
    pages = []
    for i in range(0, len(values), BATCH_SIZE):
        data = fetch({"action": "query", "prop": "revisions", "rvslots": "main",
                      "rvprop": "ids|timestamp|content", key: "|".join(values[i:i + BATCH_SIZE])})
        for page in data["query"]["pages"]:
            if page.get("missing") or "revisions" not in page:
                raise SystemExit(f"missing page or revision: {page.get('title')}")
            rev = page["revisions"][0]
            pages.append({"pageid": page["pageid"], "title": page["title"], "revid": rev["revid"],
                          "timestamp": rev["timestamp"], "content": rev["slots"]["main"]["content"]})
        time.sleep(REQUEST_SLEEP_SECONDS)
    return pages


def capture() -> dict:
    titles, cont = [], {}
    while True:
        data = fetch({"action": "query", "list": "categorymembers", "cmtitle": CATEGORY,
                      "cmnamespace": "0", "cmlimit": "max", **cont})
        titles += [m["title"] for m in data["query"]["categorymembers"]]
        if "continue" not in data:
            break
        cont = data["continue"]
    return {"category": CATEGORY, "pages": sorted(fetch_revisions("titles", titles), key=lambda p: p["pageid"])}


def refetch(facts: dict) -> dict:
    revids = [str(p["revid"]) for p in facts["pages"]] + [str(p["revid"]) for p in facts["skipped_pages"]]
    return {"category": CATEGORY, "pages": sorted(fetch_revisions("revids", revids), key=lambda p: p["pageid"])}


def template_body(text: str) -> str | None:
    """Return the Infobox Achievement text between its braces, or None; nesting of {{ }} and [[ ]] is tracked."""
    start = text.find(INFOBOX)
    if start < 0:
        return None
    depth, i = 0, start
    while i < len(text) - 1:
        pair = text[i:i + 2]
        if pair in ("{{", "[["):
            depth, i = depth + 1, i + 2
        elif pair in ("}}", "]]"):
            depth, i = depth - 1, i + 2
            if depth == 0:
                return text[start + 2:i - 2]
        else:
            i += 1
    raise ValueError("unterminated Infobox Achievement")


def split_params(body: str) -> dict[str, str]:
    parts, depth, current, i = [], 0, [], 0
    while i < len(body):
        tag = TAG_RE.match(body, i) if body[i] == "<" else None
        if tag:
            current.append(tag.group())
            i = tag.end()
            continue
        pair = body[i:i + 2]
        if pair in ("{{", "[[", "}}", "]]"):
            depth += 1 if pair in ("{{", "[[") else -1
            current.append(pair)
            i += 2
            continue
        if body[i] == "|" and depth == 0:
            parts.append("".join(current))
            current = []
        else:
            current.append(body[i])
        i += 1
    parts.append("".join(current))
    params: dict[str, str] = {}
    for part in parts[1:]:
        key, sep, value = part.partition("=")
        key = key.strip()
        if not sep or key not in KNOWN_FIELDS:
            raise ValueError(f"unknown infobox parameter {key!r}")
        if key in params:
            raise ValueError(f"duplicate infobox parameter {key!r}")
        params[key] = value.strip()
    return params


def links(value: str) -> list[str]:
    return sorted({m.strip().replace("_", " ") for m in LINK_RE.findall(value)})


def build_facts(raw: dict) -> dict:
    pages, skipped, seen_ids = [], [], set()
    for page in raw["pages"]:
        content = page["content"]
        record = {"pageid": page["pageid"], "title": page["title"], "revid": page["revid"],
                  "timestamp": page["timestamp"], "sha256": sha256(content.encode())}
        body = template_body(content)
        if body is None:
            skipped.append(record)
            continue
        try:
            params = split_params(body)
        except ValueError as exc:
            raise ValueError(f"{page['title']}: {exc}") from None
        fields = {k: params[k] for k in KEPT_FIELDS if params.get(k)}
        for key in LINK_FIELDS:
            if params.get(key):
                fields[f"{key}_links"] = links(params[key])
        match = ID_RE.fullmatch(fields.get("achievementid", ""))
        if match is None or match.group(1) in seen_ids:
            raise ValueError(f"{page['title']}: missing, invalid or duplicate achievementid")
        seen_ids.add(match.group(1))
        pages.append({**record, "fields": fields})
    digest = "".join(f"{p['pageid']}:{p['revid']}:{p['sha256']}\n"
                     for p in sorted(pages + skipped, key=lambda p: p["pageid"]))
    return {"schema": FACTS_SCHEMA, "authority": AUTHORITY, "license": LICENSE, "api": API_URL,
            "category": raw["category"], "captured_on": CAPTURE_DATE,
            "pages_digest": sha256(digest.encode()), "snapshot_sha256": sha256(canonical(raw)),
            "counts": {"achievements": len(pages), "skipped_pages": len(skipped),
                       "secret": sum(p["fields"].get("secret", "").lower() == "yes" for p in pages)},
            "pages": sorted(pages, key=lambda p: int(ID_RE.fullmatch(p["fields"]["achievementid"]).group(1))),
            "skipped_pages": skipped}


def anomalies(page: dict) -> list[dict]:
    fields, found = page["fields"], []

    def note(field: str, reason: str) -> None:
        found.append({"title": page["title"], "field": field, "value": fields.get(field), "reason": reason})

    for field in REQUIRED_FIELDS:
        if field not in fields:
            note(field, "missing")
    for field in ("secret", "premium"):
        if field in fields and fields[field] not in ("yes", "no"):
            note(field, "not lowercase yes/no")
    grade = int(fields["grade"]) if fields.get("grade", "").isdigit() else None
    points = int(fields["points"]) if fields.get("points", "").isdigit() else None
    if "grade" in fields and grade not in GRADE_POINTS:
        note("grade", "not a grade 1-4")
    if "points" in fields and points is None:
        note("points", "not an integer")
    if grade in GRADE_POINTS and points is not None and points not in GRADE_POINTS[grade]:
        note("points", f"outside the grade {grade} range")
    return found


def build_join(facts: dict, staticdata: dict) -> dict:
    wiki = {int(p["fields"]["achievementid"]): p for p in facts["pages"] if p["fields"]["achievementid"].isdigit()}
    uncertain = [{"achievementid": p["fields"]["achievementid"], "title": p["title"]}
                 for p in facts["pages"] if not p["fields"]["achievementid"].isdigit()]
    static = {r["source_id"]: r for r in staticdata["records"]}
    joined = []
    for source_id in sorted(static.keys() & wiki.keys()):
        record, fields = static[source_id], wiki[source_id]["fields"]
        observed = {"name": fields.get("actualname", fields.get("name")), "grade": fields.get("grade"),
                    "description": fields.get("description")}
        expected = {"name": record["name"], "grade": str(record["grade"]), "description": record["description"]}
        differences = [{"field": k, "staticdata": expected[k], "wiki": observed[k]}
                       for k in expected if expected[k] != observed[k]]
        joined.append({"source_id": source_id, "title": wiki[source_id]["title"], "differences": differences})
    return {
        "schema": JOIN_SCHEMA, "authority": AUTHORITY,
        "inputs": {"facts": f"{OUTPUT_DIR}/{FACTS_FILE}", "staticdata": str(STATICDATA)},
        "join": "wiki achievementid == staticdata source_id",
        "counts": {"staticdata": len(static), "wiki": len(wiki), "joined": len(joined),
                   "joined_with_differences": sum(bool(j["differences"]) for j in joined),
                   "wiki_only": len(wiki.keys() - static.keys()),
                   "staticdata_only": len(static.keys() - wiki.keys()), "uncertain_id": len(uncertain)},
        "joined_with_differences": [j for j in joined if j["differences"]],
        "wiki_only": [{"achievementid": i, "title": wiki[i]["title"], "secret": wiki[i]["fields"].get("secret")}
                      for i in sorted(wiki.keys() - static.keys())],
        "staticdata_only": [{"source_id": i, "name": static[i]["name"]} for i in sorted(static.keys() - wiki.keys())],
        "uncertain_id": uncertain,
        "anomalies": [a for p in facts["pages"] for a in anomalies(p)],
    }


def outputs(facts: dict, root: Path) -> dict[str, bytes]:
    staticdata = json.loads((root / STATICDATA).read_text(encoding="utf-8"))
    return {FACTS_FILE: canonical(facts), JOIN_FILE: canonical(build_join(facts, staticdata))}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=("capture", "refetch", "build", "check"))
    parser.add_argument("raw", nargs="?", type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    args = parser.parse_args(argv)
    out_dir = args.root / OUTPUT_DIR
    if args.command in ("capture", "refetch", "build") and args.raw is None:
        parser.error(f"{args.command} needs RAW")
    if args.command == "capture":
        args.raw.write_bytes(canonical(capture()))
        return 0
    if args.command == "refetch":
        facts = json.loads((out_dir / FACTS_FILE).read_text(encoding="utf-8"))
        args.raw.write_bytes(canonical(refetch(facts)))
        return 0
    if args.command == "build":
        out_dir.mkdir(parents=True, exist_ok=True)
        for name, data in outputs(build_facts(json.loads(args.raw.read_text(encoding="utf-8"))), args.root).items():
            (out_dir / name).write_bytes(data)
        return 0
    committed = {name: (out_dir / name).read_bytes() for name in (FACTS_FILE, JOIN_FILE)}
    facts = json.loads(committed[FACTS_FILE])
    if args.raw is not None:
        facts = build_facts(json.loads(args.raw.read_text(encoding="utf-8")))
    stale = [name for name, data in outputs(facts, args.root).items() if committed[name] != data]
    if stale:
        print(f"stale or tampered: {', '.join(stale)}", file=sys.stderr)
        return 1
    print(f"ok ({len(committed)} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
