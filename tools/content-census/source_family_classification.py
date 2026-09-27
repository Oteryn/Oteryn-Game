#!/usr/bin/env python3
"""Classify the first G3 cohort from pinned, structured source-definition signatures.

The output remains reference-source evidence. It does not resolve identities,
consume Item crosswalk dispositions, or promote gameplay semantics.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Any

REPOSITORY = "Oteryn/Oteryn-Game"
ARTIFACT = {
    "id": 10800249169,
    "name": "global-source-overlap-9c333fbba17f60cbf8eb107ecb623c5cda679d6c",
    "size_in_bytes": 1246747,
    "digest": "sha256:8b5a78e6daa72b5120a2218d395e498c8c2aa236bf7152f730f67ea99693af50",
    "run_id": 35982151518,
    "head_sha": "9c333fbba17f60cbf8eb107ecb623c5cda679d6c",
    "members": ("global-source-universe.json", "manifest.json"),
}
EXPECTED_UNIVERSE_SHA256 = "59b6c85e18a6e55b65491f53fa701b9596009228e6e06ea7a3b0b6d493d47d6c"
EXPECTED_COUNTS = {
    "g1_live_unique_pages": 9373,
    "protected_item_pages": 6918,
    "exact_id_overlap_pages": 504,
    "cross_lane_title_divergence_pages": 0,
    "global_unique_pages": 15787,
}
HARD_EXCLUSIONS = {"Kalkulatory", "Narzędzie do nasycania", "Dostawca"}

# Each G1 rule is a conjunctive structured-source signature. Canonical source
# categories are an additional guard for Creature and Mount list-page ambiguity.
G1_SIGNATURES = {
    "Creature": {
        "id": "G1_CREATURES_STWORZENIA_INFOBOX_CRIATURA_CRIATURAS",
        "root": "creatures",
        "surface": "Stworzenia",
        "template": "Predefinição:Infobox Criatura",
        "category": "Categoria:Criaturas",
    },
    "NPC": {
        "id": "G1_NPCS_NPC_E_INFOBOX_NPC",
        "root": "npcs",
        "surface": "NPC-e",
        "template": "Predefinição:Infobox NPC",
    },
    "Achievement": {
        "id": "G1_ACHIEVEMENTS_OSIAGNIECIA_INFOBOX_ACHIEVEMENT",
        "root": "achievements",
        "surface": "Osiągnięcia",
        "template": "Predefinição:Infobox Achievement",
    },
    "Mount": {
        "id": "G1_MOUNTS_MOCOWANIA_INFOBOX_MOUNT_MONTARIAS",
        "root": "mounts",
        "surface": "Mocowania",
        "template": "Predefinição:Infobox Mount",
        "category": "Categoria:Montarias",
    },
    "Outfit": {
        "id": "G1_OUTFITS_STROJE_INFOBOX_OUTFIT",
        "root": "outfits",
        "surface": "Stroje",
        "template": "Predefinição:Infobox Outfit",
    },
    "Quest": {
        "id": "G1_QUESTS_ZADANIA_INFOBOX_QUEST",
        "root": "quests",
        "surface": "Zadania",
        "template": "Predefinição:Infobox Quest",
    },
    "Ability": {
        "id": "G1_MAGICAL_ARCHIVE_INFOBOX_SPELL",
        "root": "magical-archive",
        "surface": "Magiczne Archiwum",
        "template": "Predefinição:Infobox Spell",
    },
}


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_pinned_universe(universe: dict[str, Any], manifest: dict[str, Any], archive_sha256: str) -> None:
    _require(universe.get("schema") == "OTERYN_GLOBAL_SOURCE_ID_UNIVERSE/v1", "unexpected G2 universe schema")
    _require(manifest.get("schema") == "OTERYN_GLOBAL_SOURCE_OVERLAP_MANIFEST/v1", "unexpected G2 manifest schema")
    _require(archive_sha256 == ARTIFACT["digest"].removeprefix("sha256:"), "G2 artifact ZIP digest mismatch")
    _require(sha256_bytes(canonical_bytes(universe)) == EXPECTED_UNIVERSE_SHA256, "G2 universe content digest mismatch")
    _require(manifest.get("global_universe_sha256") == EXPECTED_UNIVERSE_SHA256, "G2 manifest universe digest mismatch")
    _require(manifest.get("counts") == EXPECTED_COUNTS, "G2 input counts mismatch")
    _require(universe.get("counts") == EXPECTED_COUNTS, "G2 universe counts mismatch")
    _require(universe.get("identity_basis") == "EXACT_MEDIAWIKI_PAGE_ID", "G2 identity basis mismatch")
    _require(universe.get("authority") == {
        "identity_resolution": "NOT_PERFORMED",
        "canonical_identity_selection": "NOT_PERFORMED",
        "semantic_promotion": "NOT_PERFORMED",
        "gameplay_truth": "NONE",
    }, "G2 authority boundary mismatch")
    _require(manifest.get("invariants", {}).get("semantic_promotion_performed") is False, "G2 semantic-promotion invariant missing")
    _require(manifest.get("invariants", {}).get("canonical_identity_selection_performed") is False, "G2 canonical-identity invariant missing")
    inputs = manifest.get("input_digests", {})
    _require(inputs.get("g1_artifact_id") == "10798668295", "G2 G1 artifact bind mismatch")
    _require(inputs.get("g1_run_id") == "35977349690", "G2 G1 run bind mismatch")
    _require(inputs.get("protected_item_crosswalk_artifact_id") == "10778892407", "G2 Item artifact bind mismatch")
    _require(inputs.get("protected_item_crosswalk_run_id") == "35925860576", "G2 Item run bind mismatch")
    _require(inputs.get("g1_archive_sha256") == "bf08a2715891d138b5db4ed4be0a69034770315e8d67f3e83867f22b9afc9863", "G2 G1 archive bind mismatch")
    _require(inputs.get("protected_item_crosswalk_archive_sha256") == "834c10d3dfb24b8857ffd666046743c96ee4093e7e09ef170dcd477ffea91b43", "G2 Item archive bind mismatch")
    _require(inputs.get("g1_head_sha") == "f1d7dbd6577b033c53545d8650ffba05aafc9480", "G2 G1 head bind mismatch")
    _require(inputs.get("protected_item_crosswalk_head_sha") == "61d051a13329c51ae04d8a011655e279664c334c", "G2 Item head bind mismatch")
    _require(inputs.get("g1_stable_without_retrieval_timestamp_sha256") == "17f72a8f63861244b3193e639c3e33a530b641b77a7539653cc066a70c093c6c", "G2 G1 stable digest mismatch")
    _require(inputs.get("protected_item_stable_digest") == "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a", "G2 Item stable digest mismatch")
    _require(isinstance(universe.get("pages"), list) and len(universe["pages"]) == EXPECTED_COUNTS["global_unique_pages"], "G2 page population mismatch")


def _g1_direct_signatures(provenance: dict[str, Any]) -> list[tuple[str, str]]:
    if provenance.get("lane") != "G1_LIVE_NON_ITEM":
        return []
    if provenance.get("source_shape") != "STRUCTURED_PRIMARY" or provenance.get("redirect") is not False:
        return []
    roots = provenance.get("discovery_roots", [])
    surfaces = provenance.get("source_surfaces", [])
    templates = provenance.get("templates", [])
    categories = provenance.get("categories", [])
    if not all(isinstance(value, list) for value in (roots, surfaces, templates, categories)):
        raise ValueError("malformed G1 provenance arrays")
    matches = []
    for family, signature in G1_SIGNATURES.items():
        if signature["root"] not in roots or signature["surface"] not in surfaces or signature["template"] not in templates:
            continue
        category = signature.get("category")
        if category is not None and category not in categories:
            continue
        matches.append((family, signature["id"]))
    return matches


def direct_signatures(page: dict[str, Any]) -> list[tuple[str, str]]:
    provenance = page.get("provenance")
    if not isinstance(provenance, list):
        raise ValueError("G2 page provenance must be a list")
    matches: list[tuple[str, str]] = []
    for item in provenance:
        if not isinstance(item, dict):
            raise ValueError("G2 provenance row must be an object")
        if (item.get("lane") == "PROTECTED_ITEM" and item.get("source_shape") == "INFOBOX_ITEM"
                and item.get("redirect") is not True):
            matches.append(("Item", "PROTECTED_ITEM_INFOBOX_ITEM"))
        matches.extend(_g1_direct_signatures(item))
    return sorted(set(matches))


def classify_page(page: dict[str, Any]) -> dict[str, Any]:
    signatures = direct_signatures(page)
    if len(signatures) == 1:
        family, signature_id = signatures[0]
        state = "SOURCE_DEFINITION_PRIMARY"
        basis = signature_id
    elif not signatures:
        family = "UNKNOWN"
        state = "UNKNOWN"
        basis = "NO_STRICT_DIRECT_SIGNATURE"
    else:
        family = "UNKNOWN"
        state = "UNKNOWN"
        basis = "MULTIPLE_STRICT_DIRECT_SIGNATURES"
    g1_evidence = []
    for provenance in page.get("provenance", []):
        if provenance.get("lane") == "G1_LIVE_NON_ITEM":
            g1_evidence.append({
                "artifact_id": provenance.get("artifact_id"),
                "observed_title": provenance.get("observed_title"),
                "source_shape": provenance.get("source_shape"),
                "discovery_roots": provenance.get("discovery_roots", []),
                "source_surfaces": provenance.get("source_surfaces", []),
                "candidate_families_evidence_only": provenance.get("candidate_families_evidence_only", []),
                "family_classification_state_evidence_only": provenance.get("family_classification_state_evidence_only"),
            })
    return {
        "primary_definition_family": family,
        "state": state,
        "basis_signature_ids": [signature_id for _, signature_id in signatures],
        "assignment_rule": basis,
        "candidate_relationships_evidence_only": g1_evidence,
        "candidate_relations_resolved": False,
        "target_identity_selected": False,
        "gameplay_semantics_promoted": False,
    }


def classify_universe(universe: dict[str, Any], manifest: dict[str, Any], archive_sha256: str) -> tuple[dict[str, Any], dict[str, Any]]:
    validate_pinned_universe(universe, manifest, archive_sha256)
    seen: set[int] = set()
    output_pages = []
    counts: dict[str, int] = {"Item": 0, "Creature": 0, "NPC": 0, "Achievement": 0, "Mount": 0, "Outfit": 0, "Quest": 0, "Ability": 0, "UNKNOWN": 0}
    unknown_shapes: dict[str, int] = {}
    for page in universe["pages"]:
        if not isinstance(page, dict):
            raise ValueError("G2 page must be an object")
        page_id = page.get("page_id")
        if isinstance(page_id, bool) or not isinstance(page_id, int) or page_id <= 0:
            raise ValueError(f"invalid G2 page_id {page_id!r}")
        if page_id in seen:
            raise ValueError(f"duplicate G2 page_id {page_id}")
        seen.add(page_id)
        for observation in page.get("title_observations", []):
            if observation.get("title") in HARD_EXCLUSIONS:
                raise ValueError(f"hard-excluded title present: {observation['title']}")
        classification = classify_page(page)
        family = classification["primary_definition_family"]
        counts[family] += 1
        if family == "UNKNOWN":
            shapes = [row.get("source_shape", "MISSING") for row in page.get("provenance", [])]
            for shape in shapes or ["MISSING"]:
                unknown_shapes[shape] = unknown_shapes.get(shape, 0) + 1
        output_pages.append({**page, "source_family_classification": classification})
    _require(len(seen) == EXPECTED_COUNTS["global_unique_pages"], "G2 exact-ID population mismatch")
    expected_cohort = {"Item": 5475, "Creature": 2149, "NPC": 1253, "Achievement": 569, "Mount": 252, "Outfit": 134, "Quest": 272, "Ability": 171}
    observed_cohort = {key: counts[key] for key in expected_cohort}
    _require(observed_cohort == expected_cohort, f"G3 direct cohort count mismatch: {observed_cohort}")
    _require(counts["UNKNOWN"] == 5512, f"G3 UNKNOWN count mismatch: {counts['UNKNOWN']}")
    _require(sum(observed_cohort.values()) == 10275, "G3 direct family count total mismatch")
    output = {
        "schema": "OTERYN_SOURCE_FAMILY_CLASSIFIED_UNIVERSE/v1",
        "classification_basis": "SOURCE_DEFINITION_DIRECT_SIGNATURES_ONLY",
        "authority": {
            "source_family_classification": "SOURCE_EVIDENCE_ONLY",
            "candidate_relationship_resolution": "NOT_PERFORMED",
            "identity_resolution": "NOT_PERFORMED",
            "canonical_identity_selection": "NOT_PERFORMED",
            "target_identity_selection": "NOT_PERFORMED",
            "semantic_promotion": "NOT_PERFORMED",
            "gameplay_truth": "NONE",
        },
        "input": {
            "g2_artifact_id": ARTIFACT["id"],
            "g2_artifact_name": ARTIFACT["name"],
            "g2_run_id": ARTIFACT["run_id"],
            "g2_head_sha": ARTIFACT["head_sha"],
            "g2_archive_sha256": archive_sha256,
            "g2_universe_sha256": EXPECTED_UNIVERSE_SHA256,
        },
        "counts": counts,
        "unknown_source_shape_counts": dict(sorted(unknown_shapes.items())),
        "pages": output_pages,
    }
    output_digest = sha256_bytes(canonical_bytes(output))
    result_manifest = {
        "schema": "OTERYN_SOURCE_FAMILY_CLASSIFICATION_MANIFEST/v1",
        "status": "PARTIAL_SOURCE_DEFINITION_CLASSIFICATION_NO_IDENTITY_OR_SEMANTIC_PROMOTION",
        "input": output["input"],
        "input_schema": universe["schema"],
        "input_counts": universe["counts"],
        "input_manifest_sha256": sha256_bytes(canonical_bytes(manifest)),
        "classified_universe_sha256": output_digest,
        "classified_page_count": len(output_pages),
        "counts": counts,
        "unknown_source_shape_counts": dict(sorted(unknown_shapes.items())),
        "signature_registry": G1_SIGNATURES,
        "invariants": {
            "exact_page_id_only": True,
            "same_title_distinct_ids_preserved": True,
            "source_shape_preserved": True,
            "g1_candidate_relationships_retained_as_evidence_only": True,
            "candidate_relations_resolved": False,
            "identity_resolution_performed": False,
            "canonical_identity_selection_performed": False,
            "target_identity_selection_performed": False,
            "semantic_promotion_performed": False,
            "unknown_pages_retained": True,
        },
    }
    return output, result_manifest


def fetch_pinned_g2_artifact(out_dir: Path) -> tuple[Path, Path, str]:
    """Reuse the protected G2 artifact downloader's generic verified functions."""
    try:
        from global_source_overlap_fetch_artifacts import download_archive, extract_exact_members
    except ImportError as error:
        raise RuntimeError("protected G2 downloader is unavailable") from error
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        raise RuntimeError("GITHUB_TOKEN is required to fetch pinned G2 artifact")
    out_dir.mkdir(parents=True, exist_ok=True)
    archive_dir = out_dir / "zip"
    archive_dir.mkdir(exist_ok=True)
    archive = archive_dir / f"g2-{ARTIFACT['id']}.zip"
    archive_sha256 = download_archive(ARTIFACT, token, archive)
    extracted = extract_exact_members(archive, ARTIFACT, out_dir, "g2")
    by_name = {path.name: path for path in extracted}
    universe = by_name["g2-global-source-universe.json"]
    manifest = by_name["g2-manifest.json"]
    return universe, manifest, archive_sha256


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--input-dir", help="local pinned G2 extracted inputs for offline reproduction")
    parser.add_argument("--archive-sha256", help="required with --input-dir")
    parser.add_argument("--work-dir", default=".g3-work", help="temporary artifact download directory")
    args = parser.parse_args()
    if args.input_dir:
        if not args.archive_sha256:
            raise SystemExit("--archive-sha256 is required with --input-dir")
        input_dir = Path(args.input_dir)
        universe_path = input_dir / "global-source-universe.json"
        manifest_path = input_dir / "manifest.json"
        archive_sha256 = args.archive_sha256
    else:
        universe_path, manifest_path, archive_sha256 = fetch_pinned_g2_artifact(Path(args.work_dir))
    result, result_manifest = classify_universe(load_json(universe_path), load_json(manifest_path), archive_sha256)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "source-family-classified-universe.json").write_bytes(canonical_bytes(result))
    (out / "manifest.json").write_bytes(canonical_bytes(result_manifest))
    print(json.dumps(result_manifest, ensure_ascii=False, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
