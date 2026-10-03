"""Deterministic offline bridge for missing NPC dialogue classes; never runs NPC Lua.

Generated Oteryn replies and typed source callback ASTs are PROPOSED_NONCANONICAL.
No native Dialogue, Quest, track, Transition or runtime behavior is admitted here.
"""

from __future__ import annotations
import argparse
import contextlib
import copy
import enum
import hashlib
import io
import json
import pathlib

SCHEMA = "npc-missing-class-profile-bridge/v1"
TEMPLATES = {
    "greet": "Hello, |PLAYERNAME|. I am {npc_name}.",
    "farewell": "Goodbye, |PLAYERNAME|.",
    "trade": "I can help you trade.",
    "travel": "I can help you travel.",
    "service": "I can help you with my services.",
}


class BridgeError(ValueError):
    pass


def sha(b):
    return hashlib.sha256(b).hexdigest()


def proof_digest(value):
    return sha(
        (
            json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
            + "\n"
        ).encode()
    )


def canonical(obj):
    return (
        json.dumps(obj, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    ).encode()


def verify(base, capture):
    raw = (base / capture["capture"]).read_bytes()
    if (
        len(raw) != capture["bytes"]
        or sha(raw) != capture["sha256"]
        or hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        != capture["git_blob_sha1"]
    ):
        raise BridgeError("source object content mismatch")
    return raw


def parse_source(raw):
    try:
        from importlib.metadata import version
        from luaparser import ast
        from luaparser.astnodes import Node

        if version("luaparser") != "3.3.0":
            raise BridgeError("source AST bridge requires luaparser==3.3.0")
    except ImportError as exc:
        raise BridgeError(
            "Install requirements-dialogue-bridge.txt for callback AST parsing"
        ) from exc
    text = raw.decode("utf-8")
    offset = [0]
    for c in text:
        offset.append(offset[-1] + len(c.encode()))
    with (
        contextlib.redirect_stdout(io.StringIO()),
        contextlib.redirect_stderr(io.StringIO()),
    ):
        tree = ast.parse(text)

    def span(node):
        a, b = node.first_token, node.last_token
        if a is None or b is None:
            return None
        start, end = offset[a.start], offset[b.stop + 1]
        return {
            "byte_start": start,
            "byte_end_exclusive": end,
            "line_start": a.line,
            "line_end": b.line,
            "sha256": sha(raw[start:end]),
        }

    def ir(value):
        if isinstance(value, Node):
            fields = {
                k: ir(v)
                for k, v in vars(value).items()
                if not k.startswith("_") and k not in ("comments", "wrapped")
            }
            return {
                "source_node_kind": type(value).__name__,
                "source_span": span(value),
                "fields": fields,
            }
        if isinstance(value, list):
            return [ir(v) for v in value]
        if isinstance(value, enum.Enum):
            return value.name
        if isinstance(value, bytes):
            return value.decode("utf-8")
        if value is None or isinstance(value, (str, int, float, bool)):
            return value
        raise BridgeError("unrecognized AST field type " + type(value).__name__)

    defs = {}
    for statement in tree.body.body:
        if (
            type(statement).__name__ in ("LocalFunction", "Function")
            and type(getattr(statement, "name", None)).__name__ == "Name"
        ):
            defs[statement.name.id] = statement
    callbacks = []
    for statement in tree.body.body:
        if (
            type(statement).__name__ != "Invoke"
            or getattr(statement.func, "id", None) != "setCallback"
            or len(statement.args) != 2
        ):
            continue
        slot, ref = statement.args
        callback_id = getattr(slot, "id", None)
        name = getattr(ref, "id", None)
        if not callback_id:
            raise BridgeError("computed callback registration ID cannot be flattened")
        entry = {
            "callback_slot": callback_id,
            "registration_ir": ir(statement),
            "function_symbol": name,
            "status": "UNRESOLVED_SOURCE_SYMBOL",
            "callback_ir": None,
            "speech_observations": [],
            "guard_count": 0,
            "effect_or_opaque_call_count": 0,
            "safe_const_callback_candidate": False,
            "native_guards": None,
            "runtime_eligible": False,
        }
        fn = defs.get(name)
        if fn is not None:
            entry.update({"status": "SOURCE_CALLBACK_IR_ONLY", "callback_ir": ir(fn)})
            references = {n.id for n in ast.walk(fn) if type(n).__name__ == "Name"}
            local_bindings = [
                n
                for n in tree.body.body
                if type(n).__name__ == "LocalAssign"
                and any(getattr(t, "id", None) in references for t in n.targets)
            ]
            local_names = {
                getattr(t, "id", None) for n in local_bindings for t in n.targets
            }
            effects = [
                n
                for n in tree.body.body
                if type(n).__name__ == "Invoke"
                and getattr(n.source, "id", None) in local_names
                and getattr(n.func, "id", None) != "setCallback"
            ]
            entry["referenced_module_bindings_ir"] = [ir(n) for n in local_bindings]
            entry["module_initialization_calls_ir"] = [ir(n) for n in effects]
            entry["external_symbol_resolution"] = (
                "Source lexical bindings and pinned raw/library context only; no native Quest or engine ownership inferred."
            )
            for node in ast.walk(fn):
                kind = type(node).__name__
                if kind in ("If", "ElseIf", "While", "Repeat", "Fornum", "Forin"):
                    entry["guard_count"] += 1
                if kind in ("Invoke", "Call"):
                    is_speech = (
                        kind == "Invoke"
                        and getattr(node.source, "id", None) == "npcHandler"
                        and getattr(node.func, "id", None) == "say"
                    )
                    if is_speech:
                        first = node.args[0] if node.args else None
                        text = None
                        if type(first).__name__ == "String":
                            text = (
                                first.s.decode()
                                if isinstance(first.s, bytes)
                                else first.s
                            )
                        elif type(first).__name__ == "Table" and all(
                            type(f.value).__name__ == "String" for f in first.fields
                        ):
                            text = [
                                f.value.s.decode()
                                if isinstance(f.value.s, bytes)
                                else f.value.s
                                for f in first.fields
                            ]
                        entry["speech_observations"].append(
                            {
                                "source_span": span(node),
                                "static_parts": text,
                                "dynamic_text": text is None,
                                "origin": "SOURCE_CALLBACK_REFERENCE_ONLY",
                            }
                        )
                    else:
                        entry["effect_or_opaque_call_count"] += 1

            # Strict test: all calls are say, no control-flow predicates, all speech constant.
            def const_statement(statement):
                if type(statement).__name__ == "Return":
                    return all(
                        type(v).__name__
                        in ("TrueExpr", "FalseExpr", "True", "False", "Nil")
                        for v in statement.values
                    )
                if (
                    type(statement).__name__ != "Invoke"
                    or getattr(statement.source, "id", None) != "npcHandler"
                    or getattr(statement.func, "id", None) != "say"
                ):
                    return False
                if len(statement.args) != 3 or any(
                    type(v).__name__ != "Name" for v in statement.args[1:]
                ):
                    return False
                if [v.id for v in statement.args[1:]] != ["npc", "creature"]:
                    return False
                first = statement.args[0]
                return type(first).__name__ == "String" or (
                    type(first).__name__ == "Table"
                    and all(type(f.value).__name__ == "String" for f in first.fields)
                )

            entry["safe_const_callback_candidate"] = (
                bool(entry["speech_observations"])
                and not entry["guard_count"]
                and not entry["effect_or_opaque_call_count"]
                and all(const_statement(s) for s in fn.body.body)
            )
        callbacks.append(entry)
    return callbacks


def generate_profile(npc, program, services):
    name = program["source_program"]["definition"]["display_name"]
    service_lines = []
    for ref in npc["services"]:
        if ref["family"] != "Service" or ref["key"] not in services:
            raise BridgeError("unknown native service reference")
        service = services[ref["key"]]
        if service["identity"]["revision"] != ref["revision"]:
            raise BridgeError("native service revision mismatch")
        kind = (
            "trade"
            if service.get("offers")
            else "travel"
            if service.get("routes")
            else "service"
        )
        service_lines.append(
            {
                "service_ref": copy.deepcopy(ref),
                "reply": [TEMPLATES[kind]],
                "reference_only": True,
                "executes_service": False,
            }
        )
    return {
        "origin": "OTERYN_GENERATED_TEMPLATE_PROPOSED",
        "npc_name": name,
        "greet": [TEMPLATES["greet"].replace("{npc_name}", name)],
        "farewell": [TEMPLATES["farewell"]],
        "service_lines": service_lines,
        "global_transcript_claim": False,
        "runtime_eligible": False,
    }


def bridge(summary, qualification, programs, base, native):
    byhash = {p["source_program_sha256"]: p for p in programs["records"]}
    byname = {q["name"]: q for q in qualification["records"]}
    bykey = {d["identity"]["key"]: d for d in native["records"]}
    services = {k: v for k, v in bykey.items() if v["kind"] == "Service"}
    rows = []
    classes = [
        "SOURCE_DEFAULT_ONLY_NO_EXPLICIT_STATIC_DIALOGUE",
        "SCRIPTED_CALLBACK_ONLY",
        "SOURCE_AUTHORED_TEXT_HELD_BY_D16",
    ]
    for group in classes:
        for name in summary["existing_176_dispositions"][group]["names"]:
            q = byname[name]
            npc = bykey.get(q["npc_key"])
            if npc is not None and (
                npc["kind"] != "NPC" or npc["dialogue"] is not None
            ):
                raise BridgeError(
                    "bridge requires a source candidate or existing NPC without admitted Dialogue"
                )
            bound = [byhash[s["source_program_sha256"]] for s in q["accepted_sources"]]
            if not bound:
                raise BridgeError("accepted source program missing")
            row = {
                "npc_key": q["npc_key"] if npc is not None else None,
                "source_candidate_native_key_not_allocated": q["npc_key"]
                if npc is None
                else None,
                "native_target_admitted": npc is not None,
                "name": name,
                "source_class": group,
                "status": "PROPOSED_NONCANONICAL",
                "npc_before_sha256": sha(canonical(npc)) if npc is not None else None,
                "admitted_native_dialogue": False,
                "runtime_eligible": False,
                "generated_profile": None,
                "source_programs": [],
            }
            for p in bound:
                if (
                    proof_digest(p["source_program"]) != p["source_program_sha256"]
                    or proof_digest(p["static_candidate"])
                    != p["static_candidate_sha256"]
                ):
                    raise BridgeError(
                        "structured source program or static candidate digest mismatch"
                    )
                if name not in p["source_program"]["source_registered_npc_names"]:
                    raise BridgeError(
                        "source registered name does not bind this class record"
                    )
                raw = verify(base, p["source_capture"])
                for library in p["supporting_libraries"]:
                    library_path = (
                        base / "raw" / p["source_capture"]["source"] / library["path"]
                    )
                    library_bytes = library_path.read_bytes()
                    blob = hashlib.sha1(
                        b"blob "
                        + str(len(library_bytes)).encode()
                        + b"\0"
                        + library_bytes
                    ).hexdigest()
                    if (
                        sha(library_bytes) != library["sha256"]
                        or blob != library["git_blob_sha1"]
                    ):
                        raise BridgeError("supporting source library object mismatch")
                entry = {
                    "source_capture": p["source_capture"],
                    "source_program_sha256": p["source_program_sha256"],
                    "static_candidate_sha256": p["static_candidate_sha256"],
                    "source_registered_names": p["source_program"][
                        "source_registered_npc_names"
                    ],
                    "supporting_library_bindings": [
                        {
                            **library,
                            "url": "https://raw.githubusercontent.com/"
                            + p["source_capture"]["repository"]
                            + "/"
                            + p["source_capture"]["revision"]
                            + "/"
                            + library["path"],
                        }
                        for library in p["supporting_libraries"]
                    ],
                }
                if group == "SCRIPTED_CALLBACK_ONLY":
                    entry["typed_callback_ir"] = parse_source(raw)
                else:
                    entry["typed_callback_ir"] = []
                row["source_programs"].append(entry)
            if group != "SCRIPTED_CALLBACK_ONLY":
                displays = {
                    p["source_program"]["definition"]["display_name"] for p in bound
                }
                if len(displays) != 1:
                    row["generated_profile_hold"] = "SOURCE_DISPLAY_NAME_CONFLICT"
                else:
                    row["generated_profile"] = generate_profile(
                        npc if npc is not None else {"services": []}, bound[0], services
                    )
            if group == "SOURCE_AUTHORED_TEXT_HELD_BY_D16":
                row["source_authored_text_kept_excluded"] = True
            rows.append(row)
    callbacks = [
        c for r in rows for p in r["source_programs"] for c in p["typed_callback_ir"]
    ]
    return {
        "schema": SCHEMA,
        "status": "PROPOSED_NONCANONICAL",
        "scope": "Missing-class authoring bridge only; existing full source compiler is not replaced. Lua syntax is parsed without running any NPC function. Source AST preserves source ordering/branches/expressions/actions verbatim by typed tags and byte/hash witnesses; it is not an executable native Quest or Dialogue program.",
        "templates": TEMPLATES,
        "records": rows,
        "counts": {
            "source_class_records": len(rows),
            "existing_npcs": sum(r["native_target_admitted"] for r in rows),
            "unadmitted_npc_candidates": sum(
                not r["native_target_admitted"] for r in rows
            ),
            "generated_profiles": sum(r["generated_profile"] is not None for r in rows),
            "callback_registrations": len(callbacks),
            "resolved_source_callbacks": sum(
                c["callback_ir"] is not None for c in callbacks
            ),
            "unresolved_source_symbols": sum(
                c["callback_ir"] is None for c in callbacks
            ),
            "safe_const_callback_candidates": sum(
                c["safe_const_callback_candidate"] for c in callbacks
            ),
            "native_dialogues_admitted": 0,
            "native_quest_ids_allocated": 0,
        },
        "runtime_eligible": False,
        "global_complete": False,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-base", type=pathlib.Path, required=True)
    parser.add_argument("--native-declarations", type=pathlib.Path, required=True)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    args = parser.parse_args()
    paths = {
        "summary": args.source_base / "dialogue-completion-summary-r7.json",
        "qualification": args.source_base / "program-qualification-r7.json",
        "programs": args.source_base / "source-programs-r7.json",
        "native": args.native_declarations,
    }
    data = {k: json.loads(p.read_bytes()) for k, p in paths.items()}
    out = bridge(
        data["summary"],
        data["qualification"],
        data["programs"],
        args.source_base,
        data["native"],
    )
    out["inputs"] = {
        k: {"path": str(p), "sha256": sha(p.read_bytes())} for k, p in paths.items()
    }
    args.out.write_bytes(canonical(out))
    print(json.dumps(out["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
