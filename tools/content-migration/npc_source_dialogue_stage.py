#!/usr/bin/env python3
"""Prepare proof-linked NPC source dialogue candidates without activating them.

Transcript player prompts are observations, not proof of a native keyword matcher.
Original complete source branches and their semantic proofs are retained separately
from transformed candidates. Admission requires independent matcher, precedence
and guard/action evidence; source VERIFIED alone never grants admission.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path
from typing import Any


class StageError(ValueError):
    pass


def encode(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def source_encode(value: Any) -> bytes:
    # Frozen R4 keyword custody uses the export packet's indent=1 serializer.
    return (
        json.dumps(value, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    ).encode("utf-8")


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def semantics(node: dict) -> dict:
    return {
        key: [semantics(child) for child in value] if key == "children" else value
        for key, value in node.items()
        if key != "key"
    }


def semantic_hash(node: dict) -> str:
    return digest(
        json.dumps(
            semantics(node), ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    )


def unique(records: list[dict], key: str) -> dict[str, dict]:
    out = {record[key]: record for record in records}
    if len(out) != len(records):
        raise StageError(f"duplicate {key}")
    return out


def walk(nodes: list[dict], pointer: str = "/keywords"):
    for index, node in enumerate(nodes):
        path = f"{pointer}/{index}"
        yield node, path
        yield from walk(node.get("children", []), path + "/children")


def load_packet(packet: Path) -> dict[str, Any]:
    manifest = json.loads((packet / "manifest.json").read_bytes())
    if manifest.get("scope") != "SOURCE_AUTHORING_CANDIDATES_ONLY":
        raise StageError("packet is not source authoring evidence")
    result = {}
    for item in manifest["files"]:
        name = item["path"]
        if Path(name).name != name:
            raise StageError("packet path escapes directory")
        raw = (packet / name).read_bytes()
        if digest(raw) != item["sha256"] or len(raw) != item["bytes"]:
            raise StageError(f"custody mismatch: {name}")
        result[name] = json.loads(raw)
    result["manifest"] = manifest
    return result


def recorded_context(
    source: dict, recaptures: dict[str, bytes]
) -> tuple[set[str], str | None]:
    """Use only a digest-bound transcript with an explicit Player actor label."""
    metadata = source.get("source_metadata") or {}
    expected = metadata.get("raw_sha256")
    raw = recaptures.get(source["npc_key"])
    if raw is None:
        return set(), None
    if not expected or digest(raw) != expected:
        raise StageError(f"recaptured transcript digest mismatch: {source['npc_key']}")
    text = raw.decode("utf-8")
    if not any(line.startswith("Player:") for line in text.splitlines()):
        return set(), expected
    # Do not infer a nickname, alternate actor, or source substitution syntax.
    prefix = source["name"] + ":"
    spoken = {
        line[len(prefix) :].strip()
        for line in text.splitlines()
        if line.startswith(prefix)
    }
    return spoken, expected


def transform_branch(
    branch: dict, pointer: str, spoken: set[str]
) -> tuple[dict | None, list[dict]]:
    """Suppress the entire root when a generated macro occurs, never truncate children."""
    transforms = []
    for node, path in walk([branch], ""):
        for index, reply in enumerate(node["reply"]):
            if re.search(r"<[A-Za-z_][A-Za-z0-9_]*\([^<>]*\)>", reply):
                transforms.append(
                    {
                        "kind": "GENERATED_MACRO_HOLD",
                        "pointer": pointer + path[2:] + f"/reply/{index}",
                        "text": reply,
                    }
                )
    if transforms:
        return None, transforms
    candidate = copy.deepcopy(branch)
    for node, path in walk([candidate], ""):
        for index, reply in enumerate(node["reply"]):
            if reply in spoken and re.search(r"\bPlayer\b", reply):
                normalized = re.sub(r"\bPlayer\b", "|PLAYERNAME|", reply)
                node["reply"][index] = normalized
                transforms.append(
                    {
                        "kind": "PROVEN_RECORDED_PLAYER_PLACEHOLDER",
                        "pointer": pointer + path[2:] + f"/reply/{index}",
                        "original": reply,
                        "normalized": normalized,
                    }
                )
    return candidate, transforms


def stage(
    payloads: dict,
    active_npcs: list[dict],
    active_dialogues: list[dict],
    recaptures: dict[str, bytes] | None = None,
    qualifications: dict[str, dict] | None = None,
) -> dict:
    recaptures = recaptures or {}
    qualifications = qualifications or {}
    source_records = payloads["source-index.json"]["records"]
    sources = unique(source_records, "npc_key")
    dialogue_records = payloads["dialogue-candidates.json"]["records"]
    dialogues = {d["identity"]["key"]: d for d in dialogue_records}
    if len(dialogues) != len(dialogue_records):
        raise StageError("duplicate source Dialogue identity")
    proofs = unique(payloads["root-proofs.json"]["npcs"], "npc_key")
    npcs = {n["identity"]["key"]: n for n in active_npcs}
    active = {d["identity"]["key"]: d for d in active_dialogues}
    if set(recaptures) - set(sources):
        raise StageError("recapture refers to unknown NPC")
    if set(proofs) != {
        key for key, source in sources.items() if source["accepted_roots"]
    }:
        raise StageError("root proof NPC inventory mismatch")
    if set(qualifications) - set(sources):
        raise StageError("qualification refers to unknown NPC")
    rows = []
    for index, source in enumerate(source_records):
        key = source["npc_key"]
        ref = source.get("dialogue")
        declaration = dialogues.get(ref["key"]) if ref else None
        roots = declaration.get("keywords", []) if declaration else []
        if len(roots) != source["accepted_roots"]:
            raise StageError(f"root count mismatch: {key}")
        if declaration and (
            declaration["identity"]["revision"] != ref["revision"]
            or digest(source_encode(roots)) != source["candidate_keywords_sha256"]
        ):
            raise StageError(f"dialogue custody mismatch: {key}")
        branch_proofs = proofs.get(key, {}).get("root_proofs", [])
        if len(branch_proofs) != len(roots):
            raise StageError(f"root proof count mismatch: {key}")
        spoken, raw_hash = recorded_context(source, recaptures)
        candidate_roots = []
        transformations = []
        proof_links = []
        for root_index, (root, proof) in enumerate(zip(roots, branch_proofs)):
            if (
                len(proof) != 3
                or root["key"] != proof[0]
                or semantic_hash(root) != proof[1]
            ):
                raise StageError(f"root proof mismatch: {key}/{root_index}")
            candidate, changes = transform_branch(
                root, f"/keywords/{root_index}", spoken
            )
            transformations.extend(changes)
            link = {
                "root_key": root["key"],
                "source_semantic_sha256": proof[1],
                "source_locators": copy.deepcopy(proof[2]),
                "source_root_index": root_index,
                "native_candidate_held": candidate is None,
            }
            if candidate is not None:
                link["candidate_root_index"] = len(candidate_roots)
                link["candidate_semantic_sha256"] = semantic_hash(candidate)
                candidate_roots.append(candidate)
            proof_links.append(link)
        qualification = qualifications.get(key, {})
        # The caller may provide independently qualified static semantics, never runtime execution.
        if qualification and qualification.get("source_keywords_sha256") != source.get(
            "candidate_keywords_sha256"
        ):
            raise StageError(
                f"qualification is not bound to current source keywords: {key}"
            )
        qualified = (
            bool(candidate_roots)
            and all(
                qualification.get(flag) is True
                for flag in (
                    "game_keyword_matcher_proven",
                    "game_handler_precedence_proven",
                    "quest_guards_focus_actions_proven",
                )
            )
            and bool(qualification.get("evidence"))
        )
        candidate = copy.deepcopy(declaration) if declaration else None
        if candidate:
            candidate["keywords"] = candidate_roots
        old_npc = npcs.get(key, {})
        old_ref = old_npc.get("dialogue")
        old = active.get(old_ref["key"]) if old_ref else None
        holds = []
        if not roots:
            holds.append("EMPTY_SOURCE_HOLD" if declaration else "NO_SOURCE_PROGRAM")
        if source["held_root_groups"]:
            holds.append("UNRESOLVED_SOURCE_ROOT_GROUPS")
        if not qualified and roots:
            holds.append("MATCHER_PRECEDENCE_GUARDS_ACTIONS_UNPROVEN")
        if any(t["kind"] == "GENERATED_MACRO_HOLD" for t in transformations):
            holds.append("GENERATED_MACRO_REQUIRES_NATIVE_IMPLEMENTATION")
        rows.append(
            {
                "npc_key": key,
                "name": source["name"],
                "state": "STATIC_ADMISSION_CANDIDATE" if qualified else "SOURCE_ONLY",
                "runtime_qualified": False,
                "global_branch_complete": False,
                "source_status": source.get("source_status"),
                "source_id": source.get("source_id"),
                "source_index_pointer": f"/records/{index}",
                "held_root_groups": source["held_root_groups"],
                "held_groups_sha256": source.get("held_groups_sha256"),
                "holds": holds,
                "source_declaration": copy.deepcopy(declaration),
                "candidate_declaration": candidate,
                "source_root_proofs": copy.deepcopy(branch_proofs),
                "candidate_proof_links": proof_links,
                "transformations": transformations,
                "recaptured_raw_sha256": raw_hash,
                "static_qualification": copy.deepcopy(qualification),
                "active_npc_present": key in npcs,
                "active_dialogue_reference": old_ref,
                "active_dialogue_sha256": digest(encode(old)) if old else None,
                "proposed_dialogue_reference": ref,
                "stored_source_differs_from_active": old != declaration,
            }
        )
    return {
        "schema": "oteryn.npc-source-dialogue-stage.v1",
        "scope": "UNACTIVATED_SOURCE_AND_STATIC_ADMISSION_CANDIDATES",
        "runtime_qualified": False,
        "global_complete": False,
        "active_content_modified": False,
        "packet_custody": copy.deepcopy(payloads.get("manifest", {}).get("files", [])),
        "counts": {
            "npcs": len(rows),
            "nonempty_source_programs": sum(
                bool(
                    r["source_declaration"] and r["source_declaration"].get("keywords")
                )
                for r in rows
            ),
            "static_admission_candidates": sum(
                r["state"] == "STATIC_ADMISSION_CANDIDATE" for r in rows
            ),
            "source_roots": sum(len(r["source_root_proofs"]) for r in rows),
            "held_root_groups": sum(r["held_root_groups"] for r in rows),
        },
        "records": rows,
    }


def transcript_blocks(source: dict, raw: bytes) -> list[dict]:
    """Recorded prompt alternatives support speech observations, never actions."""
    recorded_context(source, {source["npc_key"]: raw})
    blocks = []
    current = None
    prefix = source["name"] + ":"
    for line_number, line in enumerate(raw.decode("utf-8").splitlines(), 1):
        if line.startswith("Player:"):
            prompts = [part.strip().lower() for part in line[7:].split("/")]
            current = {
                "prompts": prompts,
                "reply": [],
                "player_line": line_number,
                "reply_lines": [],
            }
            blocks.append(current)
        elif current is not None and line.startswith(prefix):
            current["reply"].append(line[len(prefix) :].strip())
            current["reply_lines"].append(line_number)
    return blocks


def same_topology(left: dict, right: dict) -> bool:
    return (
        left.get("triggers", []) == right.get("triggers", [])
        and len(left.get("children", [])) == len(right.get("children", []))
        and all(
            same_topology(a, b)
            for a, b in zip(left.get("children", []), right.get("children", []))
        )
    )


def static_repairs(
    payloads: dict,
    active_npcs: list[dict],
    active_dialogues: list[dict],
    recaptures: dict[str, bytes],
    admitted_stage: dict,
) -> dict:
    """Repair proven static speech; retain native matchers, flags and all other roots."""
    validated = stage(payloads, active_npcs, active_dialogues, recaptures)
    sources = {r["npc_key"]: r for r in payloads["source-index.json"]["records"]}
    active = {d["identity"]["key"]: d for d in active_dialogues}
    admitted = {r["npc"]: r for r in admitted_stage["dialogues"]}
    rows = []
    held_aliases = []
    for row in validated["records"]:
        key = row["npc_key"]
        ref = row["active_dialogue_reference"]
        native = active.get(ref["key"]) if ref else None
        selected = admitted.get(key)
        if (
            not native
            or not selected
            or selected["declaration"] != native
            or key not in recaptures
            or not row["source_declaration"]
        ):
            continue
        blocks = transcript_blocks(sources[key], recaptures[key])
        source_roots = row["source_declaration"]["keywords"]
        source_by_triggers = {}
        for index, node in enumerate(source_roots):
            source_by_triggers.setdefault(tuple(node["triggers"]), []).append(
                (index, node)
            )
        candidate = copy.deepcopy(native)
        changes = []
        alias_holds = []
        for index, node in enumerate(native.get("keywords", [])):
            matches = source_by_triggers.get(tuple(node.get("triggers", [])), [])
            if len(matches) != 1 or node.get("fallback"):
                continue
            source_index, observed = matches[0]
            if not same_topology(node, observed):
                continue
            supporting = [
                block
                for block in blocks
                if len(observed["triggers"]) == 1
                and observed["triggers"][0] in block["prompts"]
                and block["reply"] == observed["reply"]
            ]
            # Multiple recordings can agree; conflicting replies for this prompt remain held.
            prompt_variants = {
                tuple(block["reply"])
                for block in blocks
                if len(observed["triggers"]) == 1
                and observed["triggers"][0] in block["prompts"]
            }
            if not supporting or len(prompt_variants) != 1:
                continue
            observed_candidate, normalization = transform_branch(
                observed,
                f"/keywords/{source_index}",
                {reply for block in supporting for reply in block["reply"]},
            )
            if observed_candidate is None:
                continue
            new_reply = observed_candidate["reply"]
            proof = {
                "source_root_index": source_index,
                "source_root_proof": row["source_root_proofs"][source_index],
                "source_raw_sha256": row["recaptured_raw_sha256"],
                "recorded_player_lines": [block["player_line"] for block in supporting],
                "recorded_reply_lines": [block["reply_lines"] for block in supporting],
                "native_root_semantic_sha256": semantic_hash(node),
                "native_static_provenance": selected["provenance"],
                "native_stage_source": admitted_stage["source"],
            }

            def comparable(reply_parts):
                return [
                    re.sub(r"[{}]", "", text).strip().rstrip(".!?").casefold()
                    for text in reply_parts
                ]

            if comparable(node["reply"]) != comparable(new_reply):
                candidate["keywords"][index]["reply"] = new_reply
                changes.append(
                    {
                        "kind": "STATIC_REPLY_REPLACEMENT",
                        "pointer": f"/keywords/{index}/reply",
                        "before": node["reply"],
                        "after": new_reply,
                        "normalization": normalization,
                        "proof": proof,
                    }
                )
            # A slash-separated recorded alias can share an already admitted static branch.
            # Require a one-word literal, no conversation flag or children, and no matching
            # earlier native pattern. Add no trade, quest or focus/action handler.
            if node.get("children") or any(
                node.get(flag)
                for flag in (
                    "only_focus",
                    "only_unfocus",
                    "reset",
                    "ungreet",
                    "move_up",
                )
            ):
                continue
            for block in supporting:
                for alias in block["prompts"]:
                    if (
                        not re.fullmatch(r"[a-z]+", alias)
                        or alias == observed["triggers"][0]
                    ):
                        continue
                    other_source = source_by_triggers.get((alias,), [])
                    if (
                        len(other_source) != 1
                        or other_source[0][1]["reply"] != observed["reply"]
                    ):
                        continue
                    if any(
                        all(trigger in alias for trigger in sibling.get("triggers", []))
                        for sibling in candidate.get("keywords", [])
                        if not sibling.get("fallback")
                    ):
                        continue
                    alias_node = copy.deepcopy(candidate["keywords"][index])
                    alias_node["triggers"] = [alias]
                    alias_node["key"] = alias
                    existing_keys = {
                        sibling["key"] for sibling in candidate["keywords"]
                    }
                    suffix = 2
                    while alias_node["key"] in existing_keys:
                        alias_node["key"] = f"{alias}_{suffix}"
                        suffix += 1
                    if any(
                        held["after"]["triggers"] == [alias] for held in alias_holds
                    ):
                        continue
                    alias_holds.append(
                        {
                            "kind": "RECORDED_ALIAS_SOURCE_ONLY",
                            "hold": "ORIGINAL_NATIVE_MATCHER_AND_PRECEDENCE_NOT_PROVEN_FOR_ALIAS",
                            "after": alias_node,
                            "template_native_root_index": index,
                            "alias_source_root_proof": row["source_root_proofs"][
                                other_source[0][0]
                            ],
                            "recorded_prompt_line": block["player_line"],
                            "proof": proof,
                        }
                    )
        # send_trade is an already admitted native message hook. Replace only an
        # unresolved generated category macro with its exact recorded expansion.
        event_matches = source_by_triggers.get(("trade",), [])
        if (
            native.get("send_trade")
            and any(
                "<GetFormattedShopCategoryNames()>" in text
                for text in native["send_trade"]
            )
            and len(event_matches) == 1
        ):
            event_index, observed_event = event_matches[0]
            event_blocks = [block for block in blocks if block["prompts"] == ["trade"]]
            if (
                not observed_event.get("children")
                and event_blocks
                and {tuple(block["reply"]) for block in event_blocks}
                == {tuple(observed_event["reply"])}
                and all("<" not in reply for reply in observed_event["reply"])
            ):
                candidate["send_trade"] = copy.deepcopy(observed_event["reply"])
                changes.append(
                    {
                        "kind": "RECORDED_NATIVE_MESSAGE_EXPANSION",
                        "pointer": "/send_trade",
                        "before": native["send_trade"],
                        "after": candidate["send_trade"],
                        "proof": {
                            "source_root_proof": row["source_root_proofs"][event_index],
                            "source_raw_sha256": row["recaptured_raw_sha256"],
                            "recorded_player_lines": [
                                block["player_line"] for block in event_blocks
                            ],
                            "recorded_reply_lines": [
                                block["reply_lines"] for block in event_blocks
                            ],
                            "native_message_hook": "send_trade",
                            "native_static_provenance": selected["provenance"],
                            "native_stage_source": admitted_stage["source"],
                        },
                    }
                )
        held_aliases.extend(dict(proposal, npc_key=key) for proposal in alias_holds)
        if changes:
            rows.append(
                {
                    "npc_key": key,
                    "name": row["name"],
                    "classification": "PROVEN",
                    "scope": "EXISTING_ADMITTED_STATIC_DIALOGUE_SPEECH",
                    "runtime_qualified": False,
                    "global_branch_complete": False,
                    "held_root_groups": row["held_root_groups"],
                    "active_dialogue_sha256": digest(encode(native)),
                    "native_declaration": candidate,
                    "changes": changes,
                    "unqualified_alias_proposals": alias_holds,
                }
            )
    return {
        "schema": "oteryn.npc-static-dialogue-repairs.v1",
        "runtime_qualified": False,
        "global_complete": False,
        "active_content_modified": False,
        "counts": {
            "npcs_with_repairs": len(rows),
            "reply_replacements": sum(
                c["kind"] == "STATIC_REPLY_REPLACEMENT"
                for r in rows
                for c in r["changes"]
            ),
            "held_alias_proposals": len(held_aliases),
            "native_message_expansions": sum(
                c["kind"] == "RECORDED_NATIVE_MESSAGE_EXPANSION"
                for r in rows
                for c in r["changes"]
            ),
        },
        "records": rows,
        "unqualified_alias_proposals": held_aliases,
    }


def active_records(root: Path, family: str) -> list[dict]:
    return [
        record["declaration"]
        for path in sorted((root / "content" / family / "definitions").glob("*.json"))
        for record in json.loads(path.read_bytes()).get("records", [])
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--active-root", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument(
        "--recaptures",
        type=Path,
        help="JSON records: npc_key, path; reads local digest-bound files",
    )
    parser.add_argument(
        "--static-repairs-out",
        type=Path,
        help="Prepare concrete repairs anchored to the committed D9/D10 static stage",
    )
    parser.add_argument(
        "--qualifications",
        type=Path,
        help="Per-NPC independently proven static semantics and evidence",
    )
    args = parser.parse_args()
    recaptures = {}
    if args.recaptures:
        for row in json.loads(args.recaptures.read_bytes())["records"]:
            if row["npc_key"] in recaptures:
                raise StageError("duplicate recapture NPC")
            recaptures[row["npc_key"]] = Path(row["path"]).read_bytes()
    qualifications = (
        json.loads(args.qualifications.read_bytes()) if args.qualifications else {}
    )
    result = stage(
        load_packet(args.packet),
        active_records(args.active_root, "npcs"),
        active_records(args.active_root, "dialogues"),
        recaptures,
        qualifications,
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(encode(result))
    print(json.dumps(result["counts"], sort_keys=True))
    if args.static_repairs_out:
        admitted = json.loads(
            (
                args.active_root
                / "docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json"
            ).read_bytes()
        )
        repairs = static_repairs(
            load_packet(args.packet),
            active_records(args.active_root, "npcs"),
            active_records(args.active_root, "dialogues"),
            recaptures,
            admitted,
        )
        args.static_repairs_out.parent.mkdir(parents=True, exist_ok=True)
        args.static_repairs_out.write_bytes(encode(repairs))
        print(json.dumps(repairs["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
