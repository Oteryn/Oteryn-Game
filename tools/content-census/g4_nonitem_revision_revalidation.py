#!/usr/bin/env python3
"""Revalidate only the two pinned G4 non-Item revision-drift rows.

This tool emits exact source provenance and a current direct-family source-shape
observation. It never keeps page text or promotes a target/gameplay identity.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from typing import Any

HERE = Path(__file__).resolve().parent
REPOSITORY = "Oteryn/Oteryn-Game"
SOURCE_API = "https://www.tibiawiki.com.br/api.php"
SOURCE_KEY = "TIBIAWIKI_STRUCTURED"
SOURCE_NAMESPACE = "mediawiki/tibiawiki.com.br"
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_CONTENT_BYTES = 16 * 1024 * 1024
MAX_API_PAGES = 32
SCHEMA = "OTERYN_G4_NONITEM_REVISION_REVALIDATION/v1"
MANIFEST_SCHEMA = "OTERYN_G4_NONITEM_REVISION_REVALIDATION_MANIFEST/v1"

EXPECTED = {
    63947: "Creature",
    46925: "Quest",
}


class RevalidationError(RuntimeError):
    pass


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RevalidationError(f"{name.upper()}_IMPORT_FAILED")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


crosswalk = load_module("g4_direct_nonitem_family_crosswalk", HERE / "g4_direct_nonitem_family_crosswalk.py")
classifier = load_module("source_family_classification", HERE / "source_family_classification.py")
ARTIFACTS = crosswalk.ARTIFACTS


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json_bytes(payload: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(payload)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise RevalidationError(f"{label}_JSON_INVALID") from exc
    if not isinstance(value, dict):
        raise RevalidationError(f"{label}_ROOT_INVALID")
    return value


def load_artifact_zip(path: Path, name: str) -> tuple[dict[str, Any], dict[str, Any], str]:
    spec = ARTIFACTS[name]
    payload = path.read_bytes()
    if len(payload) != spec["size_in_bytes"] or "sha256:" + sha256_bytes(payload) != spec["digest"]:
        raise RevalidationError(f"{name.upper()}_ARCHIVE_SIZE_OR_DIGEST_MISMATCH")
    try:
        archive = zipfile.ZipFile(path)
    except zipfile.BadZipFile as exc:
        raise RevalidationError(f"{name.upper()}_ZIP_INVALID") from exc
    with archive:
        if sorted(archive.namelist()) != sorted(spec["members"]):
            raise RevalidationError(f"{name.upper()}_MEMBER_LIST_MISMATCH")
        corpus = read_json_bytes(archive.read(spec["members"][0]), f"{name.upper()}_CORPUS")
        manifest = read_json_bytes(archive.read(spec["members"][1]), f"{name.upper()}_MANIFEST")
    return corpus, manifest, sha256_bytes(payload)


def artifact_api_json(url: str, token: str) -> dict[str, Any]:
    request = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "Oteryn-G4-NonItem-Revision-Revalidation/1.0",
    })
    with urllib.request.urlopen(request, timeout=30) as response:
        value = json.load(response)
    if not isinstance(value, dict):
        raise RevalidationError("GITHUB_API_OBJECT_INVALID")
    return value


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req: Any, fp: Any, code: int, msg: str, headers: Any, newurl: str) -> None:
        return None


def download_artifact(token: str, name: str, destination: Path) -> str:
    spec = ARTIFACTS[name]
    base = "https://api.github.com/repos/Oteryn/Oteryn-Game/actions/artifacts"
    run = artifact_api_json(f"https://api.github.com/repos/Oteryn/Oteryn-Game/actions/runs/{spec['run_id']}", token)
    if run.get("id") != spec["run_id"] or run.get("head_sha") != spec["head_sha"] or run.get("status") != "completed" or run.get("conclusion") != "success":
        raise RevalidationError(f"{name.upper()}_RUN_ID_HEAD_OR_CONCLUSION_MISMATCH")
    metadata = artifact_api_json(f"{base}/{spec['id']}", token)
    for field in ("id", "name", "size_in_bytes", "digest"):
        if metadata.get(field) != spec[field]:
            raise RevalidationError(f"{name.upper()}_ARTIFACT_METADATA_MISMATCH:{field}")
    workflow = metadata.get("workflow_run", {})
    if metadata.get("expired") is not False or workflow.get("id") != spec["run_id"] or workflow.get("head_sha") != spec["head_sha"]:
        raise RevalidationError(f"{name.upper()}_ARTIFACT_RUN_OR_EXPIRY_MISMATCH")
    request = urllib.request.Request(f"{base}/{spec['id']}/zip", headers={
        "Accept": "application/vnd.github+json", "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "Oteryn-G4-NonItem-Revision-Revalidation/1.0",
    })
    try:
        urllib.request.build_opener(NoRedirect()).open(request, timeout=30)
    except urllib.error.HTTPError as exc:
        location = exc.headers.get("Location")
        if exc.code != 302 or not location or urllib.parse.urlparse(location).scheme != "https":
            raise RevalidationError("ARTIFACT_REDIRECT_INVALID") from exc
    else:
        raise RevalidationError("ARTIFACT_ZIP_DID_NOT_REDIRECT")
    digest = hashlib.sha256()
    size = 0
    destination.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(location, timeout=120) as response, destination.open("wb") as output:
        while block := response.read(1024 * 1024):
            size += len(block)
            if size > min(spec["size_in_bytes"], MAX_ARCHIVE_BYTES):
                raise RevalidationError(f"{name.upper()}_ARCHIVE_MAX_PLUS_ONE")
            digest.update(block)
            output.write(block)
    actual = "sha256:" + digest.hexdigest()
    if size != spec["size_in_bytes"] or actual != spec["digest"]:
        raise RevalidationError(f"{name.upper()}_ARCHIVE_SIZE_OR_DIGEST_MISMATCH")
    return digest.hexdigest()


def validate_api_page(value: Any, expected_page_id: int, continuation: dict[str, Any] | None = None) -> tuple[dict[str, Any], list[str], list[str], str, dict[str, Any] | None]:
    if not isinstance(value, dict) or "error" in value:
        raise RevalidationError("CURRENT_API_RESPONSE_INVALID")
    query = value.get("query")
    pages = query.get("pages") if isinstance(query, dict) else None
    if not isinstance(pages, list) or len(pages) != 1 or not isinstance(pages[0], dict):
        raise RevalidationError(f"CURRENT_PAGE_CARDINALITY_INVALID:{expected_page_id}")
    page = pages[0]
    if page.get("missing") is True:
        raise RevalidationError(f"CURRENT_PAGE_MISSING:{expected_page_id}")
    if page.get("pageid") != expected_page_id:
        raise RevalidationError(f"CURRENT_PAGE_ID_MISMATCH:{expected_page_id}")
    namespace_id = page.get("ns")
    if not isinstance(namespace_id, int) or isinstance(namespace_id, bool) or namespace_id != 0:
        raise RevalidationError(f"CURRENT_NAMESPACE_INVALID:{expected_page_id}")
    if page.get("redirect") is True:
        raise RevalidationError(f"CURRENT_PAGE_REDIRECT:{expected_page_id}")
    revisions = page.get("revisions")
    if not isinstance(revisions, list) or len(revisions) != 1 or not isinstance(revisions[0], dict):
        raise RevalidationError(f"CURRENT_REVISION_CARDINALITY_INVALID:{expected_page_id}")
    revision = revisions[0]
    revision_id = revision.get("revid")
    timestamp = revision.get("timestamp")
    if not isinstance(revision_id, int) or isinstance(revision_id, bool) or revision_id <= 0:
        raise RevalidationError(f"CURRENT_REVISION_ID_INVALID:{expected_page_id}")
    if not isinstance(timestamp, str) or re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", timestamp) is None:
        raise RevalidationError(f"CURRENT_REVISION_TIMESTAMP_INVALID:{expected_page_id}")
    latest_revision = page.get("lastrevid")
    if latest_revision is not None and latest_revision != revision_id:
        raise RevalidationError(f"CURRENT_REVISION_ID_MISMATCH:{expected_page_id}")
    slots = revision.get("slots")
    main = slots.get("main") if isinstance(slots, dict) else None
    content = main.get("content") if isinstance(main, dict) else None
    if not isinstance(content, str):
        raise RevalidationError(f"CURRENT_REVISION_CONTENT_MISSING:{expected_page_id}")
    try:
        content_bytes = content.encode("utf-8", errors="strict")
    except UnicodeEncodeError as exc:
        raise RevalidationError(f"CURRENT_REVISION_UTF8_INVALID:{expected_page_id}") from exc
    if len(content_bytes) > MAX_CONTENT_BYTES:
        raise RevalidationError(f"CURRENT_CONTENT_MAX_PLUS_ONE:{expected_page_id}")
    templates = page.get("templates", [])
    categories = page.get("categories", [])
    if not isinstance(templates, list) or not isinstance(categories, list):
        raise RevalidationError(f"CURRENT_SHAPE_ARRAYS_INVALID:{expected_page_id}")
    template_names = sorted({item.get("title") for item in templates if isinstance(item, dict) and isinstance(item.get("title"), str)})
    category_names = sorted({item.get("title") for item in categories if isinstance(item, dict) and isinstance(item.get("title"), str)})
    if len(template_names) != len(templates) or len(category_names) != len(categories):
        raise RevalidationError(f"CURRENT_SHAPE_ENTRY_INVALID:{expected_page_id}")
    next_continuation = value.get("continue")
    if next_continuation is not None and not isinstance(next_continuation, dict):
        raise RevalidationError(f"CURRENT_CONTINUATION_INVALID:{expected_page_id}")
    metadata = {"page_id": expected_page_id, "namespace_id": namespace_id, "revision_id": revision_id, "revision_timestamp": timestamp,
                "raw_utf8_sha256": sha256_bytes(content_bytes), "templates": template_names, "categories": category_names}
    return metadata, template_names, category_names, content, next_continuation


def fetch_current_page(page_id: int, *, opener: Any = None) -> dict[str, Any]:
    opener = opener or urllib.request.build_opener()
    continuation: dict[str, Any] = {}
    pages_seen = 0
    templates: set[str] = set()
    categories: set[str] = set()
    initial: dict[str, Any] | None = None
    while True:
        pages_seen += 1
        if pages_seen > MAX_API_PAGES:
            raise RevalidationError(f"CURRENT_API_PAGE_BOUND_EXCEEDED:{page_id}")
        params: dict[str, str] = {
            "action": "query", "pageids": str(page_id), "prop": "info|revisions|templates|categories",
            "rvprop": "ids|timestamp|content", "rvslots": "main", "tllimit": "max", "cllimit": "max",
            "format": "json", "formatversion": "2",
        }
        params.update({key: str(value) for key, value in continuation.items()})
        url = SOURCE_API + "?" + urllib.parse.urlencode(params)
        request = urllib.request.Request(url, headers={
            "Accept": "application/json", "User-Agent": "Oteryn-G4-NonItem-Revision-Revalidation/1.0 (source evidence only)",
        })
        try:
            with opener.open(request, timeout=60) as response:
                value = json.load(response)
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as exc:
            raise RevalidationError(f"CURRENT_API_FETCH_FAILED:{page_id}") from exc
        metadata, current_templates, current_categories, _, next_continuation = validate_api_page(value, page_id)
        if initial is not None and any(metadata[key] != initial[key] for key in ("revision_id", "revision_timestamp", "raw_utf8_sha256", "namespace_id")):
            raise RevalidationError(f"CURRENT_REVISION_DRIFT_DURING_PAGINATION:{page_id}")
        initial = metadata
        templates.update(current_templates)
        categories.update(current_categories)
        if next_continuation is None:
            break
        if not next_continuation or any(not isinstance(k, str) or not isinstance(v, str) for k, v in next_continuation.items()):
            raise RevalidationError(f"CURRENT_CONTINUATION_MALFORMED:{page_id}")
        continuation = next_continuation
    assert initial is not None
    initial["templates"] = sorted(templates)
    initial["categories"] = sorted(categories)
    return initial


def _validate_pinned_rows(g3: dict[str, Any], g3_manifest: dict[str, Any], g4: dict[str, Any], g4_manifest: dict[str, Any]) -> dict[int, dict[str, Any]]:
    g3_rows = crosswalk.verify_g3(g3, g3_manifest)
    g4_rows = crosswalk.verify_g4(g4, g4_manifest)
    selected: dict[int, dict[str, Any]] = {}
    for page_id, family in EXPECTED.items():
        g3_row = g3_rows.get(page_id)
        g4_row = g4_rows.get(page_id)
        if not isinstance(g3_row, dict) or not isinstance(g4_row, dict):
            raise RevalidationError(f"PINNED_PAGE_MISSING:{page_id}")
        g3_identity = g3_row.get("source_family_classification", {})
        if g3_identity.get("primary_definition_family") != family or g3_identity.get("state") != "SOURCE_DEFINITION_PRIMARY":
            raise RevalidationError(f"PINNED_DIRECT_FAMILY_MISMATCH:{page_id}")
        g3_revision = crosswalk._validate_direct_row(page_id, g3_row, family)
        for field, expected in (("source", SOURCE_KEY), ("source_role", "STRUCTURED_REFERENCE_DATA"),
                                ("source_namespace", SOURCE_NAMESPACE), ("external_id", str(page_id)),
                                ("page_key", f"{SOURCE_NAMESPACE}/page_id/{page_id}")):
            if g4_row.get(field) != expected:
                raise RevalidationError(f"PINNED_G4_IDENTITY_MISMATCH:{page_id}:{field}")
        if g3_revision["revision_id"] == g4_row.get("revision_id") and g3_revision["revision_timestamp"] == g4_row.get("revision_timestamp"):
            raise RevalidationError(f"PINNED_REVISION_DRIFT_NOT_FOUND:{page_id}")
        provenance = g3_row["provenance"][0]
        source_signature = classifier.G1_SIGNATURES[family]
        selected[page_id] = {
            "family": family,
            "signature_id": source_signature["id"],
            "signature": source_signature,
            "g3": g3_revision,
            "g3_shape": {
                "source_shape": provenance.get("source_shape"),
                "redirect": provenance.get("redirect"),
                "discovery_roots": provenance.get("discovery_roots"),
                "source_surfaces": provenance.get("source_surfaces"),
                "templates": provenance.get("templates"),
                "categories": provenance.get("categories"),
            },
            "g4": {
                "source_key": g4_row["source"], "source_namespace": g4_row["source_namespace"],
                "identity_namespace": "mediawiki/page_id", "external_id": g4_row["external_id"], "page_key": g4_row["page_key"],
                "revision_id": g4_row["revision_id"], "revision_timestamp": g4_row["revision_timestamp"],
                "raw_utf8_sha256": g4_row["raw_utf8_sha256"],
            },
        }
    return selected


def validate_live_shape(page_id: int, pinned: dict[str, Any], current: dict[str, Any]) -> dict[str, Any]:
    if current.get("page_id") != page_id:
        raise RevalidationError(f"CURRENT_PAGE_ID_MISMATCH:{page_id}")
    if current.get("namespace_id") != 0:
        raise RevalidationError(f"CURRENT_NAMESPACE_INVALID:{page_id}")
    if not isinstance(current.get("revision_id"), int) or isinstance(current.get("revision_id"), bool) or current["revision_id"] <= 0:
        raise RevalidationError(f"CURRENT_REVISION_ID_INVALID:{page_id}")
    if not isinstance(current.get("revision_timestamp"), str) or re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", current["revision_timestamp"]) is None:
        raise RevalidationError(f"CURRENT_REVISION_TIMESTAMP_INVALID:{page_id}")
    if not isinstance(current.get("raw_utf8_sha256"), str) or re.fullmatch(r"[0-9a-f]{64}", current["raw_utf8_sha256"]) is None:
        raise RevalidationError(f"CURRENT_REVISION_DIGEST_INVALID:{page_id}")
    shape = pinned["g3_shape"]
    if shape.get("source_shape") != "STRUCTURED_PRIMARY" or shape.get("redirect") is not False:
        raise RevalidationError(f"PINNED_DIRECT_SOURCE_SHAPE_INVALID:{page_id}")
    if not isinstance(shape.get("discovery_roots"), list) or not isinstance(shape.get("source_surfaces"), list):
        raise RevalidationError(f"PINNED_DIRECT_PROVENANCE_INVALID:{page_id}")
    observation = {
        "lane": "G1_LIVE_NON_ITEM", "source_shape": "STRUCTURED_PRIMARY", "redirect": False,
        "discovery_roots": shape["discovery_roots"], "source_surfaces": shape["source_surfaces"],
        "templates": current.get("templates"), "categories": current.get("categories"),
    }
    try:
        signatures = classifier._g1_direct_signatures(observation)
    except (TypeError, ValueError) as exc:
        raise RevalidationError(f"CURRENT_FAMILY_SHAPE_AMBIGUOUS:{page_id}") from exc
    if signatures != [(pinned["family"], pinned["signature_id"])]:
        raise RevalidationError(f"CURRENT_FAMILY_SHAPE_UNSUPPORTED_OR_AMBIGUOUS:{page_id}")
    expected_signature = pinned["signature"]
    if expected_signature["template"] not in current["templates"]:
        raise RevalidationError(f"CURRENT_REQUIRED_TEMPLATE_MISSING:{page_id}")
    required_category = expected_signature.get("category")
    if required_category is not None and required_category not in current["categories"]:
        raise RevalidationError(f"CURRENT_REQUIRED_CATEGORY_MISSING:{page_id}")
    return {
        "state": "DIRECT_FAMILY_SOURCE_SHAPE_SUPPORTED",
        "direct_family_signature": pinned["signature_id"],
        "required_template_observed": expected_signature["template"],
        "required_category_observed": required_category,
    }


def build_revalidation(g3: dict[str, Any], g3_manifest: dict[str, Any], g4: dict[str, Any], g4_manifest: dict[str, Any], current_pages: dict[int, dict[str, Any]]) -> tuple[dict[str, Any], dict[str, Any]]:
    pinned = _validate_pinned_rows(g3, g3_manifest, g4, g4_manifest)
    if set(current_pages) != set(EXPECTED):
        raise RevalidationError("CURRENT_PAGE_ID_SET_MISMATCH")
    records = []
    for page_id in (63947, 46925):
        current = current_pages[page_id]
        family = EXPECTED[page_id]
        shape_result = validate_live_shape(page_id, pinned[page_id], current)
        records.append({
            "family": family,
            "source_identity": {
                "source_key": SOURCE_KEY, "source_namespace": SOURCE_NAMESPACE,
                "identity_namespace": "mediawiki/page_id", "external_id": str(page_id),
                "page_key": f"{SOURCE_NAMESPACE}/page_id/{page_id}",
            },
            "pinned_observations": pinned[page_id],
            "current_observation": {
                "source_key": SOURCE_KEY, "source_namespace": SOURCE_NAMESPACE,
                "identity_namespace": "mediawiki/page_id", "external_id": str(page_id),
                "page_key": f"{SOURCE_NAMESPACE}/page_id/{page_id}",
                "mediawiki_namespace_id": 0,
                "revision_id": current["revision_id"], "revision_timestamp": current["revision_timestamp"],
                "raw_utf8_sha256": current["raw_utf8_sha256"],
                "direct_family_source_shape": shape_result,
            },
        })
    artifact = {
        "schema": SCHEMA,
        "authority": {
            "exact_page_id_scope_only": True, "title_identity_used": False,
            "current_tibiawiki_primary": True, "second_wiki": "UNKNOWN", "ots": "HYPOTHESIS_ONLY",
            "canonical_identity_selection": False, "production_key_minting": False,
            "target_binding": False, "definition_population": False, "field_promotion": False,
            "presentation_asset_runtime_or_client_id_claims": False, "raw_page_content_retained": False,
        },
        "records": records,
    }
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "status": "G4_NONITEM_REVISION_REVALIDATION_SOURCE_ONLY",
        "repository": REPOSITORY,
        "allocation": {"issue": 162, "comment": 5822760897, "base_sha": "c516182255d3ea1724e91671c8d2187eb622e3df"},
        "inputs": {
            "g3": {"artifact_id": ARTIFACTS["g3"]["id"], "run_id": ARTIFACTS["g3"]["run_id"], "head_sha": ARTIFACTS["g3"]["head_sha"], "archive_sha256": ARTIFACTS["g3"]["digest"].removeprefix("sha256:")},
            "g4": {"artifact_id": ARTIFACTS["g4"]["id"], "run_id": ARTIFACTS["g4"]["run_id"], "head_sha": ARTIFACTS["g4"]["head_sha"], "archive_sha256": ARTIFACTS["g4"]["digest"].removeprefix("sha256:")},
        },
        "current_source": {"api": SOURCE_API, "primary": "TibiaWiki current page ID/revision", "second_wiki": "UNKNOWN", "ots": "HYPOTHESIS_ONLY"},
        "scope": {"page_ids": [63947, 46925], "families": {"63947": "Creature", "46925": "Quest"}, "records": len(records)},
        "invariants": {
            "exact_page_id_only": True, "revision_content_hashed_then_discarded": True,
            "only_direct_family_source_shape_rechecked": True, "no_title_identity": True,
            "no_canonical_or_gameplay_promotion": True, "workflow_artifact_only": True,
        },
        "artifact": {"schema": SCHEMA, "sha256": sha256_bytes(canonical_bytes(artifact)), "committed_bulk_corpus": False},
    }
    return artifact, manifest


def write_canonical(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_bytes(value))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--g3-zip", type=Path)
    parser.add_argument("--g4-zip", type=Path)
    parser.add_argument("--download-inputs", action="store_true")
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    if args.download_inputs:
        token = os.environ.get("GITHUB_TOKEN")
        if not token:
            raise RevalidationError("GITHUB_TOKEN_REQUIRED")
        args.g3_zip = args.work_dir / "g3-source-family.zip"
        args.g4_zip = args.work_dir / "g4-non-item-source.zip"
        download_artifact(token, "g3", args.g3_zip)
        download_artifact(token, "g4", args.g4_zip)
    if args.g3_zip is None or args.g4_zip is None:
        raise RevalidationError("BOTH_INPUT_ARCHIVES_REQUIRED")
    g3, g3_manifest, _ = load_artifact_zip(args.g3_zip, "g3")
    g4, g4_manifest, _ = load_artifact_zip(args.g4_zip, "g4")
    current = {page_id: fetch_current_page(page_id) for page_id in (63947, 46925)}
    artifact, manifest = build_revalidation(g3, g3_manifest, g4, g4_manifest, current)
    write_canonical(args.output, artifact)
    write_canonical(args.manifest_output, manifest)
    print(f"G4 non-Item revision revalidation: PASS ({len(artifact['records'])} exact page IDs)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RevalidationError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
