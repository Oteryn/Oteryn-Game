#!/usr/bin/env python3
"""Join pinned G3 direct non-Item family evidence to exact G4 page provenance.

This is an evidence-only inventory. It deliberately never reads page titles for
matching and never selects or emits a canonical target identity.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import urllib.error
import urllib.request
import zipfile
from urllib.parse import urlparse
from typing import Any

SCHEMA = "OTERYN_G4_DIRECT_NONITEM_FAMILY_CROSSWALK/v1"
MANIFEST_SCHEMA = "OTERYN_G4_DIRECT_NONITEM_FAMILY_CROSSWALK_MANIFEST/v1"
REPOSITORY = "Oteryn/Oteryn-Game"
API_ROOT = "https://api.github.com/repos/Oteryn/Oteryn-Game/actions/artifacts"
BASE_SHA = "6185045c20631b8614b1d4ad1dacc5d918f73b0a"
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_JSON_BYTES = 128 * 1024 * 1024

FAMILIES: dict[str, dict[str, Any]] = {
    "Creature": {"count": 2149, "signature": "G1_CREATURES_STWORZENIA_INFOBOX_CRIATURA_CRIATURAS"},
    "NPC": {"count": 1253, "signature": "G1_NPCS_NPC_E_INFOBOX_NPC"},
    "Achievement": {"count": 569, "signature": "G1_ACHIEVEMENTS_OSIAGNIECIA_INFOBOX_ACHIEVEMENT"},
    "Quest": {"count": 272, "signature": "G1_QUESTS_ZADANIA_INFOBOX_QUEST"},
    "Ability": {"count": 171, "signature": "G1_MAGICAL_ARCHIVE_INFOBOX_SPELL"},
    "Outfit": {"count": 134, "signature": "G1_OUTFITS_STROJE_INFOBOX_OUTFIT"},
    "Mount": {"count": 252, "signature": "G1_MOUNTS_MOCOWANIA_INFOBOX_MOUNT_MONTARIAS"},
}
EXPECTED_G3_COUNTS = {
    "Ability": 171, "Achievement": 569, "Creature": 2149, "Item": 5475,
    "Mount": 252, "NPC": 1253, "Outfit": 134, "Quest": 272, "UNKNOWN": 5512,
}
ARTIFACTS: dict[str, dict[str, Any]] = {
    "g3": {
        "id": 10801778929,
        "name": "source-family-classification-20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a",
        "size_in_bytes": 1427897,
        "digest": "sha256:a63635e4cdfcd237cf69b5e2fe0471c30723e45fe3ab2ccc9095730a9337d2e7",
        "run_id": 35986883931,
        "head_sha": "20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a",
        "members": ("source-family-classified-universe.json", "manifest.json"),
    },
    "g4": {
        "id": 10831362943,
        "name": "g4-non-item-source-capture-b2e77ba01677b484fdfa6271c5745d2280c13df6",
        "size_in_bytes": 690595,
        "digest": "sha256:5d8e886e31a0f64a035a61dc5c3f8ce031a8af3b06926117750deebc841394ca",
        "run_id": 36053194532,
        "head_sha": "b2e77ba01677b484fdfa6271c5745d2280c13df6",
        "members": ("non-item-source-capture.json", "manifest.json"),
    },
}
G3_CORPUS_SHA256 = "1562382c66ad471a46309eaa5ae2fe9e7b3b1e3c8c2d5fc047ee8faa65bd1666"
G4_CORPUS_SHA256 = "f47dbe5832e7b1accd652852303d638a4f19367bab3951f260cc39a0d93b7713"
SOURCE_NAMESPACE = "mediawiki/tibiawiki.com.br"
SOURCE_KEY = "TIBIAWIKI_STRUCTURED"
CANONICAL_ROLE_SUFFIXES = (
    "definitions/reference.json",
    "definitions/declarations.json",
    "presentations/bindings.json",
    "assets/catalog.json",
    "provenance/sources.json",
)
NON_PRODUCTION_PREFIXES = ("docs/", "tools/", "tests/", "vendor/")


class CrosswalkError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json_bytes(payload: bytes, label: str) -> dict[str, Any]:
    if len(payload) > MAX_JSON_BYTES:
        raise CrosswalkError(f"{label}_MAX_PLUS_ONE")
    try:
        value = json.loads(payload)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise CrosswalkError(f"{label}_JSON_INVALID") from exc
    if not isinstance(value, dict):
        raise CrosswalkError(f"{label}_ROOT_INVALID")
    return value


def api_json(url: str, token: str) -> dict[str, Any]:
    request = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "Oteryn-G4-Direct-NonItem-Crosswalk/1.0",
    })
    with urllib.request.urlopen(request, timeout=30) as response:
        value = json.load(response)
    if not isinstance(value, dict):
        raise CrosswalkError("GITHUB_API_OBJECT_INVALID")
    return value


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req: Any, fp: Any, code: int, msg: str, headers: Any, newurl: str) -> None:
        return None


def artifact_redirect(zip_api_url: str, token: str) -> str:
    request = urllib.request.Request(zip_api_url, headers={
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "Oteryn-G4-Direct-NonItem-Crosswalk/1.0",
    })
    try:
        urllib.request.build_opener(NoRedirect()).open(request, timeout=30)
    except urllib.error.HTTPError as error:
        if error.code != 302:
            raise CrosswalkError(f"ARTIFACT_ZIP_HTTP_{error.code}") from error
        location = error.headers.get("Location")
        if not location or urlparse(location).scheme != "https":
            raise CrosswalkError("ARTIFACT_REDIRECT_INVALID") from error
        return location
    raise CrosswalkError("ARTIFACT_ZIP_DID_NOT_REDIRECT")


def download_artifact(token: str, name: str, destination: Path) -> str:
    spec = ARTIFACTS[name]
    run = api_json(f"https://api.github.com/repos/{REPOSITORY}/actions/runs/{spec['run_id']}", token)
    if run.get("id") != spec["run_id"] or run.get("head_sha") != spec["head_sha"]:
        raise CrosswalkError(f"{name.upper()}_RUN_ID_OR_HEAD_MISMATCH")
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        raise CrosswalkError(f"{name.upper()}_RUN_NOT_SUCCESSFUL")
    metadata = api_json(f"{API_ROOT}/{spec['id']}", token)
    for field in ("id", "name", "size_in_bytes", "digest"):
        if metadata.get(field) != spec[field]:
            raise CrosswalkError(f"{name.upper()}_ARTIFACT_METADATA_MISMATCH:{field}")
    run_meta = metadata.get("workflow_run", {})
    if metadata.get("expired") is not False or run_meta.get("id") != spec["run_id"] or run_meta.get("head_sha") != spec["head_sha"]:
        raise CrosswalkError(f"{name.upper()}_ARTIFACT_RUN_OR_EXPIRY_MISMATCH")
    location = artifact_redirect(f"{API_ROOT}/{spec['id']}/zip", token)
    hasher = hashlib.sha256()
    size = 0
    destination.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(urllib.request.Request(location), timeout=120) as response, destination.open("wb") as output:
        while block := response.read(1024 * 1024):
            size += len(block)
            if size > min(spec["size_in_bytes"], MAX_ARCHIVE_BYTES):
                raise CrosswalkError(f"{name.upper()}_ARCHIVE_MAX_PLUS_ONE")
            hasher.update(block)
            output.write(block)
    digest = "sha256:" + hasher.hexdigest()
    if size != spec["size_in_bytes"] or digest != spec["digest"]:
        raise CrosswalkError(f"{name.upper()}_ARCHIVE_SIZE_OR_DIGEST_MISMATCH")
    return hasher.hexdigest()


def load_artifact_zip(path: Path, name: str) -> tuple[dict[str, Any], dict[str, Any], str]:
    spec = ARTIFACTS[name]
    payload = path.read_bytes()
    if len(payload) != spec["size_in_bytes"] or "sha256:" + sha256_bytes(payload) != spec["digest"]:
        raise CrosswalkError(f"{name.upper()}_ARCHIVE_SIZE_OR_DIGEST_MISMATCH")
    try:
        archive = zipfile.ZipFile(path)
    except zipfile.BadZipFile as exc:
        raise CrosswalkError(f"{name.upper()}_ZIP_INVALID") from exc
    with archive:
        if sorted(archive.namelist()) != sorted(spec["members"]):
            raise CrosswalkError(f"{name.upper()}_MEMBER_LIST_MISMATCH")
        corpus = read_json_bytes(archive.read(spec["members"][0]), f"{name.upper()}_CORPUS")
        manifest = read_json_bytes(archive.read(spec["members"][1]), f"{name.upper()}_MANIFEST")
    return corpus, manifest, sha256_bytes(payload)


def verify_g3(
    corpus: dict[str, Any],
    manifest: dict[str, Any],
    *,
    strict_artifact: bool = True,
    expected_counts: dict[str, int] = EXPECTED_G3_COUNTS,
) -> dict[int, dict[str, Any]]:
    if corpus.get("schema") != "OTERYN_SOURCE_FAMILY_CLASSIFIED_UNIVERSE/v1":
        raise CrosswalkError("G3_SCHEMA_INVALID")
    if manifest.get("schema") != "OTERYN_SOURCE_FAMILY_CLASSIFICATION_MANIFEST/v1":
        raise CrosswalkError("G3_MANIFEST_SCHEMA_INVALID")
    corpus_sha = sha256_bytes(canonical_bytes(corpus))
    if manifest.get("classified_universe_sha256") != corpus_sha or (strict_artifact and corpus_sha != G3_CORPUS_SHA256):
        raise CrosswalkError("G3_CORPUS_DIGEST_MISMATCH")
    if manifest.get("classified_page_count") != len(corpus.get("pages", [])) or any(manifest.get("counts", {}).get(family) != count for family, count in expected_counts.items()):
        raise CrosswalkError("G3_EXPECTED_COUNTS_MISMATCH")
    inputs = manifest.get("input", {})
    expected_spec = ARTIFACTS["g3"]
    if strict_artifact and (inputs.get("g2_artifact_id") != 10800249169 or inputs.get("g2_run_id") != 35982151518 or inputs.get("g2_head_sha") != "9c333fbba17f60cbf8eb107ecb623c5cda679d6c"):
        raise CrosswalkError("G3_PINNED_INPUT_MISMATCH")
    if manifest.get("schema") != "OTERYN_SOURCE_FAMILY_CLASSIFICATION_MANIFEST/v1" or (strict_artifact and expected_spec["id"] != 10801778929):
        raise CrosswalkError("G3_ARTIFACT_IDENTITY_MISMATCH")
    invariants = manifest.get("invariants", {})
    if invariants.get("candidate_relations_resolved") is not False or invariants.get("target_identity_selection_performed") is not False or invariants.get("semantic_promotion_performed") is not False:
        raise CrosswalkError("G3_AUTHORITY_BOUNDARY_INVALID")
    authority = corpus.get("authority", {})
    if authority.get("gameplay_truth") != "NONE" or authority.get("identity_resolution") != "NOT_PERFORMED" or authority.get("target_identity_selection") != "NOT_PERFORMED" or authority.get("semantic_promotion") != "NOT_PERFORMED":
        raise CrosswalkError("G3_CORPUS_AUTHORITY_BOUNDARY_INVALID")
    rows = corpus.get("pages")
    if not isinstance(rows, list) or len(rows) != manifest.get("classified_page_count"):
        raise CrosswalkError("G3_PAGE_COUNT_MISMATCH")
    indexed: dict[int, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("page_id"), int) or isinstance(row.get("page_id"), bool) or row["page_id"] <= 0:
            raise CrosswalkError("G3_PAGE_ROW_INVALID")
        if row["page_id"] in indexed:
            raise CrosswalkError(f"G3_DUPLICATE_PAGE_ID:{row['page_id']}")
        indexed[row["page_id"]] = row
    return indexed


def verify_g4(corpus: dict[str, Any], manifest: dict[str, Any], *, strict_artifact: bool = True) -> dict[int, dict[str, Any]]:
    if corpus.get("schema") != "OTERYN_G4_NON_ITEM_SOURCE_CAPTURE/v1":
        raise CrosswalkError("G4_SCHEMA_INVALID")
    if manifest.get("schema") != "OTERYN_G4_NON_ITEM_SOURCE_CAPTURE_MANIFEST/v1" or manifest.get("status") != "G4_NON_ITEM_SOURCE_PROVENANCE_ONLY":
        raise CrosswalkError("G4_MANIFEST_SCHEMA_OR_STATUS_INVALID")
    corpus_sha = sha256_bytes(canonical_bytes(corpus))
    if manifest.get("artifact", {}).get("sha256") != corpus_sha or (strict_artifact and corpus_sha != G4_CORPUS_SHA256):
        raise CrosswalkError("G4_CORPUS_DIGEST_MISMATCH")
    expected_counts = {"g1_live_unique_pages": 9373, "protected_item_pages": 6918, "g1_item_id_overlap_pages": 504, "unique_non_item_pages": 8869}
    if (strict_artifact and manifest.get("counts") != expected_counts) or (not strict_artifact and manifest.get("counts", {}).get("unique_non_item_pages") != len(corpus.get("pages", []))):
        raise CrosswalkError("G4_EXPECTED_COUNTS_MISMATCH")
    if corpus.get("authority", {}).get("identity_resolution") != "NOT_PERFORMED" or corpus.get("authority", {}).get("gameplay_truth") != "NONE":
        raise CrosswalkError("G4_AUTHORITY_BOUNDARY_INVALID")
    invariants = manifest.get("invariants", {})
    if invariants.get("raw_content_retained") is not False or invariants.get("normalized_fields_emitted") is not False or invariants.get("candidate_families_are_not_assignments") is not True or invariants.get("identity_promotion_performed") is not False or invariants.get("semantic_promotion_performed") is not False:
        raise CrosswalkError("G4_MANIFEST_AUTHORITY_BOUNDARY_INVALID")
    rows = corpus.get("pages")
    if not isinstance(rows, list) or (strict_artifact and len(rows) != 8869) or len(rows) != manifest.get("counts", {}).get("unique_non_item_pages"):
        raise CrosswalkError("G4_PAGE_COUNT_MISMATCH")
    indexed: dict[int, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise CrosswalkError("G4_PAGE_ROW_INVALID")
        page_id = row.get("page_id")
        if not isinstance(page_id, int) or isinstance(page_id, bool) or page_id <= 0 or page_id in indexed:
            raise CrosswalkError("G4_PAGE_ID_INVALID_OR_DUPLICATE")
        external_id = str(page_id)
        if row.get("source") != SOURCE_KEY or row.get("source_role") != "STRUCTURED_REFERENCE_DATA":
            raise CrosswalkError(f"G4_SOURCE_INVALID:{page_id}")
        if row.get("source_namespace") != SOURCE_NAMESPACE or row.get("external_id") != external_id or row.get("page_key") != f"{SOURCE_NAMESPACE}/page_id/{external_id}":
            raise CrosswalkError(f"G4_SOURCE_IDENTITY_INVALID:{page_id}")
        if not isinstance(row.get("revision_id"), int) or isinstance(row.get("revision_id"), bool) or row["revision_id"] <= 0:
            raise CrosswalkError(f"G4_REVISION_ID_INVALID:{page_id}")
        if not isinstance(row.get("revision_timestamp"), str) or re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", row["revision_timestamp"]) is None:
            raise CrosswalkError(f"G4_REVISION_TIMESTAMP_INVALID:{page_id}")
        if not isinstance(row.get("raw_utf8_sha256"), str) or re.fullmatch(r"[0-9a-f]{64}", row["raw_utf8_sha256"]) is None:
            raise CrosswalkError(f"G4_RAW_DIGEST_INVALID:{page_id}")
        indexed[page_id] = row
    return indexed


def list_tree_paths(repo_root: Path, commit: str) -> list[str]:
    if commit != BASE_SHA:
        raise CrosswalkError("REPOSITORY_ABSENCE_PROOF_BASE_MISMATCH")
    result = subprocess.run(["git", "-C", str(repo_root), "ls-tree", "-r", "--name-only", commit], check=True, text=True, capture_output=True)
    return [line for line in result.stdout.splitlines() if line]


def canonical_target_inventory(tracked_paths: list[str]) -> dict[str, Any]:
    """Find accepted WorldProject content documents, not test/example records."""
    matches = sorted({
        path for path in tracked_paths
        if any(path.endswith(suffix) for suffix in CANONICAL_ROLE_SUFFIXES)
        and not path.startswith(NON_PRODUCTION_PREFIXES)
        and "/tests/" not in f"/{path}/"
        and "/fixtures/" not in f"/{path}/"
    })
    return {
        "method": "exact tracked-path inventory at protected base; match accepted WorldProject/v1-v2 canonical role documents; exclude docs/tools/tests/fixtures/vendor examples",
        "protected_base_sha": BASE_SHA,
        "canonical_role_suffixes": list(CANONICAL_ROLE_SUFFIXES),
        "matching_production_documents": matches,
        "absence_proven": not matches,
        "evidence_only": True,
    }


def _validate_direct_row(page_id: int, row: dict[str, Any], family: str) -> dict[str, Any]:
    identity = row.get("source_family_classification")
    expected = FAMILIES[family]
    if not isinstance(identity, dict) or identity.get("primary_definition_family") != family or identity.get("state") != "SOURCE_DEFINITION_PRIMARY":
        raise CrosswalkError(f"G3_DIRECT_FAMILY_EVIDENCE_INVALID:{family}:{page_id}")
    if identity.get("target_identity_selected") is not False or identity.get("candidate_relations_resolved") is not False:
        raise CrosswalkError(f"G3_TARGET_OR_RELATIONSHIP_AUTHORITY_INVALID:{family}:{page_id}")
    if identity.get("basis_signature_ids") != [expected["signature"]] or identity.get("assignment_rule") != expected["signature"]:
        raise CrosswalkError(f"G3_DIRECT_SIGNATURE_MISMATCH:{family}:{page_id}")
    provenance = row.get("provenance")
    if not isinstance(provenance, list) or len(provenance) != 1:
        raise CrosswalkError(f"G3_PROVENANCE_CARDINALITY_INVALID:{family}:{page_id}")
    source_row = provenance[0]
    if not isinstance(source_row, dict) or source_row.get("lane") != "G1_LIVE_NON_ITEM" or source_row.get("source") != SOURCE_KEY:
        raise CrosswalkError(f"G3_SOURCE_PROVENANCE_INVALID:{family}:{page_id}")
    revision_id = source_row.get("revision_id")
    timestamp = source_row.get("revision_timestamp")
    if not isinstance(revision_id, int) or isinstance(revision_id, bool) or revision_id <= 0:
        raise CrosswalkError(f"G3_REVISION_ID_INVALID:{family}:{page_id}")
    if not isinstance(timestamp, str) or re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", timestamp) is None:
        raise CrosswalkError(f"G3_REVISION_TIMESTAMP_INVALID:{family}:{page_id}")
    return {"revision_id": revision_id, "revision_timestamp": timestamp}


def build_crosswalk(
    g3: dict[str, Any],
    g3_manifest: dict[str, Any],
    g4: dict[str, Any],
    g4_manifest: dict[str, Any],
    tracked_paths: list[str],
    *,
    expected_family_counts: dict[str, int] | None = None,
    strict_artifacts: bool = True,
) -> tuple[dict[str, Any], dict[str, Any]]:
    expected_counts = expected_family_counts or {family: int(spec["count"]) for family, spec in FAMILIES.items()}
    g3_rows = verify_g3(g3, g3_manifest, strict_artifact=strict_artifacts, expected_counts={**(EXPECTED_G3_COUNTS if strict_artifacts else {}), **expected_counts})
    g4_rows = verify_g4(g4, g4_manifest, strict_artifact=strict_artifacts)
    inventory = canonical_target_inventory(tracked_paths)
    output_rows: list[dict[str, Any]] = []
    family_summaries: dict[str, Any] = {}
    for family, expected in FAMILIES.items():
        selected = [row for row in g3_rows.values() if row.get("source_family_classification", {}).get("primary_definition_family") == family]
        if len(selected) != expected_counts.get(family):
            raise CrosswalkError(f"G3_FAMILY_COUNT_MISMATCH:{family}")
        revision_revalidation_ids: list[int] = []
        missing_count = 0
        for g3_row in sorted(selected, key=lambda item: item["page_id"]):
            page_id = g3_row["page_id"]
            if page_id not in g4_rows:
                raise CrosswalkError(f"G3_PAGE_MISSING_FROM_G4:{family}:{page_id}")
            g3_source = _validate_direct_row(page_id, g3_row, family)
            g4_source = g4_rows[page_id]
            same_revision = g3_source["revision_id"] == g4_source["revision_id"] and g3_source["revision_timestamp"] == g4_source["revision_timestamp"]
            if not same_revision:
                disposition = "SOURCE_REVISION_REVALIDATION_REQUIRED"
                reason = "G3 direct-family observation is not pinned to the current G4 page revision and timestamp"
                revision_revalidation_ids.append(page_id)
            elif inventory["absence_proven"]:
                disposition = "MISSING_CANONICAL_IDENTITY"
                reason = "No production canonical WorldProject definition corpus is tracked at the exact protected base"
                missing_count += 1
            else:
                disposition = "CANONICAL_TARGET_INVENTORY_REVIEW_REQUIRED"
                reason = "A canonical-role project document exists; this source-only crosswalk does not inspect or select targets"
            output_rows.append({
                "family": family,
                "source_key": SOURCE_KEY,
                "source_namespace": g4_source["source_namespace"],
                "identity_namespace": "mediawiki/page_id",
                "external_id": g4_source["external_id"],
                "page_key": g4_source["page_key"],
                "current_revision_id": g4_source["revision_id"],
                "current_revision_timestamp": g4_source["revision_timestamp"],
                "current_raw_utf8_sha256": g4_source["raw_utf8_sha256"],
                "g3_revision_id": g3_source["revision_id"],
                "g3_revision_timestamp": g3_source["revision_timestamp"],
                "direct_family_signature": FAMILIES[family]["signature"],
                "disposition": disposition,
                "reason": reason,
                "canonical_target": None,
            })
        family_summaries[family] = {
            "expected_g3_direct_rows": expected_counts[family],
            "exact_page_id_join_rows": len(selected),
            "source_revision_revalidation_required": len(revision_revalidation_ids),
            "revision_revalidation_page_ids": revision_revalidation_ids,
            "missing_canonical_identity": missing_count,
            "canonical_target_inventory_review_required": 0 if inventory["absence_proven"] else len(selected) - len(revision_revalidation_ids),
            "status": "SOURCE_ONLY_NO_TARGET_SELECTION",
        }
    if len(output_rows) != sum(expected_counts.values()):
        raise CrosswalkError("CROSSWALK_TOTAL_COUNT_MISMATCH")
    artifact = {
        "schema": SCHEMA,
        "authority": {
            "page_id_join_only": True,
            "title_matching_performed": False,
            "canonical_identity_selection": False,
            "source_identity_binding_emitted": False,
            "definition_population": False,
            "semantic_promotion": False,
            "presentation_asset_or_runtime_ids_emitted": False,
        },
        "repository_base": BASE_SHA,
        "source_family_classification": "G3 direct signature is source-family evidence only",
        "source_provenance": "G4 exact page identity/revision/digest only; no page content retained",
        "records": output_rows,
    }
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "status": "G4_DIRECT_NONITEM_CROSSWALK_SOURCE_ONLY",
        "repository": REPOSITORY,
        "protected_base_sha": BASE_SHA,
        "canonical_target_inventory": inventory,
        "inputs": {
            "g3": {
                "artifact_id": ARTIFACTS["g3"]["id"], "run_id": ARTIFACTS["g3"]["run_id"],
                "head_sha": ARTIFACTS["g3"]["head_sha"], "archive_sha256": ARTIFACTS["g3"]["digest"].removeprefix("sha256:"),
                "classified_universe_sha256": G3_CORPUS_SHA256, "classified_page_count": 15787,
            },
            "g4": {
                "artifact_id": ARTIFACTS["g4"]["id"], "run_id": ARTIFACTS["g4"]["run_id"],
                "head_sha": ARTIFACTS["g4"]["head_sha"], "archive_sha256": ARTIFACTS["g4"]["digest"].removeprefix("sha256:"),
                "capture_sha256": G4_CORPUS_SHA256, "unique_non_item_pages": 8869,
            },
        },
        "family_summaries": family_summaries,
        "totals": {
            "records": len(output_rows),
            "exact_revision_and_timestamp_joins": sum(1 for row in output_rows if row["disposition"] != "SOURCE_REVISION_REVALIDATION_REQUIRED"),
            "source_revision_revalidation_required": sum(1 for row in output_rows if row["disposition"] == "SOURCE_REVISION_REVALIDATION_REQUIRED"),
            "missing_canonical_identity": sum(1 for row in output_rows if row["disposition"] == "MISSING_CANONICAL_IDENTITY"),
            "canonical_target_inventory_review_required": sum(1 for row in output_rows if row["disposition"] == "CANONICAL_TARGET_INVENTORY_REVIEW_REQUIRED"),
        },
        "invariants": {
            "exact_page_id_only_join": True,
            "no_title_fields_used_for_join_or_targeting": True,
            "source_page_id_preserved_as_verbatim_decimal": True,
            "source_namespace_external_id_page_key_revision_timestamp_digest_preserved": True,
            "all_nonmatching_or_stale_rows_remain_evidence_only": True,
            "missing_canonical_identity_requires_repository_absence_proof": True,
            "no_source_identity_bindings": True,
            "no_definitions_or_population": True,
            "no_gameplay_semantics": True,
            "no_presentation_asset_or_runtime_ids": True,
            "workflow_artifact_only": True,
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
    parser.add_argument("--repository-root", type=Path, default=Path.cwd())
    parser.add_argument("--base-sha", default=BASE_SHA)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    if args.base_sha != BASE_SHA:
        raise CrosswalkError("PROTECTED_BASE_SHA_MISMATCH")
    if args.download_inputs:
        token = os.environ.get("GITHUB_TOKEN")
        if not token:
            raise CrosswalkError("GITHUB_TOKEN_REQUIRED")
        args.work_dir.mkdir(parents=True, exist_ok=True)
        args.g3_zip = args.work_dir / "g3-source-family.zip"
        args.g4_zip = args.work_dir / "g4-non-item-source.zip"
        download_artifact(token, "g3", args.g3_zip)
        download_artifact(token, "g4", args.g4_zip)
    if args.g3_zip is None or args.g4_zip is None:
        raise CrosswalkError("BOTH_INPUT_ARCHIVES_REQUIRED")
    g3, g3_manifest, _ = load_artifact_zip(args.g3_zip, "g3")
    g4, g4_manifest, _ = load_artifact_zip(args.g4_zip, "g4")
    paths = list_tree_paths(args.repository_root, args.base_sha)
    artifact, manifest = build_crosswalk(g3, g3_manifest, g4, g4_manifest, paths)
    write_canonical(args.output, artifact)
    write_canonical(args.manifest_output, manifest)
    print(json.dumps({"records": manifest["totals"]["records"], "totals": manifest["totals"], "artifact_sha256": manifest["artifact"]["sha256"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except CrosswalkError as exc:
        print(f"g4-direct-nonitem-family-crosswalk: FAIL {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
