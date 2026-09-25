#!/usr/bin/env python3
"""Build an artifact-only, current-source G4 crosswalk for direct non-Item pages.

The sealed G3/G4 artifacts establish the exact source universe and prior
revision tuple. Current page bodies are fetched by page ID, hashed in memory,
and reduced to a strict allowlist of short structured infobox values. The
result is evidence only: it never mints ProductionKeys, emits v2 bindings, or
creates canonical project definitions.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import re
import sys
import time
import unicodedata
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

HERE = Path(__file__).resolve().parent
REPOSITORY = "Oteryn/Oteryn-Game"
API = "https://www.tibiawiki.com.br/api.php"
SCHEMA = "OTERYN_G4_NONITEM_BULK_CROSSWALK/v1"
MANIFEST_SCHEMA = "OTERYN_G4_NONITEM_BULK_CROSSWALK_MANIFEST/v1"
BASE_SHA = "1680eb5dc6145aa3e271ac8665f33a50ed837b76"
MAX_RESPONSE_BYTES = 20 * 1024 * 1024
MAX_PAGE_BYTES = 16 * 1024 * 1024
BATCH_SIZE = 10

FAMILY_COUNTS = {
    "Creature": 2149,
    "NPC": 1253,
    "Achievement": 569,
    "Quest": 272,
    "Ability": 171,
    "Outfit": 134,
    "Mount": 252,
}

# This is intentionally a small evidence allowlist, not a content schema.
# Long-form prose, loot tables, shop rows, exact spell formulas and appearance
# identifiers are excluded; they need their own field/relationship evidence.
SAFE_FIELDS = {
    "Creature": {
        "name", "nome", "health", "hp", "vida", "experience", "exp",
        "speed", "velocidade", "armor", "armour", "armadura", "mitigation",
        "pushable", "pushableobjects", "pushobjects", "paralyzable",
        "paralyzeable", "summonable", "convinceable", "convincible",
        "immune", "immunities", "resistances", "resistance",
    },
    "NPC": {"name", "nome", "occupation", "profession", "job", "cargo", "race", "raca"},
    "Achievement": {"name", "nome", "points", "pontos", "grade", "grau", "degree", "secret", "secreto", "premium"},
    "Quest": {"name", "nome", "level", "requiredlevel", "nivel", "premium", "repeatable", "repetivel", "type", "tipo"},
    "Ability": {"name", "nome", "level", "requiredlevel", "nivel", "mana", "manacost", "vocation", "vocations", "vocacao", "premium", "cooldown", "groupcooldown", "range", "type", "tipo"},
    "Outfit": {"name", "nome", "premium", "gender", "sexo", "addons", "addon", "race", "raca"},
    "Mount": {"name", "nome", "speed", "speedbonus", "premium", "tamingitem", "itemdetamagem"},
}

TEMPLATE_TOKENS = {
    "Creature": ("infobox criatura", "infobox creature"),
    "NPC": ("infobox npc",),
    "Achievement": ("infobox achievement", "infobox conquista"),
    "Quest": ("infobox quest", "infobox missao"),
    "Ability": ("infobox spell", "infobox magia"),
    "Outfit": ("infobox outfit", "infobox traje"),
    "Mount": ("infobox mount", "infobox montaria"),
}


class CensusError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_previous_crosswalk_module() -> Any:
    path = HERE / "g4_direct_nonitem_family_crosswalk.py"
    spec = importlib.util.spec_from_file_location("g4_previous_crosswalk", path)
    if spec is None or spec.loader is None:
        raise CensusError("PRIOR_CROSSWALK_MODULE_UNAVAILABLE")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _norm(value: str) -> str:
    folded = unicodedata.normalize("NFKD", value.casefold())
    return "".join(ch for ch in folded if not unicodedata.combining(ch))


def _split_top_level(value: str, separator: str) -> list[str]:
    parts: list[str] = []
    start = 0
    template_depth = 0
    link_depth = 0
    i = 0
    while i < len(value):
        if value.startswith("{{", i):
            template_depth += 1
            i += 2
            continue
        if value.startswith("}}", i) and template_depth:
            template_depth -= 1
            i += 2
            continue
        if value.startswith("[[", i):
            link_depth += 1
            i += 2
            continue
        if value.startswith("]]", i) and link_depth:
            link_depth -= 1
            i += 2
            continue
        if value[i] == separator and template_depth == 0 and link_depth == 0:
            parts.append(value[start:i])
            start = i + 1
        i += 1
    parts.append(value[start:])
    return parts


def _outer_templates(text: str) -> list[str]:
    found: list[str] = []
    stack: list[int] = []
    i = 0
    while i < len(text) - 1:
        if text.startswith("{{", i):
            if not stack:
                stack.append(i)
            else:
                stack.append(-1)
            i += 2
            continue
        if text.startswith("}}", i) and stack:
            start = stack.pop()
            if not stack and start >= 0:
                found.append(text[start:i + 2])
            i += 2
            continue
        i += 1
    return found


def _plain_value(value: str) -> str | None:
    value = re.sub(r"<!--.*?-->", "", value, flags=re.S)
    value = re.sub(r"<ref\b[^>]*>.*?</ref\s*>", "", value, flags=re.I | re.S)
    value = re.sub(r"<[^>]{0,256}>", "", value)
    if "{{" in value or "}}" in value:
        return None
    value = re.sub(r"\[\[([^\]|]+\|)?([^\]]+)\]\]", lambda m: m.group(2), value)
    value = re.sub(r"\[(?:https?://\S+\s+)?([^\]]+)\]", r"\1", value)
    value = value.replace("'''", "").replace("''", "")
    value = re.sub(r"\s+", " ", value).strip()
    if not value or len(value.encode("utf-8")) > 128:
        return None
    # No sentence-like prose, markup residue, or control characters in the
    # bounded candidate evidence. Keep short categorical/numeric values only.
    if any(ch in value for ch in "!?;{}<>\n\r"):
        return None
    if not re.fullmatch(r"[\wÀ-ÖØ-öø-ÿ .,+/%()'’\-]+", value, flags=re.UNICODE):
        return None
    return value


def _template_name(invocation: str) -> str:
    inner = invocation[2:-2]
    return _norm(_split_top_level(inner, "|")[0].strip().replace("_", " ").removeprefix(":"))


def _family_templates(text: str, family: str) -> list[str]:
    tokens = TEMPLATE_TOKENS[family]
    result = []
    for invocation in _outer_templates(text):
        name = _template_name(invocation)
        name = re.sub(r"^(predefinicao|template|predefinicao de|predefinição):\s*", "", name)
        if any(token in name for token in tokens):
            result.append(invocation)
    return result


def parse_infobox(text: str, family: str) -> tuple[str, dict[str, str], str | None]:
    candidates = _family_templates(text, family)
    if len(candidates) != 1:
        return ("UNSUPPORTED_SOURCE_SHAPE" if not candidates else "AMBIGUOUS_SOURCE_SHAPE", {}, None)
    raw_name = _split_top_level(candidates[0][2:-2], "|")[0].strip()
    fields: dict[str, str] = {}
    pieces = _split_top_level(candidates[0][2:-2], "|")[1:]
    for piece in pieces:
        pair = _split_top_level(piece, "=")
        if len(pair) < 2:
            continue
        key = _norm(re.sub(r"[^\w]", "", pair[0], flags=re.UNICODE))
        if key not in SAFE_FIELDS[family]:
            continue
        # Split only at the first top-level equals; values may contain equals.
        raw_value = "=".join(pair[1:]).strip()
        value = _plain_value(raw_value)
        if value is None:
            continue
        fields[key] = value
    return "STRUCTURED_FAMILY_INFOBOX", dict(sorted(fields.items())), raw_name[:160]


def _redirect_target(text: str) -> str | None:
    match = re.match(r"\s*#(?:redirect|redirecionamento)\s*\[\[([^\]|]+)", text, flags=re.I)
    if not match:
        return None
    target = match.group(1).strip()
    return target[:256] or ""


def _api_json(page_ids: list[int]) -> dict[str, Any]:
    query = urlencode({
        "action": "query", "format": "json", "formatversion": "2",
        "pageids": "|".join(str(page_id) for page_id in page_ids),
        "prop": "revisions", "rvprop": "ids|timestamp|content", "rvslots": "main",
    })
    request = Request(API + "?" + query, headers={
        "Accept": "application/json", "User-Agent": "Oteryn-G4-NonItem-BulkCrosswalk/1.0",
    })
    for attempt in range(5):
        try:
            with urlopen(request, timeout=45) as response:
                payload = response.read(MAX_RESPONSE_BYTES + 1)
            if len(payload) > MAX_RESPONSE_BYTES:
                raise CensusError("WIKI_RESPONSE_MAX_PLUS_ONE")
            value = json.loads(payload)
            if not isinstance(value, dict) or not isinstance(value.get("query"), dict):
                raise CensusError("WIKI_RESPONSE_SHAPE_INVALID")
            return value
        except HTTPError as error:
            if error.code not in (429, 500, 502, 503, 504) or attempt == 4:
                raise
            retry = error.headers.get("Retry-After")
            time.sleep(min(20, int(retry)) if retry and retry.isdigit() else 2 ** attempt)
        except (URLError, TimeoutError, json.JSONDecodeError):
            if attempt == 4:
                raise
            time.sleep(2 ** attempt)
    raise CensusError("WIKI_RETRY_EXHAUSTED")


def fetch_current_pages(page_families: dict[int, str]) -> tuple[dict[int, dict[str, Any]], dict[str, int]]:
    fetched: dict[int, dict[str, Any]] = {}
    errors: dict[str, int] = defaultdict(int)
    page_ids = sorted(page_families)
    for offset in range(0, len(page_ids), BATCH_SIZE):
        batch = page_ids[offset:offset + BATCH_SIZE]
        try:
            payload = _api_json(batch)
            pages = payload["query"].get("pages", [])
            seen: set[int] = set()
            for page in pages:
                if not isinstance(page, dict) or not isinstance(page.get("pageid"), int):
                    errors["WIKI_PAGE_ROW_INVALID"] += 1
                    continue
                page_id = page["pageid"]
                if page_id not in batch or page_id in seen:
                    errors["WIKI_PAGE_ID_DUPLICATE_OR_UNREQUESTED"] += 1
                    continue
                seen.add(page_id)
                revisions = page.get("revisions") or []
                if page.get("missing") is not None or not revisions:
                    fetched[page_id] = {"fetch_state": "SOURCE_UNAVAILABLE", "title": page.get("title")}
                    continue
                revision = revisions[0]
                slots = revision.get("slots", {})
                main = slots.get("main", {}) if isinstance(slots, dict) else {}
                content = main.get("content", main.get("*")) if isinstance(main, dict) else None
                if not isinstance(content, str):
                    fetched[page_id] = {"fetch_state": "UNSUPPORTED_SOURCE_SHAPE", "title": page.get("title")}
                    continue
                encoded = content.encode("utf-8")
                metadata = {
                    "title": page.get("title"), "revision_id": revision.get("revid"),
                    "revision_timestamp": revision.get("timestamp"),
                    "raw_utf8_sha256": sha256_bytes(encoded), "raw_utf8_bytes": len(encoded),
                }
                if len(encoded) > MAX_PAGE_BYTES:
                    fetched[page_id] = {**metadata, "fetch_state": "SOURCE_PAGE_MAX_PLUS_ONE"}
                    continue
                redirect = _redirect_target(content)
                if redirect:
                    fetched[page_id] = {**metadata, "fetch_state": "REDIRECT", "redirect_target": redirect}
                else:
                    source_shape, fields, template_name = parse_infobox(content, page_families[page_id])
                    fetched[page_id] = {
                        **metadata, "fetch_state": source_shape, "source_shape": source_shape,
                        "structured_fields": fields, "current_infobox_template": template_name,
                    }
                del content, encoded
            for page_id in batch:
                if page_id not in fetched and page_id not in seen:
                    fetched[page_id] = {"fetch_state": "SOURCE_UNAVAILABLE"}
        except Exception as error:  # retain a row for every exact input on any bounded batch failure
            errors[type(error).__name__] += 1
            for page_id in batch:
                fetched[page_id] = {"fetch_state": "LIVE_FETCH_ERROR", "error_code": type(error).__name__}
        time.sleep(0.08)
    return fetched, dict(sorted(errors.items()))


def _candidate_concept(row: dict[str, Any]) -> tuple[str, tuple[tuple[str, str], ...]] | None:
    fields = row.get("structured_fields", {})
    display = fields.get("name") or fields.get("nome")
    title = row.get("current_title")
    if not isinstance(display, str) or not isinstance(title, str) or _norm(display) != _norm(title):
        return None
    identity_fields = tuple((key, _norm(value)) for key, value in fields.items() if key not in {"name", "nome"})
    return (_norm(display), identity_fields)


def classify_concepts(rows: list[dict[str, Any]]) -> None:
    duplicates: dict[tuple[str, str], list[int]] = defaultdict(list)
    fingerprints: dict[tuple[str, str], set[tuple[tuple[str, str], ...]]] = defaultdict(set)
    for row in rows:
        candidate = _candidate_concept(row)
        if candidate is None:
            row["concept_resolution"] = "UNRESOLVED_MULTI_SIGNAL"
            continue
        name, signature = candidate
        group = (row["family"], name)
        duplicates[group].append(int(row["external_id"]))
        fingerprints[group].add(signature)
    for row in rows:
        candidate = _candidate_concept(row)
        if candidate is None:
            continue
        group = (row["family"], candidate[0])
        ids = duplicates[group]
        if len(ids) > 1 and len(fingerprints[group]) > 1:
            row["concept_resolution"] = "CONFLICTING_DUPLICATE_SOURCE_CONCEPT"
            row["possible_duplicate_page_ids"] = sorted(ids)
        elif len(ids) > 1:
            row["concept_resolution"] = "AMBIGUOUS_DUPLICATE_SOURCE_CONCEPT"
            row["possible_duplicate_page_ids"] = sorted(ids)
        else:
            row["concept_resolution"] = "UNIQUE_MULTI_SIGNAL_SOURCE_CONCEPT_CANDIDATE"


def build_output(g3: dict[str, Any], g3_manifest: dict[str, Any], g4: dict[str, Any], g4_manifest: dict[str, Any], fetched: dict[int, dict[str, Any]], tracked_paths: list[str], retrieval_time: str, api_errors: dict[str, int]) -> tuple[dict[str, Any], dict[str, Any]]:
    prior = load_previous_crosswalk_module()
    g3_rows = prior.verify_g3(g3, g3_manifest, strict_artifact=True)
    g4_rows = prior.verify_g4(g4, g4_manifest, strict_artifact=True)
    selected: list[tuple[str, dict[str, Any], dict[str, Any]]] = []
    for family, expected_count in FAMILY_COUNTS.items():
        family_rows = [row for row in g3_rows.values() if row.get("source_family_classification", {}).get("primary_definition_family") == family]
        if len(family_rows) != expected_count:
            raise CensusError(f"G3_FAMILY_COUNT_MISMATCH:{family}")
        for row in sorted(family_rows, key=lambda item: item["page_id"]):
            page_id = row["page_id"]
            if page_id not in g4_rows:
                raise CensusError(f"G3_PAGE_MISSING_FROM_G4:{family}:{page_id}")
            prior._validate_direct_row(page_id, row, family)
            selected.append((family, row, g4_rows[page_id]))
    expected_total = sum(FAMILY_COUNTS.values())
    if len(selected) != expected_total or len({g3["page_id"] for _, g3, _ in selected}) != expected_total:
        raise CensusError("DIRECT_ROW_PARTITION_NOT_UNIQUE")

    raw_rows: list[dict[str, Any]] = []
    baseline_changed = 0
    baseline_changed_ids: list[int] = []
    revision_revalidated = 0
    for family, g3row, g4row in selected:
        page_id = g3row["page_id"]
        live = fetched.get(page_id, {"fetch_state": "LIVE_FETCH_ERROR", "error_code": "MISSING_FETCH_RESULT"})
        g3prov = g3row["provenance"][0]
        changed = g3prov["revision_id"] != g4row["revision_id"] or g3prov["revision_timestamp"] != g4row["revision_timestamp"]
        if changed:
            baseline_changed += 1
            baseline_changed_ids.append(page_id)
        status = live.get("fetch_state", "LIVE_FETCH_ERROR")
        fields: dict[str, str] = {}
        live_revision = live.get("revision_id")
        live_timestamp = live.get("revision_timestamp")
        current_title = live.get("title")
        live_digest = live.get("raw_utf8_sha256")
        old_tuple_matches = (
            status == "STRUCTURED_FAMILY_INFOBOX" and live_revision == g4row["revision_id"]
            and live_timestamp == g4row["revision_timestamp"] and live_digest == g4row["raw_utf8_sha256"]
        )
        redirect = live.get("redirect_target") if status == "REDIRECT" else None
        template_state = live.get("source_shape")
        current_infobox_template = live.get("current_infobox_template")
        if status == "STRUCTURED_FAMILY_INFOBOX":
            fields = live.get("structured_fields", {})
            if live_revision != g4row["revision_id"] or live_digest != g4row["raw_utf8_sha256"]:
                revision_revalidated += 1
            status = "CURRENT_REVISION_REVALIDATED" if changed or not old_tuple_matches else "SOURCE_REVISION_STABLE"
        raw_rows.append({
            "family": family,
            "source": "TIBIAWIKI_STRUCTURED",
            "source_namespace": "mediawiki/tibiawiki.com.br",
            "identity_namespace": "mediawiki/page_id",
            "external_id": str(page_id),
            "page_key": f"mediawiki/tibiawiki.com.br/page_id/{page_id}",
            "g3_title_observation": g3prov.get("observed_title"),
            "current_title": current_title,
            "g3_revision_id": g3prov["revision_id"],
            "g3_revision_timestamp": g3prov["revision_timestamp"],
            "g3_source_shape": g3prov.get("source_shape"),
            "g3_direct_family_signature": g3row["source_family_classification"].get("assignment_rule"),
            "g4_revision_id": g4row["revision_id"],
            "g4_revision_timestamp": g4row["revision_timestamp"],
            "g4_raw_utf8_sha256": g4row["raw_utf8_sha256"],
            "current_revision_id": live_revision,
            "current_revision_timestamp": live_timestamp,
            "current_raw_utf8_sha256": live_digest,
            "current_raw_utf8_bytes": live.get("raw_utf8_bytes"),
            "prior_revision_drift": changed,
            "source_state": status,
            "source_shape": template_state,
            "current_infobox_template": current_infobox_template,
            "redirect_target_observation": redirect,
            "structured_fields": fields,
            "field_authority": "CANDIDATE_EVIDENCE_ONLY_NOT_PROMOTED",
            "candidate_target_state": "MISSING_CANONICAL_IDENTITY",
            "canonical_target": None,
            "production_key": None,
            "source_identity_binding": None,
        })

    classify_concepts(raw_rows)
    raw_rows.sort(key=lambda row: (row["family"], int(row["external_id"])))

    role_suffixes = ("definitions/reference.json", "definitions/declarations.json", "presentations/bindings.json", "assets/catalog.json", "provenance/sources.json")
    production_docs = sorted(path for path in tracked_paths if any(path.endswith(suffix) for suffix in role_suffixes) and not path.startswith(("docs/", "tools/", "tests/", "vendor/")) and "/tests/" not in f"/{path}/" and "/fixtures/" not in f"/{path}/")
    families: dict[str, list[dict[str, Any]]] = {family: [] for family in FAMILY_COUNTS}
    for row in raw_rows:
        families[row["family"]].append(row)
    artifact = {
        "schema": SCHEMA,
        "repository_base_sha": BASE_SHA,
        "retrieval_timestamp": retrieval_time,
        "families": families,
        "authority": {
            "page_id_join_only": True,
            "current_source_revalidated_by_exact_page_id": True,
            "article_prose_retained": False,
            "structured_field_allowlist_only": True,
            "source_identity_binding_emitted": False,
            "production_key_minted": False,
            "canonical_definition_population": False,
            "semantic_field_promotion": False,
            "presentation_asset_runtime_population": False,
            "second_structured_source_corroboration": "NOT_IN_THIS_BATCH",
        },
    }
    state_counts: dict[str, dict[str, int]] = {}
    for family, rows in families.items():
        counts: dict[str, int] = defaultdict(int)
        for row in rows:
            counts[row["source_state"]] += 1
        state_counts[family] = dict(sorted(counts.items()))
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "task_id": "OTV2-20260925-g4-nonitem-bulk-crosswalk",
        "repository": REPOSITORY,
        "repository_base_sha": BASE_SHA,
        "inputs": {
            "g3": {"artifact_id": 10801778929, "run_id": 35986883931, "head_sha": "20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a", "archive_sha256": "a63635e4cdfcd237cf69b5e2fe0471c30723e45fe3ab2ccc9095730a9337d2e7", "corpus_sha256": "1562382c66ad471a46309eaa5ae2fe9e7b3b1e3c8c2d5fc047ee8faa65bd1666"},
            "g4": {"artifact_id": 10831362943, "run_id": 36053194532, "head_sha": "b2e77ba01677b484fdfa6271c5745d2280c13df6", "archive_sha256": "5d8e886e31a0f64a035a61dc5c3f8ce031a8af3b06926117750deebc841394ca", "corpus_sha256": "f47dbe5832e7b1accd652852303d638a4f19367bab3951f260cc39a0d93b7713"},
        },
        "counts": {"direct_rows": len(raw_rows), "expected_direct_rows": sum(FAMILY_COUNTS.values()), "baseline_revision_drift_rows_g3_to_g4": baseline_changed, "baseline_revision_drift_page_ids": baseline_changed_ids, "current_revision_revalidated_rows": revision_revalidated, "duplicate_or_dropped_page_ids": len(raw_rows) - len({row["external_id"] for row in raw_rows})},
        "families": {family: {"expected_rows": FAMILY_COUNTS[family], "rows": len(rows), "source_states": state_counts[family]} for family, rows in families.items()},
        "canonical_target_inventory": {"method": "exact tracked WorldProject/v1-v2 role suffix inventory at protected base", "role_suffixes": list(role_suffixes), "production_documents": production_docs, "absence_proven": not production_docs},
        "source_fetch_errors_by_class": api_errors,
        "invariants": {
            "exact_g3_direct_family_and_count_partition": True,
            "exact_g4_page_identity_index_and_pinned_artifact_checks": True,
            "exact_current_page_fetch_or_explicit_failure_row": True,
            "zero_dropped_or_duplicate_source_ids_required": True,
            "no_title_only_identity_or_crosswalk": True,
            "safe_allowlisted_short_fields_only": True,
            "no_source_id_as_production_key": True,
            "no_bindings_or_canonical_population": True,
            "workflow_artifact_only": True,
        },
        "artifact_sha256": sha256_bytes(canonical_bytes(artifact)),
    }
    return artifact, manifest


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_bytes(value))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--repository-root", type=Path, default=Path.cwd())
    parser.add_argument("--base-sha", default=BASE_SHA)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    if args.base_sha != BASE_SHA:
        raise CensusError("ALLOCATED_BASE_SHA_MISMATCH")
    token = __import__("os").environ.get("GITHUB_TOKEN")
    if not token:
        raise CensusError("GITHUB_TOKEN_REQUIRED")
    previous = load_previous_crosswalk_module()
    args.work_dir.mkdir(parents=True, exist_ok=True)
    g3_zip = args.work_dir / "g3-source-family.zip"
    g4_zip = args.work_dir / "g4-non-item-source.zip"
    previous.download_artifact(token, "g3", g3_zip)
    previous.download_artifact(token, "g4", g4_zip)
    g3, g3_manifest, _ = previous.load_artifact_zip(g3_zip, "g3")
    g4, g4_manifest, _ = previous.load_artifact_zip(g4_zip, "g4")
    g3_rows = previous.verify_g3(g3, g3_manifest, strict_artifact=True)
    g4_rows = previous.verify_g4(g4, g4_manifest, strict_artifact=True)
    page_families = {page_id: row["source_family_classification"]["primary_definition_family"] for page_id, row in g3_rows.items() if row.get("source_family_classification", {}).get("primary_definition_family") in FAMILY_COUNTS}
    selected_ids = sorted(page_families)
    if len(selected_ids) != sum(FAMILY_COUNTS.values()) or any(page_id not in g4_rows for page_id in selected_ids):
        raise CensusError("PINNED_SOURCE_UNIVERSE_PARTITION_INVALID")
    fetched, api_errors = fetch_current_pages(page_families)
    commit = __import__("subprocess").run(["git", "-C", str(args.repository_root), "ls-tree", "-r", "--name-only", args.base_sha], check=True, capture_output=True, text=True)
    tracked = [line for line in commit.stdout.splitlines() if line]
    artifact, manifest = build_output(g3, g3_manifest, g4, g4_manifest, fetched, tracked, datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"), api_errors)
    write_json(args.output, artifact)
    write_json(args.manifest_output, manifest)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except CensusError as error:
        print(f"G4_NONITEM_BULK_CROSSWALK_FAILED:{error}", file=sys.stderr)
        raise SystemExit(1)
