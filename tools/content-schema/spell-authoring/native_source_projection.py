"""Project source-native models onto an unchanged, closed accepted profile.

This is an authoring gate, not a new native consumer. Source-only display metadata
and external Item identities are mapped explicitly. Every other controller value
must match the accepted profile; header overrides retain their existing manifest
field provenance and do not claim equality with the donor header.
"""
import copy
from functools import lru_cache
import hashlib
import importlib.util
import json
from pathlib import Path
import re

_spec = importlib.util.spec_from_file_location(
    "_native_current_spell_adapter", Path(__file__).with_name("current_spell_adapter.py"))
_adapter = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_adapter)
DONORS, canonical_bytes, identity = _adapter.DONORS, _adapter.canonical_bytes, _adapter.identity

_FAMILIAR_SELECTIONS = {
    "Summon Druid Familiar": ("druid", 197, "utevo gran res dru"),
    "Summon Knight Familiar": ("knight", 194, "utevo gran res eq"),
    "Summon Paladin Familiar": ("paladin", 195, "utevo gran res sac"),
    "Summon Sorcerer Familiar": ("sorcerer", 196, "utevo gran res ven"),
}


@lru_cache(maxsize=1)
def _profiles():
    return json.loads(Path(__file__).with_name("samples").joinpath(
        "native-spell-profiles.json").read_text())["profiles"]


def _qualified(entry):
    spell = entry["bundle"]["spell"]
    profiles = [row for row in _profiles() if (row["name"], row["carrier"]) ==
                (spell["name"], spell["carrier"])]
    if len(profiles) != 1 or canonical_bytes(profiles[0]["spell"]) != canonical_bytes(spell) or \
            canonical_bytes(profiles[0]["dependencies"]) != canonical_bytes(entry["dependencies"]):
        raise ValueError("CURRENT_NATIVE_PROFILE_UNQUALIFIED")


def _familiar_alias_receipt(candidate, current_entry, source_selection):
    """Bind the four Canary names through the existing explicit S21 selection."""
    source_identity = identity(candidate["identity"])
    if not re.fullmatch(r"candidate:spell/source/canary-main-current/[^/]+", source_identity["key"]) or \
            candidate.get("name") not in _FAMILIAR_SELECTIONS:
        raise ValueError("NATIVE_FAMILIAR_ALIAS_UNSUPPORTED")
    vocation, number, words = _FAMILIAR_SELECTIONS[candidate["name"]]
    current = current_entry["bundle"]["spell"]
    selected = {"key": "candidate:spell/" + vocation + "_familiar", "revision": "spell-p2-r20"}
    alternative = {"key": "candidate:spell/summon_" + vocation + "_familiar", "revision": "spell-p2-r20"}
    if current["identity"] != selected or current["name"] != vocation.capitalize() + " familiar":
        raise ValueError("NATIVE_FAMILIAR_SELECTED_TARGET_MISMATCH")
    for field, expected in (("carrier", "instant"), ("reference_spell_id", number), ("words", words)):
        if type(candidate.get(field)) is not type(expected) or candidate[field] != expected or \
                current.get(field) != expected:
            raise ValueError("NATIVE_FAMILIAR_ALIAS_HEADER_MISMATCH:" + field)
    execution, requirements = candidate.get("execution"), candidate.get("requirements")
    behavior = execution.get("native_behavior") if isinstance(execution, dict) else None
    params = behavior.get("parameters") if isinstance(behavior, dict) else None
    if not isinstance(requirements, dict) or not isinstance(params, dict) or \
            requirements.get("vocations") != current["requirements"]["vocations"] or \
            behavior.get("key") != "familiar_summon" or params.get("vocation") != vocation:
        raise ValueError("NATIVE_FAMILIAR_ALIAS_VOCATION_MISMATCH")
    if not isinstance(source_selection, dict) or source_selection.get("schema") != "OTERYN_SPELL_SOURCE_SELECTION/v1" or \
            not isinstance(source_selection.get("selections"), list):
        raise ValueError("NATIVE_FAMILIAR_SELECTION_SHAPE")
    matches = [(index, row) for index, row in enumerate(source_selection["selections"])
               if isinstance(row, dict) and row.get("words") == words]
    if len(matches) != 1:
        raise ValueError("NATIVE_FAMILIAR_SELECTION_AMBIGUOUS")
    index, row = matches[0]
    if row.get("policy") != "S21" or row.get("selected") != selected or row.get("alternatives") != [alternative]:
        raise ValueError("NATIVE_FAMILIAR_SELECTION_POLICY_MISMATCH")
    proof_fields = ("/words", "/reference_spell_id", "/requirements/vocations")
    field_proofs = {path: [index for index, _ in _field_rows(current_entry, path)] for path in proof_fields}
    if any(not indices for indices in field_proofs.values()):
        raise ValueError("NATIVE_FAMILIAR_ALIAS_FIELD_UNPROVEN")
    return {"policy": "S21", "selection_index": index,
            "selection_sha256": hashlib.sha256(canonical_bytes(source_selection)).hexdigest(),
            "source_identity": copy.deepcopy(source_identity), "source_name": candidate["name"],
            "accepted_identity": copy.deepcopy(selected), "accepted_name": current["name"],
            "alternative_identity": alternative, "same_carrier_reference_spell_id_words": True,
            "manifest_field_indices": field_proofs}


def select_familiar_candidate(bundle, current_entries, source_selection):
    """Resolve a reviewed Canary alias without choosing a target by fuzzy names."""
    if not isinstance(bundle, dict) or set(bundle) != {"spell"} or not isinstance(bundle["spell"], dict):
        raise ValueError("NATIVE_BUNDLE_SHAPE")
    candidate = bundle["spell"]
    rule = _FAMILIAR_SELECTIONS.get(candidate.get("name"))
    if rule is None:
        raise ValueError("NATIVE_FAMILIAR_ALIAS_UNSUPPORTED")
    key = "candidate:spell/" + rule[0] + "_familiar"
    entries = [entry for entry in current_entries if entry["bundle"]["spell"]["identity"]["key"] == key]
    if len(entries) != 1:
        raise ValueError("NATIVE_FAMILIAR_TARGET_AMBIGUOUS")
    _qualified(entries[0])
    return entries[0], _familiar_alias_receipt(candidate, entries[0], source_selection)


def _item_map(source_catalog, target_catalog, source_namespace):
    if set(source_catalog) != {"definitions"} or set(target_catalog) != {"definitions"}:
        raise ValueError("NATIVE_REFERENCE_CATALOG_SHAPE")
    targets, mapping, source_numbers = {}, {}, set()
    for reference in target_catalog["definitions"]:
        if not isinstance(reference, dict) or any(not isinstance(reference.get(field), str) or
                not reference[field] for field in ("family", "key", "revision")):
            raise ValueError("NATIVE_REFERENCE_TARGET_UNSUPPORTED")
        match = re.fullmatch(r"candidate:item/([0-9]+)", reference.get("key", ""))
        if set(reference) != {"family", "key", "revision"} or reference["family"] != "Item" or not match:
            raise ValueError("NATIVE_REFERENCE_TARGET_UNSUPPORTED")
        number = match[1]
        if number in targets:
            raise ValueError("NATIVE_REFERENCE_AMBIGUOUS")
        targets[number] = reference
    for reference in source_catalog["definitions"]:
        if not isinstance(reference, dict) or any(not isinstance(reference.get(field), str) or
                not reference[field] for field in ("family", "key", "revision")):
            raise ValueError("NATIVE_REFERENCE_SOURCE_UNSUPPORTED")
        match = re.fullmatch(r"candidate:item/source/([^/]+)/([0-9]+)", reference.get("key", ""))
        if set(reference) != {"family", "key", "revision"} or reference["family"] != "Item" or not match or \
                match[1] != source_namespace:
            raise ValueError("NATIVE_REFERENCE_SOURCE_UNSUPPORTED")
        number = match[2]
        source = canonical_bytes(reference)
        if number not in targets or source in mapping or number in source_numbers:
            raise ValueError("NATIVE_REFERENCE_UNBOUND_OR_DUPLICATE")
        source_numbers.add(number)
        mapping[source] = copy.deepcopy(targets[number])
    if len(mapping) != len(targets):
        raise ValueError("NATIVE_REFERENCE_CATALOG_INCOMPLETE")
    return mapping


def source_reference_catalog(bundle, source_catalog, current_entry):
    """Declare existing Item IDs evidenced by explicit source controller fields.

    This never normalizes the controller itself. Its full equality gate still
    rejects private parameters, world-context behavior and changed source values.
    """
    candidate = bundle["spell"]
    source_identity = identity(candidate["identity"])
    match = re.fullmatch(r"candidate:spell/source/([^/]+)/[^/]+", source_identity["key"])
    if not match or match[1] not in DONORS:
        raise ValueError("NATIVE_SOURCE_IDENTITY_NAMESPACE")
    if not isinstance(source_catalog, dict) or set(source_catalog) != {"definitions"} or \
            not isinstance(source_catalog["definitions"], list):
        raise ValueError("NATIVE_REFERENCE_CATALOG_SHAPE")
    # Validate every declared reference before extending an incomplete catalog.
    for reference in source_catalog["definitions"]:
        if not isinstance(reference, dict) or set(reference) != {"family", "key", "revision"} or \
                reference["family"] != "Item" or not isinstance(reference["key"], str) or \
                not isinstance(reference["revision"], str) or not reference["revision"] or \
                not re.fullmatch(r"candidate:item/source/" + re.escape(match[1]) + r"/[0-9]+", reference["key"]):
            raise ValueError("NATIVE_REFERENCE_SOURCE_UNSUPPORTED")
    execution = candidate.get("execution")
    behavior = execution.get("native_behavior") if isinstance(execution, dict) else None
    params = behavior.get("parameters") if isinstance(behavior, dict) else None
    descriptors = []
    if isinstance(params, dict) and params.get("source_model") in ("r62/magic_wall", "r62/wild_growth"):
        expected = ("ITEM_MAGICWALL", "ITEM_MAGICWALL_SAFE") if params["source_model"] == "r62/magic_wall" else \
                   ("ITEM_WILDGROWTH", "ITEM_WILDGROWTH_SAFE")
        variants = params.get("item_variants")
        if not isinstance(variants, dict) or set(variants) != set(expected):
            raise ValueError("NATIVE_NUMERIC_ITEM_DESCRIPTOR_INVALID")
        descriptors = [("/execution/native_behavior/parameters/item_variants/" + name, variants[name])
                       for name in expected]
    elif isinstance(params, dict) and params.get("source_model") == "r62/desintegrate_rune":
        removal = params.get("removal")
        values = removal.get("excluded_item_ids") if isinstance(removal, dict) else None
        if not isinstance(values, list) or any(type(value) is not int for value in values) or len(set(values)) != len(values):
            raise ValueError("NATIVE_NUMERIC_ITEM_DESCRIPTOR_INVALID")
        descriptors = [("/execution/native_behavior/parameters/removal/excluded_item_ids/" + str(index), value)
                       for index, value in enumerate(values)]
    targets = {row["key"]: row for row in current_entry["catalog"]["definitions"]}
    completed, receipts = copy.deepcopy(source_catalog), []
    declared = {row["key"] for row in completed["definitions"]}
    for path, number in descriptors:
        if type(number) is not int or number <= 0:
            raise ValueError("NATIVE_NUMERIC_ITEM_DESCRIPTOR_INVALID")
        accepted = targets.get("candidate:item/" + str(number))
        if accepted is None:
            raise ValueError("NATIVE_REFERENCE_UNBOUND_OR_DUPLICATE")
        reference = {"family": "Item", "key": "candidate:item/source/" + match[1] + "/" + str(number),
                     "revision": source_identity["revision"]}
        if reference["key"] not in declared:
            completed["definitions"].append(reference)
            declared.add(reference["key"])
            receipts.append({"field": path, "kind": "source_numeric_item_descriptor", "source_value": number,
                             "source_reference": reference, "accepted_reference": copy.deepcopy(accepted)})
    _item_map(completed, current_entry["catalog"], match[1])
    return completed, receipts


def _rewrite_items(value, mapping):
    if isinstance(value, list):
        return [_rewrite_items(child, mapping) for child in value]
    if not isinstance(value, dict):
        return value
    if set(value) == {"family", "key", "revision"} and value["family"] == "Item":
        if canonical_bytes(value) not in mapping:
            raise ValueError("NATIVE_REFERENCE_NOT_DECLARED")
        return copy.deepcopy(mapping[canonical_bytes(value)])
    return {key: _rewrite_items(child, mapping) for key, child in value.items()}


def _field_rows(entry, pointer):
    return [(index, row) for index, row in enumerate(entry["manifest"]["entries"])
            if row.get("destination") == "/spell/spell" + pointer and row.get("status") == "mapped"]


def _controller_conflicts(source, target, pointer="/execution"):
    """Same typed numerical/boolean controller fields are actual data conflicts."""
    if isinstance(source, dict) and isinstance(target, dict):
        return [path for key in sorted(source.keys() & target.keys())
                if not key.startswith("source_")
                for path in _controller_conflicts(source[key], target[key], pointer + "/" + key)]
    if isinstance(source, list) and isinstance(target, list) and len(source) == len(target):
        return [path for index, (before, after) in enumerate(zip(source, target))
                for path in _controller_conflicts(before, after, pointer + "/" + str(index))]
    if type(source) is type(target) and type(source) in (int, float, bool) and source != target:
        return [pointer]
    return []


def _header_projection(source, target, entry, pointer="", receipts=None):
    receipts = [] if receipts is None else receipts
    if canonical_bytes(source) == canonical_bytes(target):
        return receipts
    if isinstance(source, dict) and isinstance(target, dict):
        extras = source.keys() - target.keys()
        if extras:
            raise ValueError("NATIVE_HEADER_FIELD_UNSUPPORTED:" + pointer + "/" + sorted(extras)[0])
        for key in sorted(target):
            path = pointer + "/" + key
            if key in source:
                _header_projection(source[key], target[key], entry, path, receipts)
            else:
                if isinstance(target[key], dict) and target[key]:
                    _header_projection({}, target[key], entry, path, receipts)
                    continue
                rows = _field_rows(entry, path)
                if not rows:
                    raise ValueError("NATIVE_MISSING_HEADER_UNPROVEN:" + path)
                receipts.append({"field": path, "kind": "accepted_missing_field",
                                 "accepted": copy.deepcopy(target[key]),
                                 "manifest_indices": [index for index, _ in rows]})
        return receipts
    if isinstance(source, list) and isinstance(target, list) and len(source) == len(target) and not _field_rows(entry, pointer):
        for index, (before, after) in enumerate(zip(source, target)):
            _header_projection(before, after, entry, pointer + "/" + str(index), receipts)
        return receipts
    if type(source) is not type(target):
        raise ValueError("NATIVE_HEADER_TYPE_MISMATCH:" + pointer)
    rows = [(index, row) for index, row in _field_rows(entry, pointer)
            if row.get("resolution", "").startswith(("S3:", "S8:", "S15:"))]
    if pointer == "/requirements/level" and type(target) is int and target == 0 and type(source) is int and source > 0 and \
            entry["bundle"]["spell"]["requirements"].get("wheel_unlock") is True:
        rows += [(index, row) for index, row in _field_rows(entry, pointer) if row.get("resolution") ==
                 "S22: a Wheel of Destiny revelation spell has level 0, as the client spell list shows and tibia.com states no level; the Wheel unlock gates it."]
    if pointer == "/requirements/learning_required" and source is True and target is False:
        rows += [(index, row) for index, row in enumerate(entry["manifest"]["entries"])
                 if row.get("status") == "approved_omission" and row.get("source_field") == "needLearn" and row.get("resolution") ==
                 "S16: since patch 15.22 (27 January 2026) spells unlock automatically and free at their level; trainers no longer teach spells (https://tibiopedia.pl/updates/15.22.c93366). learning_required is false; the source needLearn only marks Wheel spells (wheel_unlock)."]
    if not rows:
        raise ValueError("SOURCE_CONFLICT_KEPT:" + pointer)
    receipts.append({"field": pointer, "kind": "accepted_source_policy_override" if any(row["resolution"].startswith(("S16:", "S22:"))
                     for _, row in rows) else "accepted_official_wiki_override",
                     "source": copy.deepcopy(source), "accepted": copy.deepcopy(target),
                     "manifest_indices": [index for index, _ in rows]})
    return receipts


def project_native_candidate(bundle, dependencies, current_entry, explicit_identity, source_catalog,
                             source_selection=None):
    """Return the accepted graph plus a receipt, without changing native profiles.

    A successful receipt qualifies only this normalized model. Source-file hashes,
    helper proofs and engine defaults are the importing caller's responsibility.
    Wheel unlock requirements remain visible even if the normalized controller matches.
    """
    _qualified(current_entry)
    target = identity(explicit_identity)
    current = current_entry["bundle"]["spell"]
    if target != current["identity"] or not current_entry["source_identities"] or any(row["identity"] != target
                                           for row in current_entry["source_identities"]):
        raise ValueError("NATIVE_EXPLICIT_TARGET_MISMATCH")
    if set(bundle) != {"spell"}:
        raise ValueError("NATIVE_BUNDLE_SHAPE")
    candidate = copy.deepcopy(bundle["spell"])
    identity(candidate["identity"])
    source_match = re.fullmatch(r"candidate:spell/source/([^/]+)/[^/]+", candidate["identity"]["key"])
    if not source_match or source_match[1] not in DONORS:
        raise ValueError("NATIVE_SOURCE_IDENTITY_NAMESPACE")
    alias = None
    if candidate.get("name") != current.get("name") and source_selection is not None:
        alias = _familiar_alias_receipt(candidate, current_entry, source_selection)
        candidate["name"] = current["name"]
    if any(candidate.get(field) != current.get(field) for field in ("name", "carrier")):
        raise ValueError("NATIVE_SOURCE_NAME_CARRIER_MISMATCH")
    source_catalog, numeric_references = source_reference_catalog(bundle, source_catalog, current_entry)
    mapping = _item_map(source_catalog, current_entry["catalog"], source_match[1])
    candidate = _rewrite_items(candidate, mapping)
    deps = _rewrite_items(dependencies, mapping)
    if canonical_bytes(candidate["execution"]) != canonical_bytes(current["execution"]):
        conflicts = _controller_conflicts(candidate["execution"], current["execution"])
        if conflicts:
            raise ValueError("SOURCE_CONFLICT_KEPT:" + ",".join(conflicts))
        raise ValueError("NATIVE_CONTROLLER_ADAPTER_MISSING")
    if canonical_bytes(deps) != canonical_bytes(current_entry["dependencies"]):
        raise ValueError("NATIVE_DEPENDENCIES_ADAPTER_MISSING")
    metadata = []
    requirements = candidate.get("requirements", {})
    if not isinstance(requirements, dict):
        raise ValueError("NATIVE_HEADER_TYPE_MISMATCH:/requirements")
    if "vocation_display_flags" in requirements:
        flags = requirements.pop("vocation_display_flags")
        if not isinstance(flags, list) or any(not isinstance(row, dict) or
                set(row) != {"vocation", "show_in_description"} or
                type(row["show_in_description"]) is not bool or
                not isinstance(row["vocation"], str) or
                row["vocation"] not in candidate["requirements"]["vocations"] for row in flags) or \
                len({row["vocation"] for row in flags}) != len(flags):
            raise ValueError("NATIVE_DISPLAY_METADATA_INVALID")
        metadata.append({"field": "/requirements/vocation_display_flags", "kind": "authoring_display_metadata",
                         "value": copy.deepcopy(flags)})
    if "reference_rune_item_id" in candidate:
        rune_id = candidate.pop("reference_rune_item_id")
        rune = candidate.get("rune", {})
        item = rune.get("item", {}) if isinstance(rune, dict) else None
        if type(rune_id) is not int or not isinstance(item, dict) or item.get("key") != \
                "candidate:item/" + str(rune_id):
            raise ValueError("NATIVE_RUNE_ITEM_PROOF_MISMATCH")
        metadata.append({"field": "/reference_rune_item_id", "kind": "item_reference_binding", "value": rune_id})
    candidate["identity"] = copy.deepcopy(target)
    source_header = {key: value for key, value in candidate.items() if key != "execution"}
    target_header = {key: value for key, value in current.items() if key != "execution"}
    fields = _header_projection(source_header, target_header, current_entry)
    return (copy.deepcopy(current_entry["bundle"]), copy.deepcopy(current_entry["dependencies"]),
            {"scope": "accepted_native_profile", "normalized_controller_equality": True,
             "raw_source_parity": False, "runtime_activation": False,
             "wheel_unlock_required": current.get("requirements", {}).get("wheel_unlock") is True,
             "reference_bindings": [{"source": json.loads(source), "accepted": target}
                                    for source, target in sorted(mapping.items())],
             "field_provenance": fields, "metadata": metadata,
             **({"numeric_reference_provenance": numeric_references} if numeric_references else {}),
             **({"source_selection": alias} if alias is not None else {})})


def _remove_declared_extensions(parameters, extensions):
    """Remove only explicit, exact source extensions into retained proof fields."""
    projected, retained = copy.deepcopy(parameters), []
    for path, expected, kind in extensions:
        parts = path.split("/")
        parent = projected
        for part in parts[:-1]:
            if not isinstance(parent, dict) or part not in parent:
                raise ValueError("NATIVE_EXTENSION_FIELD_MISSING:" + path)
            parent = parent[part]
        if not isinstance(parent, dict) or parts[-1] not in parent or \
                canonical_bytes(parent[parts[-1]]) != canonical_bytes(expected):
            raise ValueError("NATIVE_EXTENSION_VALUE_UNSUPPORTED:" + path)
        value = parent.pop(parts[-1])
        retained.append({"field": "/execution/native_behavior/parameters/" + path,
                         "value": value, "kind": kind})
    return projected, retained


def _familiar_controller(source, accepted):
    reference_id = accepted["reference_spell_id"]
    order = ["premium_guard", "summon_count_and_account_guard", "vocation_lookup",
             "compute_half_config_duration_seconds", "compute_vip_cooldown", "create_owned_monster",
             "apply_player_familiar_look", "register_familiar_death", "increase_speed_nonnegative",
             "caster_magic_blue", "creature_teleport", "save_unix_expiry", "schedule_expiry", "schedule_two_warnings"]
    if source.get("register_party_protection") is True:
        order.append("register_party_protection_all_owned_summons")
    order += ["apply_shared_spell_cooldown", "return_true"]
    extensions = [
        ("source_model", "r61-familiar_summon", "source_authoring_metadata"),
        ("source_contract_version", "current-pinned-r61", "source_authoring_metadata"),
        ("condition_sharing", {"automatic_clone_all_owner_conditions": False,
                               "future_haste_sharing": "companion_haste_source_policy",
                               "owner_speed_applied_at_creation": True, "spell_cooldown_subid": reference_id}, "unprojected_source_owner_binding"),
        ("current_helper_commit_order", order, "unprojected_source_owner_binding"),
        ("dynamic_inputs", {"cooldown_rate": "config.RATE_SPELL_COOLDOWN", "duration": "config.FAMILIAR_TIME",
                            "look": "player.familiar_look_type", "unix_expiry": "player.kv.familiar-summon-time",
                            "vip_reduction": "config.VIP_FAMILIAR_TIME_COOLDOWN_REDUCTION",
                            "vocation": "player.vocation_base_id", "warning_handles": "player.FAMILIAR_TIMER_storage"}, "unprojected_source_owner_binding"),
        ("selection", {"base_vocation_lookup": "FAMILIAR_ID", "creature_name": accepted["creature_name"],
                       "fallback_look_type_on_login": accepted["default_look_type"], "look_type_from_player": True,
                       "unknown_vocation_returns_false": True}, "unprojected_source_owner_binding"),
        ("expiry/absent_player_or_creature_returns_true", True, "unprojected_source_owner_binding"),
        ("expiry/warning_storage_reset", -1, "unprojected_source_owner_binding"),
        ("login/register_advance_event_before_selection", True, "unprojected_source_owner_binding"),
        ("login/remove_look_guard", "(not_premium_and_has_look)_or_level_below_200", "unprojected_source_owner_binding"),
        ("warning_dispatch/message_class", "loot", "unprojected_source_owner_binding")]
    return _remove_declared_extensions(source, extensions)


def _world_item_policy(entry):
    rows = []
    for index, row in enumerate(entry["manifest"]["entries"]):
        prefix = "Native source qualification: "
        if row.get("status") == "metadata_only" and row.get("resolution", "").startswith(prefix):
            proof = json.loads(row["resolution"][len(prefix):])
            if proof.get("family_module") == "native_world_items":
                rows.append((index, proof))
    if len(rows) != 1:
        raise ValueError("NATIVE_WORLD_ITEM_ACCEPTED_POLICY_UNPROVEN")
    return rows[0]


def _exact_fields(value, expected, reason):
    if not isinstance(value, dict) or set(value) != set(expected):
        raise ValueError(reason)


def _world_source_metadata(source, model):
    if source["source_model"] != model or source["source_costs_unchanged"] is not True or \
            source["formula_math_semantics"] != "Lua_binary64_with_ordered_left_fold_for_nary_arithmetic" or \
            source["source_monk_spell_type"] is not None or not isinstance(source["source_sound_bindings"], dict) or \
            not isinstance(source["source_function_scopes"], list) or not source["source_function_scopes"] or \
            not isinstance(source["helper_definitions"], list):
        raise ValueError("NATIVE_WORLD_ITEM_SOURCE_METADATA_UNSUPPORTED")
    _exact_fields(source["source_sound_bindings"], {"castSound", "impactSound"}, "NATIVE_WORLD_ITEM_SOURCE_SOUND_FIELDS_UNSUPPORTED")
    if any(not isinstance(value, str) or not value for value in source["source_sound_bindings"].values()) or \
            any(not isinstance(helper, dict) or not isinstance(helper.get("kind"), str) for helper in source["helper_definitions"]):
        raise ValueError("NATIVE_WORLD_ITEM_SOURCE_METADATA_UNSUPPORTED")
    for function in source["source_function_scopes"]:
        _exact_fields(function, {"body_sha256", "line_start", "symbol", "typed_controller_binding"},
                      "NATIVE_WORLD_ITEM_FUNCTION_PROOF_SHAPE")
        if not isinstance(function["body_sha256"], str) or not re.fullmatch(r"[a-f0-9]{64}", function["body_sha256"]) or \
                type(function["line_start"]) is not int or function["line_start"] <= 0 or \
                function["typed_controller_binding"] != model.split("/")[1] or not isinstance(function["symbol"], str):
            raise ValueError("NATIVE_WORLD_ITEM_FUNCTION_PROOF_INVALID")


def _barrier_controller(source, current_entry):
    current = current_entry["bundle"]["spell"]
    model = {"Magic Wall Rune": "magic_wall", "Wild Growth Rune": "wild_growth"}.get(current["name"])
    if current["carrier"] != "rune" or model is None:
        raise ValueError("NATIVE_BARRIER_SOURCE_TARGET_MISMATCH")
    _exact_fields(source, {"callback_success", "combat_bindings", "combat_definitions", "create_count", "creation_failed",
        "creation_position", "description_template", "duration_seconds", "expert_pvp_context_from_caster", "formula_math_semantics",
        "helper_definitions", "insert_error", "insert_flag", "item_duration", "item_variants", "safe_item_when", "source_cast_return",
        "source_costs_unchanged", "source_function_scopes", "source_model", "source_monk_spell_type", "source_sound_bindings", "tile_guards"},
        "NATIVE_BARRIER_SOURCE_FIELDS_UNSUPPORTED")
    _world_source_metadata(source, "r62/" + model)
    if source["callback_success"] != "implicit_nil" or canonical_bytes(source["combat_bindings"]) != canonical_bytes({"primary": 0}) or \
            type(source["create_count"]) is not int or source["create_count"] != 1 or source["creation_failed"] != "callback_nil" or \
            source["description_template"] != "Casted by: %s" or source["source_cast_return"] != "combat_result" or \
            type(source["expert_pvp_context_from_caster"]) is not bool or \
            canonical_bytes(source["item_duration"]) != canonical_bytes({"decay_to_default": 0, "mutation_scope": "shared_ItemType", "show_duration": True, "start_decaying": True}) or \
            source["tile_guards"] != {"floor_change": "false", "missing": "false", "top_non_player_creature": "false"}:
        raise ValueError("NATIVE_BARRIER_SOURCE_OPERATION_UNSUPPORTED")
    expert = source["expert_pvp_context_from_caster"]
    expected_route = ("none_then_addItemEx", "callback_false", "FLAG_NOLIMIT", ["IsExpertPVP", "WORLD_TYPE_NO_PVP"]) if expert else \
                     ("target_position", "Game_createItem_failure_nil", None, ["WORLDTYPE_OPTIONAL"])
    if tuple(source[field] for field in ("creation_position", "insert_error", "insert_flag", "safe_item_when")) != expected_route:
        raise ValueError("NATIVE_BARRIER_SOURCE_WORLD_ROUTE_UNSUPPORTED")
    if not isinstance(source["combat_definitions"], list) or len(source["combat_definitions"]) != 1:
        raise ValueError("NATIVE_BARRIER_SOURCE_COMBAT_COUNT_UNSUPPORTED")
    combat = source["combat_definitions"][0]
    expected_combat = {"aggressive": True, "area": None, "block_armor": False,
        "callback_bindings": [{"function": "onCreateMagicWall" if model == "magic_wall" else "onCreateWildGrowth", "kind": "CALLBACK_PARAM_TARGETTILE"}],
        "chain_effect": "CONST_ME_NONE", "condition_declarations": [], "dispel_condition": None,
        "health_type": "COMBAT_NONE", "impact_effect": "CONST_ME_NONE", "projectile_effect": "CONST_ANI_ENERGY",
        "source_combat_index": 0, "source_parameter_bindings": {"COMBAT_PARAM_DISTANCEEFFECT": "CONST_ANI_ENERGY"}, "use_charges": False}
    if canonical_bytes(combat) != canonical_bytes(expected_combat):
        raise ValueError("NATIVE_BARRIER_SOURCE_COMBAT_UNSUPPORTED")
    symbol = "ITEM_MAGICWALL" if model == "magic_wall" else "ITEM_WILDGROWTH"
    _exact_fields(source["item_variants"], {symbol, symbol + "_SAFE"}, "NATIVE_BARRIER_ITEM_VARIANTS_UNSUPPORTED")
    effect = current_entry["dependencies"]["effects"][0]
    policy_index, policy = _world_item_policy(current_entry)
    decision = "Use per-instance item duration and caster description; preserve normal and optional-PvP item variants. Common blocking refuses creature occupied tiles."
    if not any(row.get("decision") == decision and row.get("policy") == "S27 D.6.1" for row in policy["observations"]):
        raise ValueError("NATIVE_BARRIER_ACCEPTED_PLACEMENT_POLICY_UNPROVEN")
    duration = source["duration_seconds"]
    expected_duration = {"distribution": "uniform_random_inclusive", "minimum": 16, "maximum": 24} if model == "magic_wall" else \
                        {"distribution": "constant", "minimum": 30, "maximum": 30}
    if canonical_bytes(duration) != canonical_bytes(expected_duration):
        raise ValueError("SOURCE_CONFLICT_KEPT:/execution/native_behavior/parameters/duration_seconds")
    maximum = duration["maximum"] * 1000
    if model == "wild_growth":
        duration_decision = "Wiki duration 30000..60000ms overrides source fixed30000ms; uniform integer-second choice follows the pinned item:setDuration(min,max) engine implementation."
        if not any(row.get("decision") == duration_decision and row.get("policy") == "S27 D.6.1 / F1056364" for row in policy["observations"]):
            raise ValueError("NATIVE_BARRIER_ACCEPTED_DURATION_POLICY_UNPROVEN")
        maximum = 60000
    for name, reference in ((symbol, effect["created_item"]), (symbol + "_SAFE", effect["pvp_safe_item"])):
        number = source["item_variants"][name]
        if type(number) is not int or reference["key"] != "candidate:item/" + str(number):
            raise ValueError("SOURCE_CONFLICT_KEPT:/execution/native_behavior/parameters/item_variants/" + name)
    projected_effect = {"identity": copy.deepcopy(effect["identity"]), "operation": "create_item",
        "created_item": copy.deepcopy(effect["created_item"]), "pvp_safe_item": copy.deepcopy(effect["pvp_safe_item"]),
        "description_template": source["description_template"].replace("%s", "{caster_name}"),
        "duration_range_ms": {"minimum": duration["minimum"] * 1000, "maximum": maximum},
        "duration_selection": "uniform_integer_seconds", "presentation": {"projectile_asset_binding": "canary.appearance:missile/energy"},
        "refuse_on": ["floor_change_tile", "creature_on_tile"], "safe_world_type": "optional_pvp"}
    if canonical_bytes(projected_effect) != canonical_bytes(effect):
        raise ValueError("NATIVE_BARRIER_ACCEPTED_EFFECT_MISMATCH")
    return projected_effect, policy_index


def _disintegrate_controller(source, current_entry):
    _exact_fields(source, {"combat_bindings", "combat_definitions", "final_cancel", "final_effect", "formula_math_semantics",
        "helper_definitions", "items_missing", "iteration", "removal", "source_cast_return", "source_costs_unchanged",
        "source_function_scopes", "source_model", "source_monk_spell_type", "source_sound_bindings", "tile_missing", "visited_item_limit"},
        "NATIVE_DISINTEGRATE_SOURCE_FIELDS_UNSUPPORTED")
    _world_source_metadata(source, "r62/desintegrate_rune")
    if source["combat_bindings"] != {} or source["combat_definitions"] != [] or source["helper_definitions"] != [] or \
            source["final_cancel"] != "RETURNVALUE_NOTPOSSIBLE" or source["final_effect"] != "CONST_ME_POFF" or \
            source["items_missing"] != "skip_to_final_presentation" or source["tile_missing"] != "skip_to_final_presentation" or \
            source["iteration"] != "ipairs" or source["source_cast_return"] != "literal_true_after_final_presentation" or \
            type(source["visited_item_limit"]) is not int or source["visited_item_limit"] <= 0:
        raise ValueError("NATIVE_DISINTEGRATE_SOURCE_OPERATION_UNSUPPORTED")
    removal = source["removal"]
    _exact_fields(removal, {"action_id_required", "excluded_item_ids", "movable_required", "unique_id_strictly_greater_than"},
                  "NATIVE_DISINTEGRATE_SOURCE_REMOVAL_FIELDS_UNSUPPORTED")
    if type(removal["action_id_required"]) is not int or removal["action_id_required"] != 0 or \
            removal["movable_required"] is not True or type(removal["unique_id_strictly_greater_than"]) is not int or \
            removal["unique_id_strictly_greater_than"] != 65535:
        raise ValueError("NATIVE_DISINTEGRATE_SOURCE_REMOVAL_RULE_UNSUPPORTED")
    accepted = current_entry["bundle"]["spell"]["execution"]["native_behavior"]
    policy_index, policy = _world_item_policy(current_entry)
    if not any(canonical_bytes(row.get("extracted_parameters")) == canonical_bytes(accepted) for row in policy["observations"]):
        raise ValueError("NATIVE_DISINTEGRATE_ACCEPTED_POLICY_UNPROVEN")
    references = accepted["parameters"]["exclude_items"]
    if not isinstance(removal["excluded_item_ids"], list) or \
            canonical_bytes(removal["excluded_item_ids"]) != canonical_bytes([int(row["key"].rsplit("/", 1)[1]) for row in references]):
        raise ValueError("SOURCE_CONFLICT_KEPT:/execution/native_behavior/parameters/removal/excluded_item_ids")
    projected = {"aggressive": False, "allow_in_pz": True, "empty_tile_succeeds": True, "exclude_action_tagged": True,
        "exclude_items": copy.deepcopy(references), "exclude_script_tagged": True,
        "max_items": source["visited_item_limit"], "max_items_counts": "visited", "missing_tile_succeeds": True,
        "operation": "disintegrate", "require_movable": removal["movable_required"], "send_cancel_on_success": False,
        "source": ["tile_items"], "success_effect_asset_binding": "canary.appearance:effect/poff"}
    return projected, policy_index


def _s27_normalized_contract(contract, spell_name):
    expected = {"authoring_contract_extension_pending": True, "chain_initial_selector": None,
        "native_execution_qualified": False, "normalization": "S27_ACCEPTED_CANONICAL_NATIVE_PARAMETERS",
        "raw_source_equivalence": False, "runtime_activation": False, "separate_augment_parameters": {},
        "version": 1, "wheel_augments_owner": "ProjectV2AugmentBinding"}
    _exact_fields(contract, expected, "NATIVE_S27_SOURCE_CONTRACT_FIELDS_UNSUPPORTED")
    fixed = {**contract, "separate_augment_parameters": {}}
    if canonical_bytes(fixed) != canonical_bytes(expected):
        raise ValueError("NATIVE_S27_SOURCE_CONTRACT_SEMANTICS_UNSUPPORTED")
    augments = contract["separate_augment_parameters"]
    if not isinstance(augments, dict):
        raise ValueError("NATIVE_S27_SOURCE_AUGMENT_TYPE_UNSUPPORTED")
    if not augments:
        return
    _exact_fields(augments, {"damage_or_heal_bonus_percent", "enhanced_area", "enhanced_area_from_stage", "wheel"},
                  "NATIVE_S27_SOURCE_AUGMENT_FIELDS_UNSUPPORTED")
    bonuses = augments["damage_or_heal_bonus_percent"]
    _exact_fields(bonuses, {"0", "1", "2"}, "NATIVE_S27_SOURCE_AUGMENT_STAGE_UNSUPPORTED")
    if any(type(value) is not int or value < 0 for value in bonuses.values()) or \
            type(augments["enhanced_area_from_stage"]) is not int or augments["enhanced_area_from_stage"] not in {1, 2}:
        raise ValueError("NATIVE_S27_SOURCE_AUGMENT_VALUE_UNSUPPORTED")
    area = augments["enhanced_area"]
    _exact_fields(area, {"clear_sight", "diagonal", "directional", "encoding", "orthogonal", "same_floor"},
                  "NATIVE_S27_SOURCE_AUGMENT_AREA_FIELDS_UNSUPPORTED")
    if type(area["directional"]) is not bool or area["clear_sight"] is not True or area["same_floor"] is not True or \
            area["encoding"] != "0_inactive_1_hit_2_origin_3_origin_hit":
        raise ValueError("NATIVE_S27_SOURCE_AUGMENT_AREA_SEMANTICS_UNSUPPORTED")
    for role in ("orthogonal", "diagonal"):
        matrix = area[role]
        if matrix is None and role == "diagonal":
            continue
        if not isinstance(matrix, list) or not matrix or any(not isinstance(row, list) or not row for row in matrix) or \
                len({len(row) for row in matrix}) != 1 or any(type(cell) is not int or cell not in {0, 1, 2, 3} for row in matrix for cell in row) or \
                sum(cell in {2, 3} for row in matrix for cell in row) != 1:
            raise ValueError("NATIVE_S27_SOURCE_AUGMENT_AREA_MATRIX_UNSUPPORTED")
    wheel = {"owner": "wheel_of_destiny", "perk": spell_name.casefold(), "snapshot": "cast_start", "stage_kind": "augment",
             "stage_max": 2, "stage_min": 0, "stage_zero": "base_cast"}
    if canonical_bytes(augments["wheel"]) != canonical_bytes(wheel):
        raise ValueError("NATIVE_S27_SOURCE_AUGMENT_WHEEL_BINDING_UNSUPPORTED")


def project_native_controller_candidate(bundle, dependencies, current_entry, explicit_identity, source_catalog,
                                        source_selection=None):
    """Qualify specific existing controller bases while retaining honest gaps.

    The output still passes the full unchanged native-profile gate. This separate
    API never admits an entire private controller by dropping unknown fields.
    Its receipt preserves every original value and the declared partial scope.
    """
    _qualified(current_entry)
    current = current_entry["bundle"]["spell"]
    if not isinstance(bundle, dict) or set(bundle) != {"spell"} or not isinstance(bundle["spell"], dict):
        raise ValueError("NATIVE_BUNDLE_SHAPE")
    candidate = bundle["spell"]
    _exact_fields(candidate.get("execution"), {"native_behavior"}, "NATIVE_CONTROLLER_SOURCE_EXECUTION_UNSUPPORTED")
    source_native = candidate["execution"]["native_behavior"]
    target_native = current.get("execution", {}).get("native_behavior")
    if not isinstance(source_native, dict) or set(source_native) != {"key", "parameters"} or \
            not isinstance(source_native["parameters"], dict):
        raise ValueError("NATIVE_CONTROLLER_PROJECTION_FAMILY_UNSUPPORTED")
    source = source_native["parameters"]
    header_conflicts = []
    for field in ("reference_spell_id", "words"):
        if type(candidate.get(field)) is not type(current.get(field)) or candidate.get(field) != current.get(field):
            known_pair = {("Ice Burst", "reference_spell_id"): (262, 263),
                          ("Terra Burst", "reference_spell_id"): (263, 262),
                          ("Sharpshooter", "words"): ("utito tempo san", "utori con")}.get((current["name"], field))
            if source.get("source_model") != "r59_accepted_" + source_native["key"] or known_pair is None or \
                    canonical_bytes([candidate.get(field), current.get(field)]) != canonical_bytes(list(known_pair)):
                raise ValueError("NATIVE_CONTROLLER_SOURCE_HEADER_MISMATCH:" + field)
            proofs = _header_projection(candidate[field], current[field], current_entry, "/" + field)
            header_conflicts.append({"field": "/" + field, "source": copy.deepcopy(candidate[field]),
                                     "accepted": copy.deepcopy(current[field]), "field_provenance": proofs})
    completed_catalog, numeric_references = source_reference_catalog(bundle, source_catalog, current_entry)
    namespace = candidate["identity"]["key"].split("/")[2]
    item_mapping = _item_map(completed_catalog, current_entry["catalog"], namespace)
    reverse_items = {canonical_bytes(target): json.loads(original) for original, target in item_mapping.items()}
    barrier = source_native["key"] == "tile_item_operation" and source.get("source_model") in {"r62/magic_wall", "r62/wild_growth"}
    if not barrier and (not isinstance(target_native, dict) or source_native["key"] != target_native["key"]):
        raise ValueError("NATIVE_CONTROLLER_PROJECTION_FAMILY_UNSUPPORTED")
    accepted = target_native["parameters"] if isinstance(target_native, dict) else None
    projected, projected_dependencies = copy.deepcopy(bundle), copy.deepcopy(dependencies)
    controller_provenance, extra_scope = [], []
    if source_native["key"] == "familiar_summon" and source.get("source_model") == "r61-familiar_summon":
        parameters, retained = _familiar_controller(source, accepted)
        reason, owner = "source_familiar_lifecycle_and_dynamic_owner_bindings_remain_unprojected", "familiar-summon-owner"
    elif source_native["key"] == "acquire_summon" and source.get("source_model") == "r61-acquire_summon":
        parameters, retained = _remove_declared_extensions(source, [
            ("source_model", "r61-acquire_summon", "source_authoring_metadata"),
            ("source_contract_version", "acquire-target-copy-r61", "source_authoring_metadata"),
            ("inherit_master_attack_target", {"clear_when_master_has_no_target": True, "enabled": True,
                                             "order": "after_set_master", "overwrite_existing": True}, "unprojected_source_owner_binding")])
        reason, owner = "source_acquire_summon_master_attack_target_copy_remains_unprojected", "acquire-summon-owner"
    elif source_native["key"] in {"delayed_strike", "wheel_combat", "mass_spirit_mend", "avatar_state", "owned_field_buff", "mana_shield_capacity", "stance_toggle"} and \
            source.get("source_model") == "r59_accepted_" + source_native["key"]:
        _s27_normalized_contract(candidate.get("source_state_contract"), current["name"])
        parameters, retained = _remove_declared_extensions(source, [
            ("source_model", "r59_accepted_" + source_native["key"], "source_authoring_metadata")])
        contract = projected["spell"].pop("source_state_contract")
        retained.append({"field": "/source_state_contract", "value": contract,
                         "kind": "unprojected_source_private_authoring_contract"})
        reason, owner = "accepted_S27_normalized_base_profile_imported_private_authoring_and_augment_contract_remain_unqualified", "native-" + source_native["key"] + "-owner"
    elif barrier:
        if dependencies != {"abilities": [], "effects": [], "formulas": []}:
            raise ValueError("NATIVE_BARRIER_SOURCE_DEPENDENCIES_UNSUPPORTED")
        effect, policy_index = _barrier_controller(source, current_entry)
        projected["spell"]["execution"] = copy.deepcopy(current["execution"])
        projected_dependencies = copy.deepcopy(current_entry["dependencies"])
        projected_dependencies["effects"] = [effect]
        projected_dependencies = _rewrite_items(projected_dependencies, reverse_items)
        retained = [{"field": "/execution/native_behavior/parameters/" + field,
                     "value": copy.deepcopy(source[field]), "kind": "unprojected_source_owner_binding"}
                    for field in ("helper_definitions", "item_duration", "safe_item_when", "expert_pvp_context_from_caster",
                                  "creation_position", "insert_error", "insert_flag", "callback_success", "creation_failed")]
        reason, owner = "source_barrier_callback_shared_itemtype_duration_and_expert_world_context_remain_unprojected", "barrier-item-owner"
        controller_provenance = [{"accepted_effect": copy.deepcopy(effect["identity"]), "manifest_index": policy_index,
            "source_to_accepted_fields": {"item_variants": ["created_item", "pvp_safe_item"], "duration_seconds": ["duration_range_ms"],
                "description_template": ["description_template"], "combat_definitions/0/projectile_effect": ["presentation/projectile_asset_binding"]}}]
        extra_scope = [{"missing_class": "SOURCE_CONFLICT_KEPT", "reason": "accepted_S27_barrier_placement_and_duration_policy_retained",
                        "owner": owner, "source_fields": ["/execution/native_behavior/parameters/tile_guards",
                                                             "/execution/native_behavior/parameters/duration_seconds"],
                        "manifest_index": policy_index}]
    elif source_native["key"] == "tile_item_operation" and source.get("source_model") == "r62/desintegrate_rune":
        parameters, policy_index = _disintegrate_controller(source, current_entry)
        retained = [{"field": "/execution/native_behavior/parameters/final_cancel", "value": source["final_cancel"],
                     "kind": "unprojected_source_policy_conflict"}]
        reason, owner = "accepted_S27_disintegrate_success_cancel_and_aggression_policy_retained", "tile-item-operation-owner"
        controller_provenance = [{"manifest_index": policy_index,
            "source_to_accepted_fields": {"visited_item_limit": ["max_items"], "removal/excluded_item_ids": ["exclude_items"],
                "removal/movable_required": ["require_movable"], "removal/action_id_required": ["exclude_action_tagged"],
                "removal/unique_id_strictly_greater_than": ["exclude_script_tagged"], "final_effect": ["success_effect_asset_binding"]}}]
        if candidate["targeting"]["aggressive"] is not False:
            evidence = [(index, row) for index, row in enumerate(current_entry["manifest"]["entries"])
                if row.get("destination") == "/spell/spell/targeting/aggressive" and row.get("status") == "resolved_native_behavior" and
                row.get("resolution") == "S27 D.2: Disintegrate is non-aggressive; protected item deletion remains governed by the native item operation."]
            if candidate["targeting"]["aggressive"] is not True or not evidence:
                raise ValueError("NATIVE_DISINTEGRATE_AGGRESSION_POLICY_UNPROVEN")
            retained.append({"field": "/targeting/aggressive", "value": True, "kind": "unprojected_source_policy_conflict",
                             "manifest_indices": [index for index, _ in evidence]})
            projected["spell"]["targeting"]["aggressive"] = False
    else:
        raise ValueError("NATIVE_CONTROLLER_PROJECTION_MODEL_UNSUPPORTED")
    if not barrier:
        projected["spell"]["execution"]["native_behavior"]["parameters"] = _rewrite_items(parameters, reverse_items)
    if source_native["key"] == "acquire_summon" and "range_tiles" in candidate.get("targeting", {}) and \
            "range_tiles" not in current.get("targeting", {}):
        value = projected["spell"]["targeting"].pop("range_tiles")
        if type(value) is not int or value != 0:
            raise ValueError("NATIVE_ACQUIRE_SOURCE_RANGE_UNSUPPORTED")
        retained.append({"field": "/targeting/range_tiles", "value": value,
                         "kind": "unprojected_source_registrar_default"})
    result, deps, receipt = project_native_candidate(projected, projected_dependencies, current_entry,
                                                     explicit_identity, completed_catalog, source_selection)
    receipt.update({"scope": "accepted_partial_native_profile", "whole_source_controller_equivalence": False,
                    "preserved_fields": retained,
                    "raw_source_model": {"bundle": copy.deepcopy(bundle), "dependencies": copy.deepcopy(dependencies),
                                         "catalog": copy.deepcopy(source_catalog)},
                    "controller_provenance": controller_provenance,
                    "numeric_reference_provenance": numeric_references,
                    "partial_source_scope": [{"missing_class": "SOURCE_CONFLICT_KEPT" if source.get("source_model") == "r62/desintegrate_rune" else "ADAPTER_MISSING",
                        "reason": reason, "owner": owner,
                        "source_fields": [row["field"] for row in retained if row["kind"].startswith("unprojected_source_")]}] + extra_scope +
                    ([{"missing_class": "SOURCE_CONFLICT_KEPT", "reason": "accepted_explicit_wiki_or_official_native_header_pair_retained",
                       "owner": "S24-source-precedence", "source_fields": [row["field"] for row in header_conflicts],
                       "field_provenance": header_conflicts}] if header_conflicts else [])})
    return result, deps, receipt
