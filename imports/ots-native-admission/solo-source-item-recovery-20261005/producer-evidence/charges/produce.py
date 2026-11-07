"""Prospective full-own-cohort Source scalar replay; no Native data writes."""

import argparse
import gzip
import hashlib
import json
import re
from pathlib import Path

from xml_attribute_law import attribute_value

CUTS = {
    "canary-47df": (
        "CANARY_47DF",
        "canary",
        "47dfd51f45280a59a1d3e50ba7edd573d7234446",
    ),
    "crystal-ff7": (
        "CRYSTAL_FF7",
        "crystalserver",
        "ff7ede593c69d4c658b382c97443e8155926924a",
    ),
    "crystal-00ce": (
        "CRYSTAL_00CE",
        "crystalserver",
        "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
    ),
}
LAW_SHA = "9c8cb3e70dfd9d166a1e1bd93917f730f72124a6934fca52e79c1ffe40daa69b"
KEYS = {"charges": "charges_default_u32", "leveldoor": "level_door_u32"}
ELIGIBILITY_SHA = "bd07a2f89768fa621bd32be4a295ee82cd21a00f0889703869bb066940200e6c"
PREREQUISITES = "imports/ots-native-admission/source-definition-observations/definition-prerequisites.json"
PREREQUISITES_SHA = "a9ee5f308a26fa63c4cc9672ec7bff45208e21a96ee223c2244d1b5f085058df"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def file_sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def pinned(root, relative, expected):
    require(
        not Path(relative).is_absolute() and ".." not in Path(relative).parts,
        "SAFE_INPUT",
    )
    path = root / relative
    require(file_sha(path) == expected, "INPUT_DIGEST:" + relative)
    return path


def own_tuple(binding):
    return tuple(
        binding[k]
        for k in ("source_key", "source_revision", "identity_namespace", "external_id")
    )


def check_eligible(row, binding):
    cut = CUTS[row["source_label"]]
    require(
        row["binding"] == binding and row["target"] == binding["target"],
        "EXACT_OWN_BINDING",
    )
    require(
        binding["source_revision"] == cut[2]
        and binding["identity_namespace"] == "ots/item_server_id"
        and binding["disposition"] == "EXACT"
        and binding["target"]["family"] == "Item",
        "OWN_DOMAIN",
    )
    require(
        row["eligibility"] == "ELIGIBLE_STORED_OWN_PROTO_AND_FULL_XML_BEFORE_LUA"
        and not row["holds"]
        and row["xml_loaded"],
        "LOADED_OWN_PROTOTYPE",
    )
    proto = row["prototype"]
    require(
        proto
        and proto["flags_parent_count"] == 1
        and proto["explicit_id"]
        and proto["prototype_initialized"],
        "STORED_PARENT_ID",
    )
    identifier = binding["external_id"]
    require(
        re.fullmatch(r"[1-9][0-9]*", identifier) is not None, "CANONICAL_EXTERNAL_ID"
    )
    require(
        1 <= int(identifier) <= 65535
        and row["source_id"] == row["final_stored_id"] == int(identifier),
        "STORED_ID",
    )
    applied = [
        r for r in row["xml_records"] if r["disposition"] == "APPLIED_FULL_ORDERED_XML"
    ]
    require(len(applied) == 1, "UNAMBIGUOUS_XML_RECORD")
    return applied[0]


def u32(lexeme):
    # Full own integral from_chars success; all other paths return T{} on
    # conditional normal return. Avoid Python's integer-string digit limit.
    if re.fullmatch(r"[0-9]+", lexeme):
        significant = lexeme.lstrip("0") or "0"
        if len(significant) <= 10:
            value = int(significant)
            if value <= 4294967295:
                return value
    return 0


def replay_record(tree):
    parameter = {
        "kind": "CHARGES_AND_LEVEL_DOOR",
        "charges_default_u32": 0,
        "level_door_u32": 0,
        "charges_origin": "OWN_CPP_INITIALIZER",
        "level_door_origin": "OWN_CPP_INITIALIZER",
    }
    assignments = []
    # Exact qualified Flags lexical tree contract and direct-child ordinals.
    # No own loader restriction to an 'attribute' tag or leaf-only child.
    for ordinal, child in enumerate(tree.get("content", [])):
        if child.get("kind") != "element":
            continue
        attrs = child["node"].get("attributes", [])
        keys = [a for a in attrs if a["name"] == "key"]
        values = [a for a in attrs if a["name"] == "value"]
        if not keys or not values:
            continue
        key = attribute_value(keys[0]["lexical_value"])
        require(key.isascii(), "OWN_KEY_CASEFOLD_BOUNDARY")
        if key.lower() not in KEYS:
            continue
        require(len(keys) == len(values) == 1, "DUPLICATE_XML_ATTRIBUTE")
        require(ordinal <= 65535, "ASSIGNMENT_ORDINAL")
        value = values[0]["lexical_value"]
        parameter[KEYS[key.lower()]] = u32(attribute_value(value))
        parameter[
            "charges_origin" if key.lower() == "charges" else "level_door_origin"
        ] = "EXPLICIT_ORDERED_XML"
        assignments.append(
            {"key": key, "value_lexeme": value, "attribute_ordinal": ordinal}
        )
    return parameter, assignments


def observation(row, record):
    applied = check_eligible(row, row["binding"])
    source = record["source"]
    require(
        source["label"] == row["source_label"]
        and source["revision"] == CUTS[row["source_label"]][2],
        "OWN_XML_CUT",
    )
    witness = record["source_record"]
    require(
        witness["ordinal"] == applied["record_ordinal"]
        and witness["raw_xml_sha256"] == applied["raw_xml_record_sha256"],
        "XML_RECORD_IDENTITY",
    )
    require(
        hashlib.sha256(
            witness["raw_xml"].encode(source["raw_text_encoding"])
        ).hexdigest()
        == witness["raw_xml_sha256"],
        "RAW_XML_HASH",
    )
    parameter, assignments = replay_record(record["tree"])
    return {
        "source_cut": CUTS[row["source_label"]][0],
        "external_item_id": row["source_id"],
        "phase": "FRESH_CPP_PROTOBUF_THEN_FULL_ORDERED_XML_BEFORE_LUA",
        "prototype_message_sha256": row["prototype"]["record_sha256"],
        "xml_record_ordinal": witness["ordinal"],
        "xml_record_sha256": witness["raw_xml_sha256"],
        "source_group": row["final_cpp_group"],
        "current_world_owner_pointers": [
            r["world_identity"] for r in row["current_world_owner_routes"]
        ],
        "ordered_assignments": assignments,
        "parameter": parameter,
    }


CONDITIONAL_LAW_SHA = "04defcdd817dac673770e644e6dadfd7dc1ab2b8309bbb1fca1df38a51b9f39f"


def produce(root, law_path, *, heavy_release=False):
    require(heavy_release, "ROOT_EXCLUSIVE_SOURCE_REPLAY_RELEASE_REQUIRED")
    closure = Path(__file__).with_name("conditional-law.json")
    require(file_sha(closure) == CONDITIONAL_LAW_SHA, "CONDITIONAL_LAW_PIN")
    for entry in json.loads(closure.read_bytes())["files"]:
        require(
            file_sha(entry["path"]) == entry["sha256"], "OWN_CONDITIONAL_PARSER_PIN"
        )
    require(file_sha(law_path) == LAW_SHA, "SEALED_OWN_CPP_SDK_LAW")
    law = json.loads(law_path.read_bytes())
    for cut, files in law["own_files"].items():
        require(cut in CUTS, "LAW_CUT")
        for entry in files:
            require(
                file_sha(entry["path"]) == entry["sha256"], "OWN_LOADER_CONSTRUCTOR_PIN"
            )
    require(
        law["verified_no_proto_member_writes"]
        and law["verified_own_u32_zero_initializers"],
        "OWN_INITIALIZER_LAW",
    )
    prereq = json.loads(pinned(root, PREREQUISITES, PREREQUISITES_SHA).read_bytes())
    eligible_path = pinned(root, prereq["eligibility_replay"]["path"], ELIGIBILITY_SHA)
    with gzip.open(eligible_path, "rt") as stream:
        eligibility = json.load(stream)
    require(
        eligibility["schema"] == "OWN_EXPANDED_STORED_ITEMTYPE_ELIGIBILITY/v1",
        "ELIGIBILITY_SCHEMA",
    )
    live = {}
    for path, expected in prereq["own_bindings_sha256"].items():
        for binding in json.loads(pinned(root, path, expected).read_bytes())[
            "bindings"
        ]:
            key = own_tuple(binding)
            require(key not in live, "AMBIGUOUS_BINDING_TUPLE")
            live[key] = binding
    rows = eligibility["rows"]
    require(len(rows) == 45723, "SEALED_FULL_OWN_COHORT")
    selected, held, targets = {}, [], {}
    for row in rows:
        if row["source_id"] == 901 or row["target"]["key"] == "oteryn:item.tibia.i901":
            continue
        applied = check_eligible(row, live.get(own_tuple(row["binding"])))
        key = (row["source_label"], applied["record_ordinal"])
        selected.setdefault(key, []).append(row)
    require(sum(map(len, selected.values())) == 45721, "PROTECTED_COHORT_EXCLUSION")
    for path, expected in prereq["xml_inputs"].items():
        ledger = pinned(root, path, expected)
        with gzip.open(ledger, "rt") as stream:
            for line in stream:
                record = json.loads(line)
                key = (record["source"]["label"], record["source_record"]["ordinal"])
                for row in selected.pop(key, []):
                    try:
                        entry = observation(row, record)
                    except ValueError as error:
                        held.append({"binding": row["binding"], "reason": str(error)})
                        continue
                    identity = row["target"]
                    target = targets.setdefault(
                        identity["key"], {"target": identity, "observations": []}
                    )
                    require(target["target"] == identity, "TARGET_REVISION_COLLISION")
                    target["observations"].append(entry)
    require(not selected, "MISSING_OWN_XML_RECORD")
    return {
        "schema": "SOURCE_CHARGES_LEVEL_DOOR_PROSPECTIVE_OBSERVATIONS/v1",
        "input_rows": 45721,
        "records": sorted(targets.values(), key=lambda r: r["target"]["key"]),
        "held": held,
        "completion_assumption": "NORMAL_RETURN_OF_CPP_PROTOBUF_AND_FULL_XML_PHASE",
        "actual_CPP_execution": "UNKNOWN_NOT_EXECUTED",
        "runtime_instance_and_door_authority": "UNKNOWN",
        "native_storage": "NOT_APPLIED",
        "runtime_export": "HELD",
        "existing_definition_vectors": "UNMODIFIED; MEASURED_CLOSED_REGISTRY_SUCCESSOR_REQUIRED",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--law", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--root-released-heavy-slot", action="store_true")
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(
            produce(args.root, args.law, heavy_release=args.root_released_heavy_slot),
            sort_keys=True,
            indent=2,
        )
        + "\n"
    )
