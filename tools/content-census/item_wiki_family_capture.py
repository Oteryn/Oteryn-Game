#!/usr/bin/env python3
"""Capture English TibiaWiki (tibia.fandom.com) family_profile evidence for every
`family_profile_unresolved` engine Item.

Network tool, run manually; not part of repository CI. For every (engine, item_id) whose
`engine_items.convert_item` blocks on `family_profile_unresolved`, this looks up the
engine's own item name on tibia.fandom.com through the MediaWiki API (`redirects=1`),
fetches the current revision of whichever page resolves, and parses its `{{Infobox
Object`/`{{Infobox Item` `primarytype`/`objectclass`/`status` fields (in that admitted
priority order; `status` is consulted only once neither `primarytype` nor `objectclass`
has resolved). A disambiguation page
(`{{Disambig}}`) has every linked candidate page fetched and classified in turn. Only
`engine_items.resolve_wiki_family_value` (the checked-in admitted mapping) decides
whether a value names one real family; nothing here fuzzy-matches or guesses. A record is
only ever written when that admitted mapping resolves the match (or, for a
disambiguation, when every candidate resolves to the exact same profile); every other
outcome is a report-only entry, never a committed record. No wikitext body or image is
ever stored -- only page/revision identity, digests and the one or two structured field
observations this needs.

Before any name matching, every unresolved (engine, item_id) is first tried against an
exact-id join (`match_basis: "itemid"`): every main-namespace page embedding `{{Infobox
Object` (via `list=embeddedin&eititle=Template:Infobox Object`) is fetched once and
indexed by the integer(s) in its own `| itemid = ...` field (comma-separated lists
included). When one or more pages list the engine's own numeric id, ONLY those pages are
ever considered for that id -- one page resolves through the admitted fields as usual;
2+ pages must all agree on the exact same profile -- and this evidence is authoritative:
if the id-matched pages exist but do not resolve (or disagree), the item stays
unresolved with no fallback to name matching. Only when no page lists the id at all does
this fall back to the pre-existing name-based join (`match_basis: "title"`) described
next.

Every engine name without an id match is resolved against an exact, case-insensitive
index of every main-namespace TibiaWiki title (`action=query&list=allpages&apnamespace=0
&apfilterredir=all`, walked to completion) instead of any capitalisation guess: the
candidate titles for a name are every title whose lower case equals the name itself, or
the name plus a `" (item)"`/`" (object)"` disambiguating suffix (TibiaWiki's own
convention for an item that shares a name with an NPC or other page), all compared
case-insensitively. Redirects are resolved (`redirects=1`) when fetching content. That is
the full extent of the fuzzing this ever does: exact case-insensitivity, redirects and the
two admitted suffixes -- nothing here title-cases or guesses. When a name's candidates
include two or more pages that each carry an admitted item/object infobox (directly, or
through a disambiguation page's own linked candidates), they must all resolve to the exact
same family_profile before any of them is admitted; a page with no item/object infobox
(e.g. an NPC page) is never itself a candidate for resolution. A disambiguation page's
candidates are read from both `[[wikilinks]]` and the positional entries of an
`{{ItemList ...}}` template (Fandom's own alternative to linking each variant),
`key=value` parameters skipped either way. Generic placeholder engine names (e.g. "old
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
ALLPAGES_BATCH_LIMIT = "max"
# TibiaWiki's own disambiguating-suffix convention for an item that shares a bare name
# with an NPC, creature or other non-item page; the only name-widening this ever does.
DISAMBIGUATING_SUFFIXES = (" (item)", " (object)")

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
    r"^\|[ \t]*(primarytype|objectclass|status|itemid)[ \t]*=[ \t]*(.*?)[ \t]*$",
    re.IGNORECASE | re.MULTILINE,
)
DISAMBIG_RE = re.compile(r"\{\{\s*Disambig\b", re.IGNORECASE)
LINK_RE = re.compile(r"\[\[([^|\]\n#]+)")
ITEMLIST_RE = re.compile(r"\{\{\s*ItemList\b(.*?)\}\}", re.IGNORECASE | re.DOTALL)
ITEMLIST_KEY_VALUE_RE = re.compile(r"^[A-Za-z_][\w ]*=")
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
# Every main-namespace page embedding this template carries `{{Infobox Object ... |
# itemid = <n>[, <n>, ...] }}`; the exact-id join (`build_itemid_index`) indexes them all.
INFOBOX_OBJECT_TEMPLATE = "Template:Infobox Object"
EMBEDDEDIN_BATCH_LIMIT = "max"
# The owner-measured page count (~9,980) for `INFOBOX_OBJECT_TEMPLATE`; content is
# fetched in larger batches than the default `WikiFetcher` batch size for this one bulk
# pass, since there is no per-name candidate-title fan-out to keep small here.
ITEMID_JOIN_FETCH_BATCH_SIZE = 50


def fetch_all_page_titles():
    """Enumerate every main-namespace title (`apnamespace=0`, redirects included via
    `apfilterredir=all`) by walking `list=allpages` continuation to completion. Returns
    the sorted list of exact titles (~93k on the live wiki as of this writing)."""
    params = {
        "action": "query",
        "format": "json",
        "formatversion": "2",
        "list": "allpages",
        "apnamespace": "0",
        "apfilterredir": "all",
        "aplimit": ALLPAGES_BATCH_LIMIT,
    }
    titles = []
    request_params = dict(params)
    while True:
        data = fetch_json(request_params)
        for page in data.get("query", {}).get("allpages", []):
            titles.append(page["title"])
        cont = data.get("continue")
        if not cont:
            break
        request_params = dict(params)
        request_params.update(cont)
        time.sleep(REQUEST_SLEEP_SECONDS)
    return sorted(titles)


def fetch_infobox_object_titles():
    """Enumerate every main-namespace page embedding `Template:Infobox Object`
    (`list=embeddedin`, walked to continuation) -- every real item/object page on the
    wiki, independent of any name (~9,980 pages as measured by the owner)."""
    params = {
        "action": "query",
        "format": "json",
        "formatversion": "2",
        "list": "embeddedin",
        "eititle": INFOBOX_OBJECT_TEMPLATE,
        "einamespace": "0",
        "eilimit": EMBEDDEDIN_BATCH_LIMIT,
    }
    titles = []
    request_params = dict(params)
    while True:
        data = fetch_json(request_params)
        for page in data.get("query", {}).get("embeddedin", []):
            titles.append(page["title"])
        cont = data.get("continue")
        if not cont:
            break
        request_params = dict(params)
        request_params.update(cont)
        time.sleep(REQUEST_SLEEP_SECONDS)
    return sorted(titles)


def parse_itemids(value):
    """Every integer in an infobox `itemid` field's raw value (comma-separated lists
    included, e.g. `37187, 37519`); TibiaWiki uses no other separator for this field."""
    return {int(match) for match in re.findall(r"\d+", value)}


def build_itemid_index(fetcher, titles):
    """{item_id: [page, ...]} from the own `itemid` field of every already-fetched
    `Infobox Object` page in `titles` (see `fetch_infobox_object_titles`); `fetcher`'s
    cache must already hold their content. No wikitext is retained beyond this call's
    return value -- only the same page/revision identity every other evidence row keeps."""
    index = defaultdict(list)
    for title in titles:
        page = fetcher.cache.get(title)
        if page is None:
            continue
        fields = parse_infobox_fields(page["content"])
        if not fields or "itemid" not in fields:
            continue
        for item_id in parse_itemids(fields["itemid"]):
            bucket = index[item_id]
            if not any(existing["page_id"] == page["page_id"] for existing in bucket):
                bucket.append(page)
    return index


def resolve_id_matched_pages(pages):
    """Resolve one item id's already-fetched, id-matched `pages` (see
    `build_itemid_index`) to a `match_basis: "itemid"` record base, or `None` when they
    do not all resolve to the exact same admitted profile -- id evidence is
    authoritative once it exists, so the caller never falls back to name matching on
    `None`. Mirrors `resolve_direct_page`/the multi-candidate branch of `resolve_name`,
    but never reports (id-join outcomes are counted separately by the caller) and never
    needs a `{{Disambig}}` page: every page here already carries its own admitted
    infobox by construction (it embeds `Template:Infobox Object`)."""
    resolved = []
    for page in pages:
        fields = parse_infobox_fields(page["content"])
        if not fields:
            return None
        profile, field, value = resolve_infobox_fields(fields)
        if profile is None:
            return None
        resolved.append((profile, field, value, page))
    if len({profile for profile, _f, _v, _p in resolved}) != 1:
        return None
    if len(resolved) == 1:
        profile, field, value, page = resolved[0]
        return {
            "resolution": "direct",
            "field": field,
            "value": value,
            "match_basis": "itemid",
            **evidence_row(page),
        }
    return {
        "resolution": "disambiguation",
        "candidates": [
            {**evidence_row(page), "field": field, "value": value}
            for _profile, field, value, page in resolved
        ],
        "match_basis": "itemid",
    }


def build_title_index(titles):
    """{lower-cased title: [exact title, ...]}; almost always a single exact title per
    lower-cased key, but the index preserves every case variant the wiki actually has."""
    index = defaultdict(list)
    for title in titles:
        index[title.lower()].append(title)
    return index


def candidate_titles_for_name(name, title_index):
    """`name` is already lower-cased and stripped. The candidate titles are every exact
    wiki title whose lower case equals `name`, or `name` plus an admitted disambiguating
    suffix (see `DISAMBIGUATING_SUFFIXES`) -- compared case-insensitively via
    `title_index`. This is the full extent of the name matching this tool ever does."""
    titles = []
    for lookup in (name, *(name + suffix for suffix in DISAMBIGUATING_SUFFIXES)):
        for title in title_index.get(lookup, ()):
            if title not in titles:
                titles.append(title)
    return titles


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


def split_template_params(body):
    """Split one `{{Template...}}` match's inner `body` on top-level `|` separators,
    respecting `[[...]]`/`{{...}}` nesting depth so a piped wikilink or a nested
    template inside one positional entry is never split apart."""
    parts = []
    current = []
    depth = 0
    i = 0
    n = len(body)
    while i < n:
        two = body[i : i + 2]
        if two in ("[[", "{{"):
            depth += 1
            current.append(two)
            i += 2
            continue
        if two in ("]]", "}}"):
            depth = max(0, depth - 1)
            current.append(two)
            i += 2
            continue
        if body[i] == "|" and depth == 0:
            parts.append("".join(current))
            current = []
            i += 1
            continue
        current.append(body[i])
        i += 1
    parts.append("".join(current))
    return parts


def extract_itemlist_titles(content):
    """Positional entries of every `{{ItemList ...}}` template (Fandom's alternative to
    linking each disambiguation variant, e.g. `{{ItemList|type=ItemList/Sorted\n |Kraken
    Buoy Lamp (Lit)\n |Kraken Buoy Lamp (Unlit)\n}}`); `key=value` parameters (the first
    entry is almost always `type=...`) are skipped, never treated as a candidate title.
    An entry may itself be a `[[wikilink]]` (its target is used) or bare text."""
    titles = []
    for match in ITEMLIST_RE.finditer(content):
        for raw in split_template_params(match.group(1)):
            text = raw.strip()
            if not text or ITEMLIST_KEY_VALUE_RE.match(text):
                continue
            link_match = LINK_RE.match(text)
            title = link_match.group(1).strip() if link_match else text
            if title and title not in titles:
                titles.append(title)
    return titles


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
    for title in extract_itemlist_titles(content):
        if title.lower().startswith(IGNORED_LINK_PREFIXES):
            continue
        if title not in seen:
            seen.append(title)
    return seen


class WikiFetcher:
    """Batches title lookups and content fetches; caches by exact title within one run."""

    def __init__(self):
        self.cache = {}

    def fetch_titles(self, titles, batch_size=BATCH_SIZE):
        """Resolve+fetch a list of titles; return {input_title: page_or_None}.

        `page_or_None` is `None` for a missing/invalid title, else a dict with
        `page_id`, `title` (post-redirect canonical title), `revision_id`,
        `revision_timestamp`, `revision_sha1`, `content_sha256`, `content`.
        """
        to_fetch = sorted({t for t in titles if t not in self.cache})
        for start in range(0, len(to_fetch), batch_size):
            batch = to_fetch[start : start + batch_size]
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
    """Yield (registry_key, item_id, name_lower) for every family_profile_unresolved id.

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
        out.append((key, item_id, name.lower()))
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


def resolve_infobox_fields(fields):
    """Try `primarytype` then `objectclass` (in that admitted order) against
    already-parsed infobox `fields`. An admitted field present with an EMPTY value falls
    through to the next admitted field instead of failing outright, so a page with no
    `primarytype` but an admitted `objectclass` (or vice versa) still resolves. `status`
    (admitting only `event` -> `event_collectible`) is the lowest-priority admitted field:
    it is tried only once neither `primarytype` nor `objectclass` has resolved, and a
    non-empty, unadmitted `primarytype`/`objectclass` value never blocks that `status`
    attempt. Returns `(profile, field, value)`; `profile` is `None` when nothing
    resolved, and `field`/`value` then describe whichever admitted field was present
    (the first with a non-empty, unresolved value, else the first with an empty value)
    for reporting -- both are `None` only when no admitted field is present at all."""
    empty = None
    for field in ("primarytype", "objectclass"):
        if field not in fields:
            continue
        value = fields[field]
        profile = engine_items.resolve_wiki_family_value(field, value)
        if profile is not None:
            return profile, field, value
        if not value.strip():
            if empty is None:
                empty = (field, value)
            continue
        primarytype_or_objectclass_failure = (field, value)
        break
    else:
        primarytype_or_objectclass_failure = empty
    status_value = fields.get("status")
    if status_value is not None:
        status_profile = engine_items.resolve_wiki_family_value("status", status_value)
        if status_profile is not None:
            return status_profile, "status", status_value
    if primarytype_or_objectclass_failure is not None:
        return (
            None,
            primarytype_or_objectclass_failure[0],
            (primarytype_or_objectclass_failure[1]),
        )
    return None, None, None


def resolve_direct_page(name, page, report, report_examples):
    """Resolve one already-fetched, non-disambiguation page (no network calls)."""
    fields = parse_infobox_fields(page["content"])
    if not fields:
        report["no_infobox"] += 1
        report_examples["no_infobox"].append(name)
        return None
    profile, field, value = resolve_infobox_fields(fields)
    if profile is not None:
        return {
            "matched_names": [name],
            "resolution": "direct",
            "field": field,
            "value": value,
            "match_basis": "title",
            **evidence_row(page),
        }
    if field is None:
        report["no_admitted_field"] += 1
        report_examples["no_admitted_field"].append(name)
        return None
    folded = value.strip().lower()
    if folded in ("fireworks", "clothing accessories"):
        reason = "owner_decision_pending"
    elif not folded:
        reason = f"{field}_empty"
    else:
        reason = f"{field}_forbidden_bucket"
    report[reason] += 1
    report_examples[reason].append(f"{name} ({field}={value!r})")
    return None


def resolve_disambig_links(page, fetcher):
    """Fetch (from an already-warmed fetcher cache) every linked candidate for one
    disambiguation page and require 2+ of them, each carrying an admitted infobox field
    that resolves via the admitted mapping, and all agreeing on the exact same profile.
    Returns `(status, payload)`: `("ok", (profile, candidates_evidence))`, or one of
    `("no_candidates", None)` / `("candidate_unresolved", None)` / `("divergent", None)`.
    """
    candidate_titles = extract_candidate_links(page["content"])
    if len(candidate_titles) < 2:
        # No real candidate list, or just one link: there is no cross-candidate family
        # invariant to check, so this is never admitted as a disambiguation resolution
        # (the schema and loader both require 2+ candidates for the same reason).
        return "no_candidates", None
    candidates = []
    profiles = set()
    for candidate_title in candidate_titles:
        candidate_page = fetcher.cache.get(candidate_title)
        if candidate_page is None:
            return "candidate_unresolved", None
        fields = parse_infobox_fields(candidate_page["content"])
        if not fields:
            return "candidate_unresolved", None
        profile, field, value = resolve_infobox_fields(fields)
        if profile is None:
            return "candidate_unresolved", None
        profiles.add(profile)
        candidates.append(
            {**evidence_row(candidate_page), "field": field, "value": value}
        )
    if len(profiles) != 1:
        return "divergent", None
    return "ok", (next(iter(profiles)), candidates)


def resolve_disambiguation_page(name, page, fetcher, report, report_examples):
    """Resolve one already-fetched disambiguation page using an already-warmed
    fetcher cache (candidate titles must already have been bulk-fetched)."""
    status, payload = resolve_disambig_links(page, fetcher)
    if status != "ok":
        reason = {
            "no_candidates": "disambiguation_no_candidates",
            "candidate_unresolved": "disambiguation_candidate_unresolved",
            "divergent": "disambiguation_divergent",
        }[status]
        report[reason] += 1
        report_examples[reason].append(name)
        return None
    _profile, candidates = payload
    return {
        "matched_names": [name],
        "resolution": "disambiguation",
        "candidates": candidates,
        "match_basis": "title",
    }


def resolve_name(name, candidate_titles, fetcher, report, report_examples):
    """Resolve one engine name from its already-fetched candidate titles (an
    already-warmed fetcher cache; disambiguation-page links must already be
    bulk-fetched too, same as `resolve_disambiguation_page`).

    Exactly one candidate carrying admitted evidence (an item/object infobox, or a
    disambiguation page) resolves exactly as it always has. Two or more such candidates
    are held to the same invariant a single disambiguation page's own links already
    enforce: every one must independently resolve, and all must agree on the exact same
    profile, or the name stays unresolved. A candidate page with neither an item/object
    infobox nor a `{{Disambig}}` template (e.g. an NPC page sharing the bare name) is
    simply not evidence and is ignored.
    """
    pages = []
    seen_page_ids = set()
    for title in candidate_titles:
        page = fetcher.cache.get(title)
        # Two candidate titles (e.g. the bare name and its " (Item)" suffix) can both be
        # redirects (or one a redirect, one canonical) to the exact same page; that is
        # one real page, not two agreeing candidates, so it is only ever counted once.
        if page is not None and page["page_id"] not in seen_page_ids:
            seen_page_ids.add(page["page_id"])
            pages.append(page)
    if not pages:
        report["no_wiki_page"] += 1
        report_examples["no_wiki_page"].append(name)
        return None

    disambig_pages = []
    infobox_pages = []  # [(page, fields), ...]
    for page in pages:
        if DISAMBIG_RE.search(page["content"]):
            disambig_pages.append(page)
            continue
        fields = parse_infobox_fields(page["content"])
        if fields:
            infobox_pages.append((page, fields))

    if not disambig_pages and not infobox_pages:
        report["no_infobox"] += 1
        report_examples["no_infobox"].append(name)
        return None

    if len(disambig_pages) + len(infobox_pages) == 1:
        if infobox_pages:
            page, _fields = infobox_pages[0]
            return resolve_direct_page(name, page, report, report_examples)
        return resolve_disambiguation_page(
            name, disambig_pages[0], fetcher, report, report_examples
        )

    profiles = set()
    evidence = []
    for page, fields in infobox_pages:
        profile, field, value = resolve_infobox_fields(fields)
        if profile is None:
            report["candidate_unresolved"] += 1
            report_examples["candidate_unresolved"].append(name)
            return None
        profiles.add(profile)
        evidence.append({**evidence_row(page), "field": field, "value": value})
    for page in disambig_pages:
        status, payload = resolve_disambig_links(page, fetcher)
        if status != "ok":
            report["candidate_unresolved"] += 1
            report_examples["candidate_unresolved"].append(name)
            return None
        profile, candidates = payload
        profiles.add(profile)
        evidence.extend(candidates)
    if len(profiles) != 1:
        report["candidate_divergent"] += 1
        report_examples["candidate_divergent"].append(name)
        return None
    return {
        "matched_names": [name],
        "resolution": "disambiguation",
        "candidates": evidence,
        "match_basis": "title",
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

    # {key: {name_lower, ...}} / {key: {item_id, ...}}
    names_by_key = defaultdict(set)
    ids_by_key = defaultdict(set)
    for key, item_id, name in pairs:
        names_by_key[key].add(name)
        ids_by_key[key].add(item_id)
    print(f"unresolved (engine,id) pairs: {len(pairs)}", file=sys.stderr)
    print(
        f"unique engine names: {len({name for _k, _i, name in pairs})}", file=sys.stderr
    )

    fetcher = WikiFetcher()

    # Exact-id join first: authoritative over name matching whenever it has an opinion
    # at all (resolved or not) -- see `resolve_id_matched_pages`.
    print("fetching the Infobox Object embeddedin page list...", file=sys.stderr)
    infobox_object_titles = fetch_infobox_object_titles()
    print(
        f"itemid join: fetching {len(infobox_object_titles)} Infobox Object pages",
        file=sys.stderr,
    )
    fetcher.fetch_titles(infobox_object_titles, batch_size=ITEMID_JOIN_FETCH_BATCH_SIZE)
    itemid_index = build_itemid_index(fetcher, infobox_object_titles)
    print(
        f"itemid join: {len(itemid_index)} distinct item ids indexed", file=sys.stderr
    )

    id_matched_records = {}  # key -> record-base-or-None
    for key, ids in ids_by_key.items():
        pages = []
        seen_page_ids = set()
        for item_id in ids:
            for page in itemid_index.get(item_id, ()):
                if page["page_id"] not in seen_page_ids:
                    seen_page_ids.add(page["page_id"])
                    pages.append(page)
        if not pages:
            continue
        base = resolve_id_matched_pages(pages)
        id_matched_records[key] = base
        if base is None:
            report["itemid_match_unresolved"] += 1
            report_examples["itemid_match_unresolved"].append(key)
        elif base["resolution"] == "direct":
            report["itemid_match_resolved_direct"] += 1
        else:
            report["itemid_match_resolved_disambiguation"] += 1
    print(
        f"itemid join: {len(id_matched_records)} registry keys matched by id "
        f"({sum(1 for v in id_matched_records.values() if v is not None)} resolved)",
        file=sys.stderr,
    )

    # Only a key with no id match at all falls back to name matching; a key whose id
    # evidence failed to resolve stays unresolved with no name fallback (the id join is
    # authoritative once it has anything to say for that id).
    unique_names = sorted(
        {name for key, _item_id, name in pairs if key not in id_matched_records}
    )
    print(f"names needing title lookup: {len(unique_names)}", file=sys.stderr)

    lookup_names = []
    for name in unique_names:
        if name in GENERIC_PLACEHOLDER_NAMES:
            report["generic_placeholder_name"] += 1
            report_examples["generic_placeholder_name"].append(name)
        else:
            lookup_names.append(name)

    print("fetching the full main-namespace title index...", file=sys.stderr)
    all_titles = fetch_all_page_titles()
    title_index = build_title_index(all_titles)
    title_index_count = len(all_titles)
    title_index_sha256 = hashlib.sha256(
        ("\n".join(all_titles) + "\n").encode("utf-8")
    ).hexdigest()
    print(f"title index: {title_index_count} titles", file=sys.stderr)

    # Wave 1: bulk-resolve every name's exact-title and " (item)"/" (object)" candidate
    # titles (see `candidate_titles_for_name`) in a handful of batched API calls.
    candidate_titles_by_name = {
        name: candidate_titles_for_name(name, title_index) for name in lookup_names
    }
    wave1_titles = sorted(
        {title for titles in candidate_titles_by_name.values() for title in titles}
    )
    print(f"wave 1: fetching {len(wave1_titles)} candidate titles", file=sys.stderr)
    fetcher.fetch_titles(wave1_titles)

    names_with_pages = []
    for name in lookup_names:
        if any(
            fetcher.cache.get(title) is not None
            for title in candidate_titles_by_name[name]
        ):
            names_with_pages.append(name)
        else:
            report["no_wiki_page"] += 1
            report_examples["no_wiki_page"].append(name)

    # Wave 2: bulk-resolve every disambiguation-page candidate's own linked candidates
    # in one pass.
    wave2_titles = set()
    for name in names_with_pages:
        for title in candidate_titles_by_name[name]:
            page = fetcher.cache.get(title)
            if page is not None and DISAMBIG_RE.search(page["content"]):
                wave2_titles.update(extract_candidate_links(page["content"]))
    wave2_titles = sorted(wave2_titles)
    print(f"wave 2: fetching {len(wave2_titles)} candidate titles", file=sys.stderr)
    fetcher.fetch_titles(wave2_titles)

    resolution_by_name = {}
    for name in names_with_pages:
        record = resolve_name(
            name, candidate_titles_by_name[name], fetcher, report, report_examples
        )
        if record is not None:
            resolution_by_name[name] = record

    records = {}
    for key, names in sorted(names_by_key.items()):
        if key in id_matched_records:
            base = id_matched_records[key]
            if base is None:
                continue  # id evidence exists but does not resolve: no name fallback.
            # The id join matches by numeric id, not by name; every one of the engine's
            # own names for this key is equally covered by that one id's evidence.
            matched_names = sorted(names)
        else:
            # A registry key can be shared by more than one engine name only when every
            # name that reaches it resolves to the exact same admitted profile;
            # otherwise the whole key stays unresolved rather than picking one name's
            # evidence.
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
            matched_names = resolved_names
        record = dict(base)
        record["matched_names"] = matched_names
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

    match_basis_counts = Counter(record["match_basis"] for record in records.values())
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(
        json.dumps(
            {
                "unresolved_pairs": len(pairs),
                "unique_names": len({name for _k, _i, name in pairs}),
                "records_written": len(records),
                "records_by_match_basis": dict(sorted(match_basis_counts.items())),
                "itemid_join": {
                    "pages_indexed": len(infobox_object_titles),
                    "distinct_item_ids": len(itemid_index),
                    "keys_with_id_match": len(id_matched_records),
                    "keys_resolved_by_id": sum(
                        1 for v in id_matched_records.values() if v is not None
                    ),
                    "keys_unresolved_by_id_no_fallback": sum(
                        1 for v in id_matched_records.values() if v is None
                    ),
                },
                "title_index": {
                    "count": title_index_count,
                    "sha256": title_index_sha256,
                },
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
