#!/usr/bin/env python3
"""Validate current research answers while keeping OTS and public evidence distinct."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
PACKET = HERE / "samples/current-behavior-answers.json"
TARGET = "global-tibia-current-2026-10-01"
AS_OF = "2026-10-01"
QUESTION_IDS = frozenset({
    "exact_target_snapshot", "etcher_consumption", "scroll_application_equipped_target",
    "fine_grained_timers", "combat_pipeline", "vibrancy_pvp_gate",
    "transaction_payment_sources", "failed_transaction_consumption_and_rollback",
    "native_effect_composition", "transfer_preserves_imbuement_state_and_remaining_duration",
    "etcher_npc_purchase_requires_premium", "scroll_consumption",
})
ENGINE_IDS = frozenset({"canary", "crystal_imbuements", "crystal_summer_update"})
GLOBAL_STATUSES = frozenset({
    "SOURCE_QUALIFIED_CURRENT_PUBLIC_REFERENCE", "PARTIALLY_SOURCED_CURRENT_PUBLIC_REFERENCE",
    "NO_EXPLICIT_PUBLIC_CONFIRMATION", "OWNER_SCOPE_RESOLVED",
})
CATALOGUES = {"global-rules-evidence.json", "imbuement-combat.json"}


def nonempty(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def validate(packet: dict, *, global_evidence: dict | None = None,
             combat_evidence: dict | None = None) -> list[str]:
    """Return errors; optional source catalogues keep fixture tests entirely offline."""
    errors: list[str] = []

    def require(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)

    if not isinstance(packet, dict):
        return ["behavior answers must be an object"]
    for field, expected in {
        "schema": "OTERYN_IMBUEMENT_CURRENT_BEHAVIOR_ANSWERS/v1",
        "target": TARGET, "activation": "DRAFT_NOT_RUNTIME_READY", "as_of": AS_OF,
    }.items():
        require(packet.get(field) == expected, f"{field}: expected {expected}")

    registry = packet.get("sources")
    if not isinstance(registry, dict) or not registry:
        errors.append("sources: expected a nonempty OTS source registry")
        registry = {}
    for source_id, source in registry.items():
        label = f"sources.{source_id}"
        require(nonempty(source_id), f"{label}: source ID must be nonempty")
        if not isinstance(source, dict):
            errors.append(f"{label}: source must be an object")
            continue
        revision = source.get("revision")
        require(isinstance(revision, str) and bool(re.fullmatch(r"[0-9a-f]{40}", revision)),
                f"{label}: revision must be a lowercase 40-hex commit")
        sha = source.get("sha256")
        require(isinstance(sha, str) and bool(re.fullmatch(r"[0-9a-f]{64}", sha)),
                f"{label}: sha256 must be a lowercase 64-hex file digest")
        require(source.get("method") == "NORMAL_HTTP_PUBLIC_SOURCE_FILE",
                f"{label}: method must describe normal public source-file retrieval")
        require(source.get("role") == "OTS_IMPLEMENTATION_REFERENCE",
                f"{label}: OTS implementation cannot become official Global evidence")
        require(nonempty(source.get("digest_scope")), f"{label}: missing digest scope")
        url = source.get("url")
        match = None
        if isinstance(url, str):
            try:
                parts = urlsplit(url)
                if (parts.scheme == "https" and parts.netloc == "github.com"
                        and not parts.query and not parts.fragment
                        and not re.search(r"[\s\[\]()<>{}]", url)):
                    match = re.fullmatch(r"/([^/]+)/([^/]+)/blob/([0-9a-f]{40})/(.+)", parts.path)
            except ValueError:
                pass
        require(match is not None, f"{label}: URL must be an immutable public GitHub blob URL")
        if match:
            require(match.group(3) == revision, f"{label}: URL commit disagrees with revision")
            require(not any(segment in {".", "..", ""} for segment in match.group(4).split("/")),
                    f"{label}: malformed source-file path")
            for field in ("repo", "repository"):
                if field in source:
                    require(source[field] == f"{match.group(1)}/{match.group(2)}",
                            f"{label}: {field} disagrees with immutable source URL")
            if "path" in source:
                require(source["path"] == match.group(4), f"{label}: path disagrees with source URL")
        if "engine" in source:
            require(isinstance(source["engine"], str) and source["engine"] in ENGINE_IDS,
                    f"{label}: unknown source engine")
        if "branch" in source:
            require(nonempty(source["branch"]), f"{label}: empty contextual branch label")
        for field in ("line_start", "line_end"):
            if field in source:
                require(type(source[field]) is int and source[field] > 0,
                        f"{label}: {field} must be a positive integer")
        if type(source.get("line_start")) is int and type(source.get("line_end")) is int:
            require(source["line_end"] >= source["line_start"], f"{label}: reversed source line range")

    public_sources: dict[str, dict] = {}
    for name, supplied in (("global-rules-evidence.json", global_evidence),
                           ("imbuement-combat.json", combat_evidence)):
        if supplied is None:
            try:
                supplied = json.loads((HERE / "samples" / name).read_bytes())
            except (OSError, ValueError) as exc:
                errors.append(f"{name}: cannot read source catalogue: {exc}")
                supplied = {}
        sources = supplied.get("sources", {}) if isinstance(supplied, dict) else {}
        if isinstance(sources, list):
            sources = {row["id"]: row for row in sources
                       if isinstance(row, dict) and isinstance(row.get("id"), str)}
        require(isinstance(sources, dict), f"{name}: invalid source catalogue")
        public_sources[name] = sources if isinstance(sources, dict) else {}

    questions = packet.get("questions")
    if not isinstance(questions, list):
        errors.append("questions: expected the twelve original questions")
        questions = []
    ids = [q.get("id") for q in questions if isinstance(q, dict)]
    require(len(questions) == 12 and len(ids) == 12 and all(isinstance(x, str) for x in ids)
            and set(ids) == QUESTION_IDS, "questions: exact original twelve IDs required once each")
    owner_count = 0
    engine_count = 0
    for question in questions:
        if not isinstance(question, dict):
            errors.append("question must be an object")
            continue
        qid = question.get("id")
        label = f"questions.{qid}"
        owner = qid == "exact_target_snapshot"
        expected_resolution = "OWNER_SCOPE_RESOLVED" if owner else "SOURCE_QUALIFIED_ANSWER"
        require(question.get("resolution_status") == expected_resolution,
                f"{label}: expected resolution_status {expected_resolution}")
        owner_count += question.get("resolution_status") == "OWNER_SCOPE_RESOLVED"
        require(nonempty(question.get("answer")), f"{label}: missing qualified answer")
        engines = question.get("engine_answers")
        if not isinstance(engines, dict):
            errors.append(f"{label}: engine_answers must be an object")
            engines = {}
        require(set(engines) == (set() if owner else ENGINE_IDS),
                f"{label}: expected no owner-scope engines or all three named engine answers")
        engine_count += bool(engines)
        for engine, answer in engines.items():
            if not isinstance(answer, dict) or not answer:
                errors.append(f"{label}.{engine}: engine answer must be a nonempty object")
                continue
            require(nonempty(answer.get("answer")), f"{label}.{engine}: missing engine answer")
            refs = answer.get("source_refs")
            if not isinstance(refs, list) or not refs:
                errors.append(f"{label}.{engine}: nonempty OTS source_refs required")
                refs = []
            for ref in refs:
                valid = isinstance(ref, str) and ref in registry
                require(valid,
                        f"{label}.{engine}: unknown OTS source reference {ref!r}")
                source = registry[ref] if valid else None
                if isinstance(source, dict) and engine in ENGINE_IDS:
                    repo = "opentibiabr/canary" if engine == "canary" else "zimbadev/crystalserver"
                    url = source.get("url")
                    require(isinstance(url, str) and url.startswith(f"https://github.com/{repo}/blob/"),
                            f"{label}.{engine}: source belongs to another engine repository")
                    if "engine" in source:
                        require(source["engine"] == engine, f"{label}.{engine}: source belongs to another engine")
                    expected_branch = {"crystal_imbuements": "imbuements",
                                       "crystal_summer_update": "summer-update"}.get(engine)
                    if expected_branch and "branch" in source:
                        require(source["branch"] == expected_branch,
                                f"{label}.{engine}: source context belongs to another Crystal branch")

            def check_context(node: object) -> None:
                if isinstance(node, list):
                    for child in node:
                        check_context(child)
                elif isinstance(node, dict):
                    ref = node.get("source_ref")
                    if "source_ref" in node:
                        valid_ref = isinstance(ref, str) and ref in registry and ref in refs
                        require(valid_ref, f"{label}.{engine}: anchor/trace source_ref is not an answer source")
                        if valid_ref and "github_anchor" in node:
                            anchor = node["github_anchor"]
                            base = registry[ref].get("url") if isinstance(registry[ref], dict) else None
                            require(isinstance(anchor, str) and isinstance(base, str)
                                    and bool(re.fullmatch(re.escape(base) + r"#L[1-9][0-9]*(?:-L[1-9][0-9]*)?", anchor)),
                                    f"{label}.{engine}: anchor URL disagrees with pinned answer source")
                    for field in ("line", "line_start", "line_end", "start_line", "end_line"):
                        if field in node:
                            require(type(node[field]) is int and node[field] > 0,
                                    f"{label}.{engine}: invalid anchor/trace {field}")
                    for start, end in (("line_start", "line_end"), ("start_line", "end_line")):
                        if type(node.get(start)) is int and type(node.get(end)) is int:
                            require(node[end] >= node[start], f"{label}.{engine}: reversed anchor/trace range")
                    for field in ("excerpt_sha256", "full_function_sha256"):
                        if field in node:
                            value = node[field]
                            require(isinstance(value, str) and bool(re.fullmatch(r"[0-9a-f]{64}", value)),
                                    f"{label}.{engine}: malformed anchor/trace digest")
                    if "expression" in node and "excerpt_sha256" in node and isinstance(node["expression"], str):
                        require(hashlib.sha256(node["expression"].encode()).hexdigest() == node["excerpt_sha256"],
                                f"{label}.{engine}: expression excerpt digest disagrees")
                    for child in node.values():
                        check_context(child)

            if "code_anchors" in answer:
                anchors = answer["code_anchors"]
                require(isinstance(anchors, list) and bool(anchors)
                        and all(isinstance(x, dict) and bool(x) for x in anchors),
                        f"{label}.{engine}: code_anchors must contain nonempty evidence objects")
            check_context(answer)
        global_answer = question.get("global_evidence")
        if not isinstance(global_answer, dict):
            errors.append(f"{label}: global_evidence must be a separate object")
            global_answer = {}
        status = global_answer.get("status")
        require(status in GLOBAL_STATUSES if isinstance(status, str) else False,
                f"{label}: unknown Global evidence status")
        require((status == "OWNER_SCOPE_RESOLVED") == owner,
                f"{label}: only exact_target_snapshot has owner-resolved Global scope")
        require(nonempty(global_answer.get("answer")), f"{label}: missing public Global answer")
        refs = global_answer.get("source_refs")
        if not isinstance(refs, list):
            errors.append(f"{label}: Global source_refs must be a list of catalogue/ID objects")
            refs = []
        if isinstance(status, str) and status in {
            "SOURCE_QUALIFIED_CURRENT_PUBLIC_REFERENCE", "PARTIALLY_SOURCED_CURRENT_PUBLIC_REFERENCE",
        }:
            require(bool(refs), f"{label}: public-reference status needs public source references")
        for ref in refs:
            if not isinstance(ref, dict) or set(ref) != {"catalogue", "id"}:
                errors.append(f"{label}: malformed Global source reference")
                continue
            catalogue, source_id = ref["catalogue"], ref["id"]
            valid = (isinstance(catalogue, str) and catalogue in CATALOGUES
                     and isinstance(source_id, str) and source_id in public_sources[catalogue])
            require(valid, f"{label}: unknown public source reference {ref!r}")
            if valid:
                source = public_sources[catalogue][source_id]
                role = source.get("role", "") if isinstance(source, dict) else ""
                require(not isinstance(role, str) or not role.startswith("OTS_"),
                        f"{label}: OTS source cannot qualify public Global evidence")
        limitations = question.get("global_limitations")
        require(isinstance(limitations, list) and all(nonempty(x) for x in limitations),
                f"{label}: global_limitations must be a list of nonempty strings")
        if isinstance(status, str) and status in {
            "PARTIALLY_SOURCED_CURRENT_PUBLIC_REFERENCE", "NO_EXPLICIT_PUBLIC_CONFIRMATION",
        }:
            require(bool(limitations), f"{label}: partial/unconfirmed Global answer needs explicit limitations")

    counts = packet.get("counts")
    expected_counts = {"original_questions": 12, "owner_scope_resolved": 1, "engine_answered_questions": 11}
    require(isinstance(counts, dict) and set(counts) == set(expected_counts)
            and all(type(counts.get(k)) is int and counts[k] == v for k, v in expected_counts.items()),
            "counts: expected twelve questions, one owner scope, eleven engine-answered questions")
    require(owner_count == 1 and engine_count == 11, "actual question resolution counts disagree")
    return errors


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate only; never regenerate or write")
    parser.add_argument("--path", type=Path, default=PACKET)
    args = parser.parse_args()
    try:
        packet = json.loads(args.path.read_bytes())
    except (OSError, ValueError) as exc:
        raise SystemExit(f"Cannot read behavior answers: {exc}") from exc
    errors = validate(packet)
    if errors:
        raise SystemExit("\n".join(errors))
    print("Twelve current behavior answers validated; OTS and public Global evidence remain distinct.")


if __name__ == "__main__":
    main()
