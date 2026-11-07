#!/usr/bin/env python3
"""Stage source quest associations on existing NPC identities; never emit native Quest/runtime records.

All original transitions, tracks, claims and gates remain source candidates. An NPC filename
binding proves the identity association, not that historical guards or scripts execute in Game.
Helper ownership requires pinned source bytes, not stripping a filename suffix. Item bindings
must match the source revision and an active native Item; other candidates remain held.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import xml.parsers.expat
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SAMPLES = "tools/content-schema/quest-authoring/samples/"
CANARY_QUEST_REVISION = "04b83b512114bfd888000d6e1433ed8ecaec7c5b"
HELPERS = {
    "canary:npc/alesar_functions": {
        "target": "canary:npc/alesar",
        "symbol": "ParseAlesarSay",
        "mode": "loaded_helper",
        "caller_sha256": "57ae144469c9532bf93ac1ed4b5b0af357db4b6203c55a1e2bc489adbe2f1db6",
        "helper_sha256": "83d224212043e87b4a7605e8286c3335339ef9d5a1625dd713424858acbd6590",
    },
    "canary:npc/tereban_functions": {
        "target": "canary:npc/tereban",
        "symbol": "ParseTerebanSay",
        "mode": "called_helper",
        "caller_sha256": "7a68df10d7877f2c2a40a78bb61f7bc6c12812c48d9beb2f025b3230c0ba0aa8",
        "helper_sha256": "ad89e2023757c4d2e76f539ec879185392b544963d8ce25054e9b197654e403b",
    },
}


ITEM_SOURCES = {
    "canary": {
        "repository": "opentibiabr/canary",
        "revision": CANARY_QUEST_REVISION,
        "xml_sha256": "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2",
        "loader_sha256": "f695ed647e111923cf8e4d487c4a0706f19e7edbfb7896655c20494d6d927fd7",
        "loader_lines": [[154, 174], [229, 229], [281, 315], [330, 339], [398, 403]],
    },
    "crystalserver": {
        "repository": "zimbadev/crystalserver",
        "revision": "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d",
        "xml_sha256": "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb",
        "loader_sha256": "f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d",
        "loader_lines": [[148, 171], [226, 226], [277, 321], [413, 422], [481, 486]],
    },
}


def source_item_nodes(data):
    """Index literal XML item nodes, preserving their exact original byte hashes."""
    parser = xml.parsers.expat.ParserCreate()
    nodes, stack, ordinal = defaultdict(list), [], 0

    def start(name, attrs):
        nonlocal ordinal
        if name == "item":
            ordinal += 1
            stack.append(
                (
                    parser.CurrentByteIndex,
                    parser.CurrentLineNumber,
                    dict(attrs),
                    ordinal,
                )
            )

    def end(name):
        if name != "item":
            return
        offset, line, attrs, order = stack.pop()
        quote = None
        first_end = offset
        for position in range(offset, len(data)):
            char = data[position]
            if quote is not None:
                if char == quote:
                    quote = None
            elif char in (34, 39):
                quote = char
            elif char == 62:
                first_end = position + 1
                break
        end_offset = (
            first_end
            if data[offset:first_end].rstrip().endswith(b"/>")
            else data.find(b">", parser.CurrentByteIndex) + 1
        )
        if "id" in attrs:
            first = last = int(attrs["id"])
        elif "fromid" in attrs and "toid" in attrs:
            first, last = int(attrs["fromid"]), int(attrs["toid"])
        else:
            return
        if not 0 <= first <= last <= 65535:
            return
        proof = {
            "xpath": f"/items/item[{order}]",
            "line": line,
            "byte_start": offset,
            "byte_end_exclusive": end_offset,
            "node_sha256": digest(data[offset:end_offset]),
            "attributes": attrs,
        }
        for item_id in range(first, last + 1):
            nodes[item_id].append(proof)

    parser.StartElementHandler, parser.EndElementHandler = start, end
    parser.Parse(data, True)
    return nodes


def load_item_sources(source_dir, revisions):
    """No caller-supplied mapping: only these pinned, reviewed loader/XML pairs qualify."""
    proofs = {}
    if source_dir is None:
        return proofs
    for source, spec in ITEM_SOURCES.items():
        if revisions.get(source) != spec["revision"]:
            continue
        xml_path, loader_path = (
            Path(source_dir) / source / "data/items/items.xml",
            Path(source_dir) / source / "src/items/items.cpp",
        )
        if not xml_path.is_file() or not loader_path.is_file():
            continue
        data, loader = xml_path.read_bytes(), loader_path.read_bytes()
        if (
            digest(data) != spec["xml_sha256"]
            or digest(loader) != spec["loader_sha256"]
        ):
            raise StageError("unqualified Item source bytes: " + source)
        proofs[source] = {"nodes": source_item_nodes(data), "source": spec}
    return proofs


def bridge_item(reference, proof, aliases, registry, membership):
    match = re.fullmatch(r"(canary|crystalserver):item/(\d+)", reference["key"])
    if not match or match[1] not in proof:
        return {
            "state": "PINNED_ITEM_IDENTITY_LOADER_EVIDENCE_MISSING",
            "native_reference": None,
        }
    source, item_id = match[1], int(match[2])
    spec = proof[source]["source"]
    nodes = proof[source]["nodes"].get(item_id, [])
    if item_id < 100:
        return {"state": "SOURCE_ID_IS_FLUID_ENUM_NOT_OBJECT", "native_reference": None}
    if len(nodes) != 1:
        return {
            "state": "SOURCE_XML_ITEM_MISSING"
            if not nodes
            else "SOURCE_XML_ITEM_AMBIGUOUS",
            "native_reference": None,
            "searched_source_catalogue": {
                "repository": spec["repository"],
                "revision": spec["revision"],
                "path": "data/items/items.xml",
                "sha256": spec["xml_sha256"],
                "source_server_id": item_id,
            },
        }
    spec = proof[source]["source"]
    source_proof = {
        "repository": spec["repository"],
        "revision": spec["revision"],
        "xml": {
            "path": "data/items/items.xml",
            "sha256": spec["xml_sha256"],
            **nodes[0],
        },
        "loader": {
            "path": "src/items/items.cpp",
            "sha256": spec["loader_sha256"],
            "line_ranges": spec["loader_lines"],
        },
        "mapping": "XML_ID_TO_INDEXED_ITEMTYPE_TO_PROTOBUF_OBJECT_ID",
        "source_server_id": item_id,
        "source_client_object_id": item_id,
    }
    current = {}
    for row in aliases["entries"]:
        if row["key"] not in current or row["version"] > current[row["key"]]["version"]:
            current[row["key"]] = row
    rows = [
        row
        for row in current.values()
        if row.get("state") == "ALIAS"
        and row.get("evidence", {}).get("source_item_id") == item_id
    ]
    targets = {row["target"] for row in rows}
    expected = "oteryn:item.tibia.i" + str(item_id)
    if aliases.get("key_rule") != "OTERYN_TIBIA_ID_KEY_RULE_V1" or targets != {
        expected
    }:
        return {
            "state": "PROTECTED_ITEM_ALIAS_TARGET_MISSING_OR_AMBIGUOUS",
            "native_reference": None,
            "identity_bridge": source_proof,
        }
    if expected not in registry or item_id not in membership:
        return {
            "state": "NATIVE_ITEM_OR_PROTECTED_APPEARANCE_MISSING",
            "native_reference": None,
            "identity_bridge": source_proof,
        }
    return {
        "state": "BOUND",
        "native_reference": registry[expected],
        "identity_bridge": source_proof,
        "protected_aliases": rows,
        "qualification": "SOURCE_IDENTITY_ONLY; no gameplay or mint authority",
    }


class StageError(ValueError):
    pass


def canonical(value):
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def helper_proof(source_ref, source_dir, source_revisions, specifications=HELPERS):
    spec = specifications.get(source_ref)
    if (
        not spec
        or source_dir is None
        or source_revisions.get("canary") != CANARY_QUEST_REVISION
    ):
        return None
    caller_name = spec["target"].split("/")[-1] + ".lua"
    helper_name = source_ref.split("/")[-1] + ".lua"
    blobs = {}
    for role, name in [("caller", caller_name), ("helper", helper_name)]:
        path = Path(source_dir) / name
        if not path.is_file():
            return None
        data = path.read_bytes()
        if digest(data) != spec[f"{role}_sha256"]:
            return None
        blobs[role] = data.decode("utf-8")
    symbol = re.escape(spec["symbol"])
    definition = re.compile(r"^[ \t]*function[ \t]+" + symbol + r"\s*\(", re.MULTILINE)
    if not definition.search(blobs["helper"]):
        return None
    if spec["mode"] == "called_helper":
        relation = re.compile(r"^[ \t]*" + symbol + r"\s*\(", re.MULTILINE)
    else:
        relation = re.compile(
            r'^[ \t]*dofile\s*\(\s*DATA_DIRECTORY\s*\.\.\s*"/npc/'
            + re.escape(helper_name)
            + r'"\s*\)',
            re.MULTILINE,
        )
    match = relation.search(blobs["caller"])
    if not match:
        return None

    def evidence(role, name, line):
        return {
            "repository": "opentibiabr/canary",
            "revision": CANARY_QUEST_REVISION,
            "path": "data-otservbr-global/npc/" + name,
            "sha256": spec[f"{role}_sha256"],
            "line": line,
        }

    return {
        "target_source_npc": spec["target"],
        "relationship": spec["mode"],
        "caller": evidence(
            "caller", caller_name, blobs["caller"][: match.start()].count("\n") + 1
        ),
        "helper": evidence(
            "helper",
            helper_name,
            blobs["helper"][: definition.search(blobs["helper"]).start()].count("\n")
            + 1,
        ),
        "execution_proven": False,
        "qualification": "HELPER_OWNERSHIP_ONLY; historical runtime dispatch and Game semantics are not qualified",
    }


def npc_bindings(records):
    index = defaultdict(list)
    identities = set()
    for record in records:
        declaration = record["declaration"]
        identity = declaration["identity"]
        key = identity["key"]
        if key in identities or not key.startswith("oteryn:npc."):
            raise StageError("duplicate or nonnative NPC identity: " + key)
        identities.add(key)
        for bi, binding in enumerate(record.get("source_bindings", [])):
            prefix = {
                "canary/npc-file": "canary:npc/",
                "crystalserver/npc-file": "crystal:npc/",
            }.get(binding["identity_namespace"])
            if prefix and binding.get("disposition") == "EXACT":
                if binding["target"] != {"family": "NPC", **identity}:
                    raise StageError(
                        "NPC source binding target differs from record identity"
                    )
                index[prefix + binding["external_id"]].append(
                    {
                        "npc": {"family": "NPC", **identity},
                        "binding": binding,
                        "evidence": dict(
                            record.get("_source_evidence", {}),
                            json_pointer=record.get("_source_evidence", {}).get(
                                "json_pointer", ""
                            )
                            + f"/source_bindings/{bi}",
                        ),
                    }
                )
    return index


def resolve_npc(source_ref, index, source_dir, revisions):
    matches = index.get(source_ref, [])
    proof = None
    if not matches:
        proof = helper_proof(source_ref, source_dir, revisions)
        if proof:
            matches = index.get(proof["target_source_npc"], [])
    keys = {m["npc"]["key"] for m in matches}
    if len(keys) != 1:
        return None, {
            "reason": "AMBIGUOUS_NPC_SOURCE_BINDING"
            if keys
            else "NO_PROVEN_NPC_SOURCE_BINDING",
            "source_npc": source_ref,
            "candidates": sorted(keys),
        }
    return matches[0], proof


def walk_refs(value, pointer=""):
    if isinstance(value, dict):
        if value.get("family") == "Item" and isinstance(value.get("key"), str):
            yield pointer, value
        for k, v in value.items():
            yield from walk_refs(
                v, pointer + "/" + k.replace("~", "~0").replace("/", "~1")
            )
    elif isinstance(value, list):
        for i, v in enumerate(value):
            yield from walk_refs(v, pointer + "/" + str(i))


def join_item(reference, bindings, registry, revisions):
    if reference["key"].startswith("oteryn:item."):
        target = registry.get(reference["key"])
        return {
            "state": "BOUND"
            if target == reference
            else "NATIVE_ITEM_MISSING_OR_REVISION_MISMATCH",
            "native_reference": target if target == reference else None,
        }
    match = re.fullmatch(r"(canary|crystalserver):item/(\d+)", reference["key"])
    if not match:
        return {"state": "UNSUPPORTED_SOURCE_ITEM_NAMESPACE", "native_reference": None}
    source, external_id = match.groups()
    candidates = [
        b
        for b in bindings
        if b.get("source_key") == "oteryn:source." + source
        and b.get("identity_namespace") == "ots/item_server_id"
        and b.get("external_id") == external_id
        and b.get("disposition") == "EXACT"
    ]
    exact = [
        b
        for b in candidates
        if b.get("source_revision") == revisions.get(source)
        and registry.get(b["target"]["key"]) == b["target"]
    ]
    targets = {canonical(b["target"]) for b in exact}
    if len(targets) == 1:
        return {
            "state": "BOUND",
            "native_reference": exact[0]["target"],
            "binding": exact[0],
        }
    reason = (
        "AMBIGUOUS_SOURCE_ITEM_BINDING"
        if len(targets) > 1
        else "SOURCE_ITEM_REVISION_OR_NATIVE_REGISTRY_MISMATCH"
        if candidates
        else "SOURCE_ITEM_BINDING_MISSING"
    )
    return {"state": reason, "native_reference": None, "candidate_bindings": candidates}


def stage(root=ROOT, helper_source_dir=None, item_source_dir=None):
    root = Path(root)
    custody = []

    def read(relative):
        data = (root / relative).read_bytes()
        custody.append({"path": relative, "sha256": digest(data)})
        return json.loads(data), digest(data)

    def located(relative, array):
        data, sha = read(relative)
        return [
            {
                "record": row,
                "evidence": {
                    "path": relative,
                    "sha256": sha,
                    "json_pointer": f"/{array}/{i}",
                },
            }
            for i, row in enumerate(data[array])
        ]

    quest_path = SAMPLES + "questlog/quests.json"
    quests = located(quest_path, "quests")
    progress = located(SAMPLES + "questlog/progress.json", "progress")
    gates = located(SAMPLES + "doors/gates.json", "gates")
    claims = located(SAMPLES + "chests/claims.json", "claims")
    manifest, manifest_sha = read(SAMPLES + "questlog/manifest.json")
    revisions = {}
    for source in manifest["sources"]:
        name = {
            "opentibiabr/canary": "canary",
            "zimbadev/crystalserver": "crystalserver",
        }.get(source["repository"])
        if name:
            if name in revisions and revisions[name] != source["revision"]:
                raise StageError("conflicting quest source revisions")
            revisions[name] = source["revision"]
    npc_records = []
    for path in sorted((root / "content/npcs/definitions").glob("npcs-*.json")):
        relative = path.relative_to(root).as_posix()
        data, sha = read(relative)
        npc_records.extend(
            dict(
                row,
                _source_evidence={
                    "path": relative,
                    "sha256": sha,
                    "json_pointer": f"/records/{i}",
                },
            )
            for i, row in enumerate(data["records"])
        )
    index = npc_bindings(npc_records)
    registry = {}
    registry_evidence = {}
    item_bindings = []
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        relative = path.relative_to(root).as_posix()
        data, sha = read(relative)
        for ri, row in enumerate(data["records"]):
            identity = row["definition"]["identity"]
            if identity["family"] != "Item" or identity["key"] in registry:
                raise StageError("invalid or duplicate Item identity")
            registry[identity["key"]] = identity
            registry_evidence[identity["key"]] = {
                "path": relative,
                "sha256": sha,
                "json_pointer": f"/records/{ri}/definition/identity",
            }
            item_bindings.extend(row.get("source_bindings", []))
    for path in sorted((root / "imports").glob("*/bindings/items.json")):
        data, _ = read(path.relative_to(root).as_posix())
        item_bindings.extend(data["bindings"])
    assignments = defaultdict(list)
    held = []
    selected_quests = set()
    total = 0
    for source_quest in quests:
        q = source_quest["record"]
        for mi, mission in enumerate(q.get("missions", [])):
            for ti, transition in enumerate(mission.get("transitions", [])):
                if transition.get("owner") != "npc":
                    continue
                total += 1
                requested = transition.get("requested_by", {})
                evidence = dict(
                    source_quest["evidence"],
                    json_pointer=source_quest["evidence"]["json_pointer"]
                    + f"/missions/{mi}/transitions/{ti}",
                )
                item = {
                    "source_quest": q["identity"],
                    "source_mission": mission["key"],
                    "source_track": mission["progress"],
                    "transition": transition,
                    "evidence": evidence,
                    "runtime_eligible": False,
                    "hold_reasons": ["TYPED_GAME_QUEST_BINDING_NOT_IMPLEMENTED"],
                }
                resolved, proof = resolve_npc(
                    requested.get("npc"), index, helper_source_dir, revisions
                )
                if not resolved:
                    held.append(dict(item, identity_hold=proof))
                    continue
                item["native_identity_binding"] = resolved["binding"]
                item["native_identity_binding_evidence"] = resolved["evidence"]
                item["identity_qualification"] = (
                    "EXACT_FILENAME_BINDING; source gameplay revision is retained separately"
                )
                if proof:
                    item["helper_ownership_evidence"] = proof
                if "computed" in transition:
                    item["hold_reasons"].append(
                        "COMPUTED_TRANSITION_EFFECT_NOT_SUPPORTED"
                    )
                assignments[resolved["npc"]["key"]].append(item)
                selected_quests.add(q["identity"]["key"])
    selected = [q for q in quests if q["record"]["identity"]["key"] in selected_quests]
    tracks = {m["progress"] for q in selected for m in q["record"].get("missions", [])}
    tracks.update((q["record"].get("start") or {}).get("progress") for q in selected)
    selected_progress = [
        p
        for p in progress
        if p["record"]["key"] in tracks
        or any(
            m.split("#", 1)[0] in selected_quests
            for m in p["record"].get("missions", [])
        )
    ]
    gate_keys = {g["key"] for q in selected for g in q["record"].get("gates", [])}
    claim_keys = {c["key"] for q in selected for c in q["record"].get("claims", [])}
    selected_gates = [g for g in gates if g["record"]["identity"]["key"] in gate_keys]
    selected_claims = [
        c for c in claims if c["record"]["identity"]["key"] in claim_keys
    ]
    item_proofs = load_item_sources(item_source_dir, revisions)
    aliases, membership = None, set()
    if item_proofs:
        aliases, alias_sha = read("content/items/aliases.json")
        admitted, admitted_sha = read(
            "imports/official/appearance-membership/admitted.json"
        )
        if (
            aliases.get("decision") != admitted.get("decision")
            or admitted.get("decision") != "A12-ITEM-IDENTITY-TIBIA-ID-V1"
        ):
            raise StageError("protected Item identity decision mismatch")
        for entry in admitted["files"]:
            data, sha = read(
                "imports/official/appearance-membership/" + entry["manifest"]
            )
            if sha != entry["manifest_sha256"]:
                raise StageError("protected appearance manifest digest mismatch")
            membership.update(row[0] for row in data["entries"])
    dependencies = []
    for context in selected + selected_progress + selected_gates + selected_claims:
        for pointer, reference in walk_refs(context["record"]):
            provenance = dict(
                context["evidence"],
                json_pointer=context["evidence"]["json_pointer"] + pointer,
            )
            joined = join_item(reference, item_bindings, registry, revisions)
            if joined["state"] != "BOUND" and item_proofs:
                joined = bridge_item(
                    reference, item_proofs, aliases, registry, membership
                )
                joined["protected_alias_file"] = {
                    "path": "content/items/aliases.json",
                    "sha256": alias_sha,
                }
                joined["protected_appearance_index"] = {
                    "path": "imports/official/appearance-membership/admitted.json",
                    "sha256": admitted_sha,
                }
            if joined["state"] == "BOUND":
                joined["native_registry_evidence"] = registry_evidence[
                    joined["native_reference"]["key"]
                ]
            dependencies.append(
                {
                    "source_reference": reference,
                    "scope": "RELATED_QUEST_CONTEXT; not NPC hand-in or reward authority",
                    "evidence": provenance,
                    **joined,
                }
            )
    records = []
    for key, links in sorted(assignments.items()):
        links.sort(
            key=lambda link: (
                link["source_quest"]["key"],
                link["source_mission"],
                link["transition"]["key"],
            )
        )
        records.append(
            {
                "npc_key": key,
                "source_assignments": links,
                "candidate_field": {
                    "field_path": "oteryn:source.npc.quest_bindings",
                    "value": {
                        "type": "Text",
                        "value": canonical(
                            {
                                "classification": "OTS_HYPOTHESIS_ONLY",
                                "runtime_eligible": False,
                                "assignments": links,
                            }
                        )
                        .decode()
                        .rstrip("\n"),
                    },
                },
            }
        )
    bound = sum(len(row["source_assignments"]) for row in records)
    if total != bound + len(held):
        raise StageError("source NPC transition partition failed")
    return {
        "schema": "OTERYN_NPC_SOURCE_QUEST_BINDING_STAGE/v1",
        "classification": "SOURCE_CANDIDATE_ONLY",
        "runtime_eligible": False,
        "native_quest_declarations": [],
        "inputs": sorted(custody, key=lambda entry: entry["path"]),
        "source_revisions": revisions,
        "quest_manifest_sha256": manifest_sha,
        "item_source_captures": [p["source"] for _, p in sorted(item_proofs.items())],
        "counts": {
            "source_npc_transitions": total,
            "associated_transitions": bound,
            "associated_npcs": len(records),
            "held_identity_transitions": len(held),
            "source_quests": len(selected),
            "helper_associations": sum(
                "helper_ownership_evidence" in link
                for row in records
                for link in row["source_assignments"]
            ),
            "item_dependency_occurrences": len(dependencies),
            "item_dependencies_bound": sum(d["state"] == "BOUND" for d in dependencies),
            "item_dependencies_held": sum(d["state"] != "BOUND" for d in dependencies),
            "item_source_references_bound": len(
                {
                    d["source_reference"]["key"]
                    for d in dependencies
                    if d["state"] == "BOUND"
                }
            ),
            "item_native_targets_bound": len(
                {
                    d["native_reference"]["key"]
                    for d in dependencies
                    if d["state"] == "BOUND"
                }
            ),
        },
        "records": records,
        "held": held,
        "source_quests": selected,
        "source_progress": selected_progress,
        "source_gates": selected_gates,
        "source_claims": selected_claims,
        "item_dependencies": dependencies,
        "runtime_dependencies": [
            "accepted QUEST-STATE0 and QUEST-GATE0 contracts",
            "NPC-QUEST-CONTENT-1 child allocation and native quest identity/track lowering",
            "QUEST-STATE-1/QUEST-PRED-1/NPC-QUEST-1 runtime and durable writers",
            "typed claim/exchange bindings; placement owners for held source Item-appearance occurrences",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--helper-source-dir", type=Path)
    parser.add_argument("--item-source-dir", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = stage(args.root, args.helper_source_dir, args.item_source_dir)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(canonical(result))
    print(json.dumps(result["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
