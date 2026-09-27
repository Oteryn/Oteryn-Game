#!/usr/bin/env python3
"""Capture English TibiaWiki (tibia.fandom.com) family_profile evidence for every
`family_profile_unresolved` engine Item.

Network tool, run manually; not part of repository CI. For every (engine, item_id) whose
`engine_items.convert_item` blocks on `family_profile_unresolved`, this looks up the
engine's own item name on tibia.fandom.com through the MediaWiki API (`redirects=1`),
fetches the current revision of whichever page resolves, and parses its `{{Infobox
Object`/`{{Infobox Item` `primarytype`/`objectclass` fields. A disambiguation page
(`{{Disambig}}`) has every linked candidate page fetched and classified in turn. Only
`engine_items.resolve_wiki_family_value` (the checked-in admitted mapping) decides
whether a value names one real family; nothing here fuzzy-matches or guesses. A record is
only ever written when that admitted mapping resolves the match (or, for a
disambiguation, when every candidate resolves to the exact same profile); every other
outcome is a report-only entry, never a committed record. No wikitext body or image is
ever stored -- only page/revision identity, digests and the one or two structured field
observations this needs.

Two title forms are tried per engine name: the small-word title-cased form first (the
convention almost every TibiaWiki item page title actually uses), then the literal engine
name with only its first character upper-cased (MediaWiki's own first-letter
normalization) as a fallback for the rest. Generic placeholder engine names (e.g. "old
tibia item") are never looked up at all; they are always report-only.
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
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
ITEM_AUTHORING = HERE.parents[1] / "tools" / "content-schema" / "item-authoring"
sys.path.insert(0, str(ITEM_AUTHORING))

import engine_items

API_URL = "https://tibia.fandom.com/api.php"
USER_AGENT = (
    "OterynItemFamilyFallbackCapture/1.0 (+https://github.com/Oteryn/Oteryn-Game)"
)
BATCH_SIZE = 20
REQUEST_SLEEP_SECONDS = 0.3
RETRY_ATTEMPTS = 3
RETRY_SLEEP_SECONDS = 2.0

SCHEMA = engine_items.WIKI_FAMILY_FALLBACK_SCHEMA
SMALL_WORDS = {"of", "the", "a", "an", "and", "in", "on", "for", "to"}

# Names with no real, distinct item identity behind them; never looked up, always
# report-only ("generic_placeholder_name").
GENERIC_PLACEHOLDER_NAMES = {
    "weapon of carving",
    "old tibia item",
    "event item",
    "unknown item",
    "unknow",
}

INFOBOX_RE = re.compile(r"\{\{\s*Infobox[ _](Object|Item)\b", re.IGNORECASE)
FIELD_RE = re.compile(
    r"^\|[ \t]*(primarytype|objectclass)[ \t]*=[ \t]*(.*?)[ \t]*$",
    re.IGNORECASE | re.MULTILINE,
)
DISAMBIG_RE = re.compile(r"\{\{\s*Disambig\b", re.IGNORECASE)
LINK_RE = re.compile(r"\[\[([^|\]\n#]+)")
IGNORED_LINK_PREFIXES = (
    "category:",
    "file:",
    "image:",
    "special:",
    "talk:",
    "user:",
    "template:",
    "help:",
)


def title_case(name):
    words = name.split(" ")
    out = []
    for index, word in enumerate(words):
        if index > 0 and word.lower() in SMALL_WORDS:
            out.append(word.lower())
        else:
            out.append(word[:1].upper() + word[1:] if word else word)
    return " ".join(out)


def first_letter_case(name):
    return name[:1].upper() + name[1:] if name else name


def wiki_url(title):
    return "https://tibia.fandom.com/wiki/" + urllib.parse.quote(
        title.replace(" ", "_"), safe="/:()',!"
    )


def fetch_json(params):
    url = API_URL + "?" + urllib.parse.urlencode(params, safe="|")
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    last_error = None
    for _attempt in range(RETRY_ATTEMPTS):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except (urllib.error.URLError, TimeoutError) as exc:
            last_error = exc
            time.sleep(RETRY_SLEEP_SECONDS)
    raise SystemExit(f"MediaWiki API request failed after retries: {last_error}")


def parse_infobox_fields(content):
    match = INFOBOX_RE.search(content)
    if match is None:
        return None
    window = content[match.start() : match.start() + 4000]
    fields = {}
    for field_match in FIELD_RE.finditer(window):
        field_name = field_match.group(1).lower()
        field_value = field_match.group(2).strip()
        if field_name not in fields:
            fields[field_name] = field_value
    return fields


def extract_candidate_links(content):
    seen = []
    for match in LINK_RE.finditer(content):
        raw = match.group(1).strip()
        if not raw:
            continue
        if raw.lower().startswith(IGNORED_LINK_PREFIXES):
            continue
        if raw not in seen:
            seen.append(raw)
    return seen


class WikiFetcher:
    """Batches title lookups and content fetches; caches by exact title within one run."""

    def __init__(self):
        self.cache = {}

    def fetch_titles(self, titles):
        """Resolve+fetch a list of titles; return {input_title: page_or_None}.

        `page_or_None` is `None` for a missing/invalid title, else a dict with
        `page_id`, `title` (post-redirect canonical title), `revision_id`,
        `revision_timestamp`, `revision_sha1`, `content_sha256`, `content`.
        """
        to_fetch = sorted({t for t in titles if t not in self.cache})
        for start in range(0, len(to_fetch), BATCH_SIZE):
            batch = to_fetch[start : start + BATCH_SIZE]
            data = fetch_json(
                {
                    "action": "query",
                    "format": "json",
                    "formatversion": "2",
                    "redirects": "1",
                    "titles": "|".join(batch),
                    "prop": "revisions",
                    "rvprop": "content|ids|timestamp|sha1",
                    "rvslots": "main",
                }
            )
            query = data.get("query", {})
            redirects = {r["from"]: r["to"] for r in query.get("redirects", [])}
            normalized = {n["from"]: n["to"] for n in query.get("normalized", [])}
            resolved_title = {}
            for source_title in batch:
                current = normalized.get(source_title, source_title)
                current = redirects.get(current, current)
                resolved_title[source_title] = current
            by_title = {p.get("title"): p for p in query.get("pages", [])}
            for source_title in batch:
                page = by_title.get(resolved_title[source_title])
                if page is None or page.get("missing") or "revisions" not in page:
                    self.cache[source_title] = None
                    continue
                revision = page["revisions"][0]
                content = revision["slots"]["main"]["content"]
                self.cache[source_title] = {
                    "page_id": page["pageid"],
                    "title": page["title"],
                    "revision_id": revision["revid"],
                    "revision_timestamp": revision["timestamp"],
                    "revision_sha1": revision["sha1"],
                    "content_sha256": hashlib.sha256(
                        content.encode("utf-8")
                    ).hexdigest(),
                    "content": content,
                }
            time.sleep(REQUEST_SLEEP_SECONDS)
        return {t: self.cache.get(t) for t in titles}


_NO_EXISTING_WIKI_FALLBACK = Path("/dev/null/no-existing-wiki-family-fallback-snapshot")


def collect_unresolved(engine, source_root, rule_source, captured_at, report):
    """Yield (registry_key, name_lower) for every family_profile_unresolved id.

    Always run as if no wiki-evidence fallback snapshot were committed yet
    (`wiki_fallback_path` points at a path that can never exist): this discovers new
    evidence from the engine's own unresolved universe, never re-reads (and is never
    blocked re-validating) whatever snapshot a previous run already committed.
    """
    sources = engine_items.load_engine_sources(
        engine,
        source_root,
        rule_source=rule_source,
        wiki_fallback_path=_NO_EXISTING_WIKI_FALLBACK,
    )
    identity_index = sources["identity_index"]
    out = []
    for item_id in sorted(sources["items"]):
        _item, _deps, item_report = engine_items.convert_item(sources, item_id)
        if "family_profile_unresolved" not in item_report.get("blockers", ()):
            continue
        entry = identity_index.get(item_id)
        if entry is None:
            continue  # no allocator key at all; already a distinct, non-wiki blocker.
        key = entry[0]
        xml_record = sources["items"].get(item_id)
        appearance = sources["appearances"].get(item_id)
        name = (
            (xml_record.get("name") if xml_record else None)
            or (appearance.get("name") if appearance else None)
            or ""
        )
        name = name.strip()
        if not name:
            report["no_name"] += 1
            continue
        out.append((key, name.lower()))
    return out


def evidence_row(page):
    return {
        "wiki_title": page["title"],
        "page_id": page["page_id"],
        "revision_id": page["revision_id"],
        "revision_timestamp": page["revision_timestamp"],
        "url": wiki_url(page["title"]),
        "revision_sha1": page["revision_sha1"],
        "content_sha256": page["content_sha256"],
        "captured_at": None,  # filled in by the caller with the run's captured_at
    }


def resolve_direct_page(name, page, report, report_examples):
    """Resolve one already-fetched, non-disambiguation page (no network calls)."""
    fields = parse_infobox_fields(page["content"])
    if not fields:
        report["no_infobox"] += 1
        report_examples["no_infobox"].append(name)
        return None
    for field in ("primarytype", "objectclass"):
        if field not in fields:
            continue
        value = fields[field]
        profile = engine_items.resolve_wiki_family_value(field, value)
        if profile is not None:
            return {
                "matched_names": [name],
                "resolution": "direct",
                "field": field,
                "value": value,
                **evidence_row(page),
            }
        folded = value.strip().lower()
        if folded in ("fireworks", "blessing charms", "clothing accessories"):
            reason = "owner_decision_pending"
        elif not folded:
            reason = f"{field}_empty"
        else:
            reason = f"{field}_forbidden_bucket"
        report[reason] += 1
        report_examples[reason].append(f"{name} ({field}={value!r})")
        return None
    report["no_admitted_field"] += 1
    report_examples["no_admitted_field"].append(name)
    return None


def resolve_disambiguation_page(name, page, fetcher, report, report_examples):
    """Resolve one already-fetched disambiguation page using an already-warmed
    fetcher cache (candidate titles must already have been bulk-fetched)."""
    candidate_titles = extract_candidate_links(page["content"])
    if len(candidate_titles) < 2:
        # No real candidate list, or just one link: there is no cross-candidate family
        # invariant to check, so this is never admitted as a disambiguation resolution
        # (the schema and loader both require 2+ candidates for the same reason).
        report["disambiguation_no_candidates"] += 1
        report_examples["disambiguation_no_candidates"].append(name)
        return None
    candidates = []
    profiles = set()
    unresolved_candidate = False
    for candidate_title in candidate_titles:
        candidate_page = fetcher.cache.get(candidate_title)
        if candidate_page is None:
            unresolved_candidate = True
            break
        fields = parse_infobox_fields(candidate_page["content"])
        resolved = None
        if fields:
            for field in ("primarytype", "objectclass"):
                if field in fields:
                    profile = engine_items.resolve_wiki_family_value(
                        field, fields[field]
                    )
                    if profile is not None:
                        resolved = (field, fields[field], profile)
                        break
        if resolved is None:
            unresolved_candidate = True
            break
        field, value, profile = resolved
        profiles.add(profile)
        candidates.append(
            {**evidence_row(candidate_page), "field": field, "value": value}
        )
    if unresolved_candidate or len(profiles) != 1:
        reason = (
            "disambiguation_candidate_unresolved"
            if unresolved_candidate
            else "disambiguation_divergent"
        )
        report[reason] += 1
        report_examples[reason].append(name)
        return None
    return {
        "matched_names": [name],
        "resolution": "disambiguation",
        "candidates": candidates,
    }


def canonical_records_bytes(records):
    return json.dumps(
        records, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--crystal-source", type=Path, required=True)
    parser.add_argument("--canary-source", type=Path, required=True)
    parser.add_argument(
        "--output",
        type=Path,
        default=HERE.parents[1] / "imports/tibiawiki/facts/items-family-fallback.json",
    )
    parser.add_argument(
        "--report",
        type=Path,
        required=True,
        help="uncommitted JSON report of every non-admitted name, bucketed by reason",
    )
    args = parser.parse_args()

    captured_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    report = Counter()
    report_examples = defaultdict(list)

    pairs = []
    pairs += collect_unresolved(
        "crystal", args.crystal_source, None, captured_at, report
    )
    pairs += collect_unresolved(
        "canary", args.canary_source, args.crystal_source, captured_at, report
    )

    # {key: {name_lower, ...}}
    names_by_key = defaultdict(set)
    for key, name in pairs:
        names_by_key[key].add(name)
    unique_names = sorted({name for _key, name in pairs})
    print(f"unresolved (engine,id) pairs: {len(pairs)}", file=sys.stderr)
    print(f"unique engine names: {len(unique_names)}", file=sys.stderr)

    fetcher = WikiFetcher()
    lookup_names = []
    for name in unique_names:
        if name in GENERIC_PLACEHOLDER_NAMES:
            report["generic_placeholder_name"] += 1
            report_examples["generic_placeholder_name"].append(name)
        else:
            lookup_names.append(name)

    # Wave 1: bulk-resolve every name's primary (title-cased) and fallback
    # (first-letter-only) title forms in a handful of batched API calls, instead of
    # one call per name.
    primary_title = {name: title_case(name) for name in lookup_names}
    fallback_title = {name: first_letter_case(name) for name in lookup_names}
    wave1_titles = sorted(set(primary_title.values()) | set(fallback_title.values()))
    print(f"wave 1: fetching {len(wave1_titles)} titles", file=sys.stderr)
    fetcher.fetch_titles(wave1_titles)

    page_by_name = {}
    for name in lookup_names:
        page = fetcher.cache.get(primary_title[name])
        if page is None:
            page = fetcher.cache.get(fallback_title[name])
        if page is None:
            report["no_wiki_page"] += 1
            report_examples["no_wiki_page"].append(name)
            continue
        page_by_name[name] = page

    resolution_by_name = {}
    disambiguation_names = []
    for name, page in page_by_name.items():
        if DISAMBIG_RE.search(page["content"]):
            disambiguation_names.append(name)
        else:
            record = resolve_direct_page(name, page, report, report_examples)
            if record is not None:
                resolution_by_name[name] = record

    # Wave 2: bulk-resolve every disambiguation page's candidate links in one pass.
    candidate_titles_by_name = {
        name: extract_candidate_links(page_by_name[name]["content"])
        for name in disambiguation_names
    }
    wave2_titles = sorted(
        {title for titles in candidate_titles_by_name.values() for title in titles}
    )
    print(f"wave 2: fetching {len(wave2_titles)} candidate titles", file=sys.stderr)
    fetcher.fetch_titles(wave2_titles)

    for name in disambiguation_names:
        record = resolve_disambiguation_page(
            name, page_by_name[name], fetcher, report, report_examples
        )
        if record is not None:
            resolution_by_name[name] = record

    records = {}
    for key, names in sorted(names_by_key.items()):
        # A registry key can be shared by more than one engine name only when every
        # name that reaches it resolves to the exact same admitted profile; otherwise
        # the whole key stays unresolved rather than picking one name's evidence.
        resolved_names = sorted(n for n in names if n in resolution_by_name)
        if not resolved_names:
            continue
        base = resolution_by_name[resolved_names[0]]
        if len(resolved_names) > 1:
            first_shape = json.dumps(base, sort_keys=True)
            if any(
                json.dumps(resolution_by_name[n], sort_keys=True) != first_shape
                for n in resolved_names[1:]
            ):
                report["multi_name_key_divergent"] += 1
                report_examples["multi_name_key_divergent"].append(key)
                continue
        record = dict(base)
        record["matched_names"] = resolved_names
        record["registry_key"] = key
        if record["resolution"] == "direct":
            record["captured_at"] = captured_at
        else:
            record["candidates"] = [
                {**candidate, "captured_at": captured_at}
                for candidate in record["candidates"]
            ]
        records[key] = record

    snapshot_sha256 = hashlib.sha256(canonical_records_bytes(records)).hexdigest()
    snapshot = {
        "schema": SCHEMA,
        "batch_id": "g5-item-family-fallback-tibiawiki-r1",
        "family": "Item",
        "source": {
            "source_key": "oteryn:source.tibiawiki",
            "provider": "tibia_fandom",
            "api": API_URL,
        },
        "captured_at": captured_at,
        "records": records,
        "snapshot_sha256": snapshot_sha256,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(snapshot, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(
        json.dumps(
            {
                "unresolved_pairs": len(pairs),
                "unique_names": len(unique_names),
                "records_written": len(records),
                "reasons": dict(sorted(report.items())),
                "examples": {k: v[:20] for k, v in report_examples.items()},
            },
            indent=2,
            sort_keys=True,
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )
    print(
        f"wrote {len(records)} records to {args.output}; report at {args.report}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
