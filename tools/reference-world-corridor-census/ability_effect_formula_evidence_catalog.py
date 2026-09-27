#!/usr/bin/env python3
"""CW2 deterministic whole-family Ability source catalogue.

This extends the existing B4 evidence catalogue into a finite 825-record source
catalogue. Pinned legacy source bytes remain migration evidence only. The two
historical B4 records are joined as immutable evidence overlays; this module
never mints native Ability identities, promotes Reference parity, or supplies
an executable formula.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
from typing import Any, Iterable, Mapping, Sequence

SCHEMA = "OTERYN_CW2_ABILITY_SOURCE_CATALOGUE/v1"
MAPPER_PROFILE = "OTERYN_CW2_ABILITY_SOURCE_CATALOGUE_MAPPER/v1"
TASK = "OTV2-20260921-content-world-cw2-ability-family-source-catalogue-708"
ADMISSION_MAIN = "fd9dcb55a0ea55d2d20488f80abccf8f6c8ec875"
CLOSURE = "SOURCE_CATALOGUE_COMPLETE_NATIVE_UNRESOLVED"

SOURCE_REPOSITORY = "blakinio/Otheryn"
SOURCE_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
SOURCE_CLASSIFICATION = "OTERYN_LEGACY / MIGRATION_EVIDENCE"
SOURCE_EVIDENCE_STATUS = "OTS_HYPOTHESIS_ONLY"

MAPPER_PATH = (
    "tools/reference-world-corridor-census/"
    "ability_effect_formula_evidence_catalog.py"
)
EVIDENCE_PATH = (
    "docs/agents/evidence/"
    "OTV2-20260921-content-world-cw2-ability-family-source-catalogue.json"
)
HISTORICAL_B4_EVIDENCE_PATH = (
    "docs/agents/evidence/"
    "OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence.json"
)
HISTORICAL_B4_EVIDENCE_BLOB = "56b8e4b1d143cc9a68aa691aa30a292d4befe2c3"
HISTORICAL_B4_PRODUCT_DIGEST = (
    "56cef2d78442a37c00daa4cb737007e8a069e10ae3d4a298c0d38c38976f8289"
)
HISTORICAL_B4_ADMISSION_MAIN = "03a821edd828e24ccff6e2cb7fc819a776cbd238"
HISTORICAL_B4_MAPPER_BLOB = "e6d98aadd352ad36b466970e1f7182e1bf93643b"
HISTORICAL_B4_MAPPER_CANONICAL_SIZE = 16013
HISTORICAL_B4_MAPPER_CANONICAL_SHA256 = (
    "bc68f0f63a5dd6ea5ee7a3c20b708d6ea78c6033a2f5dddb518c49d6f45f8666"
)

PLAYER_ROOTS: tuple[tuple[str, int], ...] = (
    ("data/scripts/spells/attack", 69),
    ("data/scripts/spells/healing", 28),
    ("data/scripts/spells/support", 39),
    ("data/scripts/spells/conjuring", 49),
    ("data/scripts/spells/party", 5),
    ("data/scripts/spells/familiar", 5),
    ("data/scripts/spells/house", 4),
)
RUNE_ROOT = "data/scripts/runes"
MONSTER_ROOT = "data-otservbr-global/scripts/spells/monster"
EXAMPLE_PATH = "data/scripts/spells/#example.lua"
MONSTER_HELPER_PATH = f"{MONSTER_ROOT}/gaz_functions.lua"

EXPECTED_COUNTS = {
    "player_spells": 199,
    "runes": 36,
    "monster_spells": 590,
    "abilities_total": 825,
}
EXPECTED_EXCLUSIONS: Mapping[str, tuple[str, str]] = {
    EXAMPLE_PATH: (
        "46b410ff05e896d63f48813f392a6c2abaef652a",
        "EXAMPLE_TEST_CONTENT_EXCLUSION",
    ),
    MONSTER_HELPER_PATH: (
        "ae135c9159c67859701e25da33caf3280945d591",
        "HELPER_ONLY_MONSTER_SPELL_EXCLUSION",
    ),
}
OVERLAY_SOURCES: Mapping[str, tuple[str, str]] = {
    "reference-source:ability:ice_strike": (
        "data/scripts/spells/attack/ice_strike.lua",
        "8c95d439118f6d87e8b4db6c6792c15fb2adb391",
    ),
    "reference-source:ability:light_healing": (
        "data/scripts/spells/healing/light_healing.lua",
        "8699980575c3786edf96de701f8a5f2c9b1d20d3",
    ),
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


def _normalize_remote(url: str) -> str:
    value = url.strip().lower().replace("\\", "/")
    if value.endswith(".git"):
        value = value[:-4]
    if value.startswith("git@github.com:"):
        value = "https://github.com/" + value.split(":", 1)[1]
    return value.rstrip("/")


def _tree_paths(repo: Path, root: str) -> list[str]:
    raw = _git(
        repo,
        "ls-tree",
        "-r",
        "-z",
        "--name-only",
        SOURCE_REVISION,
        "--",
        root,
        binary=True,
    )
    assert isinstance(raw, bytes)
    return sorted(value.decode("utf-8") for value in raw.split(b"\0") if value)


def _direct_lua_paths(repo: Path, root: str) -> list[str]:
    return [
        path
        for path in _tree_paths(repo, root)
        if path.endswith(".lua") and Path(path).parent.as_posix() == root
    ]


def _source_object(
    source_repo: Path,
    path: str,
    *,
    expected_blob: str | None = None,
) -> tuple[str, bytes]:
    try:
        blob = str(_git(source_repo, "rev-parse", f"{SOURCE_REVISION}:{path}"))
        payload = _git(source_repo, "cat-file", "blob", blob, binary=True)
    except CatalogError as exc:
        raise CatalogError(f"SOURCE_OBJECT_UNAVAILABLE: {path}") from exc
    assert isinstance(payload, bytes)
    if expected_blob is not None and blob != expected_blob:
        raise CatalogError(
            f"SOURCE_BLOB_MISMATCH: {path}: expected {expected_blob}, got {blob}"
        )
    return blob, payload


def verify_source_repository(source_repo: Path) -> None:
    source_repo = source_repo.resolve()
    top = Path(str(_git(source_repo, "rev-parse", "--show-toplevel"))).resolve()
    if top != source_repo:
        raise CatalogError("SOURCE_REPOSITORY_ROOT_MISMATCH")
    remote = _normalize_remote(str(_git(source_repo, "remote", "get-url", "origin")))
    if remote != f"https://github.com/{SOURCE_REPOSITORY}".lower():
        raise CatalogError(f"SOURCE_REPOSITORY_REMOTE_MISMATCH: {remote}")
    revision = str(_git(source_repo, "rev-parse", f"{SOURCE_REVISION}^{{commit}}"))
    if revision != SOURCE_REVISION:
        raise CatalogError("SOURCE_REVISION_MISMATCH")


def enumerate_source_paths(source_repo: Path) -> dict[str, Any]:
    verify_source_repository(source_repo)
    player: dict[str, list[str]] = {}
    player_total = 0
    for root, expected in PLAYER_ROOTS:
        paths = _direct_lua_paths(source_repo, root)
        if len(paths) != expected:
            raise CatalogError(
                f"PLAYER_SUBFAMILY_COUNT_MISMATCH: {root}: expected {expected}, got {len(paths)}"
            )
        subfamily = root.rsplit("/", 1)[-1]
        player[subfamily] = paths
        player_total += len(paths)
    if player_total != EXPECTED_COUNTS["player_spells"]:
        raise CatalogError("PLAYER_SPELL_COUNT_MISMATCH")

    runes = _direct_lua_paths(source_repo, RUNE_ROOT)
    if len(runes) != EXPECTED_COUNTS["runes"]:
        raise CatalogError("RUNE_COUNT_MISMATCH")

    monster_raw = _direct_lua_paths(source_repo, MONSTER_ROOT)
    if len(monster_raw) != EXPECTED_COUNTS["monster_spells"] + 1:
        raise CatalogError("MONSTER_RAW_COUNT_MISMATCH")
    if MONSTER_HELPER_PATH not in monster_raw:
        raise CatalogError("MONSTER_HELPER_EXCLUSION_MISSING")
    monsters = [path for path in monster_raw if path != MONSTER_HELPER_PATH]
    if len(monsters) != EXPECTED_COUNTS["monster_spells"]:
        raise CatalogError("MONSTER_SPELL_COUNT_MISMATCH")

    selected: list[str] = []
    for subfamily, paths in player.items():
        if not subfamily:
            raise CatalogError("PLAYER_SUBFAMILY_EMPTY")
        selected.extend(paths)
    selected.extend(runes)
    selected.extend(monsters)
    if len(selected) != EXPECTED_COUNTS["abilities_total"]:
        raise CatalogError("ABILITY_TOTAL_MISMATCH")
    if len(set(selected)) != len(selected):
        raise CatalogError("DUPLICATE_SELECTED_SOURCE_PATH")
    if EXAMPLE_PATH in selected or MONSTER_HELPER_PATH in selected:
        raise CatalogError("EXCLUDED_PATH_SELECTED")

    exclusions = []
    for path, (expected_blob, reason) in sorted(EXPECTED_EXCLUSIONS.items()):
        blob, payload = _source_object(source_repo, path, expected_blob=expected_blob)
        exclusions.append(
            {
                "repository": SOURCE_REPOSITORY,
                "revision": SOURCE_REVISION,
                "path": path,
                "blob": blob,
                "byte_size": len(payload),
                "reason": reason,
            }
        )

    return {
        "player": player,
        "runes": runes,
        "monsters": monsters,
        "selected": selected,
        "exclusions": exclusions,
    }


def _classify_path(path: str) -> tuple[str, str]:
    for root, _ in PLAYER_ROOTS:
        if path.startswith(root + "/") and Path(path).parent.as_posix() == root:
            return "PLAYER_SPELL", root.rsplit("/", 1)[-1]
    if path.startswith(RUNE_ROOT + "/") and Path(path).parent.as_posix() == RUNE_ROOT:
        return "RUNE", "rune"
    if path.startswith(MONSTER_ROOT + "/") and Path(path).parent.as_posix() == MONSTER_ROOT:
        return "MONSTER_SPELL", "monster"
    raise CatalogError(f"UNCLASSIFIED_SOURCE_PATH: {path}")


def validate_selected_source_paths(
    expected_paths: Sequence[str],
    selected_paths: Iterable[str],
) -> list[str]:
    values = list(selected_paths)
    if len(values) != len(set(values)):
        raise CatalogError("DUPLICATE_SELECTED_SOURCE_PATH")
    if set(values) != set(expected_paths):
        missing = sorted(set(expected_paths) - set(values))
        extra = sorted(set(values) - set(expected_paths))
        raise CatalogError(
            f"SOURCE_SELECTION_SET_MISMATCH: missing={missing[:3]} extra={extra[:3]}"
        )
    return values


def verify_mapper_revision(game_root: Path) -> dict[str, Any]:
    game_root = game_root.resolve()
    top = Path(str(_git(game_root, "rev-parse", "--show-toplevel"))).resolve()
    if top != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
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


def _load_json(payload: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CatalogError(f"{label}_INVALID_JSON") from exc
    if not isinstance(value, dict):
        raise CatalogError(f"{label}_ROOT_INVALID")
    return value


def _verify_product_digest(value: dict[str, Any], expected: str, label: str) -> None:
    digest_scope = copy.deepcopy(value)
    recorded = digest_scope.pop("product_digest_sha256", None)
    digest_scope.pop("product_digest_scope", None)
    recomputed = sha256_bytes(canonical_bytes(digest_scope))
    if recorded != expected or recomputed != expected:
        raise CatalogError(
            f"{label}_PRODUCT_DIGEST_MISMATCH: expected {expected}, recorded {recorded}, recomputed {recomputed}"
        )


def verify_historical_b4_overlay(game_root: Path) -> dict[str, Any]:
    game_root = game_root.resolve()
    top = Path(str(_git(game_root, "rev-parse", "--show-toplevel"))).resolve()
    if top != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
    if str(_git(game_root, "rev-parse", f"{ADMISSION_MAIN}^{{commit}}")) != ADMISSION_MAIN:
        raise CatalogError("ADMISSION_MAIN_UNAVAILABLE")
    if str(_git(game_root, "rev-parse", f"{HISTORICAL_B4_ADMISSION_MAIN}^{{commit}}")) != HISTORICAL_B4_ADMISSION_MAIN:
        raise CatalogError("HISTORICAL_B4_ADMISSION_MAIN_UNAVAILABLE")

    evidence_blob = str(
        _git(game_root, "rev-parse", f"{ADMISSION_MAIN}:{HISTORICAL_B4_EVIDENCE_PATH}")
    )
    if evidence_blob != HISTORICAL_B4_EVIDENCE_BLOB:
        raise CatalogError(f"HISTORICAL_B4_EVIDENCE_BLOB_MISMATCH: {evidence_blob}")
    evidence_payload = _git(game_root, "cat-file", "blob", evidence_blob, binary=True)
    assert isinstance(evidence_payload, bytes)
    evidence = _load_json(evidence_payload, "HISTORICAL_B4_EVIDENCE")
    _verify_product_digest(evidence, HISTORICAL_B4_PRODUCT_DIGEST, "HISTORICAL_B4")

    mapper_blob = str(
        _git(game_root, "rev-parse", f"{HISTORICAL_B4_ADMISSION_MAIN}:{MAPPER_PATH}")
    )
    if mapper_blob != HISTORICAL_B4_MAPPER_BLOB:
        raise CatalogError(f"HISTORICAL_B4_MAPPER_BLOB_MISMATCH: {mapper_blob}")
    mapper_payload = _git(game_root, "cat-file", "blob", mapper_blob, binary=True)
    assert isinstance(mapper_payload, bytes)
    canonical = canonical_repository_text_bytes(mapper_payload)
    if len(canonical) != HISTORICAL_B4_MAPPER_CANONICAL_SIZE:
        raise CatalogError("HISTORICAL_B4_MAPPER_SIZE_MISMATCH")
    if sha256_bytes(canonical) != HISTORICAL_B4_MAPPER_CANONICAL_SHA256:
        raise CatalogError("HISTORICAL_B4_MAPPER_SHA256_MISMATCH")
    recorded_mapper = evidence.get("mapper_revision", {})
    if (
        recorded_mapper.get("git_blob") != HISTORICAL_B4_MAPPER_BLOB
        or recorded_mapper.get("canonical_size") != HISTORICAL_B4_MAPPER_CANONICAL_SIZE
        or recorded_mapper.get("canonical_sha256") != HISTORICAL_B4_MAPPER_CANONICAL_SHA256
    ):
        raise CatalogError("HISTORICAL_B4_RECORDED_MAPPER_MISMATCH")

    candidates = evidence.get("ability_effect_formula_candidates", [])
    if not isinstance(candidates, list) or len(candidates) != 2:
        raise CatalogError("HISTORICAL_B4_OVERLAY_COUNT_MISMATCH")
    by_id = {candidate.get("source_candidate_id"): candidate for candidate in candidates}
    if set(by_id) != set(OVERLAY_SOURCES):
        raise CatalogError("HISTORICAL_B4_OVERLAY_ID_SET_MISMATCH")
    for candidate_id, candidate in by_id.items():
        if candidate.get("target_evidence") != "UNKNOWN":
            raise CatalogError(f"HISTORICAL_B4_TARGET_PROMOTED: {candidate_id}")
        if candidate.get("source_provenance") != "PENDING":
            raise CatalogError(f"HISTORICAL_B4_PROVENANCE_PROMOTED: {candidate_id}")
        if candidate.get("legal_review") != "PENDING":
            raise CatalogError(f"HISTORICAL_B4_LEGAL_PROMOTED: {candidate_id}")
        if candidate.get("parity") != "PARITY_PENDING_EVIDENCE":
            raise CatalogError(f"HISTORICAL_B4_PARITY_PROMOTED: {candidate_id}")
        if candidate.get("native_ability_identity", {}).get("content_key") is not None:
            raise CatalogError(f"HISTORICAL_B4_NATIVE_IDENTITY_PRESENT: {candidate_id}")
        if candidate.get("effect_to_formula", {}).get("quantitative_formula") is not None:
            raise CatalogError(f"HISTORICAL_B4_FORMULA_PROMOTED: {candidate_id}")
        if candidate.get("executable_promotion", {}).get("disposition") != "BLOCKED":
            raise CatalogError(f"HISTORICAL_B4_EXECUTABLE_PROMOTED: {candidate_id}")

    return {
        "path": HISTORICAL_B4_EVIDENCE_PATH,
        "blob": evidence_blob,
        "byte_size": len(evidence_payload),
        "product_digest_sha256": HISTORICAL_B4_PRODUCT_DIGEST,
        "historical_mapper": {
            "admission_main": HISTORICAL_B4_ADMISSION_MAIN,
            "path": MAPPER_PATH,
            "blob": mapper_blob,
            "canonical_size": len(canonical),
            "canonical_sha256": sha256_bytes(canonical),
        },
        "candidates_by_id": by_id,
    }


def build_source_records(
    source_repo: Path,
    *,
    source_paths: Iterable[str] | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    enumeration = enumerate_source_paths(source_repo)
    expected_paths = list(enumeration["selected"])
    selected_paths = validate_selected_source_paths(
        expected_paths,
        expected_paths if source_paths is None else source_paths,
    )

    records: list[dict[str, Any]] = []
    for path in selected_paths:
        family, subfamily = _classify_path(path)
        blob, payload = _source_object(source_repo, path)
        records.append(
            {
                "repository": SOURCE_REPOSITORY,
                "revision": SOURCE_REVISION,
                "path": path,
                "blob": blob,
                "byte_size": len(payload),
                "source_family": family,
                "source_subfamily": subfamily,
                "source_classification": SOURCE_CLASSIFICATION,
                "evidence_status": SOURCE_EVIDENCE_STATUS,
                "native_ability_identity": {
                    "disposition": "UNRESOLVED",
                    "content_key": None,
                    "reason": "NO_ACCEPTED_NATIVE_ABILITY_BINDING",
                },
            }
        )
    return canonical_records(records), enumeration


def build_catalog(
    source_repo: Path,
    game_root: Path,
    *,
    source_paths: Iterable[str] | None = None,
) -> dict[str, Any]:
    records, enumeration = build_source_records(source_repo, source_paths=source_paths)
    historical = verify_historical_b4_overlay(game_root)
    mapper_revision = verify_mapper_revision(game_root)

    by_path = {record["path"]: record for record in records}
    if len(by_path) != EXPECTED_COUNTS["abilities_total"]:
        raise CatalogError("SOURCE_RECORD_PATH_UNIQUENESS_MISMATCH")

    overlay_joined = 0
    for candidate_id, (path, expected_blob) in sorted(OVERLAY_SOURCES.items()):
        record = by_path.get(path)
        if record is None:
            raise CatalogError(f"B4_OVERLAY_SOURCE_MISSING: {path}")
        if record["blob"] != expected_blob:
            raise CatalogError(
                f"B4_OVERLAY_SOURCE_BLOB_MISMATCH: {path}: expected {expected_blob}, got {record['blob']}"
            )
        candidate = historical["candidates_by_id"][candidate_id]
        record["historical_b4_overlay"] = copy.deepcopy(candidate)
        overlay_joined += 1
    if overlay_joined != 2:
        raise CatalogError("B4_OVERLAY_JOIN_COUNT_MISMATCH")

    records = canonical_records(records)
    player_count = sum(record["source_family"] == "PLAYER_SPELL" for record in records)
    rune_count = sum(record["source_family"] == "RUNE" for record in records)
    monster_count = sum(record["source_family"] == "MONSTER_SPELL" for record in records)
    total = len(records)
    if (
        player_count != EXPECTED_COUNTS["player_spells"]
        or rune_count != EXPECTED_COUNTS["runes"]
        or monster_count != EXPECTED_COUNTS["monster_spells"]
        or total != EXPECTED_COUNTS["abilities_total"]
    ):
        raise CatalogError("FINAL_PARTITION_COUNT_MISMATCH")

    player_subfamilies = {
        subfamily: len(paths) for subfamily, paths in sorted(enumeration["player"].items())
    }
    value: dict[str, Any] = {
        "schema": SCHEMA,
        "mapper_profile": MAPPER_PROFILE,
        "mapper_revision": mapper_revision,
        "task": TASK,
        "admission_main": ADMISSION_MAIN,
        "closure": CLOSURE,
        "production_authority": "NONE",
        "source_snapshot": {
            "repository": SOURCE_REPOSITORY,
            "revision": SOURCE_REVISION,
            "classification": SOURCE_CLASSIFICATION,
            "evidence_status": SOURCE_EVIDENCE_STATUS,
            "selected_roots": [root for root, _ in PLAYER_ROOTS] + [RUNE_ROOT, MONSTER_ROOT],
            "selection": {
                "player_spells": "direct *.lua under the seven admitted spell subroots",
                "runes": f"direct *.lua under {RUNE_ROOT}",
                "monster_spells": f"direct *.lua under {MONSTER_ROOT} excluding gaz_functions.lua",
            },
            "exclusions": enumeration["exclusions"],
        },
        "historical_b4_overlay": {
            "evidence_path": historical["path"],
            "evidence_blob": historical["blob"],
            "evidence_byte_size": historical["byte_size"],
            "product_digest_sha256": historical["product_digest_sha256"],
            "historical_mapper": historical["historical_mapper"],
            "join_count": overlay_joined,
            "join_paths": sorted(path for path, _ in OVERLAY_SOURCES.values()),
            "disposition": "EVIDENCE_OVERLAY_ONLY_NO_REFERENCE_PROMOTION",
        },
        "source_records": records,
        "counts": {
            "player_spells": player_count,
            "player_spell_subfamilies": player_subfamilies,
            "runes": rune_count,
            "monster_spells": monster_count,
            "abilities_total": total,
            "source_catalogued": total,
            "source_remaining": EXPECTED_COUNTS["abilities_total"] - total,
            "historical_b4_overlay_joined": overlay_joined,
        },
        "loss_report": {
            "silently_dropped_records": 0,
            "duplicate_selected_source_paths": 0,
            "excluded_nonproduction_modules": len(enumeration["exclusions"]),
            "native_ability_identities_resolved": 0,
            "reference_truth_promotions": 0,
            "executable_promotions": 0,
        },
        "non_claims": [
            "NO_REFERENCE_PARITY_PROMOTION",
            "NO_LEGACY_FORMULA_AS_REFERENCE_TRUTH",
            "NO_NATIVE_CONTENT_KEY_MINTING",
            "NO_EXECUTABLE_ABILITY_CLOSURE",
            "NO_RUNTIME_OR_SHARED_MODEL_CHANGE",
            "NO_PRODUCTION_AUTHORITY",
        ],
    }
    return value


def write_evidence(path: Path, evidence: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_bytes(evidence))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-repo", type=Path, required=True)
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    evidence = build_catalog(args.source_repo, args.game_root)
    write_evidence(args.output, evidence)
    counts = evidence["counts"]
    print(
        "ability-source-catalogue: PASS "
        f"total={counts['abilities_total']} "
        f"player={counts['player_spells']} "
        f"runes={counts['runes']} "
        f"monster={counts['monster_spells']} "
        f"catalogued={counts['source_catalogued']} "
        f"remaining={counts['source_remaining']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
