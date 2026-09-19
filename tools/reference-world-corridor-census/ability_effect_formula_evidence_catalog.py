#!/usr/bin/env python3
"""Build the deterministic CW2-B4 Ability -> Effect -> Formula evidence catalogue.

This is an offline, candidate-only evidence mapper.  It consumes only protected
Game repository objects.  It never mints native ContentKeys, promotes Reference
parity, or supplies an executable quantitative formula.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
from typing import Any, Iterable, Mapping

SCHEMA = "OTERYN_CW2_ABILITY_EFFECT_FORMULA_EVIDENCE_BATCH/v1"
MAPPER_PROFILE = "OTERYN_CW2_ABILITY_EFFECT_FORMULA_EVIDENCE_MAPPER/v1"
TASK = "CW2-B4 ABILITY_EFFECT_FORMULA_EVIDENCE_504"
ADMISSION_MAIN = "03a821edd828e24ccff6e2cb7fc819a776cbd238"
CLOSURE = "CANDIDATE_ONLY"

MANIFEST_PATH = "docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json"
REFERENCE_SURFACE_PATH = "apps/game-server/src/content/reference_playable.rs"
MAPPER_PATH = (
    "tools/reference-world-corridor-census/"
    "ability_effect_formula_evidence_catalog.py"
)

PROTECTED_INPUTS: dict[str, tuple[str, str]] = {
    MANIFEST_PATH: (
        "f5828732038ac2fd3ae03f3d1793505d48a61122",
        "REFERENCE_CASE_AUTHORITY",
    ),
    "docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md": (
        "5c9961a99616839a40b3ca933ac4371f93ffca48",
        "PENDING_FIXTURE_BOUNDARY",
    ),
    "docs/agents/evidence/OTV2-20260814-ability-combat-official-spell-library.md": (
        "6ada6c52a20beae37abfecb1e2792a36f5dba8ca",
        "INDEXED_OFFICIAL_DISCOVERY_EVIDENCE",
    ),
    "docs/agents/evidence/OTV2-20260815-ability-combat-reference-continuity.md": (
        "704e57840d0d3a1c84284e650c7c023d171d97dc",
        "TARGET_CONTINUITY_DISPOSITION",
    ),
    "docs/agents/evidence/OTV2-20260912-reference-combat-light-healing.md": (
        "4a96e9859f6c425d3c03be09bc7712fc4cb6cf83",
        "LIGHT_HEALING_FORMULA_DISPOSITION",
    ),
    REFERENCE_SURFACE_PATH: (
        "ec5fa303fa8a09f055c8043a560f9a6120dee6cb",
        "PROTECTED_TYPED_FAMILY_SURFACE",
    ),
}

CASE_IDS = (
    "ability_combat.light_healing.cast_metadata.v1",
    "ability_combat.light_healing.self_heal_semantics.v1",
    "ability_combat.ice_strike.cast_metadata.v1",
    "ability_combat.ice_strike.targeted_ice_damage_semantics.v1",
)

ABILITY_SPECS: dict[str, dict[str, Any]] = {
    "light_healing": {
        "display_name": "Light Healing",
        "incantation_candidate": "exura",
        "case_ids": CASE_IDS[:2],
        "effect": {
            "candidate_family": "HEAL",
            "candidate_shape": "SELF_HEAL",
            "protected_family_symbol": "ReferenceEffectFamily::Heal",
        },
        "formula_unknowns": (
            "minimum_heal_function",
            "maximum_heal_function",
            "rng_distribution",
            "intermediate_rounding",
            "final_rounding",
            "scaling_coefficients",
        ),
    },
    "ice_strike": {
        "display_name": "Ice Strike",
        "incantation_candidate": "exori frigo",
        "case_ids": CASE_IDS[2:],
        "effect": {
            "candidate_family": "DAMAGE",
            "candidate_shape": "TARGETED_ICE_DAMAGE",
            "protected_family_symbol": "ReferenceEffectFamily::Damage",
        },
        "formula_unknowns": (
            "minimum_damage_function",
            "maximum_damage_function",
            "rng_distribution",
            "rounding",
            "scaling_coefficients",
            "mitigation_and_resistance_ordering",
            "critical_block_dodge_ordering",
        ),
    },
}


class CatalogError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_repository_text_bytes(value: bytes) -> bytes:
    """Canonicalize checkout text bytes to repository LF form."""
    without_crlf = value.replace(b"\r\n", b"")
    if b"\r" in without_crlf:
        raise CatalogError("UNSUPPORTED_MAPPER_LINE_ENDING")
    return value.replace(b"\r\n", b"\n")


def canonical_records(records: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    return sorted((copy.deepcopy(record) for record in records), key=canonical_bytes)


def _git(repo: Path, *args: str, binary: bool = False) -> str | bytes:
    try:
        result = subprocess.run(
            ("git", "-C", str(repo), *args),
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError(f"GIT_OBJECT_UNAVAILABLE: {' '.join(args)}") from exc
    if binary:
        return result.stdout
    return result.stdout.decode("utf-8").strip()


def verify_protected_inputs(
    game_root: Path,
    expected_inputs: Mapping[str, tuple[str, str]] = PROTECTED_INPUTS,
    expected_sha256: Mapping[str, str] | None = None,
) -> tuple[list[dict[str, Any]], dict[str, bytes]]:
    game_root = game_root.resolve()
    top = Path(str(_git(game_root, "rev-parse", "--show-toplevel"))).resolve()
    if top != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
    if _git(game_root, "rev-parse", f"{ADMISSION_MAIN}^{{commit}}") != ADMISSION_MAIN:
        raise CatalogError("ADMISSION_MAIN_UNAVAILABLE")

    records: list[dict[str, Any]] = []
    payloads: dict[str, bytes] = {}
    for path, (expected_blob, role) in sorted(expected_inputs.items()):
        status = str(_git(game_root, "status", "--porcelain=v1", "--", path))
        if status:
            raise CatalogError(f"PROTECTED_INPUT_DIRTY: {path}")
        blob = str(_git(game_root, "rev-parse", f"{ADMISSION_MAIN}:{path}"))
        if blob != expected_blob:
            raise CatalogError(
                f"GIT_BLOB_MISMATCH: {path}: expected {expected_blob}, got {blob}"
            )
        payload = _git(game_root, "cat-file", "blob", blob, binary=True)
        assert isinstance(payload, bytes)
        digest = sha256_bytes(payload)
        if expected_sha256 is not None:
            expected_digest = expected_sha256.get(path)
            if expected_digest is not None and digest != expected_digest:
                raise CatalogError(
                    f"SHA256_MISMATCH: {path}: expected {expected_digest}, got {digest}"
                )
        payloads[path] = payload
        records.append(
            {
                "path": path,
                "blob": blob,
                "sha256": digest,
                "size": len(payload),
                "role": role,
            }
        )
    return records, payloads


def verify_mapper_revision(game_root: Path) -> dict[str, Any]:
    game_root = game_root.resolve()
    status = str(_git(game_root, "status", "--porcelain=v1", "--", MAPPER_PATH))
    if status:
        raise CatalogError(f"MAPPER_DIRTY: {MAPPER_PATH}")
    blob = str(_git(game_root, "rev-parse", f"HEAD:{MAPPER_PATH}"))
    payload = _git(game_root, "cat-file", "blob", blob, binary=True)
    assert isinstance(payload, bytes)
    canonical = canonical_repository_text_bytes(payload)
    return {
        "profile": MAPPER_PROFILE,
        "path": MAPPER_PATH,
        "git_blob": blob,
        "canonicalization": "repository text bytes; CRLF normalized to LF; lone CR rejected",
        "canonical_size": len(canonical),
        "canonical_sha256": sha256_bytes(canonical),
    }


def _load_manifest(payload: bytes) -> dict[str, Any]:
    try:
        manifest = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CatalogError("REFERENCE_MANIFEST_INVALID_JSON") from exc
    if not isinstance(manifest, dict):
        raise CatalogError("REFERENCE_MANIFEST_ROOT_INVALID")
    return manifest


def validate_manifest_cases(
    manifest: dict[str, Any], case_ids: Iterable[str] = CASE_IDS
) -> list[dict[str, Any]]:
    by_id = {case.get("case_id"): case for case in manifest.get("cases", [])}
    selected: list[dict[str, Any]] = []
    for case_id in case_ids:
        case = by_id.get(case_id)
        if case is None:
            raise CatalogError(f"REFERENCE_CASE_MISSING: {case_id}")
        target = case.get("target", {})
        provenance = case.get("provenance", {})
        parity = case.get("parity", {})
        oteryn = case.get("oteryn", {})
        required = {
            "domain": case.get("domain"),
            "target_evidence": target.get("evidence_class"),
            "provenance": provenance.get("state"),
            "legal_review": provenance.get("legal_review_state"),
            "parity": parity.get("status"),
            "implementation": oteryn.get("implementation_state"),
            "exact_revision": oteryn.get("exact_revision"),
        }
        expected = {
            "domain": "ABILITY_COMBAT",
            "target_evidence": "UNKNOWN",
            "provenance": "PENDING",
            "legal_review": "PENDING",
            "parity": "PARITY_PENDING_EVIDENCE",
            "implementation": "NOT_STARTED",
            "exact_revision": None,
        }
        if required != expected:
            raise CatalogError(
                f"REFERENCE_CASE_FAIL_CLOSED_STATE_MISMATCH: {case_id}: {required}"
            )
        selected.append(
            {
                "case_id": case_id,
                "title": case.get("title"),
                "target_evidence": "UNKNOWN",
                "source_provenance": "PENDING",
                "legal_review": "PENDING",
                "parity": "PARITY_PENDING_EVIDENCE",
                "oteryn_implementation": "NOT_STARTED",
            }
        )
    return canonical_records(selected)


def _verify_semantic_markers(payloads: Mapping[str, bytes]) -> None:
    required: dict[str, tuple[bytes, ...]] = {
        REFERENCE_SURFACE_PATH: (
            b"Ability,",
            b"Effect,",
            b"Formula,",
            b"pub enum ReferenceEffectFamily",
            b"Damage,",
            b"Heal,",
        ),
        "docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md": (
            b"PENDING_TARGET_CONTINUITY_PROVENANCE_LEGAL_IMPLEMENTATION",
            b"No target behavior is promoted above `UNKNOWN`",
        ),
        "docs/agents/evidence/OTV2-20260815-ability-combat-reference-continuity.md": (
            b"0 of 4 cases promoted",
            b"UNKNOWN / PENDING",
        ),
        "docs/agents/evidence/OTV2-20260912-reference-combat-light-healing.md": (
            b"exact_min_formula:",
            b"exact_max_formula:",
            b"admissible_as_exact_reference_formula: false",
        ),
    }
    for path, needles in required.items():
        payload = payloads[path]
        for needle in needles:
            if needle not in payload:
                raise CatalogError(
                    f"PROTECTED_SEMANTIC_MARKER_MISSING: {path}: "
                    f"{needle.decode('utf-8', 'replace')}"
                )


def _ability_record(slug: str, spec: dict[str, Any]) -> dict[str, Any]:
    return {
        "source_candidate_id": f"reference-source:ability:{slug}",
        "display_name": spec["display_name"],
        "incantation_candidate": spec["incantation_candidate"],
        "reference_case_ids": sorted(spec["case_ids"]),
        "target_evidence": "UNKNOWN",
        "source_provenance": "PENDING",
        "legal_review": "PENDING",
        "parity": "PARITY_PENDING_EVIDENCE",
        "native_ability_identity": {
            "disposition": "UNRESOLVED",
            "content_key": None,
            "reason": "NO_ACCEPTED_NATIVE_ABILITY_BINDING",
        },
        "ability_to_effect": {
            "disposition": "CANDIDATE_ONLY",
            **spec["effect"],
            "reason": "INDEXED_QUALITATIVE_SHAPE_LACKS_TARGET_CONTINUITY_AND_CLEARANCE",
        },
        "effect_to_formula": {
            "disposition": "UNRESOLVED",
            "quantitative_formula": None,
            "formula_state": "UNKNOWN",
            "unknown_fields": sorted(spec["formula_unknowns"]),
            "reason": "EXACT_TARGET_FORMULA_NOT_EVIDENCED",
        },
        "executable_promotion": {
            "disposition": "BLOCKED",
            "reason_codes": [
                "TARGET_EVIDENCE_UNKNOWN",
                "SOURCE_PROVENANCE_PENDING",
                "LEGAL_REVIEW_PENDING",
                "NATIVE_IDENTITY_UNRESOLVED",
                "EXACT_FORMULA_UNKNOWN",
            ],
        },
    }


def build_catalog(
    game_root: Path,
    *,
    case_ids: Iterable[str] = CASE_IDS,
    expected_inputs: Mapping[str, tuple[str, str]] = PROTECTED_INPUTS,
    expected_sha256: Mapping[str, str] | None = None,
) -> dict[str, Any]:
    protected, payloads = verify_protected_inputs(
        game_root, expected_inputs, expected_sha256
    )
    mapper_revision = verify_mapper_revision(game_root)
    _verify_semantic_markers(payloads)
    manifest = _load_manifest(payloads[MANIFEST_PATH])
    cases = validate_manifest_cases(manifest, case_ids)
    selected = {record["case_id"] for record in cases}
    if selected != set(CASE_IDS):
        raise CatalogError("REFERENCE_CASE_SET_MISMATCH")
    abilities = canonical_records(
        _ability_record(slug, spec) for slug, spec in ABILITY_SPECS.items()
    )
    value: dict[str, Any] = {
        "schema": SCHEMA,
        "mapper_profile": MAPPER_PROFILE,
        "mapper_revision": mapper_revision,
        "task": TASK,
        "admission_main": ADMISSION_MAIN,
        "closure": CLOSURE,
        "production_authority": "NONE",
        "source_snapshot": {
            "repository": "Oteryn/Oteryn-Game",
            "revision": ADMISSION_MAIN,
            "protected_inputs": protected,
        },
        "reference_boundary": {
            "target": manifest.get("reference_target"),
            "manifest_revision": manifest.get("manifest_revision"),
            "cases": cases,
        },
        "typed_surface": {
            "protected_definition_families": ["Ability", "Effect", "Formula"],
            "protected_effect_families": ["Damage", "Heal"],
            "scope": "CLASSIFICATION_ONLY_NO_SHARED_MODEL_WRITE",
        },
        "ability_effect_formula_candidates": abilities,
        "counts": {
            "abilities": len(abilities),
            "reference_cases": len(cases),
            "candidate_effect_bindings": len(abilities),
            "resolved_native_ability_identities": 0,
            "resolved_exact_quantitative_formulas": 0,
            "executable_promotions": 0,
        },
        "loss_report": {
            "silently_dropped_records": 0,
            "unknown_target_cases": len(cases),
            "pending_provenance_cases": len(cases),
            "pending_legal_review_cases": len(cases),
            "unresolved_native_ability_identities": len(abilities),
            "unresolved_exact_quantitative_formulas": len(abilities),
        },
        "non_claims": [
            "NO_REFERENCE_PARITY_PROMOTION",
            "NO_NATIVE_CONTENT_KEY_MINTING",
            "NO_EXECUTABLE_FORMULA",
            "NO_RUNTIME_OR_SHARED_MODEL_CHANGE",
            "NO_PRODUCTION_AUTHORITY",
        ],
    }
    digest_scope = copy.deepcopy(value)
    value["product_digest_scope"] = "canonical JSON excluding digest fields"
    value["product_digest_sha256"] = sha256_bytes(canonical_bytes(digest_scope))
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    value = build_catalog(args.game_root)
    payload = canonical_bytes(value)
    if args.output:
        args.output.write_bytes(payload)
    else:
        print(payload.decode("utf-8"), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
