#!/usr/bin/env python3
"""Capture a pinned snapshot of the English TibiaWiki (Fandom) hunting-place pages.

Evidence tooling only: TibiaWiki is a player-observed reference source (CC BY-SA). Only the
raw values of a few factual infobox fields are stored, per page with its page id, exact
revision id and the SHA-256 of the wikitext, never article prose. The committed snapshot
makes `convert_hunting_places.py` reproducible offline.

    python fandom_hunting_snapshot.py fetch [--out imports/tibiawiki/hunting-places/fandom-snapshot-v1.json]

Conventions follow `tools/content-schema/npc-authoring/wiki_fandom.py` (same API, User-Agent
style, <= 2 requests/s, bounded retries), but this module is self-contained.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SNAPSHOT_PATH = "imports/tibiawiki/hunting-places/fandom-snapshot-v1.json"
SCHEMA = "OTERYN_TIBIAWIKI_FANDOM_HUNTING_PLACES_SNAPSHOT/v1"
API = "https://tibia.fandom.com/api.php"
CATEGORY = "Category:Hunting Places"
USER_AGENT = "OterynWorldAuthoring/1.0 (+https://github.com/Oteryn/Oteryn-Game)"
LICENSE_NOTE = (
    "TibiaWiki (Fandom), CC BY-SA; only raw values of a few factual infobox fields "
    "(city, recommended levels, map coordinates, creature names) are recorded, never "
    "article prose."
)
THROTTLE_SECONDS = 1.0
RETRIES = 4
BATCH = 50
LEVEL_FIELDS = ("lvlknights", "lvlpaladins", "lvlmages")
INFOBOX = re.compile(r"\{\{\s*Infobox[ _]Hunt\s*(?=[|\n}])", re.IGNORECASE)
COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)


def canonical(value) -> bytes:
    text = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return (text + "\n").encode()


# --------------------------------------------------------------------------------------------
# Wikitext parsing
# --------------------------------------------------------------------------------------------


def split_top_level(text: str, sep: str) -> list[str]:
    """Split on `sep` outside any {{ }} or [[ ]] nesting."""
    parts, depth, buffer, i = [], 0, [], 0
    while i < len(text):
        pair = text[i : i + 2]
        if pair in ("{{", "[["):
            depth += 1
            buffer.append(pair)
            i += 2
        elif pair in ("}}", "]]"):
            depth -= 1
            buffer.append(pair)
            i += 2
        elif text[i] == sep and depth == 0:
            parts.append("".join(buffer))
            buffer = []
            i += 1
        else:
            buffer.append(text[i])
            i += 1
    parts.append("".join(buffer))
    return parts


def template_body(text: str, start: int) -> str | None:
    """Body of the `{{...}}` beginning at `start` (braces stripped), or None if unbalanced."""
    depth, i = 0, start
    while i < len(text):
        pair = text[i : i + 2]
        if pair == "{{":
            depth += 1
            i += 2
        elif pair == "}}":
            depth -= 1
            i += 2
            if depth == 0:
                return text[start + 2 : i - 2]
        else:
            i += 1
    return None


def named_fields(body: str) -> dict[str, str]:
    """`key = value` parts of a template body; the first part (the name) and bare parts are skipped."""
    fields: dict[str, str] = {}
    for part in split_top_level(body, "|")[1:]:
        key, sep, value = part.partition("=")
        if sep and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_ ]*", key.strip()):
            fields.setdefault(key.strip().lower(), value.strip())
    return fields


def find_templates(text: str, name: str) -> list[list[str]]:
    """Every `{{name|...}}` call at any depth, as its `|`-separated parts after the name."""
    calls, lname, i = [], name.lower(), 0
    while i < len(text):
        if text.startswith("{{", i):
            end = i + 2
            while end < len(text) and text[end] not in "|}\n":
                end += 1
            if re.sub(r"[ _]+", " ", text[i + 2 : end]).strip().lower() == lname:
                body = template_body(text, i)
                if body is not None:
                    calls.append(split_top_level(body, "|")[1:])
                    i += 2
                    continue
        i += 1
    return calls


def raw_facts(wikitext: str) -> dict | None:
    """Raw factual field values of the page's `Infobox Hunt`, or None when it has none."""
    text = COMMENT.sub("", wikitext)
    match = INFOBOX.search(text)
    if not match:
        return None
    body = template_body(text, match.start())
    if body is None:
        return None
    fields = named_fields(body)
    facts: dict = {}
    if "city" in fields:
        facts["city"] = fields["city"]
    for name in LEVEL_FIELDS:
        if name in fields:
            facts[name] = fields[name]
    facts["location_coordinates"] = [
        "|".join(part.strip() for part in call)
        for call in find_templates(fields.get("location", ""), "Mapper Coords")
    ]
    facts["creatures"] = [
        name
        for call in find_templates(text, "CreatureList")
        for name in (part.strip() for part in call)
        if name and not re.match(r"^[A-Za-z_][A-Za-z0-9_ ]*\s*=", name)
    ]
    return facts


def page_record(page: dict) -> dict:
    """Snapshot row for one fetched page ({pageid, title, revisions:[...]})."""
    revision = page["revisions"][0]
    wikitext = revision["slots"]["main"]["content"]
    record = {
        "pageid": page["pageid"],
        "revid": revision["revid"],
        "sha256": hashlib.sha256(wikitext.encode()).hexdigest(),
        "timestamp": revision["timestamp"],
        "title": page["title"],
    }
    facts = raw_facts(wikitext)
    if facts is not None:
        record["facts"] = facts
    return record


def build_snapshot(pages: list[dict], fetched_at: str) -> dict:
    rows = sorted((page_record(page) for page in pages), key=lambda r: r["pageid"])
    return {
        "category": CATEGORY,
        "fetched_at": fetched_at,
        "license": LICENSE_NOTE,
        "pages": [r for r in rows if "facts" in r],
        "pages_without_hunt_infobox": [r for r in rows if "facts" not in r],
        "schema": SCHEMA,
        "source_url": "https://tibia.fandom.com",
    }


# --------------------------------------------------------------------------------------------
# Fandom API access
# --------------------------------------------------------------------------------------------


def api(params: dict) -> dict:
    query = urllib.parse.urlencode({**params, "format": "json", "formatversion": "2"})
    request = urllib.request.Request(
        f"{API}?{query}", headers={"User-Agent": USER_AGENT}
    )
    delay, last = 1.0, None
    for attempt in range(RETRIES):
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return json.load(response)
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            last = error
            if attempt + 1 < RETRIES:
                time.sleep(delay)
                delay *= 2
    raise RuntimeError(f"Fandom API request failed after {RETRIES} attempts: {last}")


def category_titles() -> list[str]:
    titles, params = (
        [],
        {
            "action": "query",
            "list": "categorymembers",
            "cmtitle": CATEGORY,
            "cmnamespace": 0,
            "cmlimit": 500,
        },
    )
    while True:
        data = api(params)
        titles.extend(m["title"] for m in data["query"]["categorymembers"])
        time.sleep(THROTTLE_SECONDS)
        if "continue" not in data:
            return sorted(set(titles))
        params.update(data["continue"])


def fetch_pages(titles: list[str]) -> list[dict]:
    pages = []
    for start in range(0, len(titles), BATCH):
        data = api(
            {
                "action": "query",
                "titles": "|".join(titles[start : start + BATCH]),
                "prop": "revisions",
                "rvprop": "ids|timestamp|content",
                "rvslots": "main",
            }
        )
        pages.extend(
            page
            for page in data["query"]["pages"]
            if page.get("revisions") and not page.get("missing")
        )
        time.sleep(THROTTLE_SECONDS)
    return pages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("command", choices=["fetch"])
    parser.add_argument("--out", type=Path, default=ROOT / SNAPSHOT_PATH)
    args = parser.parse_args()
    titles = category_titles()
    pages = fetch_pages(titles)
    if len(pages) != len(titles):
        print(f"FAIL fetched {len(pages)} of {len(titles)} pages", file=sys.stderr)
        return 1
    snapshot = build_snapshot(pages, datetime.now(timezone.utc).strftime("%Y-%m-%d"))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(canonical(snapshot))
    print(
        f"wrote {args.out} ({len(snapshot['pages'])} hunt pages, "
        f"{len(snapshot['pages_without_hunt_infobox'])} without infobox)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
