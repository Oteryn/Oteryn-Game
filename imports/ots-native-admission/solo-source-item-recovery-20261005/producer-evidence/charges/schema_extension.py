"""Closed two-scalar SourceDefinition alternative; old alternatives stay exact."""

import copy


def scalar_parameter_schema():
    props = {
        "kind": {"const": "CHARGES_AND_LEVEL_DOOR"},
        "charges_default_u32": {"type": "integer", "minimum": 0, "maximum": 4294967295},
        "level_door_u32": {"type": "integer", "minimum": 0, "maximum": 4294967295},
    }
    for field in ("charges_origin", "level_door_origin"):
        props[field] = {"enum": ["OWN_CPP_INITIALIZER", "EXPLICIT_ORDERED_XML"]}
    return {
        "type": "object",
        "additionalProperties": False,
        "properties": props,
        "required": list(props),
        "allOf": [
            {
                "if": {"properties": {origin: {"const": "OWN_CPP_INITIALIZER"}}},
                "then": {"properties": {member: {"const": 0}}},
            }
            for origin, member in [
                ("charges_origin", "charges_default_u32"),
                ("level_door_origin", "level_door_u32"),
            ]
        ],
    }


def extend_definition_schema(existing):
    result = copy.deepcopy(existing)
    value = result["properties"]["source_definition_observations"]["oneOf"][-1][
        "properties"
    ]["value"]
    original = value["items"]
    scalar = copy.deepcopy(original)
    scalar["properties"]["parameter"] = scalar_parameter_schema()
    assignments = scalar["properties"]["ordered_assignments"]
    assignments["minItems"] = (
        0  # Explicit pinned constructor law permits no XML writes.
    )
    assignments["items"]["properties"]["key"] = {
        "type": "string",
        "pattern": "^(?:[cC][hH][aA][rR][gG][eE][sS]|[lL][eE][vV][eE][lL][dD][oO][oO][rR])$",
    }
    # Raw lexical values retain entities/empty/invalid tokens. Exact uint32
    # derivation belongs to the closed Source registry, never this grammar.
    assignments["items"]["properties"]["value_lexeme"] = {"type": "string"}
    value["items"] = {"oneOf": [original, scalar]}
    # Do NOT raise maxItems or assignment bounds without the measured new cohort.
    # This is a structural preview, not an accepted registry/resource amendment.
    return result
