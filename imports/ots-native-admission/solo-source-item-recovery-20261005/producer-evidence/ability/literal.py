"""Own CPP declaration trace; integer operands never evaluated into final abilities."""

import hashlib
import json

from producer_cached import BASE, HANDLERS, boolean, table, variant


def cast_kind(kind):
    if kind == "CRYSTAL_ELEMENTAL_BOND_ASSIGN_OWN_TYPE":
        return "OWN_ELEMENT_NAME_LITERAL"
    if kind.endswith("_BOOL") or kind == "CONDITIONAL_SUPPRESSION_OWN_ENUM":
        return "OWN_PUGI_AS_BOOL_PREDICATE"
    if kind == "ASSIGN_REGEN_RAW_U32_AND_ENABLE":
        return "OWN_UINT32_CAST_INPUT"
    if kind == "ASSIGN_ELEMENT_U16_AND_OWN_TYPE":
        return "OWN_UINT16_CAST_INPUT"
    if ("I16" in kind and "I32_TO_I16" not in kind) or "MANTRA" in kind:
        return "OWN_INT16_CAST_INPUT"
    return "OWN_INT32_CAST_INPUT"


def emit_literal(cut, assignments):
    descriptors = table(cut)
    recognized = set(HANDLERS[cut]["all_recognized_dispatch_keys"])
    previous, first, events = -1, None, []
    for assignment in assignments:
        if set(assignment) != {
            "attribute_ordinal",
            "key",
            "key_lexeme",
            "value_lexeme",
            "decoded_value",
            "captured_attribute_projection_sha256",
        }:
            raise ValueError("closed captured XML assignment receipt required")
        ordinal, key = assignment["attribute_ordinal"], assignment["key"]
        if type(ordinal) is not int or not previous < ordinal <= 65535:
            raise ValueError("unordered/duplicate/out-of-range captured ordinal")
        previous = ordinal
        if key not in recognized:
            continue
        for handler, descriptor in sorted(
            descriptors.items(), key=lambda pair: pair[1]["order"]
        ):
            matched = key in descriptor["reachable_keys"]
            truthy = handler == "parseSupressDrunk" and boolean(
                assignment["decoded_value"]
            )
            getter = (
                descriptor["unconditional_lazy_allocation_before_key_match"]
                or matched
                or truthy
            )
            if handler == "parseSupressDrunk":
                getter = truthy
            if getter and first is None:
                first = {
                    "attribute_ordinal": ordinal,
                    "handler": handler,
                    "handler_order": descriptor["order"],
                    "key": key,
                    "captured_attribute_projection_sha256": assignment[
                        "captured_attribute_projection_sha256"
                    ],
                    "condition": "IF_PRIOR_HANDLERS_AND_OWN_INPUT_CONVERSION_RETURN",
                }
            if matched or truthy:
                kind = variant(handler, key)
                events.append(
                    {
                        **assignment,
                        "kind": kind,
                        "handler": handler,
                        "handler_order": descriptor["order"],
                        "own_body_sha256": descriptor["own_body_sha256"],
                        "own_input_kind": cast_kind(kind),
                        "branch": "FALSE_NO_WRITE"
                        if handler == "parseSupressDrunk" and not truthy
                        else "DECLARED_CONDITIONAL_SETTER",
                        "suppression_default_NONE": truthy and not matched,
                        "numeric_result": {"state": "UNKNOWN"},
                    }
                )
        if HANDLERS[cut]["aliases"].get(key) == "DISPATCH_RECOGNIZED_HANDLER_NOOP":
            events.append(
                {
                    **assignment,
                    "kind": "RECOGNIZED_ALIAS_NOOP_ALLOCATION_EVENT",
                    "handler": "DISPATCH_ALIAS_NO_SETTER",
                    "handler_order": 65535,
                    "own_body_sha256": None,
                    "own_input_kind": "NO_SETTER",
                    "branch": "DISPATCH_ALIAS_NO_SETTER",
                    "suppression_default_NONE": False,
                    "numeric_result": {"state": "UNKNOWN"},
                }
            )
    return {
        "kind": "ABILITY_LITERAL_CONFIGURATION",
        "source_cut": cut,
        "constructor_catalog_sha256": hashlib.sha256(
            (BASE / (cut + "-default-catalog.json")).read_bytes()
        ).hexdigest(),
        "declared_first_getter": first,
        "ordered_assignments": assignments,
        "ordered_events": events,
        "actual_stored_allocation": {"state": "UNKNOWN"},
        "final_sparse_members": {"state": "UNKNOWN"},
        "runtime_getter_rates": {"state": "UNKNOWN"},
        "Lua_instance_effects": {"state": "UNKNOWN"},
        "actual_CPP_execution": {"state": "UNKNOWN"},
    }


def capture_assignments(xml_record, attribute_value):
    output = []
    for ordinal, child in enumerate(xml_record["tree"].get("content", [])):
        if child.get("kind") != "element":
            continue
        attrs = child["node"].get("attributes", [])
        keys = [value for value in attrs if value["name"] == "key"]
        values = [value for value in attrs if value["name"] == "value"]
        if not keys or not values:
            continue
        if len(keys) != 1 or len(values) != 1:
            raise ValueError("ambiguous own XML attribute")
        decoded_key = attribute_value(keys[0]["lexical_value"])
        if not decoded_key.isascii():
            raise ValueError("own nonASCII dispatch/lowercase not qualified")
        projection = json.dumps(
            child, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode()
        output.append(
            {
                "attribute_ordinal": ordinal,
                "key": decoded_key.lower(),
                "key_lexeme": keys[0]["lexical_value"],
                "value_lexeme": values[0]["lexical_value"],
                "decoded_value": attribute_value(values[0]["lexical_value"]),
                "captured_attribute_projection_sha256": hashlib.sha256(
                    projection
                ).hexdigest(),
            }
        )
    return output
