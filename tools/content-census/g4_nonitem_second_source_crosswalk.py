#!/usr/bin/env python3
"""Capture a bounded independent Fandom cross-check for the sealed G4 cohort.

The #875 artifact is the immutable primary-source cohort. This tool retains
only its source tuples and compact allowlisted facts, plus page/revision
provenance and short facts from TibiaWiki/Fandom. Raw page bodies are hashed
and discarded. No canonical identity, binding, population, or field promotion
is performed.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import html
import hashlib
import json
import os
import re
import time
import unicodedata
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from collections import defaultdict
from pathlib import Path
from typing import Any

SCHEMA = "OTERYN_G4_NONITEM_SECOND_SOURCE_CROSSWALK/v1"
MANIFEST_SCHEMA = "OTERYN_G4_NONITEM_SECOND_SOURCE_CROSSWALK_MANIFEST/v1"
REPOSITORY = "Oteryn/Oteryn-Game"
BASE_SHA = "bfc8b54548a09c59e062873a5dfa48739c477420"
BRANCH = "agent/otv2-g4-nonitem-second-source-crosswalk"
TASK_ID = "OTV2-20260925-g4-nonitem-second-source-crosswalk"
FANDOM_API = "https://tibia.fandom.com/api.php"
FANDOM_LICENSE = "https://www.fandom.com/licensing"
FANDOM_ABOUT = "https://tibia.fandom.com/wiki/TibiaWiki:About"
PRIMARY_ARTIFACT_ID = 10848111721
PRIMARY_ARTIFACT_SHA256 = "0e340f9e77987f4dea3f5ca77f875d3f36dbfc7a2b08822b0c55716e01e5c293"
PRIMARY_OUTPUT_SHA256 = "38d827ba66bb7a04a3f5a94bbb873ca4485d4b7c873de957812a9c1be20a67c5"
MAX_ZIP_BYTES = 8 * 1024 * 1024
MAX_MEMBER_BYTES = 16 * 1024 * 1024
MAX_HTTP_BYTES = 20 * 1024 * 1024
MAX_PAGE_BYTES = 16 * 1024 * 1024
MAX_TOTAL_FANDOM_PAGES = 20_000
MAX_PAGES_PER_FAMILY = 10_000
BATCH_SIZE = 10
FAMILY_COUNTS = {"Creature": 2149, "NPC": 1253, "Achievement": 569, "Quest": 272, "Ability": 171, "Outfit": 134, "Mount": 252}
FANDOM_TEMPLATES = {
    "Creature": "Template:Infobox Creature", "NPC": "Template:Infobox NPC",
    "Achievement": "Template:Infobox Achievement", "Quest": "Template:Infobox Quest",
    "Ability": "Template:Infobox Spell", "Outfit": "Template:Infobox Outfit",
    "Mount": "Template:Infobox Mount",
}
SAFE_KEYS = {
    "Creature": {"name", "health", "hp", "hitpoints", "experience", "exp", "speed", "armor", "armour", "summonable", "convinceable", "pushable", "paralyzable", "paralyzeable", "immunities", "resistances"},
    "NPC": {"name", "occupation", "profession", "job", "race", "gender"},
    "Achievement": {"name", "points", "grade", "secret", "premium"},
    "Quest": {"name", "level", "requiredlevel", "premium", "repeatable", "type"},
    "Ability": {"name", "level", "requiredlevel", "mana", "manacost", "vocation", "vocations", "premium", "cooldown", "groupcooldown", "range", "type"},
    "Outfit": {"name", "gender", "addons", "premium", "race"},
    "Mount": {"name", "speed", "speedbonus", "premium", "tamingitem"},
}
ALIASES = {
    "hp": "health", "hitpoints": "health", "exp": "experience", "armour": "armor",
    "profession": "occupation", "job": "occupation", "requiredlevel": "level",
    "manacost": "mana", "vocations": "vocation", "groupcooldown": "cooldown",
    "speedbonus": "speed",
}


class CaptureError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _origin(url: str) -> tuple[str, str, int | None]:
    parsed = urllib.parse.urlparse(url)
    if parsed.scheme.casefold() != "https" or not parsed.hostname or parsed.username or parsed.password:
        raise CaptureError("HTTPS_ORIGIN_INVALID")
    return parsed.scheme.casefold(), parsed.hostname.casefold(), parsed.port


def _http_get(url: str, *, token: str | None = None, max_bytes: int = MAX_HTTP_BYTES) -> tuple[bytes, dict[str, str], str]:
    headers = {"User-Agent": "Oteryn-G4-SecondSourceCrosswalk/1.0 (evidence-only)", "Accept": "application/json,text/html,*/*"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
        headers["X-GitHub-Api-Version"] = "2022-11-28"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=30) as response:
        body = response.read(max_bytes + 1)
        if len(body) > max_bytes:
            raise CaptureError("HTTP_RESPONSE_MAX_PLUS_ONE")
        return body, {k.casefold(): v for k, v in response.headers.items()}, response.geturl()


def _request_receipt(url: str, *, token: str | None = None, max_bytes: int = MAX_HTTP_BYTES) -> tuple[dict[str, Any], bytes | None]:
    try:
        body, headers, final_url = _http_get(url, token=token, max_bytes=max_bytes)
        receipt = {"url": url, "final_url": final_url, "state": "HTTP_200", "http_status": 200, "response_bytes": len(body), "response_sha256": sha256(body), "etag": headers.get("etag"), "last_modified": headers.get("last-modified"), "rate_limit_headers": {k: v for k, v in headers.items() if "rate" in k or k == "retry-after"}}
        if _origin(url) != _origin(final_url):
            receipt["state"] = "REDIRECT_ORIGIN_MISMATCH"
            receipt["redirect_origin_mismatch"] = True
            return receipt, None
        receipt["redirect_origin_mismatch"] = False
        return receipt, body
    except urllib.error.HTTPError as exc:
        retry_after = exc.headers.get("Retry-After") if exc.headers else None
        return ({"url": url, "state": f"HTTP_{exc.code}", "http_status": exc.code, "retry_after": retry_after, "response_sha256": None, "response_bytes": None}, None)
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        reason = getattr(exc, "reason", None)
        reason_text = str(reason if reason is not None else exc)[:256]
        return ({"url": url, "state": "NETWORK_UNAVAILABLE", "error_class": type(exc).__name__, "error_reason": reason_text, "response_sha256": None, "response_bytes": None}, None)
    except CaptureError as exc:
        return ({"url": url, "state": str(exc), "response_sha256": None, "response_bytes": None}, None)


def _api_url(params: dict[str, str]) -> str:
    return FANDOM_API + "?" + urllib.parse.urlencode(params)


def _license_marker_status(project_body: bytes | None, license_body: bytes | None) -> dict[str, bool]:
    def text_markers(body: bytes | None) -> tuple[bool, bool]:
        if body is None:
            return False, False
        text = html.unescape(body.decode("utf-8", errors="replace"))
        text = re.sub(r"<script\b[^>]*>.*?</script\s*>|<style\b[^>]*>.*?</style\s*>", " ", text, flags=re.I | re.S)
        text = re.sub(r"<[^>]{0,1024}>", " ", text)
        text = re.sub(r"\s+", " ", text).casefold()
        license_marker = any(marker in text for marker in ("creative commons attribution-share alike", "creative commons attribution share alike", "cc by-sa", "cc-by-sa", "cc by sa"))
        attribution_marker = "attribution" in text and ("license" in text or "licence" in text or license_marker)
        return license_marker, attribution_marker

    project_license, project_attribution = text_markers(project_body)
    site_license, site_attribution = text_markers(license_body)
    return {
        "project_cc_by_sa_marker": project_license,
        "project_attribution_marker": project_attribution,
        "site_cc_by_sa_marker": site_license,
        "site_attribution_marker": site_attribution,
        "verified": all((project_license, project_attribution, site_license, site_attribution)),
    }


def preflight_fandom() -> dict[str, Any]:
    tests = {
        "siteinfo": _api_url({"action": "query", "meta": "siteinfo", "siprop": "general|statistics", "format": "json", "formatversion": "2"}),
        "api_help": _api_url({"action": "help", "modules": "query+revisions", "format": "json"}),
        "enumeration_probe": _api_url({"action": "query", "list": "embeddedin", "eititle": "Template:Infobox Creature", "eilimit": "1", "format": "json", "formatversion": "2"}),
        "project_terms": FANDOM_ABOUT,
        "license_terms": FANDOM_LICENSE,
    }
    receipts: dict[str, Any] = {}
    payloads: dict[str, bytes] = {}
    for name, url in tests.items():
        receipt, body = _request_receipt(url, max_bytes=512 * 1024)
        receipts[name] = receipt
        if body is not None:
            payloads[name] = body
        # One small serial probe per endpoint; do not retry on 429/5xx.
        time.sleep(0.05)
    api_ok = all(receipts[name].get("state") == "HTTP_200" for name in ("siteinfo", "api_help", "enumeration_probe"))
    marker_status = _license_marker_status(payloads.get("project_terms"), payloads.get("license_terms"))
    terms_http_ok = all(receipts[name].get("state") == "HTTP_200" for name in ("project_terms", "license_terms"))
    receipts["license_marker_checks"] = marker_status
    terms_ok = terms_http_ok and marker_status["verified"]
    parsed_ok = True
    for name in ("siteinfo", "enumeration_probe"):
        if name in payloads:
            try:
                obj = json.loads(payloads[name])
                if not isinstance(obj, dict) or "query" not in obj:
                    parsed_ok = False
            except (json.JSONDecodeError, UnicodeDecodeError):
                parsed_ok = False
    if not parsed_ok:
        receipts["response_shape"] = {"state": "MALFORMED_API_RESPONSE"}
    return {"state": "ACCESSIBLE" if api_ok and terms_ok and parsed_ok else "SOURCE_UNAVAILABLE", "api_available": api_ok and parsed_ok, "terms_http_ok": terms_http_ok, "terms_verified": terms_ok, "receipts": receipts, "rate_limit_policy": "one serial request per family/page batch; 100ms pacing; no automatic retries; retain 429 and Retry-After as exact source state", "rate_limit_headers_observed": any(bool(r.get("rate_limit_headers")) for r in receipts.values())}


def fetch_primary_artifact() -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        raise CaptureError("GITHUB_TOKEN_REQUIRED_FOR_PROTECTED_875_ARTIFACT")
    url = f"https://api.github.com/repos/{REPOSITORY}/actions/artifacts/{PRIMARY_ARTIFACT_ID}/zip"
    # Keep the repository token on api.github.com only. Follow GitHub's
    # temporary artifact redirect without forwarding Authorization.
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            return None
    request = urllib.request.Request(url, headers={"User-Agent": "Oteryn-G4-SecondSourceCrosswalk/1.0", "Accept": "application/vnd.github+json", "Authorization": f"Bearer {token}", "X-GitHub-Api-Version": "2022-11-28"})
    try:
        with urllib.request.build_opener(NoRedirect).open(request, timeout=30) as response:
            if response.status != 200: raise CaptureError("PROTECTED_875_ARTIFACT_HTTP_STATUS_INVALID")
            archive = response.read(MAX_ZIP_BYTES + 1)
    except urllib.error.HTTPError as exc:
        if exc.code not in (302, 307, 308): raise CaptureError(f"PROTECTED_875_ARTIFACT_HTTP_{exc.code}") from exc
        signed_url = exc.headers.get("Location", "")
        parsed = urllib.parse.urlparse(signed_url)
        host = (parsed.hostname or "").casefold()
        allowed_host = host == "release-assets.githubusercontent.com" or host.endswith(".blob.core.windows.net") or host.endswith(".amazonaws.com")
        if parsed.scheme != "https" or not allowed_host or parsed.username or parsed.password:
            raise CaptureError("PROTECTED_875_ARTIFACT_REDIRECT_HOST_INVALID") from exc
        archive, _, final_url = _http_get(signed_url, max_bytes=MAX_ZIP_BYTES)
        final_host = (urllib.parse.urlparse(final_url).hostname or "").casefold()
        if final_host != host and not final_host.endswith(".blob.core.windows.net") and not final_host.endswith(".amazonaws.com"):
            raise CaptureError("PROTECTED_875_ARTIFACT_FINAL_REDIRECT_HOST_INVALID")
    if len(archive) > MAX_ZIP_BYTES: raise CaptureError("PROTECTED_875_ARTIFACT_MAX_PLUS_ONE")
    archive_sha = sha256(archive)
    if archive_sha != PRIMARY_ARTIFACT_SHA256:
        raise CaptureError("PROTECTED_875_ARTIFACT_ZIP_DIGEST_MISMATCH")
    import io
    try:
        with zipfile.ZipFile(io.BytesIO(archive)) as zf:
            names = sorted(zf.namelist())
            if names != ["bulk-crosswalk.json", "manifest.json"]:
                raise CaptureError("PROTECTED_875_ARTIFACT_MEMBER_SET_INVALID")
            bulk_bytes = zf.read("bulk-crosswalk.json")
            manifest_bytes = zf.read("manifest.json")
    except (zipfile.BadZipFile, KeyError) as exc:
        raise CaptureError("PROTECTED_875_ARTIFACT_ZIP_INVALID") from exc
    if len(bulk_bytes) > MAX_MEMBER_BYTES or len(manifest_bytes) > 64 * 1024:
        raise CaptureError("PROTECTED_875_ARTIFACT_MEMBER_MAX_PLUS_ONE")
    try:
        artifact, manifest = json.loads(bulk_bytes), json.loads(manifest_bytes)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise CaptureError("PROTECTED_875_ARTIFACT_JSON_INVALID") from exc
    if not isinstance(artifact, dict) or not isinstance(manifest, dict):
        raise CaptureError("PROTECTED_875_ARTIFACT_ROOT_INVALID")
    if artifact.get("schema") != "OTERYN_G4_NONITEM_BULK_CROSSWALK/v1" or manifest.get("schema") != "OTERYN_G4_NONITEM_BULK_CROSSWALK_MANIFEST/v1":
        raise CaptureError("PROTECTED_875_ARTIFACT_SCHEMA_MISMATCH")
    if manifest.get("artifact_sha256") != PRIMARY_OUTPUT_SHA256 or sha256(canonical_bytes(artifact)) != PRIMARY_OUTPUT_SHA256:
        raise CaptureError("PROTECTED_875_OUTPUT_DIGEST_MISMATCH")
    if manifest.get("counts", {}).get("direct_rows") != 4800 or manifest.get("counts", {}).get("duplicate_or_dropped_page_ids") != 0:
        raise CaptureError("PROTECTED_875_COHORT_COUNT_INVALID")
    if manifest.get("invariants", {}).get("no_title_only_identity_or_crosswalk") is not True:
        raise CaptureError("PROTECTED_875_TITLE_ONLY_INVARIANT_MISSING")
    for key in ("source_identity_binding_emitted", "production_key_minted", "canonical_definition_population", "semantic_field_promotion"):
        if artifact.get("authority", {}).get(key) is not False:
            raise CaptureError(f"PROTECTED_875_AUTHORITY_BOUNDARY_INVALID:{key}")
    rows: list[dict[str, Any]] = []
    families = artifact.get("families")
    if not isinstance(families, dict) or set(families) != set(FAMILY_COUNTS):
        raise CaptureError("PROTECTED_875_FAMILY_SET_INVALID")
    seen: set[str] = set()
    for family, count in FAMILY_COUNTS.items():
        family_rows = families.get(family)
        if not isinstance(family_rows, list) or len(family_rows) != count or manifest.get("families", {}).get(family, {}).get("rows") != count:
            raise CaptureError(f"PROTECTED_875_FAMILY_COUNT_INVALID:{family}")
        for row in family_rows:
            if not isinstance(row, dict) or row.get("family") != family:
                raise CaptureError(f"PROTECTED_875_ROW_INVALID:{family}")
            external_id = row.get("external_id")
            if not isinstance(external_id, str) or not re.fullmatch(r"[1-9][0-9]*", external_id) or external_id in seen:
                raise CaptureError("PROTECTED_875_SOURCE_ID_DUPLICATE_OR_INVALID")
            seen.add(external_id)
            required = {"source", "source_namespace", "identity_namespace", "external_id", "page_key", "g3_title_observation", "current_title", "g3_revision_id", "g3_revision_timestamp", "g4_revision_id", "g4_revision_timestamp", "current_revision_id", "current_revision_timestamp", "current_raw_utf8_sha256", "source_state", "structured_fields"}
            if not required.issubset(row) or row.get("source") != "TIBIAWIKI_STRUCTURED" or row.get("source_namespace") != "mediawiki/tibiawiki.com.br":
                raise CaptureError("PROTECTED_875_SOURCE_TUPLE_INVALID")
            if row.get("page_key") != f"mediawiki/tibiawiki.com.br/page_id/{external_id}" or row.get("identity_namespace") != "mediawiki/page_id":
                raise CaptureError("PROTECTED_875_SOURCE_IDENTITY_INVALID")
            if row.get("canonical_target") is not None or row.get("production_key") is not None or row.get("source_identity_binding") is not None:
                raise CaptureError("PROTECTED_875_CANONICAL_IDENTITY_PRESENT")
            rows.append(row)
    if len(rows) != 4800 or len(seen) != 4800:
        raise CaptureError("PROTECTED_875_TOTAL_ROW_COUNT_INVALID")
    return artifact, manifest, {"artifact_id": PRIMARY_ARTIFACT_ID, "archive_sha256": archive_sha, "output_sha256": PRIMARY_OUTPUT_SHA256, "row_count": len(rows)}


def _norm(s: str) -> str:
    s = unicodedata.normalize("NFKD", s.casefold())
    return "".join(c for c in s if not unicodedata.combining(c))


def _split_top_level(value: str, sep: str) -> list[str]:
    result: list[str] = []
    start = 0
    td = ld = 0
    i = 0
    while i < len(value):
        if value.startswith("{{", i): td += 1; i += 2; continue
        if value.startswith("}}", i) and td: td -= 1; i += 2; continue
        if value.startswith("[[", i): ld += 1; i += 2; continue
        if value.startswith("]]", i) and ld: ld -= 1; i += 2; continue
        if value[i] == sep and td == 0 and ld == 0:
            result.append(value[start:i]); start = i + 1
        i += 1
    result.append(value[start:])
    return result


def _templates(text: str) -> list[str]:
    stack: list[int] = []
    out: list[str] = []
    i = 0
    while i < len(text) - 1:
        if text.startswith("{{", i):
            stack.append(i if not stack else -1); i += 2; continue
        if text.startswith("}}", i) and stack:
            start = stack.pop(); i += 2
            if not stack and start >= 0: out.append(text[start:i])
            continue
        i += 1
    return out


def _plain_value(value: str) -> str | None:
    value = re.sub(r"<!--.*?-->", "", value, flags=re.S)
    value = re.sub(r"<ref\b[^>]*>.*?</ref\s*>", "", value, flags=re.I | re.S)
    value = re.sub(r"<[^>]{0,256}>", "", value)
    if "{{" in value or "}}" in value: return None
    value = re.sub(r"\[\[([^\]|]+\|)?([^\]]+)\]\]", lambda m: m.group(2), value)
    value = re.sub(r"\s+", " ", value).strip()
    if not value or len(value.encode("utf-8")) > 128 or any(c in value for c in "!?;{}<>\n\r"):
        return None
    if not re.fullmatch(r"[\wÀ-ÖØ-öø-ÿ .,+/%()'’\-]+", value, flags=re.UNICODE): return None
    return value


def parse_fandom_infobox(text: str, family: str) -> dict[str, str]:
    allowed = SAFE_KEYS[family]
    found: list[str] = []
    for invocation in _templates(text):
        name = _norm(_split_top_level(invocation[2:-2], "|")[0].strip().replace("_", " "))
        want = _norm(FANDOM_TEMPLATES[family].split(":", 1)[1])
        if name.endswith(want) or name.endswith(want.removeprefix("infobox ")): found.append(invocation)
    if len(found) != 1: return {}
    fields: dict[str, str] = {}
    for piece in _split_top_level(found[0][2:-2], "|")[1:]:
        pair = _split_top_level(piece, "=")
        if len(pair) < 2: continue
        key = _norm(re.sub(r"[^\w]", "", pair[0], flags=re.UNICODE))
        if key not in allowed: continue
        value = _plain_value("=".join(pair[1:]))
        if value is not None: fields[key] = value
    return dict(sorted(fields.items()))


def _api_json(params: dict[str, str]) -> tuple[dict[str, Any], dict[str, str]]:
    url = _api_url(params)
    body, headers, final_url = _http_get(url, max_bytes=MAX_HTTP_BYTES)
    if _origin(url) != _origin(final_url): raise CaptureError("FANDOM_API_REDIRECT_ORIGIN_MISMATCH")
    value = json.loads(body)
    if not isinstance(value, dict) or not isinstance(value.get("query"), dict): raise CaptureError("FANDOM_API_SHAPE_INVALID")
    return value, headers


def mediawiki_sha1_base36(data: bytes) -> str:
    """Return MediaWiki's base-36 encoding of SHA-1 over revision bytes."""
    value = int.from_bytes(hashlib.sha1(data).digest(), "big")
    alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
    if value == 0:
        return "0"
    digits: list[str] = []
    while value:
        value, remainder = divmod(value, 36)
        digits.append(alphabet[remainder])
    return "".join(reversed(digits))


def validate_continuation(next_cont: Any, seen: set[str], family: str) -> tuple[dict[str, str] | None, str | None]:
    if next_cont is None: return None, None
    if not isinstance(next_cont, dict) or not next_cont: raise CaptureError(f"FANDOM_CONTINUATION_INVALID:{family}")
    token = json.dumps(next_cont, sort_keys=True, separators=(",", ":"))
    if token in seen: raise CaptureError(f"FANDOM_CONTINUATION_LOOP:{family}")
    return {str(k): str(v) for k, v in next_cont.items()}, token


def validate_fandom_revision(pageid: Any, title: Any, revision: Any, content: Any, family: str) -> dict[str, Any]:
    if type(pageid) is not int or pageid <= 0 or not isinstance(title, str) or not title.strip() or len(title.encode("utf-8")) > 512:
        raise CaptureError(f"FANDOM_PAGE_ID_OR_TITLE_INVALID:{family}")
    if not isinstance(revision, dict): raise CaptureError(f"FANDOM_REVISION_OBJECT_INVALID:{family}")
    revid, timestamp, source_sha1 = revision.get("revid"), revision.get("timestamp"), revision.get("sha1")
    if type(revid) is not int or revid <= 0 or not isinstance(timestamp, str) or not re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", timestamp) or not isinstance(source_sha1, str) or not re.fullmatch(r"[0-9a-z]{1,31}", source_sha1) or not isinstance(content, str):
        raise CaptureError(f"FANDOM_REVISION_PROVENANCE_INVALID:{family}")
    try: datetime.strptime(timestamp, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError as exc: raise CaptureError(f"FANDOM_REVISION_TIMESTAMP_INVALID:{family}") from exc
    encoded = content.encode("utf-8")
    if len(encoded) > MAX_PAGE_BYTES: raise CaptureError("FANDOM_PAGE_MAX_PLUS_ONE")
    if mediawiki_sha1_base36(encoded) != source_sha1:
        raise CaptureError(f"FANDOM_REVISION_SHA1_CONTENT_MISMATCH:{family}")
    return {"family": family, "source": "TIBIAWIKI_FANDOM", "source_namespace": "mediawiki/tibia.fandom.com", "identity_namespace": "mediawiki/page_id", "external_id": str(pageid), "page_key": f"mediawiki/tibia.fandom.com/page_id/{pageid}", "title": title, "revision_id": revid, "revision_timestamp": timestamp, "source_sha1": source_sha1, "source_digest": sha256(encoded), "raw_utf8_bytes": len(encoded), "facts": parse_fandom_infobox(content, family), "attribution": "TibiaWiki community contributors; CC BY-SA as displayed by the source, subject to per-page notices", "license": "CC-BY-SA", "content_retained": False}


def capture_family(family: str) -> list[dict[str, Any]]:
    pages: dict[int, str] = {}
    cont: dict[str, str] = {}
    seen_cont: set[str] = set()
    while True:
        params = {"action": "query", "list": "embeddedin", "eititle": FANDOM_TEMPLATES[family], "eilimit": "max", "format": "json", "formatversion": "2", **cont}
        payload, _ = _api_json(params)
        listing = payload["query"].get("embeddedin")
        if not isinstance(listing, list): raise CaptureError(f"FANDOM_ENUMERATION_INVALID:{family}")
        for page in listing:
            pageid, title = page.get("pageid"), page.get("title")
            if type(pageid) is not int or pageid <= 0 or not isinstance(title, str) or not title.strip(): raise CaptureError(f"FANDOM_PAGE_ID_INVALID:{family}")
            previous = pages.get(pageid)
            if previous is not None and previous != title: raise CaptureError(f"FANDOM_PAGE_ID_TITLE_CONFLICT:{family}")
            pages[pageid] = title
            if len(pages) > MAX_PAGES_PER_FAMILY: raise CaptureError("FANDOM_FAMILY_PAGE_COUNT_MAX_PLUS_ONE")
        cont_next, token = validate_continuation(payload.get("continue"), seen_cont, family)
        if cont_next is None: break
        seen_cont.add(token); cont = cont_next
        time.sleep(0.1)
    result: list[dict[str, Any]] = []
    ids = sorted(pages)
    for offset in range(0, len(ids), BATCH_SIZE):
        batch = ids[offset:offset + BATCH_SIZE]
        payload, _ = _api_json({"action": "query", "pageids": "|".join(map(str, batch)), "prop": "revisions", "rvprop": "ids|timestamp|sha1|content", "rvslots": "main", "format": "json", "formatversion": "2"})
        listed = payload["query"].get("pages")
        if not isinstance(listed, list): raise CaptureError(f"FANDOM_REVISIONS_INVALID:{family}")
        by_id: dict[int, dict[str, Any]] = {}
        for page in listed:
            pageid = page.get("pageid")
            if type(pageid) is not int or pageid not in batch or pageid in by_id: raise CaptureError(f"FANDOM_PAGE_ID_SET_INVALID:{family}")
            by_id[pageid] = page
        if set(by_id) != set(batch): raise CaptureError(f"FANDOM_PAGE_ID_SET_MISMATCH:{family}")
        for pageid in batch:
            page = by_id[pageid]
            revisions = page.get("revisions")
            if not isinstance(revisions, list) or len(revisions) != 1: raise CaptureError(f"FANDOM_REVISION_CARDINALITY_INVALID:{family}")
            revision = revisions[0]; main = revision.get("slots", {}).get("main", {})
            content = main.get("content", main.get("*")) if isinstance(main, dict) else None
            record = validate_fandom_revision(pageid, page.get("title"), revision, content, family)
            result.append(record)
            del content
        time.sleep(0.1)
    return result


def _canonical_fact_map(fields: Any, family: str) -> dict[str, str]:
    if not isinstance(fields, dict): return {}
    out: dict[str, str] = {}
    for key, value in fields.items():
        k = _norm(str(key))
        k = ALIASES.get(k, k)
        if k in SAFE_KEYS[family] and k != "name" and isinstance(value, str): out[k] = _norm(value)
    return out


def _fandom_source_tuple(candidate: dict[str, Any]) -> dict[str, Any]:
    keys = ("source", "source_namespace", "identity_namespace", "external_id", "page_key", "title", "revision_id", "revision_timestamp", "source_sha1", "source_digest")
    return {key: candidate[key] for key in keys}


def classify_pair(primary: dict[str, Any], candidates: list[dict[str, Any]], family: str) -> dict[str, Any]:
    if primary.get("source_state") == "UNSUPPORTED_SOURCE_SHAPE":
        return {"state": "AMBIGUOUS", "reason": "UNSUPPORTED_PRIMARY_FAMILY_SHAPE", "candidates": []}
    title = primary.get("current_title")
    if not isinstance(title, str): return {"state": "AMBIGUOUS", "reason": "PRIMARY_TITLE_UNAVAILABLE", "candidates": []}
    exact = [c for c in candidates if _norm(c.get("title", "")) == _norm(title)]
    if not exact: return {"state": "NO_MATCH", "reason": "NO_EXACT_TITLE_CANDIDATE_IN_CAPTURED_FAMILY", "candidates": []}
    if len(exact) != 1: return {"state": "AMBIGUOUS", "reason": "MULTIPLE_EXACT_TITLE_CANDIDATES", "candidate_source_tuples": [_fandom_source_tuple(c) for c in exact]}
    candidate = exact[0]
    if candidate.get("source") != "TIBIAWIKI_FANDOM" or candidate.get("source_namespace") != "mediawiki/tibia.fandom.com":
        raise CaptureError("INDEPENDENT_SOURCE_IDENTITY_INVALID")
    primary_facts = _canonical_fact_map(primary.get("structured_fields"), family)
    secondary_facts = _canonical_fact_map(candidate.get("facts"), family)
    shared = sorted(set(primary_facts) & set(secondary_facts))
    matches = [key for key in shared if primary_facts[key] == secondary_facts[key]]
    mismatches = [key for key in shared if primary_facts[key] != secondary_facts[key]]
    if mismatches and not matches: return {"state": "CONFLICT", "reason": "NON_TITLE_FACTS_DISAGREE", "candidate_source_tuple": _fandom_source_tuple(candidate), "matching_facts": [], "conflicting_facts": mismatches}
    if len(matches) >= 2: state = "EXACT_CANDIDATE"
    elif len(matches) == 1: state = "PROBABLE"
    else: state = "AMBIGUOUS"
    if mismatches and matches: state = "CONFLICT"
    return {"state": state, "reason": "TITLE_PLUS_NON_TITLE_FACTS" if matches else "TITLE_ONLY_NOT_EXACT", "candidate_source_tuple": _fandom_source_tuple(candidate), "matching_facts": matches, "conflicting_facts": mismatches}


def official_receipts() -> dict[str, Any]:
    urls = {
        "creature_library": "https://www.tibia.com/library/?subtopic=creatures",
        "achievement_library": "https://www.tibia.com/library/?subtopic=achievements",
        "spell_library": "https://www.tibia.com/library/?subtopic=spells",
    }
    out = {}
    for name, url in urls.items():
        receipt, _ = _request_receipt(url, max_bytes=512 * 1024)
        receipt["source"] = "CIPSOFT_OFFICIAL_PUBLIC_LIBRARY"
        out[name] = receipt
        time.sleep(0.05)
    return out


def build_artifact(primary: dict[str, Any], primary_manifest: dict[str, Any], primary_receipt: dict[str, Any], preflight: dict[str, Any], fandom_pages: dict[str, list[dict[str, Any]]] | None, official: dict[str, Any], retrieval_time: str) -> tuple[dict[str, Any], dict[str, Any]]:
    source_unavailable = preflight.get("state") != "ACCESSIBLE" or fandom_pages is None
    families: dict[str, list[dict[str, Any]]] = {family: [] for family in FAMILY_COUNTS}
    primary_rows = primary["families"]
    family_statuses: dict[str, str] = {}
    for family, count in FAMILY_COUNTS.items():
        rows = primary_rows[family]
        if len(rows) != count: raise CaptureError(f"PRIMARY_FAMILY_COHORT_DRIFT:{family}")
        family_statuses[family] = "UNKNOWN" if source_unavailable else "CAPTURED_EVIDENCE_ONLY"
        candidates = [] if fandom_pages is None else fandom_pages.get(family, [])
        for row in rows:
            if source_unavailable:
                result = {"state": "SOURCE_UNAVAILABLE", "reason": "FANDOM_ENDPOINT_OR_TERMS_PREFLIGHT_UNAVAILABLE"}
            else:
                result = classify_pair(row, candidates, family)
            families[family].append({"source_tuple": row, "independent_source": {"source": "TIBIAWIKI_FANDOM", "corroboration_status": "UNKNOWN" if source_unavailable else result["state"], "crosswalk": result}})
    artifact = {
        "schema": SCHEMA, "task_id": TASK_ID, "base_sha": BASE_SHA, "retrieval_timestamp": retrieval_time,
        "status": "SOURCE_UNAVAILABLE" if source_unavailable else "CAPTURED_EVIDENCE_ONLY",
        "families": families,
        "source_universe": {"source": "TIBIAWIKI_BR_G4_875_ARTIFACT", "artifact_id": PRIMARY_ARTIFACT_ID, "archive_sha256": primary_receipt["archive_sha256"], "canonical_output_sha256": PRIMARY_OUTPUT_SHA256, "rows": 4800, "family_counts": FAMILY_COUNTS},
        "independent_source": {"source": "TIBIAWIKI_FANDOM", "api_base": FANDOM_API, "source_relationship": "SEPARATE_MAINTAINED_SOURCE_CANDIDATE", "fact_level_independence": "UNKNOWN", "license": "CC-BY-SA", "prose_retained": False, "assets_retained": False, "all_page_content_hashed_then_discarded": True},
        "family_corroboration_statuses": family_statuses,
        "authority": {"source_identity_tuples_preserved": True, "article_prose_retained": False, "assets_retained": False, "canonical_identity_binding_emitted": False, "production_key_minted": False, "canonical_definition_population": False, "semantic_field_promotion": False, "source_id_join_is_not_identity_match": True, "title_only_exact_match_forbidden": True, "ability_scope": "SPELL_SUBSET_ONLY"},
    }
    counts: dict[str, dict[str, int]] = {}
    for family, rows in families.items():
        c: dict[str, int] = defaultdict(int)
        for row in rows: c[row["independent_source"]["crosswalk"]["state"]] += 1
        counts[family] = dict(sorted(c.items()))
    manifest = {
        "schema": MANIFEST_SCHEMA, "task_id": TASK_ID, "repository": REPOSITORY, "base_sha": BASE_SHA,
        "status": artifact["status"], "retrieval_timestamp": retrieval_time,
        "primary_artifact": primary_receipt,
        "primary_manifest_sha256": sha256(canonical_bytes(primary_manifest)),
        "fandom_preflight": preflight, "official_public_receipts": official,
        "family_corroboration_statuses": family_statuses,
        "counts": {"source_rows": sum(FAMILY_COUNTS.values()), "family_rows": FAMILY_COUNTS, "crosswalk_states": counts},
        "license_and_prose_policy": {"fandom_license": "CC-BY-SA per source notices", "attribution": "TibiaWiki community contributors", "raw_page_content_retained": False, "short_allowlisted_facts_only": True, "long_form_prose_spoilers_transcripts_and_assets_retained": False},
        "invariants": {"exact_protected_875_artifact_verified": True, "all_source_tuples_preserved": True, "external_ids_qualified_by_source_and_namespace": True, "only_direct_fandom_api_used_not_a_mirror_or_derived_feed": True, "fact_level_independence_proven": False, "title_only_exact_match_forbidden": True, "no_identity_binding_or_canonical_population": True, "no_semantic_promotion": True, "all_family_status_unknown_when_source_unavailable": source_unavailable},
        "artifact_sha256": sha256(canonical_bytes(artifact)),
    }
    if source_unavailable:
        if any(state != "UNKNOWN" for state in family_statuses.values()): raise CaptureError("SOURCE_UNAVAILABLE_FAMILY_STATUS_NOT_UNKNOWN")
        if any(row["independent_source"]["crosswalk"]["state"] != "SOURCE_UNAVAILABLE" for rows in families.values() for row in rows): raise CaptureError("SOURCE_UNAVAILABLE_ROW_STATE_INVALID")
    return artifact, manifest


def run(output: Path, manifest_path: Path) -> int:
    primary, primary_manifest, primary_receipt = fetch_primary_artifact()
    preflight = preflight_fandom()
    official = official_receipts()
    pages = None
    if preflight.get("state") == "ACCESSIBLE":
        try:
            pages = {family: capture_family(family) for family in FAMILY_COUNTS}
            if sum(len(rows) for rows in pages.values()) > MAX_TOTAL_FANDOM_PAGES:
                raise CaptureError("FANDOM_TOTAL_PAGE_COUNT_MAX_PLUS_ONE")
        except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError, OSError, CaptureError, json.JSONDecodeError) as exc:
            preflight["capture_error"] = type(exc).__name__ + (f":HTTP_{exc.code}" if isinstance(exc, urllib.error.HTTPError) else "")
            preflight["state"] = "SOURCE_UNAVAILABLE"
            pages = None
    artifact, manifest = build_artifact(primary, primary_manifest, primary_receipt, preflight, pages, official, utc_now())
    output.parent.mkdir(parents=True, exist_ok=True); manifest_path.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_bytes(artifact)); manifest_path.write_bytes(canonical_bytes(manifest))
    print(f"{artifact['status']}: rows=4800 sha256={manifest['artifact_sha256']}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    return run(args.output, args.manifest_output)


if __name__ == "__main__":
    raise SystemExit(main())
