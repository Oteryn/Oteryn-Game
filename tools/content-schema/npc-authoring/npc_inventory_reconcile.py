"""Reconcile retained NPC evidence without inventing Game/runtime or appearance facts.

Produces an offline source-only proposal packet. Two nonremoved independent wiki
observations prove observed identity, not current Global completeness or gameplay.
Missing palette/object/movement facts are explicit holds; no content is mutated.
"""

import argparse
import copy
import hashlib
import json
import re
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path

PACKET = "docs/agents/evidence/OTV2-20261001-npc-source-audit-r4"
BR = "imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json"
TP = "imports/tibiawiki/npc-tibiopedia/2026-09-28/tibiopedia-npc-facts.json"
PROMOTION = "tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json"


def normalize(name):
    """The literal spelling plus the conventional wiki (NPC) disambiguator."""
    return (
        re.sub(
            r"\s+",
            " ",
            re.sub(r" \(NPC\)$", "", name or "", flags=re.IGNORECASE).replace("’", "'"),
        )
        .strip()
        .casefold()
    )


def slug(name):
    ascii_name = unicodedata.normalize("NFKD", name).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]+", "_", ascii_name.lower()).strip("_")


def variant_base(name):
    name = re.sub(r" \([^)]*\)$", "", name)
    return normalize(
        re.sub(r" (?:Init|Vampires Lair|Back)$", "", name, flags=re.IGNORECASE)
    )


def group_observations(br, tibiopedia, existing):
    """Prefer exact identities; variant resemblance only records an unresolved link."""
    variants = defaultdict(set)
    for name, key in existing.items():
        variants[variant_base(name)].add(key)
    groups = defaultdict(lambda: {"br": [], "tibiopedia": []})
    for source, rows in [("br", br), ("tibiopedia", tibiopedia)]:
        for index, row in enumerate(rows):
            name = row.get("name") or row.get("title", "").removeprefix("NPC: ")
            groups[normalize(name)][source].append(
                {"row": row, "pointer": f"/pages/{index}"}
            )
    result = []
    for normalized_name, sources in sorted(groups.items()):
        br_rows, tp_rows = sources["br"], sources["tibiopedia"]
        name = (br_rows or tp_rows)[0]["row"].get("name") or normalized_name
        key = existing.get(normalized_name)
        aliases = sorted(variants.get(variant_base(name), set()))
        removed = any(r["row"].get("removed") for r in br_rows)
        roles_ok = all(r["row"].get("role") == "npc" for r in br_rows)
        ambiguous = any(len(rows) > 1 for rows in sources.values())
        if key:
            status = "EXISTING_OBSERVED_IDENTITY"
        elif removed:
            status = "WIKI_REMOVAL_HOLD"
        elif aliases:
            status = "AMBIGUOUS_VARIANT_IDENTITY_HOLD"
        elif not slug(name):
            status = "EMPTY_IDENTITY_HOLD"
        elif not br_rows or not tp_rows:
            status = "SINGLE_WIKI_IDENTITY_HOLD"
        elif not roles_ok or ambiguous:
            status = "AMBIGUOUS_WIKI_IDENTITY_HOLD"
        else:
            status = "TWO_WIKI_IDENTITY_PROPOSAL"
            key = "oteryn:npc." + slug(name)
        result.append(
            {
                "name": name,
                "normalized_name": normalized_name,
                "npc_key": key,
                "status": status,
                "alias_candidates": aliases,
                "sources": sources,
            }
        )
    # A lossy ASCII slug must never decide which independently named NPC owns a key.
    keys = Counter(
        r["npc_key"] for r in result if r["status"] == "TWO_WIKI_IDENTITY_PROPOSAL"
    )
    reserved = set(existing.values())
    for row in result:
        if row["status"] == "TWO_WIKI_IDENTITY_PROPOSAL" and (
            keys[row["npc_key"]] > 1 or row["npc_key"] in reserved
        ):
            row["status"] = "NATIVE_KEY_COLLISION_HOLD"
    return result


def evidence_ref(repository, path, raw, pointer):
    return {
        "path": path,
        "sha256": hashlib.sha256(raw).hexdigest(),
        "pointer": pointer,
        "repository": repository,
    }


def reconcile(repo, source_directory=None):
    def load(path):
        raw = (repo / path).read_bytes()
        return json.loads(raw), raw

    promotion, _ = load(PROMOTION)
    npcs, _ = load(PACKET + "/npc-candidates.json")
    native, _ = load("content/world/definitions/declarations.json")
    packet_keys = {r["identity"]["key"] for r in npcs["records"]}
    native_keys = {
        r["identity"]["key"] for r in native["records"] if r["kind"] == "NPC"
    }
    existing = {
        normalize(c["name"]): c["identity"]["key"] for c in promotion["candidates"]
    }
    # Held OTS actors are not silently reintroduced by a cross-wiki title match.
    for held in promotion["held"]:
        if slug(held["name"]):
            existing.setdefault(
                normalize(held["name"]), "oteryn:npc." + slug(held["name"])
            )
    index, _ = load(PACKET + "/source-index.json")
    existing.update({normalize(r["name"]): r["npc_key"] for r in index["records"]})
    br, br_raw = load(BR)
    tp, tp_raw = load(TP)
    rows = group_observations(br["pages"], tp["pages"], existing)
    observations, proposed = [], []
    for row in rows:
        proofs = []
        facts = {"positions": [], "trade_observations": []}
        for source, path, raw in [("br", BR, br_raw), ("tibiopedia", TP, tp_raw)]:
            for observed in row["sources"][source]:
                page = observed["row"]
                proof = evidence_ref(
                    "Oteryn/Oteryn-Game", path, raw, observed["pointer"]
                )
                proof.update(
                    {
                        k: page[k]
                        for k in ("pageid", "revid", "sha256", "url")
                        if k in page
                    }
                )
                proof["source"] = source
                proofs.append(proof)
                if source == "br":
                    facts["positions"].extend(page.get("positions", []))
                if page.get("trades"):
                    facts["trade_observations"].append(
                        {"source": source, "trades": page["trades"]}
                    )
        observation = {k: v for k, v in row.items() if k != "sources"}
        observation.update(
            source_proofs=proofs,
            retained_facts=facts,
            runtime_qualified=False,
            global_current_status_qualified=False,
        )
        observations.append(observation)
        if row["status"] == "TWO_WIKI_IDENTITY_PROPOSAL":
            qualification = {
                "state": "TWO_WIKI_LITERAL_IDENTITY_OBSERVED",
                "name": row["name"],
                "source_proofs": proofs,
                "retained_facts": facts,
                "runtime_qualified": False,
                "current_global_identity_qualified": False,
                "missing_facts": [
                    "target_appearance",
                    "movement",
                    "dialogue_program",
                    "quest_guards",
                    "service_conditions",
                    "world_map_placement_admission",
                ],
            }
            proposed.append(
                {
                    "kind": "NPC",
                    "identity": {
                        "key": row["npc_key"],
                        "revision": "source-proposal-r5",
                    },
                    "presentation": None,
                    "behavior": None,
                    "dialogue": None,
                    "services": [],
                    "fields": [
                        {
                            "field_path": "oteryn:source.npc.inventory_identity_qualification",
                            "value": {
                                "type": "Text",
                                "value": json.dumps(
                                    qualification,
                                    ensure_ascii=False,
                                    sort_keys=True,
                                    separators=(",", ":"),
                                ),
                            },
                        }
                    ],
                }
            )
    corrections = []
    # Keep the invalid asset reference outside an otherwise usable source NPC declaration.
    for key, problem in [
        ("oteryn:npc.hagor", "PALETTE_INDEX_1156_OUTSIDE_0_132"),
        ("oteryn:npc.a_sleeping_dragon", "OBJECT_168_ABSENT_PINNED_CLIENT"),
    ]:
        old = next(
            r
            for r in native["records"]
            if r["kind"] == "NPC" and r["identity"]["key"] == key
        )
        held = copy.deepcopy(old)
        held["presentation"] = None
        held["fields"].append(
            {
                "field_path": "oteryn:source.npc.presentation_reference_hold",
                "value": {
                    "type": "Text",
                    "value": json.dumps(
                        {
                            "state": "UNQUALIFIED_SOURCE_REFERENCE_HOLD",
                            "known_problem": problem,
                            "previous_reference": old["presentation"],
                            "replacement_appearance": None,
                            "runtime_qualified": False,
                        },
                        sort_keys=True,
                        separators=(",", ":"),
                    ),
                },
            }
        )
        corrections.append(
            {
                "npc_key": key,
                "known_problem": problem,
                "corrected_declaration": held,
                "source_reference": old["presentation"],
                "replacement_appearance": None,
                "admission_eligible": False,
            }
        )
    missing_two = []
    for key in sorted(packet_keys - native_keys):
        proposal = next(
            c for c in promotion["candidates"] if c["identity"]["key"] == key
        )
        missing_two.append(
            {
                "npc_key": key,
                "name": proposal["name"],
                "state": "IDENTITY_OBSERVED_MOVEMENT_AND_TEXT_HOLD",
                "source_candidate": proposal,
                "missing_facts": ["movement", "qualified_dialogue_program"],
                "presentation_observation": proposal["presentation"],
                "runtime_qualified": False,
                "candidate_declaration": next(
                    r for r in npcs["records"] if r["identity"]["key"] == key
                ),
            }
        )
    source_proofs = []
    if source_directory and (source_directory / "source-manifest.json").exists():
        for record in json.loads(
            (source_directory / "source-manifest.json").read_text()
        ):
            if "sha256" not in record:
                continue
            raw = (source_directory / record["file"]).read_bytes()
            if hashlib.sha256(raw).hexdigest() != record["sha256"]:
                raise ValueError("source custody mismatch: " + record["file"])
            source_proofs.append(record)
        for candidate in missing_two:
            file_name = (
                "crystal-"
                + (
                    "szallar_mandar"
                    if candidate["npc_key"].endswith("s_zallar_m_andar")
                    else "dragon_ancestor_spirit"
                )
                + ".lua"
            )
            candidate["source_proofs"] = [
                r for r in source_proofs if r["file"] == file_name
            ]
            raw_source = (
                (source_directory / file_name).read_text()
                if (source_directory / file_name).exists()
                else ""
            )
            candidate["source_missing_movement_proof"] = {
                "walk_fields_commented_as_TODO": bool(
                    re.search(r"--\s*npcConfig\.walkInterval.*TODO", raw_source)
                    and re.search(r"--\s*npcConfig\.walkRadius.*TODO", raw_source)
                )
            }
        for correction in corrections:
            file_stem = (
                "hagor"
                if correction["npc_key"].endswith("hagor")
                else "a_sleeping_dragon"
            )
            correction["source_proofs"] = [
                r
                for r in source_proofs
                if r["file"]
                in ("canary-" + file_stem + ".lua", "crystal-" + file_stem + ".lua")
            ]
    return {
        "schema": "OTERYN_NPC_INVENTORY_RECONCILIATION/v1",
        "scope": "SOURCE_ONLY_NO_CONTENT_MUTATION",
        "global_complete": False,
        "runtime_qualified": False,
        "counts": {
            "native_npcs": len(native_keys),
            "packet_npcs": len(packet_keys),
            "missing_native_existing_candidates": len(missing_two),
            "new_identity_only_source_proposals": len(proposed),
            "presentation_reference_holds": len(corrections),
            "observations_by_status": dict(Counter(r["status"] for r in observations)),
        },
        "proposed_records": proposed,
        "presentation_corrections": corrections,
        "missing_native_candidates": missing_two,
        "observations": observations,
        "retrieved_source_proofs": source_proofs,
        "still_missing_facts": [
            "Hagor correct palette index",
            "A Sleeping Dragon correct target-client appearance",
            "Dragon Ancestor Spirit and S'Zallar M'Andar actual movement and dialogue",
            "New two-wiki identity proposals actual appearance/movement/guarded dialogues",
            "Admitted world/map placements and executable runtime qualification",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo", type=Path, default=Path(__file__).resolve().parents[3]
    )
    parser.add_argument("--source-directory", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = reconcile(args.repo, args.source_directory)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(result, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    )
    print(json.dumps(result["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
