"""Admit the converter-validated Cake Golem reference/profile closure; no authored stats."""

import argparse
import copy
import hashlib
import json
from pathlib import Path
import summer_npcs

ROOT = Path(__file__).resolve().parents[2]
INPUT = Path(__file__).with_name("cake_golem_stage.json")
SOURCE = "oteryn:source.crystalserver.srv7_cake_golem"
BATCH = "srv7-seasonal-cake-golem-r1"


def canonical_profile(entry):
    """Only the exact Cake creature's two serde-elided defaults are equivalent."""
    result = copy.deepcopy(entry)
    if result.get("target") != {
        "family": "Creature",
        "key": "oteryn:creature.cake_golem",
        "revision": "definition-r1",
    }:
        return result
    data = result.get("data", {})
    if data.get("kind") != "Creature":
        return result
    profile = data.get("profile", {})
    # ProjectV2CreatureProfile Vec<String>: missing defaults to empty; null is
    # NOT an empty Vec and must remain a conflict, as must every real immunity.
    if profile.get("immunities") == []:
        del profile["immunities"]
    details = profile.get("details", {})
    # ProjectV2CreatureDetails Option<u32>: both missing and null mean None.
    if "reference_level" in details and details["reference_level"] is None:
        del details["reference_level"]
    return result


def append_profile(rows, entry):
    identity = lambda row: (row["target"]["family"], row["target"]["key"])
    prior = [row for row in rows if identity(row) == identity(entry)]
    candidate = canonical_profile(entry)
    if prior:
        if len(prior) != 1 or canonical_profile(prior[0]) != candidate:
            raise ValueError("SUMMER_NPC_EXISTING_IDENTITY_CONFLICT")
    else:
        rows.append(candidate)


def files():
    stage = json.loads(INPUT.read_text())
    source = stage["source"]
    if (
        source["repository"] != "zimbadev/crystalserver"
        or source["revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        or len(stage["records"]) != 14
        or len(stage["profiles"]) != 13
    ):
        raise ValueError("CAKE_GOLEM_STAGE_SOURCE")
    paths = [
        "definitions/reference.json",
        "definitions/declarations.json",
        "provenance/sources.json",
        "provenance/imports.json",
    ]
    docs = {p: json.loads((ROOT / "content/world" / p).read_text()) for p in paths}
    for row in stage["records"]:
        summer_npcs.append_exact(
            docs[paths[0]]["records"],
            row,
            lambda r: (r["identity"]["family"], r["identity"]["key"]),
        )
    for row in stage["profiles"]:
        append_profile(docs[paths[1]]["authoring_profiles"], row)
    sources = docs[paths[2]]
    summer_npcs.append_exact(
        sources["sources"],
        dict(
            key=SOURCE,
            import_batch_id=BATCH,
            revision=source["revision"],
            sha256=source["sha256"],
            evidence="OtsHypothesisOnly",
        ),
        lambda r: (r["key"], r["revision"]),
    )
    for raw in stage["bindings"]:
        binding = copy.deepcopy(raw)
        binding.update(
            source_key=SOURCE,
            source_revision=source["revision"],
            identity_namespace="crystalserver/monster-file",
        )
        summer_npcs.append_exact(
            sources["source_identity_bindings"],
            binding,
            lambda r: (
                r["source_key"],
                r["source_revision"],
                r["identity_namespace"],
                r["external_id"],
            ),
        )
    batches = docs[paths[3]]["batches"]
    batches[:] = [r for r in batches if r["batch_id"] != BATCH]
    batches.append(
        dict(
            batch_id=BATCH,
            source_repository=source["repository"],
            source_revision=source["revision"],
            source_artifact_sha256=source["sha256"],
            access_disposition="PENDING",
            source_generation_profile="OTERYN_MONSTER_AUTHORING_CANDIDATE/v1",
            importer="OTERYN_MONSTER_PROMOTION_CANDIDATES/v1",
            mapper="OTERYN_CREATURE_ADMISSION_STAGE/v1",
            mapper_revision="srv7-cake-golem-r1",
            mapper_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            candidates=[],
            reimport_states=[],
        )
    )
    return summer_npcs.canonical_files(
        {p: summer_npcs.dump(d) for p, d in docs.items()}
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, raw in files().items():
        if args.check:
            if Path(path).read_text() != raw:
                raise SystemExit("CAKE_GOLEM_CANONICAL_DRIFT:" + path)
        else:
            Path(path).write_text(raw)
