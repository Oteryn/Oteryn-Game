#!/usr/bin/env python3
'''Bounded TibiaWiki current-source collector for the protected Oteryn Item family.

Consumes the already-protected #763 full scratch crosswalk and protected B3
evidence. It does not re-import Crystal/B1, allocate identities, or promote
current-source observations into Reference gameplay truth.
'''
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
import hashlib
import json
from pathlib import Path
import re
import threading
import time
from typing import Any, Callable, Iterable
import urllib.error
import urllib.parse
import urllib.request

SCHEMA = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI/v1"
MANIFEST_SCHEMA = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI_MANIFEST/v1"
PROFILE = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI_COLLECTOR/v1"
SOURCE_ROLE = "STRUCTURED_REFERENCE_DATA"
SOURCE_ID = "TIBIAWIKI_STRUCTURED"
API_BASE = "https://www.tibiawiki.com.br/api.php"
USER_AGENT = "OterynEvidenceCollector/0.1 (+https://github.com/Oteryn/Oteryn-Game)"
TARGET_CUT = "2026-07-28"
TARGET_COUNT = 38_157

CROSSWALK_SCHEMA = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK/v1"
CROSSWALK_FULL_SHA256 = "004948eeda07afb20d5560ec583eaa2a32397f19f891a7d8749962bc32fa0f8d"
CROSSWALK_RECORDS_SHA256 = "a076fa20fa92b6474a2c1d247f04bc771d83de83027fc1b31381170325241321"
B3_PATH = "docs/agents/evidence/OTV2-20260919-content-world-cw2-b3-loot-item-bindings.json"
B3_SHA256 = "be5d523b642caf10163f6e520aeec77517bbf3d59cedbb0e373d5da87dd792d1"
B3_PRODUCT_DIGEST = "5251d211b531481f340a4093a2e70af473ea721f6b51c7109ee7e516b33b1460"

MAX_API_RESPONSE_BYTES = 4 * 1024 * 1024
MAX_WIKITEXT_BYTES = 256 * 1024
MAX_CACHE_RECORD_BYTES = 384 * 1024
MAX_INFOBOX_FIELDS = 128
MAX_FIELD_KEY_BYTES = 64
MAX_FIELD_VALUE_BYTES = 2048
MAX_UNMAPPED_FIELD_VALUE_BYTES = 16 * 1024
MAX_TITLE_BYTES = 256
MAX_BATCH_TITLES = 20
MAX_BATCH_PAGEIDS = 20
MAX_RETRIES = 4
MAX_BACKOFF_SECONDS = 8.0
MIN_REQUEST_INTERVAL_SECONDS = 0.25

DISPOSITIONS = ("WIKI_MATCHED", "WIKI_NOT_FOUND", "WIKI_AMBIGUOUS", "WIKI_CONFLICT")
COMPARABLE_FIELDS = {
    "attack": "attack",
    "defense": "defense",
    "extra_defense": "defensemod",
    "range": "range",
    "hit_chance": "hit",
    "armor": "armor",
    "charge_count": "charges",
    "capacity": "volume",
}
NORMALIZED_FIELDS = {
    "name", "aliases", "itemclass", "primarytype", "secondarytype", "weight",
    "stackable", "slottype", "hands", "vocrequired", "levelrequired", "attack",
    "defense", "defensemod", "range", "hit", "armor", "charges", "duration",
    "volume", "imbuement", "resist", "skillboost",
}


class CurrentSourceError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def bounded_text(value: Any, *, label: str, max_bytes: int = MAX_FIELD_VALUE_BYTES) -> str:
    if not isinstance(value, str):
        raise CurrentSourceError(f"{label}_NOT_STRING")
    encoded = value.encode("utf-8")
    if len(encoded) > max_bytes:
        raise CurrentSourceError(f"{label}_MAX_PLUS_ONE:{len(encoded)}")
    return value


def normalized_name(value: str) -> str:
    value = bounded_text(value, label="NAME", max_bytes=MAX_TITLE_BYTES)
    return " ".join(value.casefold().split())


_LINK_RE = re.compile(r"\[\[([^\[\]]+)\]\]")
_HTML_RE = re.compile(r"<[^>]{1,256}>")
_MULTI_SPACE_RE = re.compile(r"\s+")


def clean_wiki_text(value: str) -> str:
    value = bounded_text(value, label="FIELD_VALUE")
    def repl(match: re.Match[str]) -> str:
        return match.group(1).rsplit("|", 1)[-1].strip()
    value = _LINK_RE.sub(repl, value)
    value = _HTML_RE.sub("", value)
    value = value.replace("'''", "").replace("''", "")
    return _MULTI_SPACE_RE.sub(" ", value).strip()


def parse_int(value: str) -> int | None:
    cleaned = clean_wiki_text(value).replace(" ", "")
    return int(cleaned) if re.fullmatch(r"[+-]?\d+", cleaned) else None


def parse_decimal_string(value: str) -> str | None:
    cleaned = clean_wiki_text(value).replace(" ", "").replace(",", ".")
    if not re.fullmatch(r"[+-]?\d+(?:\.\d+)?", cleaned):
        return None
    try:
        parsed = Decimal(cleaned)
    except InvalidOperation:
        return None
    text = format(parsed.normalize(), "f")
    return "0" if text in {"-0", ""} else text


def parse_bool_pt(value: str) -> bool | None:
    cleaned = clean_wiki_text(value).casefold()
    if cleaned == "sim":
        return True
    if cleaned in {"não", "nao"}:
        return False
    return None


def extract_infobox_item(wikitext: str) -> dict[str, Any]:
    bounded_text(wikitext, label="WIKITEXT", max_bytes=MAX_WIKITEXT_BYTES)
    marker = "{{Infobox_Item"
    start = wikitext.find(marker)
    if start < 0:
        return {"mapped": {}, "unmapped": {}, "infobox_present": False}
    depth = 0
    index = start
    end: int | None = None
    while index < len(wikitext) - 1:
        pair = wikitext[index:index + 2]
        if pair == "{{":
            depth += 1
            index += 2
            continue
        if pair == "}}":
            depth -= 1
            if depth == 0:
                end = index
                break
            if depth < 0:
                raise CurrentSourceError("INFOBOX_BRACE_UNDERFLOW")
            index += 2
            continue
        index += 1
    if end is None or depth != 0:
        raise CurrentSourceError("INFOBOX_UNTERMINATED")
    body = wikitext[start + len(marker): end]
    raw: dict[str, str] = {}
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        if "=" not in stripped:
            raise CurrentSourceError("INFOBOX_FIELD_WITHOUT_EQUALS")
        key, value = stripped[1:].split("=", 1)
        key, value = key.strip().casefold(), value.strip()
        bounded_text(key, label="FIELD_KEY", max_bytes=MAX_FIELD_KEY_BYTES)
        if len(value.encode("utf-8")) > MAX_UNMAPPED_FIELD_VALUE_BYTES:
            raise CurrentSourceError(f"INFOBOX_RAW_FIELD_MAX_PLUS_ONE:{key}:{len(value.encode('utf-8'))}")
        if key in NORMALIZED_FIELDS:
            bounded_text(value, label="FIELD_VALUE")
        if not key:
            raise CurrentSourceError("INFOBOX_EMPTY_KEY")
        if key in raw and raw[key] != value:
            raise CurrentSourceError(f"INFOBOX_DUPLICATE_CONFLICT:{key}")
        raw[key] = value
        if len(raw) > MAX_INFOBOX_FIELDS:
            raise CurrentSourceError(f"INFOBOX_FIELD_COUNT_MAX_PLUS_ONE:{len(raw)}")

    mapped: dict[str, Any] = {}
    for key in sorted(raw):
        value = raw[key]
        if key not in NORMALIZED_FIELDS:
            continue
        if key == "stackable":
            parsed = parse_bool_pt(value)
            mapped[key] = {"state": "VALUE" if parsed is not None else "UNPARSED", "value": parsed if parsed is not None else clean_wiki_text(value)}
        elif key in {"levelrequired", "attack", "defense", "defensemod", "range", "hit", "armor", "charges", "volume", "imbuement"}:
            parsed = parse_int(value)
            mapped[key] = {"state": "VALUE" if parsed is not None else "UNPARSED", "value": parsed if parsed is not None else clean_wiki_text(value)}
        elif key == "weight":
            parsed = parse_decimal_string(value)
            mapped[key] = {"state": "VALUE" if parsed is not None else "UNPARSED", "value": parsed if parsed is not None else clean_wiki_text(value)}
        else:
            mapped[key] = {"state": "VALUE", "value": clean_wiki_text(value)}
    unmapped: dict[str, Any] = {}
    for key in sorted(raw):
        if key in NORMALIZED_FIELDS:
            continue
        value = raw[key]
        encoded = value.encode("utf-8")
        if len(encoded) <= MAX_FIELD_VALUE_BYTES:
            unmapped[key] = {"state": "VALUE", "value": clean_wiki_text(value)}
        else:
            unmapped[key] = {"state": "OVERSIZED_DIGEST_ONLY", "utf8_bytes": len(encoded), "sha256": sha256_bytes(encoded)}
    return {"mapped": mapped, "unmapped": unmapped, "infobox_present": True}


def validate_page_metadata(page: Any) -> dict[str, Any]:
    if not isinstance(page, dict):
        raise CurrentSourceError("API_PAGE_NOT_OBJECT")
    if page.get("missing") is True:
        return {"missing": True, "title": bounded_text(page.get("title"), label="PAGE_TITLE", max_bytes=MAX_TITLE_BYTES)}
    page_id, title, revisions = page.get("pageid"), page.get("title"), page.get("revisions")
    if not isinstance(page_id, int) or isinstance(page_id, bool) or page_id <= 0:
        raise CurrentSourceError("API_PAGE_ID_INVALID")
    bounded_text(title, label="PAGE_TITLE", max_bytes=MAX_TITLE_BYTES)
    if not isinstance(revisions, list) or len(revisions) != 1 or not isinstance(revisions[0], dict):
        raise CurrentSourceError("API_REVISION_CARDINALITY_INVALID")
    rev = revisions[0]
    revid, timestamp = rev.get("revid"), rev.get("timestamp")
    if not isinstance(revid, int) or isinstance(revid, bool) or revid <= 0:
        raise CurrentSourceError("API_REVISION_ID_INVALID")
    if not isinstance(timestamp, str) or not timestamp.endswith("Z"):
        raise CurrentSourceError("API_REVISION_TIMESTAMP_INVALID")
    return {"missing": False, "page_id": page_id, "title": title, "revision_id": revid, "revision_timestamp": timestamp}


def validate_api_query(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or value.get("batchcomplete") is not True:
        raise CurrentSourceError("API_ROOT_INVALID")
    query = value.get("query")
    if not isinstance(query, dict) or not isinstance(query.get("pages"), list):
        raise CurrentSourceError("API_QUERY_INVALID")
    return query


def cache_path(cache_dir: Path, page_id: int, revision_id: int) -> Path:
    return cache_dir / f"page-{page_id}-rev-{revision_id}.json"


def load_cached_record(cache_dir: Path, page_id: int, revision_id: int) -> dict[str, Any] | None:
    path = cache_path(cache_dir, page_id, revision_id)
    if not path.exists():
        return None
    payload = path.read_bytes()
    if len(payload) > MAX_CACHE_RECORD_BYTES:
        raise CurrentSourceError("CACHE_RECORD_MAX_PLUS_ONE")
    try:
        value = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise CurrentSourceError("CACHE_JSON_INVALID") from exc
    if not isinstance(value, dict) or value.get("page_id") != page_id or value.get("revision_id") != revision_id:
        raise CurrentSourceError("CACHE_IDENTITY_MISMATCH")
    return value


def write_cached_record(cache_dir: Path, record: dict[str, Any]) -> None:
    cache_dir.mkdir(parents=True, exist_ok=True)
    payload = canonical_bytes(record)
    if len(payload) > MAX_CACHE_RECORD_BYTES:
        raise CurrentSourceError("CACHE_RECORD_MAX_PLUS_ONE")
    path = cache_path(cache_dir, int(record["page_id"]), int(record["revision_id"]))
    tmp = path.with_suffix(".tmp")
    tmp.write_bytes(payload)
    tmp.replace(path)


class ApiClient:
    def __init__(self, *, fetch: Callable[[urllib.request.Request], tuple[bytes, dict[str, str]]] | None = None,
                 sleep: Callable[[float], None] = time.sleep,
                 monotonic: Callable[[], float] = time.monotonic,
                 min_interval: float = MIN_REQUEST_INTERVAL_SECONDS) -> None:
        self._fetch = fetch or self._default_fetch
        self._sleep, self._monotonic, self._min_interval = sleep, monotonic, min_interval
        self._last_request = 0.0
        self._lock = threading.Lock()

    @staticmethod
    def _default_fetch(request: urllib.request.Request) -> tuple[bytes, dict[str, str]]:
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                payload = response.read(MAX_API_RESPONSE_BYTES + 1)
                if len(payload) > MAX_API_RESPONSE_BYTES:
                    raise CurrentSourceError(f"API_RESPONSE_MAX_PLUS_ONE:{len(payload)}")
                return payload, {key.casefold(): value for key, value in response.headers.items()}
        except urllib.error.HTTPError as exc:
            body = exc.read(min(MAX_API_RESPONSE_BYTES, 4096))
            error = CurrentSourceError(f"HTTP_ERROR:{exc.code}:{sha256_bytes(body)}")
            setattr(error, "http_status", exc.code)
            setattr(error, "retry_after", exc.headers.get("Retry-After") if exc.headers else None)
            raise error from exc
        except urllib.error.URLError as exc:
            raise CurrentSourceError(f"URL_ERROR:{type(exc.reason).__name__}") from exc

    def _wait_rate_limit(self) -> None:
        with self._lock:
            now = self._monotonic()
            delay = self._min_interval - (now - self._last_request)
            if delay > 0:
                self._sleep(delay)
            self._last_request = self._monotonic()

    def get_json(self, params: dict[str, str]) -> dict[str, Any]:
        request = urllib.request.Request(
            f"{API_BASE}?{urllib.parse.urlencode(params)}",
            headers={"User-Agent": USER_AGENT, "Accept": "application/json"},
            method="GET",
        )
        last_error: Exception | None = None
        for attempt in range(MAX_RETRIES):
            self._wait_rate_limit()
            try:
                payload, _ = self._fetch(request)
                if len(payload) > MAX_API_RESPONSE_BYTES:
                    raise CurrentSourceError(f"API_RESPONSE_MAX_PLUS_ONE:{len(payload)}")
                value = json.loads(payload)
                if not isinstance(value, dict):
                    raise CurrentSourceError("API_JSON_ROOT_INVALID")
                return value
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise CurrentSourceError("API_JSON_INVALID") from exc
            except CurrentSourceError as exc:
                last_error = exc
                status = getattr(exc, "http_status", None)
                if status is not None and status not in {429, 500, 502, 503, 504}:
                    raise
                if attempt + 1 >= MAX_RETRIES:
                    break
                retry_after = getattr(exc, "retry_after", None)
                delay = min(float(retry_after), MAX_BACKOFF_SECONDS) if isinstance(retry_after, str) and retry_after.isdigit() else min(float(2 ** attempt), MAX_BACKOFF_SECONDS)
                self._sleep(delay)
        raise CurrentSourceError(f"API_RETRIES_EXHAUSTED:{type(last_error).__name__}") from last_error


def chunks(values: Iterable[Any], size: int) -> Iterable[list[Any]]:
    batch: list[Any] = []
    for value in values:
        batch.append(value)
        if len(batch) == size:
            yield batch
            batch = []
    if batch:
        yield batch


def alias_map(query: dict[str, Any]) -> dict[str, str]:
    aliases: dict[str, str] = {}
    for key in ("normalized", "redirects"):
        rows = query.get(key, []) or []
        if not isinstance(rows, list):
            raise CurrentSourceError(f"API_{key.upper()}_INVALID")
        for row in rows:
            if not isinstance(row, dict) or not isinstance(row.get("from"), str) or not isinstance(row.get("to"), str):
                raise CurrentSourceError(f"API_{key.upper()}_ROW_INVALID")
            aliases[row["from"]] = row["to"]
    return aliases


def resolve_alias(value: str, aliases: dict[str, str]) -> str:
    seen: set[str] = set()
    while value in aliases:
        if value in seen:
            raise CurrentSourceError("API_ALIAS_CYCLE")
        seen.add(value)
        value = aliases[value]
    return value


def fetch_metadata_for_titles(client: ApiClient, titles: list[str]) -> dict[str, dict[str, Any]]:
    if not titles or len(titles) > MAX_BATCH_TITLES:
        raise CurrentSourceError("TITLE_BATCH_INVALID")
    query = validate_api_query(client.get_json({
        "action": "query", "prop": "revisions", "titles": "|".join(titles),
        "rvprop": "ids|timestamp", "redirects": "1", "format": "json", "formatversion": "2",
    }))
    aliases = alias_map(query)
    pages_by_title = {str(meta["title"]): meta for meta in (validate_page_metadata(raw) for raw in query["pages"])}
    out: dict[str, dict[str, Any]] = {}
    for title in titles:
        resolved = resolve_alias(title, aliases)
        meta = pages_by_title.get(resolved)
        if meta is None:
            folded = normalized_name(resolved)
            candidates = [item for key, item in pages_by_title.items() if normalized_name(key) == folded]
            if len(candidates) != 1:
                raise CurrentSourceError(f"API_REQUEST_TITLE_UNMAPPED:{title}")
            meta = candidates[0]
        out[title] = dict(meta)
    return out


def fetch_content_for_pageids(client: ApiClient, pageids: list[int]) -> dict[int, str]:
    if not pageids or len(pageids) > MAX_BATCH_PAGEIDS:
        raise CurrentSourceError("PAGEID_BATCH_INVALID")
    query = validate_api_query(client.get_json({
        "action": "query", "prop": "revisions", "pageids": "|".join(str(value) for value in pageids),
        "rvprop": "ids|timestamp|content", "rvslots": "main", "format": "json", "formatversion": "2",
    }))
    out: dict[int, str] = {}
    for page in query["pages"]:
        meta = validate_page_metadata(page)
        if meta["missing"]:
            raise CurrentSourceError("CONTENT_PAGE_BECAME_MISSING")
        rev = page["revisions"][0]
        slots = rev.get("slots")
        if not isinstance(slots, dict) or not isinstance(slots.get("main"), dict):
            raise CurrentSourceError("API_SLOT_MAIN_MISSING")
        content = slots["main"].get("content")
        bounded_text(content, label="WIKITEXT", max_bytes=MAX_WIKITEXT_BYTES)
        out[int(meta["page_id"])] = content
    if set(out) != set(pageids):
        raise CurrentSourceError("CONTENT_PAGEID_PARTITION_MISMATCH")
    return out


def normalize_page(meta: dict[str, Any], wikitext: str, retrieval_timestamp: str) -> dict[str, Any]:
    encoded = wikitext.encode("utf-8")
    extracted = extract_infobox_item(wikitext)
    return {
        "source": SOURCE_ID, "source_role": SOURCE_ROLE,
        "page_id": int(meta["page_id"]), "title": meta["title"],
        "revision_id": int(meta["revision_id"]), "revision_timestamp": meta["revision_timestamp"],
        "retrieval_timestamp": retrieval_timestamp, "target_cut": TARGET_CUT,
        "target_continuity": "UNKNOWN", "source_digest": sha256_bytes(encoded),
        "normalized_fields": extracted["mapped"], "unmapped_infobox_fields": extracted["unmapped"],
        "infobox_present": extracted["infobox_present"],
    }


def collect_pages(titles: Iterable[str], *, cache_dir: Path, client: ApiClient, retrieval_timestamp: str) -> dict[str, dict[str, Any]]:
    ordered = sorted(set(titles), key=lambda value: (normalized_name(value), value))
    results: dict[str, dict[str, Any]] = {}
    for batch in chunks(ordered, MAX_BATCH_TITLES):
        metadata = fetch_metadata_for_titles(client, batch)
        need_content: list[int] = []
        cached_by_page: dict[int, dict[str, Any]] = {}
        for title in batch:
            meta = metadata[title]
            if meta["missing"]:
                results[title] = {"missing": True, "title": meta["title"]}
                continue
            cached = load_cached_record(cache_dir, int(meta["page_id"]), int(meta["revision_id"]))
            if cached is None:
                need_content.append(int(meta["page_id"]))
            else:
                cached_by_page[int(meta["page_id"])] = cached
        content_by_page: dict[int, str] = {}
        for page_batch in chunks(sorted(set(need_content)), MAX_BATCH_PAGEIDS):
            content_by_page.update(fetch_content_for_pageids(client, page_batch))
        for title in batch:
            meta = metadata[title]
            if meta["missing"]:
                continue
            page_id = int(meta["page_id"])
            if page_id in cached_by_page:
                record = dict(cached_by_page[page_id])
                record["retrieval_timestamp"] = retrieval_timestamp
            else:
                record = normalize_page(meta, content_by_page[page_id], retrieval_timestamp)
                write_cached_record(cache_dir, record)
            results[title] = record
    return results


def verify_json_file(path: Path, *, expected_sha256: str | None = None) -> dict[str, Any]:
    payload = path.read_bytes()
    if expected_sha256 is not None and sha256_bytes(payload) != expected_sha256:
        raise CurrentSourceError(f"INPUT_DIGEST_MISMATCH:{path.name}:{sha256_bytes(payload)}")
    try:
        value = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise CurrentSourceError(f"INPUT_JSON_INVALID:{path.name}") from exc
    if not isinstance(value, dict):
        raise CurrentSourceError(f"INPUT_ROOT_INVALID:{path.name}")
    return value


def verify_crosswalk(path: Path) -> dict[str, Any]:
    value = verify_json_file(path, expected_sha256=CROSSWALK_FULL_SHA256)
    if value.get("schema") != CROSSWALK_SCHEMA:
        raise CurrentSourceError("CROSSWALK_SCHEMA_MISMATCH")
    records, profiles = value.get("records"), value.get("source_profiles")
    if not isinstance(records, list) or len(records) != TARGET_COUNT or not isinstance(profiles, list):
        raise CurrentSourceError("CROSSWALK_SHAPE_INVALID")
    actual = sha256_bytes(canonical_bytes(records))
    if actual != CROSSWALK_RECORDS_SHA256:
        raise CurrentSourceError(f"CROSSWALK_RECORDS_DIGEST_MISMATCH:{actual}")
    return value


def verify_b3(game_root: Path) -> dict[str, Any]:
    value = verify_json_file(game_root / B3_PATH, expected_sha256=B3_SHA256)
    recorded = value.get("product_digest_sha256")
    copy = dict(value)
    copy.pop("product_digest_sha256", None)
    copy.pop("product_digest_scope", None)
    recomputed = sha256_bytes(canonical_bytes(copy))
    if recorded != B3_PRODUCT_DIGEST or recomputed != B3_PRODUCT_DIGEST:
        raise CurrentSourceError("B3_PRODUCT_DIGEST_MISMATCH")
    if value.get("counts", {}).get("admitted_b2_loot_rows") != 17086:
        raise CurrentSourceError("B3_COUNT_MISMATCH")
    return value


def build_discovery_seeds(b3: dict[str, Any]) -> tuple[dict[int, list[str]], dict[int, str]]:
    names: dict[int, set[str]] = defaultdict(set)
    records = b3.get("loot_bindings", {}).get("records")
    if not isinstance(records, list):
        raise CurrentSourceError("B3_BINDINGS_INVALID")
    for record in records:
        if not isinstance(record, dict):
            raise CurrentSourceError("B3_BINDING_ROW_INVALID")
        resolution, provenance = record.get("source_item_resolution"), record.get("source_item_provenance")
        if not isinstance(resolution, dict) or not isinstance(provenance, dict):
            raise CurrentSourceError("B3_BINDING_SHAPE_INVALID")
        if resolution.get("disposition") != "RESOLVED":
            continue
        source_id = resolution.get("b1_source_item_id")
        if not isinstance(source_id, int) or isinstance(source_id, bool):
            raise CurrentSourceError("B3_RESOLVED_SOURCE_ID_INVALID")
        item_name = provenance.get("item_name")
        if not isinstance(item_name, str) or not item_name.strip():
            candidate = record.get("loot_candidate")
            item_name = candidate.get("item_label") if isinstance(candidate, dict) else None
        if isinstance(item_name, str) and item_name.strip():
            bounded_text(item_name.strip(), label="DISCOVERY_NAME", max_bytes=MAX_TITLE_BYTES)
            names[source_id].add(item_name.strip())
    output, conflicts = {}, {}
    for source_id, values in names.items():
        normalized: dict[str, list[str]] = defaultdict(list)
        for value in values:
            normalized[normalized_name(value)].append(value)
        if len(normalized) > 1:
            conflicts[source_id] = "PROTECTED_DISCOVERY_SEED_CONFLICT"
        output[source_id] = sorted(values, key=lambda value: (normalized_name(value), value))
    return output, conflicts


def profile_map(crosswalk: dict[str, Any]) -> dict[str, dict[str, Any]]:
    out: dict[str, dict[str, Any]] = {}
    for profile in crosswalk["source_profiles"]:
        if not isinstance(profile, dict) or not isinstance(profile.get("profile_id"), str) or profile["profile_id"] in out:
            raise CurrentSourceError("CROSSWALK_PROFILE_INVALID")
        out[profile["profile_id"]] = profile
    return out


def simple_int_observation(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str) and re.fullmatch(r"[+-]?\d+", value.strip()):
        return int(value.strip())
    return None


def protected_comparable_signals(record: dict[str, Any], profiles: dict[str, dict[str, Any]]) -> dict[str, int]:
    profile = profiles.get(record.get("source_profile_id"))
    if profile is None or not isinstance(profile.get("candidate_observations"), list):
        raise CurrentSourceError("CROSSWALK_RECORD_PROFILE_MISSING")
    out: dict[str, int] = {}
    conflicting: set[str] = set()
    for observation in profile["candidate_observations"]:
        if not isinstance(observation, dict):
            raise CurrentSourceError("CROSSWALK_OBSERVATION_INVALID")
        wiki_field = COMPARABLE_FIELDS.get(observation.get("native_field"))
        if wiki_field is None or observation.get("nested_values") not in (None, []):
            continue
        parsed = simple_int_observation(observation.get("source_value"))
        if parsed is None:
            continue
        if wiki_field in out and out[wiki_field] != parsed:
            conflicting.add(wiki_field)
        out[wiki_field] = parsed
    for key in conflicting:
        out.pop(key, None)
    return out


def wiki_scalar(page: dict[str, Any], key: str) -> int | str | bool | None:
    fields = page.get("normalized_fields")
    value = fields.get(key) if isinstance(fields, dict) else None
    if not isinstance(value, dict) or value.get("state") != "VALUE":
        return None
    scalar = value.get("value")
    return scalar if isinstance(scalar, (int, str, bool)) and not isinstance(scalar, float) else None


def compare_candidate(protected: dict[str, Any], page: dict[str, Any], profiles: dict[str, dict[str, Any]]) -> dict[str, Any]:
    matched, contradicted = [], []
    for key, expected in sorted(protected_comparable_signals(protected, profiles).items()):
        actual = wiki_scalar(page, key)
        if actual is None:
            continue
        (matched if actual == expected else contradicted).append(key)
    return {"matched_non_name_signals": matched, "contradicted_non_name_signals": contradicted}


def classify_wiki_candidates(protected: dict[str, Any], pages: list[dict[str, Any]], profiles: dict[str, dict[str, Any]], *, discovery_seed_conflict: bool = False, had_discovery_signal: bool = True) -> dict[str, Any]:
    if discovery_seed_conflict:
        return {"disposition": "WIKI_CONFLICT", "reason": "PROTECTED_DISCOVERY_SEED_CONFLICT", "candidate_page_ids": sorted({int(p["page_id"]) for p in pages if not p.get("missing")})}
    if not had_discovery_signal:
        return {"disposition": "WIKI_AMBIGUOUS", "reason": "NO_ADMITTED_DISCOVERY_SIGNAL", "candidate_page_ids": []}
    present = [page for page in pages if not page.get("missing")]
    if not present:
        return {"disposition": "WIKI_NOT_FOUND", "reason": "ADMITTED_DISCOVERY_TITLES_NOT_FOUND", "candidate_page_ids": []}
    unique = {int(page["page_id"]): page for page in present}
    comparisons = [(unique[key], compare_candidate(protected, unique[key], profiles)) for key in sorted(unique)]
    if len(comparisons) > 1:
        if any(item["contradicted_non_name_signals"] for _, item in comparisons):
            return {"disposition": "WIKI_CONFLICT", "reason": "MULTIPLE_CANDIDATES_WITH_STABLE_CONTRADICTION", "candidate_page_ids": [int(page["page_id"]) for page, _ in comparisons]}
        return {"disposition": "WIKI_AMBIGUOUS", "reason": "MULTIPLE_PLAUSIBLE_CANDIDATES", "candidate_page_ids": [int(page["page_id"]) for page, _ in comparisons]}
    page, comparison = comparisons[0]
    if comparison["contradicted_non_name_signals"]:
        return {"disposition": "WIKI_CONFLICT", "reason": "STABLE_SIGNAL_CONTRADICTION", "candidate_page_ids": [int(page["page_id"])], **comparison}
    if comparison["matched_non_name_signals"]:
        return {"disposition": "WIKI_MATCHED", "reason": "NAME_DISCOVERY_PLUS_NON_NAME_CORROBORATION", "candidate_page_ids": [int(page["page_id"])], **comparison}
    return {"disposition": "WIKI_AMBIGUOUS", "reason": "NAME_ONLY_INSUFFICIENT", "candidate_page_ids": [int(page["page_id"])], **comparison}


def compile_current_source(crosswalk: dict[str, Any], seeds: dict[int, list[str]], seed_conflicts: dict[int, str], pages_by_seed: dict[str, dict[str, Any]], *, retrieval_timestamp: str) -> dict[str, Any]:
    profiles = profile_map(crosswalk)
    output, counts, reason_counts, page_records = [], Counter(), Counter(), {}
    for record in crosswalk["records"]:
        source_id, native_key = record.get("source_item_id"), record.get("native_key")
        if not isinstance(source_id, int) or not isinstance(native_key, str):
            raise CurrentSourceError("CROSSWALK_RECORD_IDENTITY_INVALID")
        seed_names = seeds.get(source_id, [])
        candidate_pages = [pages_by_seed[name] for name in seed_names if name in pages_by_seed]
        result = classify_wiki_candidates(record, candidate_pages, profiles, discovery_seed_conflict=source_id in seed_conflicts, had_discovery_signal=bool(seed_names))
        if result["disposition"] not in DISPOSITIONS:
            raise CurrentSourceError("CURRENT_SOURCE_DISPOSITION_INVALID")
        counts[result["disposition"]] += 1
        reason_counts[result["reason"]] += 1
        for page in candidate_pages:
            if not page.get("missing"):
                page_records[int(page["page_id"])] = page
        output.append({"source_item_id": source_id, "native_key": native_key, "source_profile_id": record.get("source_profile_id"), "discovery_names": seed_names, "current_source": result})
    if len(output) != TARGET_COUNT or sum(counts.values()) != TARGET_COUNT:
        raise CurrentSourceError("CURRENT_SOURCE_PARTITION_FAILED")
    stable_pages = []
    for page_id in sorted(page_records):
        page = dict(page_records[page_id])
        page.pop("retrieval_timestamp", None)
        stable_pages.append(page)
    return {
        "schema": SCHEMA, "collector_profile": PROFILE,
        "source": {"id": SOURCE_ID, "role": SOURCE_ROLE, "api": API_BASE},
        "target_cut": TARGET_CUT, "retrieval_timestamp": retrieval_timestamp,
        "authority": {"gameplay_truth": "NONE", "identity_minting": "FORBIDDEN", "source_reimport": "FORBIDDEN", "semantic_promotion": "FORBIDDEN"},
        "records": output, "pages": stable_pages,
        "counts": {"records": len(output), "unique_discovery_names": len(pages_by_seed), "unique_page_records": len(stable_pages), "dispositions": {name: counts[name] for name in DISPOSITIONS}, "reasons": dict(sorted(reason_counts.items()))},
    }


def build_manifest(full: dict[str, Any], *, collector_sha256: str) -> dict[str, Any]:
    if full.get("schema") != SCHEMA or not isinstance(full.get("pages"), list):
        raise CurrentSourceError("FULL_SCHEMA_INVALID")
    if any("content" in page or "wikitext" in page for page in full["pages"] if isinstance(page, dict)):
        raise CurrentSourceError("RAW_WIKITEXT_IN_FULL_OUTPUT")
    stable_full = dict(full)
    retrieval_timestamp = stable_full.pop("retrieval_timestamp", None)
    return {
        "schema": MANIFEST_SCHEMA, "status": "CURRENT_SOURCE_EVIDENCE_ONLY_NO_PROMOTION",
        "collector": {"profile": PROFILE, "path": "tools/reference-world-corridor-census/item_current_source_tibiawiki.py", "sha256": collector_sha256},
        "protected_inputs": {
            "classification_crosswalk": {"schema": CROSSWALK_SCHEMA, "full_scratch_sha256": CROSSWALK_FULL_SHA256, "records_sha256": CROSSWALK_RECORDS_SHA256},
            "b3_loot_item_bindings": {"path": B3_PATH, "sha256": B3_SHA256, "product_digest_sha256": B3_PRODUCT_DIGEST},
        },
        "source": full["source"], "target_cut": TARGET_CUT, "retrieval_timestamp": retrieval_timestamp,
        "counts": full["counts"],
        "full_output": {"schema": SCHEMA, "sha256": sha256_bytes(canonical_bytes(full)), "stable_without_retrieval_timestamp_sha256": sha256_bytes(canonical_bytes(stable_full)), "committed_bulk_corpus": False},
        "invariants": {"records_exact_38157": full["counts"]["records"] == TARGET_COUNT, "name_only_never_matches": True, "missing_field_never_false_or_zero": True, "post_target_revision_not_auto_promoted": True, "raw_wikitext_committed": False, "crystal_b1_reimport_performed": False, "identity_regeneration_performed": False, "semantic_promotion_performed": False},
        "limitations": [
            "Discovery coverage is bounded to already-protected source-name evidence; records without an admitted discovery signal remain explicit residuals.",
            "TibiaWiki current-source observations are structured evidence, not Reference gameplay truth.",
            "Target-cut continuity is UNKNOWN unless separately proven by stronger evidence.",
            "Only explicitly comparable simple B1 candidate fields are used as non-name identity corroborators; unsupported unit/vocabulary mappings are not guessed.",
        ],
        "next_gate": "FIELD_VERIFICATION_AND_CONTINUITY_RULES",
    }


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--classification-crosswalk", type=Path, required=True)
    parser.add_argument("--cache-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    parser.add_argument("--retrieval-timestamp", default=None)
    args = parser.parse_args()
    retrieval_timestamp = args.retrieval_timestamp or utc_now_iso()
    crosswalk = verify_crosswalk(args.classification_crosswalk)
    b3 = verify_b3(args.game_root)
    seeds, seed_conflicts = build_discovery_seeds(b3)
    titles = sorted({name for values in seeds.values() for name in values}, key=lambda value: (normalized_name(value), value))
    pages = collect_pages(titles, cache_dir=args.cache_dir, client=ApiClient(), retrieval_timestamp=retrieval_timestamp)
    full = compile_current_source(crosswalk, seeds, seed_conflicts, pages, retrieval_timestamp=retrieval_timestamp)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(full))
    collector_payload = Path(__file__).read_bytes().replace(b"\r\n", b"\n")
    manifest = build_manifest(full, collector_sha256=sha256_bytes(collector_payload))
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    counts = manifest["counts"]["dispositions"]
    print("item-current-source-tibiawiki: PASS " f"records={manifest['counts']['records']} " f"matched={counts['WIKI_MATCHED']} " f"not_found={counts['WIKI_NOT_FOUND']} " f"ambiguous={counts['WIKI_AMBIGUOUS']} " f"conflict={counts['WIKI_CONFLICT']} " f"digest={manifest['full_output']['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
