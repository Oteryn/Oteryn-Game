"""Closed Source-only literal trace shape; final evaluation cannot be promoted."""

import copy

from producer_cached import VARIANTS


def schema():
    unknown = {
        "type": "object",
        "properties": {"state": {"const": "UNKNOWN"}},
        "required": ["state"],
        "additionalProperties": False,
    }
    sha = {"type": "string", "pattern": "^[0-9a-f]{64}$"}
    assignment = {
        "type": "object",
        "properties": {
            "attribute_ordinal": {"type": "integer", "minimum": 0, "maximum": 65535},
            "key": {"type": "string"},
            "key_lexeme": {"type": "string"},
            "value_lexeme": {"type": "string"},
            "decoded_value": {"type": "string"},
            "captured_attribute_projection_sha256": sha,
        },
        "required": [
            "attribute_ordinal",
            "key",
            "key_lexeme",
            "value_lexeme",
            "decoded_value",
            "captured_attribute_projection_sha256",
        ],
        "additionalProperties": False,
    }
    event = copy.deepcopy(assignment)
    event["properties"].update(
        {
            "kind": {"enum": VARIANTS},
            "handler": {"type": "string"},
            "handler_order": {"type": "integer", "minimum": 0, "maximum": 65535},
            "own_body_sha256": {"oneOf": [sha, {"type": "null"}]},
            "own_input_kind": {
                "enum": [
                    "OWN_ELEMENT_NAME_LITERAL",
                    "OWN_PUGI_AS_BOOL_PREDICATE",
                    "OWN_UINT32_CAST_INPUT",
                    "OWN_UINT16_CAST_INPUT",
                    "OWN_INT16_CAST_INPUT",
                    "OWN_INT32_CAST_INPUT",
                    "NO_SETTER",
                ]
            },
            "branch": {
                "enum": [
                    "FALSE_NO_WRITE",
                    "DECLARED_CONDITIONAL_SETTER",
                    "DISPATCH_ALIAS_NO_SETTER",
                ]
            },
            "suppression_default_NONE": {"type": "boolean"},
            "numeric_result": unknown,
        }
    )
    event["required"] = list(event["properties"])
    getter = {
        "type": "object",
        "properties": {
            "attribute_ordinal": {"type": "integer", "minimum": 0, "maximum": 65535},
            "handler": {"type": "string"},
            "handler_order": {"type": "integer", "minimum": 0, "maximum": 65535},
            "key": {"type": "string"},
            "captured_attribute_projection_sha256": sha,
            "condition": {"const": "IF_PRIOR_HANDLERS_AND_OWN_INPUT_CONVERSION_RETURN"},
        },
        "additionalProperties": False,
    }
    getter["required"] = list(getter["properties"])
    props = {
        "kind": {"const": "ABILITY_LITERAL_CONFIGURATION"},
        "source_cut": {"enum": ["canary-47df", "crystal-ff7", "crystal-00ce"]},
        "constructor_catalog_sha256": sha,
        "declared_first_getter": {"oneOf": [getter, {"type": "null"}]},
        "ordered_assignments": {"type": "array", "items": assignment},
        "ordered_events": {"type": "array", "items": event},
    }
    for name in [
        "actual_stored_allocation",
        "final_sparse_members",
        "runtime_getter_rates",
        "Lua_instance_effects",
        "actual_CPP_execution",
    ]:
        props[name] = copy.deepcopy(unknown)
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "urn:oteryn:source-ability-literal-configuration:proposal:v1",
        "type": "object",
        "properties": props,
        "required": list(props),
        "additionalProperties": False,
        "$comment": "PROPOSAL ONLY: exact registry/currenttarget/source provenance, actual event/string maxima and bounded combinedbudget required before Nativeadmission. Integer parsing/arithmetic/runtime unexecutedUNKNOWN; declaration traces never finalarrays. V4/V5/V6allnonUnknownfutureguard mandatory; noV7allocation.",
    }
