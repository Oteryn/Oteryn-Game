"""Explicit canonical projection with separate identity-only and supported-payload gates."""
import copy
import hashlib
import importlib.util
from functools import lru_cache
import json
import re
from pathlib import Path, PurePosixPath

DONORS = {
    "canary-main-current": ("opentibiabr/canary", "main", "04b83b512114bfd888000d6e1433ed8ecaec7c5b"),
    "crystal-summer-current": ("zimbadev/crystalserver", "summer-update", "00ce02a57ca5a12e48f32a3476e37471167e4c3f"),
    "crystal-main-current": ("zimbadev/crystalserver", "main", "96d13eff5a1afef17b11b9abc7d574020381cc53"),
}


def canonical_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def identity(value):
    if not isinstance(value, dict) or set(value) != {"key", "revision"} or any(
        not isinstance(part, str) or not part for part in value.values()
    ):
        raise ValueError("EXPLICIT_IDENTITY_REQUIRED")
    return value


def _check_candidate(candidate_bundle, current_entry, explicit_identity):
    """Bind exact spell identity and source header before either projection mode."""
    target = identity(explicit_identity)
    current = current_entry["bundle"]["spell"]
    if target != identity(current["identity"]):
        raise ValueError("TARGET_IDENTITY_MISMATCH")
    sources = current_entry["source_identities"]
    if not sources or any(row["identity"] != target for row in sources):
        raise ValueError("AMBIGUOUS_CURRENT_IDENTITY")
    if not isinstance(candidate_bundle, dict) or set(candidate_bundle) != {"spell"}:
        raise ValueError("CANDIDATE_BUNDLE_SHAPE")
    candidate = candidate_bundle["spell"]
    identity(candidate["identity"])
    for field in ("name", "carrier", "reference_spell_id"):
        if candidate.get(field) != current.get(field):
            raise ValueError("SOURCE_HEADER_MISMATCH:" + field)
    return target


def project_candidate(candidate_bundle, dependencies, current_entry, explicit_identity):
    """Accept only a pinned candidate whose sole difference is spell identity."""
    target = _check_candidate(candidate_bundle, current_entry, explicit_identity)
    projected = copy.deepcopy(candidate_bundle)
    projected["spell"]["identity"] = copy.deepcopy(target)
    if canonical_bytes(projected) != canonical_bytes(current_entry["bundle"]):
        raise ValueError("FULL_HEADER_MISMATCH")
    if canonical_bytes(dependencies) != canonical_bytes(current_entry["dependencies"]):
        raise ValueError("DEPENDENCIES_MISMATCH")
    return True


def _project_local_graph(candidate_bundle, dependencies, current_entry):
    """Bind canonical IDs directly or map ordered local IDs; retain external refs."""
    current = current_entry["bundle"]["spell"]
    if current["execution"].get("native_behavior") is not None:
        raise ValueError("NATIVE_PROFILE_REPLACEMENT_HELD")
    sections = {"abilities": "Ability", "effects": "Effect", "formulas": "Formula"}
    if not isinstance(dependencies, dict) or set(dependencies) != set(sections):
        raise ValueError("DEPENDENCY_COLLECTION_SHAPE")
    mapping, source_ids, target_ids = {}, set(), set()
    for section, family in sections.items():
        source = dependencies[section]
        target = current_entry["dependencies"][section]
        if not isinstance(source, list) or len(source) != len(target):
            raise ValueError("DEPENDENCY_COUNT_MISMATCH:" + section)
        # A reviewed projection can already use canonical IDs while preserving
        # donor execution order. Bind those IDs directly, rather than reassigning
        # semantic Effect identities according to their array positions.
        exact = {canonical_bytes(row["identity"]): row for row in target}
        if source and all(canonical_bytes(row["identity"]) in exact for row in source):
            target = [exact[canonical_bytes(row["identity"])] for row in source]
        for before, after in zip(source, target):
            old, new = identity(before["identity"]), identity(after["identity"])
            old_id, new_id = (old["key"], old["revision"]), (new["key"], new["revision"])
            if old_id in source_ids or new_id in target_ids:
                raise ValueError("AMBIGUOUS_DEPENDENCY_IDENTITY")
            source_ids.add(old_id)
            target_ids.add(new_id)
            mapping[old_id] = (family, new)

    def rewrite(value):
        if isinstance(value, list):
            return [rewrite(child) for child in value]
        if not isinstance(value, dict):
            return value
        if set(value) == {"family", "key", "revision"}:
            bound = mapping.get((value["key"], value["revision"]))
            if bound is not None:
                if value["family"] != bound[0]:
                    raise ValueError("DEPENDENCY_REFERENCE_FAMILY_MISMATCH")
                return {"family": value["family"], **copy.deepcopy(bound[1])}
        result = {key: rewrite(child) for key, child in value.items()}
        if "identity" in value:
            old = identity(value["identity"])
            bound = mapping.get((old["key"], old["revision"]))
            if bound is not None:
                result["identity"] = copy.deepcopy(bound[1])
        return result

    bundle = rewrite(copy.deepcopy(candidate_bundle))
    for field in ("library_text", "base_power"):
        if field not in bundle["spell"] and field in current:
            bundle["spell"][field] = copy.deepcopy(current[field])
    return bundle, rewrite(dependencies)


def project_local_candidate(candidate_bundle, dependencies, current_entry, explicit_identity):
    """Map ordered local dependency identities, then require the entire accepted graph."""
    bundle, deps = _project_local_graph(candidate_bundle, dependencies, current_entry)
    return project_candidate(bundle, deps, current_entry, explicit_identity)


@lru_cache(maxsize=1)
def _validator():
    """Load the adjacent formal validator also when imported by an external writer."""
    spec = importlib.util.spec_from_file_location(
        "_current_spell_adapter_validator", Path(__file__).with_name("validate_spell.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.validate


def project_supported_candidate(candidate_bundle, dependencies, current_entry, explicit_identity):
    """Project a changed non-native graph only after complete formal v1 validation.

    This qualifies authoring shape/references, not provenance or numerical source
    equivalence. Importers must separately review and pin those source proofs.
    """
    target = _check_candidate(candidate_bundle, current_entry, explicit_identity)
    for spell in (candidate_bundle["spell"], current_entry["bundle"]["spell"]):
        if spell.get("execution", {}).get("native_behavior") is not None:
            raise ValueError("NATIVE_PROFILE_REPLACEMENT_HELD")
        if spell.get("requirements", {}).get("wheel_unlock") is True:
            raise ValueError("WHEEL_UNLOCK_REPLACEMENT_HELD")
    for collection in (dependencies, current_entry["dependencies"]):
        for effect in collection.get("effects", []):
            if effect.get("pvp_safe_item") is not None:
                raise ValueError("NATIVE_PVP_SAFE_ITEM_REPLACEMENT_HELD")
    bundle, deps = _project_local_graph(candidate_bundle, dependencies, current_entry)
    bundle["spell"]["identity"] = copy.deepcopy(target)
    errors = _validator()(bundle, deps, current_entry.get("catalog"))
    if errors:
        raise ValueError("PROJECTED_V1_INVALID: " + "; ".join(errors))
    return bundle, deps


def attach_source(entry, proof):
    """Add Git provenance after projection; richer source pins stay in the receipt.

    The caller must first pass project_candidate and verify the source file hash.
    This pure function validates proof shape, not file contents or semantic origin.
    """
    if not isinstance(proof, dict) or set(proof) != {
        "source_id", "repository", "branch", "revision", "path", "sha256"
    }:
        raise ValueError("SOURCE_PROOF_SHAPE")
    if any(not isinstance(value, str) for value in proof.values()):
        raise ValueError("SOURCE_PROOF_SHAPE")
    donor = DONORS.get(proof["source_id"])
    if donor is None or tuple(proof[key] for key in ("repository", "branch", "revision")) != donor:
        raise ValueError("SOURCE_DONOR_PIN_MISMATCH")
    path = proof["path"]
    if not isinstance(path, str) or not re.fullmatch(r"[A-Za-z0-9_./'# -]+\.lua", path) or \
            PurePosixPath(path).is_absolute() or any(part in {"", ".", ".."} for part in path.split("/")) or \
            not path.startswith(("data/scripts/", "data-otservbr-global/scripts/", "data-canary/scripts/",
                                 "data-global/scripts/", "data-crystal/scripts/")):
        raise ValueError("SOURCE_PATH_INVALID")
    if not isinstance(proof["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", proof["sha256"]):
        raise ValueError("SOURCE_DIGEST_INVALID")
    result = copy.deepcopy(entry)
    target = identity(result["bundle"]["spell"]["identity"])
    rows = result["source_identities"]
    if not rows or any(row["identity"] != target or row["sources"] != result["manifest"]["sources"] for row in rows):
        raise ValueError("INCONSISTENT_SOURCE_IDENTITIES")
    source = {"repository": proof["repository"], "revision": proof["revision"]}
    if source not in result["manifest"]["sources"]:
        result["manifest"]["sources"].append(source)
    manifest_digest = hashlib.sha256(canonical_bytes(result["manifest"])).hexdigest()
    for row in rows:
        row["sources"] = copy.deepcopy(result["manifest"]["sources"])
        row["manifest_sha256"] = manifest_digest
    return result
