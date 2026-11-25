"""Combine source knowledge without replacing an already known mechanical fact."""

import copy
import json
from contextlib import contextmanager
from contextvars import ContextVar

_compact_native_profiles = ContextVar("compact_native_profiles", default=False)


@contextmanager
def compact_native_profile_scope():
    """Use compact intermediate banks while registration owns final byte encoding."""
    token = _compact_native_profiles.set(True)
    try:
        yield
    finally:
        _compact_native_profiles.reset(token)


def encode_native_profile(document):
    """Keep standalone provider bytes; avoid pretty-printing transient full banks."""
    options = (
        {"separators": (",", ":")} if _compact_native_profiles.get() else {"indent": 2}
    )
    # Preserve key insertion order in both modes. The registration owner performs
    # the final sorted canonical encoding before publishing any provider digest.
    return (json.dumps(document, ensure_ascii=False, **options) + "\n").encode()


def compact_item_profile(row):
    """Omit only defaults understood by the native Item decoder."""
    attributes = row.get("attributes")
    semantics = row.get("semantics")
    # First-party constructor admission pins the complete raw row.
    if (
        not isinstance(attributes, dict)
        or not isinstance(semantics, dict)
        or row.get("production_binding", {}).get("source_key")
        == "oteryn:source.oteryn_authored"
    ):
        return row
    optional_attributes = {
        "always_on_top",
        "always_on_top_order",
        "blocks_movement",
        "blocks_projectile",
        "field_condition",
        "field_replaceable",
        "has_height",
        "immovable_block_solid",
        "rune_consumption",
        "speed_bonus",
    }
    optional_groups = {
        "fluid",
        "imbuement",
        "protection",
        "readable_writeable",
        "skill_modifiers",
        "trade_restrictions",
        "use_transform",
        "weapon",
    }
    compact = {
        **row,
        "attributes": {
            key: value
            for key, value in attributes.items()
            if (key not in optional_attributes or value is not None)
            and (key != "spell_augments" or value != [])
        },
        "semantics": {
            key: value
            for key, value in semantics.items()
            if key not in optional_groups or value != {"state": "UNKNOWN"}
        },
    }
    if compact.get("production_binding_qualification") is None:
        compact.pop("production_binding_qualification", None)
    classification = compact["semantics"].get("classification", {})
    if (
        classification.get("state") == "KNOWN"
        and classification["value"].get("capabilities") == {"state": "UNKNOWN"}
    ):
        compact["semantics"]["classification"] = {
            **classification,
            "value": {
                key: value for key, value in classification["value"].items()
                if key != "capabilities"
            },
        }
    return compact


def merge_fact(existing, source, path):
    if isinstance(source, dict) and source.get("state") == "UNKNOWN":
        return copy.deepcopy(existing)
    if isinstance(existing, dict) and existing.get("state") == "UNKNOWN":
        return copy.deepcopy(source)
    if type(existing) is not type(source):
        raise ValueError(f"NATIVE_PROFILE_FACT_CONFLICT:{path}")
    if isinstance(existing, dict):
        if existing.keys() != source.keys():
            raise ValueError(f"NATIVE_PROFILE_FACT_CONFLICT:{path}")
        return {
            key: merge_fact(existing[key], source[key], f"{path}/{key}")
            for key in existing
        }
    if isinstance(existing, list):
        if len(existing) != len(source):
            raise ValueError(f"NATIVE_PROFILE_FACT_CONFLICT:{path}")
        return [
            merge_fact(old, new, f"{path}/{index}")
            for index, (old, new) in enumerate(zip(existing, source))
        ]
    if existing != source:
        raise ValueError(f"NATIVE_PROFILE_FACT_CONFLICT:{path}")
    return copy.deepcopy(existing)
