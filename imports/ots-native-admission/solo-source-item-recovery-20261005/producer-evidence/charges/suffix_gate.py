"""Prospective stdlib-only new-kind inverse, ahead of all historical Source gates."""

import copy
import hashlib
import json

FIELD = "source_definition_observations"
KIND = "CHARGES_AND_LEVEL_DOOR"
UNKNOWN = {"state": "UNKNOWN"}


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        ).encode()
    ).hexdigest()


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def extend_parent(record, entries):
    require(
        record["kind"] == "Item"
        and record["identity"]["key"] != "oteryn:item.tibia.i901",
        "OWN_ITEM_PROTECTED_CORE",
    )
    require(
        entries and len({e["source_cut"] for e in entries}) == len(entries),
        "OWN_CUT_UNIQUE",
    )
    require(all(e["parameter"]["kind"] == KIND for e in entries), "EXACT_NEW_KIND")
    semantics = record.get("semantics", {})
    before = semantics.get(FIELD, UNKNOWN)
    require(before["state"] in ("UNKNOWN", "KNOWN"), "PRESERVE_NA_CONFLICT")
    vector = before.get("value", [])
    require(
        not any(e["parameter"]["kind"] == KIND for e in vector),
        "NEW_KIND_ALREADY_KNOWN",
    )
    after = copy.deepcopy(record)
    after.setdefault("semantics", {})[FIELD] = {
        "state": "KNOWN",
        "value": vector + copy.deepcopy(entries),
    }
    witness = {
        "before_sha256": digest(record),
        "after_sha256": digest(after),
        "semantics_present": "semantics" in record,
        "field_present": FIELD in semantics,
        "old_field": copy.deepcopy(before),
        "entries": copy.deepcopy(entries),
    }
    require(project_parent(after, witness) == record, "EXACT_FULL_PARENT_INVERSE")
    return after, witness


def project_parent(record, witness):
    require(digest(record) == witness["after_sha256"], "AFTER_FULL_PARENT_FENCE")
    expected = {
        "state": "KNOWN",
        "value": witness["old_field"].get("value", []) + witness["entries"],
    }
    require(record["semantics"][FIELD] == expected, "EXACT_OLD_VECTOR_NEW_SUFFIX")
    result = copy.deepcopy(record)
    if witness["field_present"]:
        result["semantics"][FIELD] = copy.deepcopy(witness["old_field"])
    else:
        del result["semantics"][FIELD]
    if not witness["semantics_present"]:
        del result["semantics"]
    require(digest(result) == witness["before_sha256"], "BEFORE_FULL_PARENT_FENCE")
    return result


def project_before(records, gate, role):
    require(role in ("canonical", "reference"), "EXACT_VIEW")
    require(
        gate["schema"] == "OWN_CHARGES_LEVEL_DOOR_COMPLETE_SUFFIX_GATE/v1",
        "GATE_SCHEMA",
    )
    require(len(gate["targets"]) == 33975, "WHOLE_TARGET_GATE")
    output, seen = [], set()
    for record in records:
        identity = record["identity"]
        target = gate["targets"].get(identity["key"])
        if target:
            require(
                identity == target["identity"] and identity["key"] not in seen,
                "OWN_FULL_IDENTITY",
            )
            require(
                target["views"][role]["entries"] == target["entries"],
                "BOTH_VIEW_OWN_VALUES",
            )
            output.append(project_parent(record, target["views"][role]))
            seen.add(identity["key"])
        else:
            field = record.get("semantics", {}).get(FIELD, UNKNOWN)
            require(
                not any(e["parameter"]["kind"] == KIND for e in field.get("value", [])),
                "UNDECLARED_NEW_KIND",
            )
            output.append(record)
    require(seen == set(gate["targets"]), "MISSING_WHOLE_COHORT")
    return output
