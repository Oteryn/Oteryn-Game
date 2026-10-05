"""Lower ordinary donor models without reopening accepted numerical/source policy.

The caller must verify donor Git/file proofs separately. Every source value that
does not enter the canonical graph is retained in ``preserved_fields``. Unknown
mechanics and closed native profiles produce explicit ``remaining`` diagnostics.
"""
import copy
from decimal import Decimal
import json
from pathlib import Path
import re

from current_spell_adapter import DONORS, _project_local_graph, canonical_bytes, identity
from validate_spell import validate

PARTY_POLICY = json.loads(Path(__file__).with_name("party-behaviours.json").read_text(encoding="utf-8"))


def _nonnegative(expr):
    """Use monotone positive arithmetic, never distributive algebra, as proof."""
    if "const" in expr:
        return Decimal(str(expr["const"])) >= 0
    if "var" in expr:
        return expr["var"] in {"level", "magic_level", "attack_skill", "attack_value",
                               "attack_factor", "base_power", "shielding_skill", "shield_defense"}
    if expr.get("fn") == "level_base_damage_healing":
        return expr.get("args") == [{"var": "level"}]
    args = expr.get("args", [])
    if expr.get("op") in {"add", "mul", "floor", "ceil", "sqrt", "abs"}:
        return bool(args) and all(_nonnegative(arg) for arg in args)
    if expr.get("op") == "div" and len(args) == 2 and "const" in args[1]:
        return _nonnegative(args[0]) and Decimal(str(args[1]["const"])) > 0
    return False


def _ordered(before, after):
    """Prove ordering through matching monotone IEEE operations only."""
    if canonical_bytes(before) == canonical_bytes(after):
        return True
    if "const" in before and "const" in after:
        return float(before["const"]) <= float(after["const"])
    op = before.get("op")
    if op != after.get("op") or op not in {"add", "mul", "div", "floor", "ceil", "sqrt"}:
        return False
    left, right = before.get("args", []), after.get("args", [])
    if len(left) != len(right) or not left:
        return False
    if op == "div":
        return len(left) == 2 and left[1] == right[1] and "const" in left[1] \
            and float(left[1]["const"]) > 0 and _ordered(left[0], right[0])
    return all(_nonnegative(arg) for arg in left + right) and \
        all(_ordered(a, b) for a, b in zip(left, right))


def _normal_expression(expr, world_curve=False):
    """Strip provably redundant sign/min/max wrappers, preserving arithmetic order."""
    if not isinstance(expr, dict):
        return expr
    if world_curve and expr.get("op") in {"mul", "div"}:
        args = expr.get("args", [])
        if len(args) == 2 and args[0] == {"var": "level"} and "const" in args[1] and \
                Decimal(str(args[1]["const"])) == (Decimal("0.2") if expr["op"] == "mul" else Decimal(5)):
            return {"fn": "level_base_damage_healing", "args": [{"var": "level"}]}
    result = {key: copy.deepcopy(value) for key, value in expr.items() if key != "args"}
    if "args" not in expr:
        return result
    args = [_normal_expression(arg, world_curve) for arg in expr["args"]]
    result["args"] = args
    if result.get("op") == "abs" and len(args) == 1:
        child = args[0]
        if child.get("op") == "neg":
            child = child["args"][0]
        if _nonnegative(child):
            return child
    if result.get("op") in {"min", "max"} and len(args) == 2:
        if _ordered(args[0], args[1]):
            return args[0 if result["op"] == "min" else 1]
    return result


def _effect_role_entry(dependencies, current_entry):
    """Keep existing Effect IDs attached to their roles when source order differs."""
    source = dependencies["effects"]
    target = current_entry["dependencies"]["effects"]
    if len(source) != len(target):
        return current_entry  # The common graph gate diagnoses count mismatches.
    formula_refs = {}
    for before, after in zip(dependencies["formulas"], current_entry["dependencies"]["formulas"]):
        formula_refs[(before["identity"]["key"], before["identity"]["revision"])] = after["identity"]

    def exact_role(value):
        if isinstance(value, list):
            return [exact_role(child) for child in value]
        if not isinstance(value, dict):
            return value
        if set(value) == {"family", "key", "revision"} and value["family"] == "Formula":
            identity = formula_refs.get((value["key"], value["revision"]))
            if identity is not None:
                return {"family": "Formula", **identity}
        return {key: exact_role(child) for key, child in value.items()}

    remaining = list(target)
    ordered = []
    for effect in source:
        choices = [item for item in remaining if item["operation"] == effect["operation"]]
        if sum(item["operation"] == effect["operation"] for item in target) > 1:
            role = exact_role({key: value for key, value in effect.items() if key != "identity"})
            choices = [item for item in choices if canonical_bytes(role) == canonical_bytes(
                {key: value for key, value in item.items() if key != "identity"})]
        if len(choices) != 1:
            raise ValueError("AMBIGUOUS_OR_CHANGED_EFFECT_ROLE:" + effect["operation"])
        chosen = choices[0]
        ordered.append(chosen)
        remaining.remove(chosen)
    projected = copy.deepcopy(current_entry)
    projected["dependencies"]["effects"] = copy.deepcopy(ordered)
    return projected


def _decompose_condition_presentation(dependencies, current_entry):
    """Split an exact combined DoT effect into the two existing accepted roles.

    This is a representation decomposition, not raw source payload equality.
    The condition schedule and every presentation binding must already equal the
    accepted graph; its presentation-before-condition ordering is retained.
    """
    source, accepted = dependencies, current_entry["dependencies"]
    if len(source["effects"]) != 1 or len(accepted["effects"]) != 2:
        return dependencies, None
    if {effect["operation"] for effect in accepted["effects"]} != {"presentation_only", "condition"}:
        return dependencies, None
    if len(source["abilities"]) != 1 or len(accepted["abilities"]) != 1 or \
            source["formulas"] or accepted["formulas"]:
        raise ValueError("COMBINED_CONDITION_PRESENTATION_GRAPH_NOT_SUPPORTED")
    combined = source["effects"][0]
    if set(combined) != {"identity", "operation", "condition", "presentation"} or combined["operation"] != "condition":
        raise ValueError("COMBINED_CONDITION_PRESENTATION_FIELDS_UNREPRESENTED")
    presentation = next(effect for effect in accepted["effects"] if effect["operation"] == "presentation_only")
    condition = next(effect for effect in accepted["effects"] if effect["operation"] == "condition")
    if set(presentation) != {"identity", "operation", "presentation"} or \
            set(condition) != {"identity", "operation", "condition"}:
        raise ValueError("COMBINED_CONDITION_PRESENTATION_ACCEPTED_ROLES_NOT_EXACT")
    if canonical_bytes(combined["condition"]) != canonical_bytes(condition["condition"]) or \
            canonical_bytes(combined["presentation"]) != canonical_bytes(presentation["presentation"]):
        raise ValueError("COMBINED_CONDITION_PRESENTATION_SCHEDULE_OR_BINDING_CONFLICT")
    source_refs = source["abilities"][0].get("effects")
    source_identity = identity(combined["identity"])
    if any(source_identity == identity(ability["identity"]) for ability in source["abilities"]):
        raise ValueError("COMBINED_CONDITION_PRESENTATION_DUPLICATE_LOCAL_IDENTITY")
    old_ref = {"family": "Effect", **source_identity}
    new_refs = [{"family": "Effect", **effect["identity"]} for effect in (presentation, condition)]
    if source_refs != [old_ref] or accepted["abilities"][0].get("effects") != new_refs:
        raise ValueError("COMBINED_CONDITION_PRESENTATION_EXECUTION_ORDER_NOT_EXACT")
    projected = copy.deepcopy(dependencies)
    projected["effects"] = copy.deepcopy([presentation, condition])
    projected["abilities"][0]["effects"] = copy.deepcopy(new_refs)
    proof = {"combined_source_effect": copy.deepcopy(combined),
             "source_ability_effects": copy.deepcopy(source_refs),
             "canonical_effects": copy.deepcopy(new_refs),
             "execution_order": ["presentation_only", "condition"]}
    return projected, proof


def _pointer(document, path):
    value = document
    for part in path.split("/")[1:]:
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def _replace(document, path, value):
    parts = path.split("/")[1:]
    parent = document
    for part in parts[:-1]:
        parent = parent[int(part)] if isinstance(parent, list) else parent[part]
    if isinstance(parent, list):
        parent[int(parts[-1])] = copy.deepcopy(value)
    else:
        parent[parts[-1]] = copy.deepcopy(value)


def lower_party_source_model(bundle, dependencies, current_entry):
    """Retain the accepted C.3 party model with explicit donor differences.

    This qualifies an accepted partial model. Donor member-selection, numeric
    regeneration and commit details are retained, never asserted equivalent to
    the closed current party executor. The generic native gate stays unchanged.
    """
    result = {"bundle": None, "dependencies": None, "preserved_fields": [], "remaining": [],
              "partial_source_scope": [], "scope": "accepted_partial_model", "runtime_activation": False}

    def fail(reason, cls="CONTRACT_GAP"):
        result["remaining"].append({"path": "/spell/execution/native_behavior", "missing_class": cls,
                                    "reason": reason, "owner": "party-buff-owner"})
        return result

    def keys(value, expected):
        if not isinstance(value, dict) or set(value) != set(expected):
            raise ValueError("PARTY_SOURCE_FIELDS_NOT_CLOSED")

    try:
        incoming = bundle["spell"]
        current = current_entry["bundle"]["spell"]
        name = current["name"].casefold()
        policy = PARTY_POLICY["spells"].get(name)
        if policy is None or incoming["name"].casefold() != name or incoming["carrier"] != "instant":
            return fail("PARTY_EXISTING_IDENTITY_REQUIRED")
        identity(incoming["identity"])
        if any(type(incoming.get(field)) is not type(current.get(field)) or
               incoming.get(field) != current.get(field) for field in ("reference_spell_id", "words")):
            return fail("PARTY_SOURCE_REFERENCE_OR_WORDS_MISMATCH", "SOURCE_CONFLICT_KEPT")
        keys(dependencies, {"abilities", "effects", "formulas"})
        if any(dependencies.values()):
            return fail("PARTY_SOURCE_UNEXPECTED_DEPENDENCIES")
        keys(incoming["execution"], {"native_behavior"})
        native = incoming["execution"]["native_behavior"]
        keys(native, {"key", "parameters"})
        if native["key"] != "party_buff" or current["execution"]["native_behavior"]["key"] != "party_buff":
            return fail("PARTY_NATIVE_FAMILY_MISMATCH")
        source = native["parameters"]
        keys(source, {"combat_presentation", "commit_order", "conditions", "insufficient_mana", "mana",
                      "member_success_presentation", "membership", "no_party_or_insufficient_members",
                      "per_member_commit_order", "source_model"})
        if source["source_model"] != "r61-party_buff":
            return fail("PARTY_PRIVATE_MODEL_VERSION_NOT_SUPPORTED")
        keys(source["combat_presentation"], {"aggressive", "areas", "condition_application_independent_of_area",
                                            "effect", "execute_failure_effect", "execute_failure_message"})
        keys(source["membership"], {"deduplicate", "distance_lte", "distance_metric", "min_selected_count",
                                   "requires_party", "requires_source_list_size_greater_than_one", "same_floor_required",
                                   "source", "source_list_type_guard_after_leader_append"})
        keys(source["mana"], {"add_mana_spent", "additional_debit", "check_total_before_presentation", "count_multiplier",
                             "debit_notify", "extra_delta_can_be_negative", "geometric_ratio", "mana_spent_numeric_input_type",
                             "mode", "power_offset", "refund_on_failed_cast", "registrar_base", "registrar_cost_after_success"})
        if not isinstance(source["conditions"], list) or len(source["conditions"]) != 1:
            return fail("PARTY_SOURCE_REQUIRES_EXACTLY_ONE_CONDITION")
        combat = source["combat_presentation"]
        if type(combat["aggressive"]) is not bool or type(combat["condition_application_independent_of_area"]) is not bool or \
                combat["effect"] != "CONST_ME_MAGIC_BLUE" or combat["execute_failure_effect"] != "poff" or \
                combat["execute_failure_message"] != "not_possible" or not isinstance(combat["areas"], list):
            return fail("PARTY_SOURCE_PRESENTATION_FIELDS_NOT_TYPED")
        for area in combat["areas"]:
            keys(area, {"north"})
            matrix = area["north"]
            if not isinstance(matrix, list) or not matrix or any(not isinstance(row, list) or not row for row in matrix) or \
                    len({len(row) for row in matrix}) != 1 or any(type(cell) is not int or cell not in {0, 1, 2, 3}
                                                                            for row in matrix for cell in row) or \
                    sum(cell in {2, 3} for row in matrix for cell in row) != 1:
                return fail("PARTY_SOURCE_PRESENTATION_AREA_NOT_TYPED")
        for group, numeric, texts in ((source["membership"], {"distance_lte", "min_selected_count"},
                                      {"distance_metric", "source"}),
                                     (source["mana"], {"registrar_base", "geometric_ratio", "power_offset"},
                                      {"add_mana_spent", "additional_debit", "mana_spent_numeric_input_type", "mode"})):
            for key, value in group.items():
                if key in numeric:
                    if value is not None and type(value) not in {int, float}:
                        return fail("PARTY_SOURCE_NUMERIC_FIELD_NOT_TYPED")
                elif key in texts:
                    if not isinstance(value, str):
                        return fail("PARTY_SOURCE_TEXT_FIELD_NOT_TYPED")
                elif type(value) is not bool:
                    return fail("PARTY_SOURCE_BOOLEAN_FIELD_NOT_TYPED")
        if source["membership"]["distance_metric"] != "max_abs_xyz" or \
                source["membership"]["source"] != "members_then_append_leader" or \
                source["mana"]["mode"] not in {"ceil_geometric_party_size", "registrar_only"} or \
                source["member_success_presentation"] not in {None, "magic_blue"}:
            return fail("PARTY_SOURCE_CONTROLLER_BINDING_NOT_SUPPORTED")
        if any(type(source["membership"][key]) is not int or source["membership"][key] <= 0
               for key in ("distance_lte", "min_selected_count")) or \
                type(source["mana"]["registrar_base"]) is not int or source["mana"]["registrar_base"] < 0 or \
                source["mana"]["power_offset"] not in {None, -1} or \
                (source["mana"]["geometric_ratio"] is not None and not (0 < source["mana"]["geometric_ratio"] < 1)):
            return fail("PARTY_SOURCE_NUMERIC_DOMAIN_NOT_SUPPORTED")
        condition = source["conditions"][0]
        keys(condition, {"condition_type", "parameters", "replacement"})
        attributes = {"enchant party": {"stat_magicpoints"}, "protect party": {"skill_shield"},
                      "train party": {"skill_melee", "skill_distance"}}
        extras = attributes.get(name, {"healthgain", "healthticks"} if name == "heal party" else {"managain", "manaticks"})
        keys(condition["parameters"], {"buff_spell", "subid", "ticks"} | extras)
        expected_type = "CONDITION_ATTRIBUTES" if name in attributes else "CONDITION_REGENERATION"
        if condition["condition_type"] != expected_type or condition["replacement"] != "source_condition_type_id_subid_rules":
            return fail("PARTY_SOURCE_CONDITION_KIND_NOT_SUPPORTED")
        for key, value in condition["parameters"].items():
            if key == "buff_spell":
                if canonical_bytes(value) not in {canonical_bytes(1), canonical_bytes(True)}:
                    return fail("PARTY_SOURCE_BUFF_FLAG_NOT_SUPPORTED")
            elif type(value) is not int or value < 0:
                return fail("PARTY_SOURCE_CONDITION_VALUE_NOT_TYPED")
        keys(source["no_party_or_insufficient_members"], {"effect", "message", "return"})
        failure = source["no_party_or_insufficient_members"]
        if type(failure["return"]) is not bool or failure["return"] is not False or failure["effect"] != "poff" or \
                failure["message"] not in {None, "No party members in range."}:
            return fail("PARTY_SOURCE_FAILURE_NOT_TYPED")
        if source["insufficient_mana"] is not None:
            keys(source["insufficient_mana"], {"effect", "message", "order", "return"})
            failure = source["insufficient_mana"]
            if type(failure["return"]) is not bool or failure["return"] is not False or failure["effect"] != "poff" or \
                    failure["message"] != "RETURNVALUE_NOTENOUGHMANA" or \
                    failure["order"] != ["cancel_message", "caster_poff", "return_false"]:
                return fail("PARTY_SOURCE_FAILURE_NOT_TYPED")
        if source["commit_order"] not in (["select_members", "reject_small_selection", "compute_total_mana", "check_total_mana",
                                           "execute_combat_or_fail", "debit_extra_mana", "record_extra_mana_spent",
                                           "apply_per_member_commits_in_member_list_order", "return_true", "registrar_success_cost"],
                                          ["select_members", "reject_small_selection", "execute_combat_or_fail",
                                           "apply_per_member_commits_in_member_list_order", "return_true", "registrar_success_cost"]) or \
                source["per_member_commit_order"] not in (["add_condition"], ["add_condition", "magic_blue"]):
            return fail("PARTY_SOURCE_COMMIT_OPERATIONS_NOT_SUPPORTED", "RUNTIME_GAP")
        evidence = [row for row in current_entry.get("manifest", {}).get("entries", [])
                    if row.get("status") == "resolved_native_behavior" and
                    row.get("destination") == "/spell/spell/execution/native_behavior" and
                    row.get("resolution") == "S27 C.3: party_buff from party-behaviours.json (" + policy["sources"] + ")"]
        if not evidence:
            return fail("PARTY_EXACT_C3_FIELD_PRECEDENCE_PROOF_REQUIRED", "SOURCE_CONFLICT_KEPT")
        accepted = current["execution"]["native_behavior"]["parameters"]
        member_condition = {**copy.deepcopy(policy["condition"]), "buff_spell": True, "lifetime": "fixed_duration"}
        expected_mana = policy.get("mana", {"mode": "scaled", "base": policy.get("base_mana"),
                                              "falloff": 0.9, "rounding": "up"})
        expected = {"area": PARTY_POLICY["area"], "same_floor": True, "min_affected": 2,
                    "requires_party": True, "mana": expected_mana, "effect": accepted["effect"]}
        effect = current_entry["dependencies"]["effects"]
        if canonical_bytes(accepted) != canonical_bytes(expected) or len(effect) != 1 or \
                effect[0]["operation"] != "condition" or effect[0]["condition"] != member_condition or \
                effect[0]["duration_ms"] != policy.get("duration_ms", 120000) or \
                {"family": "Effect", **identity(effect[0]["identity"])} != accepted["effect"]:
            return fail("PARTY_ACCEPTED_PAYLOAD_DOES_NOT_MATCH_FIELD_POLICY")
        issues = validate(current_entry["bundle"], current_entry["dependencies"], current_entry.get("catalog"))
        if issues:
            return fail("PARTY_ACCEPTED_V1_INVALID:" + "; ".join(issues))
    except (KeyError, ValueError, TypeError):
        return fail("PARTY_SOURCE_OR_ACCEPTED_SHAPE_INVALID")
    source_values = {"area": source["combat_presentation"]["areas"],
                     "same_floor": source["membership"]["same_floor_required"],
                     "min_affected": source["membership"]["min_selected_count"],
                     "requires_party": source["membership"]["requires_party"],
                     "mana": source["mana"], "effect": source["conditions"]}
    for field in source_values:
        result["preserved_fields"].append({"path": "/spell/execution/native_behavior/parameters/" + field,
                                           "value": copy.deepcopy(source_values[field]), "accepted_value": copy.deepcopy(accepted[field]),
                                           "reason": "S27_C3_explicit_party_field_precedence", "evidence": evidence})
    result["preserved_fields"].append({"path": "/spell", "value": copy.deepcopy(incoming),
                                       "reason": "raw_party_header_and_controller_retained_with_explicit_accepted_partial_scope"})
    result["partial_source_scope"] = [{"scope": "donor_party_lifecycle_and_selection",
                                        "reason": "C3_accepted_model_uses_radius3_same_floor_and_separate_per_spell_conditions",
                                        "missing_class": "SOURCE_CONFLICT_KEPT", "owner": "party-buff-owner",
                                        "source_fields": ["membership", "combat_presentation", "commit_order", "conditions", "mana"]}]
    result["bundle"] = copy.deepcopy(current_entry["bundle"])
    result["dependencies"] = copy.deepcopy(current_entry["dependencies"])
    return result


def lower_source_model(bundle, dependencies, current_entry):
    """Qualify a full ordinary graph; unresolved Formula values stay held."""
    return _lower_formal_source_model(bundle, dependencies, current_entry)


def _lower_formal_source_model(bundle, dependencies, current_entry, review_roles=False):
    """Return canonical ``bundle/dependencies`` plus explicit retained source facts.

    This is data authoring qualification, never activation. Local graph sizes
    and the existing identity allocation remain closed. Formula changes require
    exact equivalence after explicit S5 normalization; unresolved coefficients stay
    a source conflict. An existing manifest's official/wiki resolutions retain
    their individual values under S24, rather than blanket-copying the header.
    """
    preserved, remaining = [], []

    def save(path, value, reason):
        preserved.append({"path": path, "value": copy.deepcopy(value), "reason": reason})

    def hold(path, reason, cls="CONTRACT_GAP", owner="spell-content"):
        remaining.append({"path": path, "missing_class": cls, "reason": reason,
                          "owner": owner})

    source = copy.deepcopy(bundle)
    deps = copy.deepcopy(dependencies)
    spell = source.get("spell", {})
    current = current_entry["bundle"]["spell"]
    result = {"bundle": None, "dependencies": None, "preserved_fields": preserved,
              "remaining": remaining, "scope": "ordinary_formal_v1_authoring",
              "runtime_activation": False}
    if spell.get("name", "").casefold() != current["name"].casefold() or \
            spell.get("carrier") != current["carrier"]:
        hold("/spell", "source_name_or_carrier_does_not_match_existing_identity")
        return result
    for label, value in (("source", spell), ("current", current)):
        if value.get("execution", {}).get("native_behavior") is not None:
            hold("/spell/execution", label + "_native_profile_requires_closed_runtime_projection",
                 "ADAPTER_MISSING", "native-profile-owner")
        if value.get("requirements", {}).get("wheel_unlock") is True:
            hold("/spell/requirements/wheel_unlock", "SPELL-WHEEL-GATE-1_access_projection_missing",
                 "RUNTIME_GAP", "SPELL-WHEEL-GATE-1")
    if remaining:
        return result
    for key in ("source_state_contract", "harmony_cost"):
        if key in spell:
            hold("/spell/" + key, "private_state_or_resource_contract_has_no_formal_v1_binding")
    if remaining:
        return result

    requirements = spell.get("requirements", {})
    if "vocation_display_flags" in requirements:
        flags = requirements.pop("vocation_display_flags")
        if not isinstance(flags, list) or any(not isinstance(row, dict) or
                set(row) != {"vocation", "show_in_description"} or
                not isinstance(row["vocation"], str) or
                row["vocation"] not in requirements.get("vocations", []) or
                type(row["show_in_description"]) is not bool for row in flags) or \
                len({row["vocation"] for row in flags}) != len(flags):
            hold("/spell/requirements/vocation_display_flags", "unrecognized_vocation_ui_metadata")
        else:
            save("/spell/requirements/vocation_display_flags", flags,
                 "source_teacher_list_visibility_is_not_a_cast_requirement_S16")

    if "source_cast_sound" in spell.get("presentation", {}):
        sound = spell["presentation"].pop("source_cast_sound")
        valid = isinstance(sound, dict) and sound.get("binding_status") in {"unbound", "knownbound"} \
            and isinstance(sound.get("constant"), str) \
            and re.fullmatch(r"SOUND_EFFECT_TYPE_[A-Z0-9_]+", sound["constant"]) \
            and set(sound) == ({"binding_status", "constant"} if sound["binding_status"] == "unbound"
                               else {"binding_status", "constant", "source_numeric_value"}) \
            and (sound["binding_status"] == "unbound" or type(sound["source_numeric_value"]) is int)
        if not valid:
            hold("/spell/presentation/source_cast_sound", "unrecognized_source_sound_binding")
        else:
            save("/spell/presentation/source_cast_sound", sound,
                 "S18_unregistered_Lua_constant_is_silence" if sound["binding_status"] == "unbound"
                 else "S18_registered_constant_mapped_to_existing_sound_namespace")
            if sound["binding_status"] == "knownbound":
                cue = "canary.sound:" + sound["constant"].removeprefix("SOUND_EFFECT_TYPE_").lower()
                if spell["presentation"].get("cast_cue", cue) != cue:
                    hold("/spell/presentation/cast_cue", "source_sound_metadata_and_cast_cue_disagree")
                else:
                    spell["presentation"]["cast_cue"] = cue
    for index, ability in enumerate(deps.get("abilities", [])):
        if ability.get("zero_damage_health_path") is False:
            save(f"/dependencies/abilities/{index}/zero_damage_health_path", False,
                 "false_matches_absent_health_hook_opt_in")
            del ability["zero_damage_health_path"]

    if "reference_rune_item_id" in spell:
        numeric_item = spell.pop("reference_rune_item_id")
        item = spell.get("rune", {}).get("item", {})
        if type(numeric_item) is not int or not item.get("key", "").endswith("/" + str(numeric_item)):
            hold("/spell/reference_rune_item_id", "numeric_rune_id_does_not_match_typed_item_reference")
        else:
            save("/spell/reference_rune_item_id", numeric_item, "same_item_preserved_by_typed_Item_reference")

    # Only numeric Item aliases are bound. A matching tail is insufficient for
    # arbitrary Creature/Interaction keys or unknown item identities.
    items = {}
    for ref in current_entry.get("catalog", {}).get("definitions", []):
        match = re.fullmatch(r"candidate:item/([0-9]+)", ref.get("key", ""))
        if ref.get("family") == "Item" and match:
            items[match[1]] = ref

    def external_refs(value, path=""):
        if isinstance(value, list):
            return [external_refs(child, path + "/" + str(i)) for i, child in enumerate(value)]
        if not isinstance(value, dict):
            return value
        if set(value) == {"family", "key", "revision"} and value["family"] == "Item":
            match = re.fullmatch(r"candidate:(?:item/source/[a-z0-9-]+|"
                                 r"spell/source/[a-z0-9-]+/[0-9a-f]{16}/item)/([0-9]+)", value["key"])
            if match:
                target = items.get(match[1])
                if target is None:
                    hold(path, "numeric_Item_identity_missing_in_current_catalog", "CONTRACT_GAP", "item-content")
                else:
                    save(path, value, "exact_numeric_Item_binding_to_existing_catalog")
                    return copy.deepcopy(target)
        return {key: external_refs(child, path + "/" + key) for key, child in value.items()}

    source = external_refs(source)
    deps = external_refs(deps, "/dependencies")
    if remaining:
        return result
    try:
        deps, decomposition = _decompose_condition_presentation(deps, current_entry)
        if decomposition is not None:
            save("/dependencies/effects", decomposition,
                 "lossless_condition_presentation_decomposition_into_existing_roles_and_approved_timing")
        role_entry = _effect_role_entry(deps, current_entry)
        projected, deps = _project_local_graph(source, deps, role_entry)
    except (ValueError, KeyError, TypeError) as exc:
        hold("/dependencies", str(exc), "ADAPTER_MISSING")
        return result
    spell = projected["spell"]
    spell["identity"] = copy.deepcopy(current["identity"])
    spell["name"] = current["name"]

    # Apply prior evidence only to the exact manifest destinations. No donor
    # gameplay field is silently replaced by a blanket accepted-header copy.
    protected = set()
    for entry in current_entry.get("manifest", {}).get("entries", []):
        path = entry.get("destination", "")
        resolution = entry.get("resolution", "").lower()
        if entry.get("status") == "mapped" and path.startswith("/spell/spell/") and \
                any(term in resolution for term in ("official", "the wiki", "s16:")):
            protected.add(path.removeprefix("/spell"))
    for path in sorted(protected):
        try:
            accepted = _pointer(current_entry["bundle"], path)
        except (KeyError, IndexError, ValueError):
            continue
        try:
            incoming = _pointer(projected, path)
        except (KeyError, IndexError, ValueError):
            try:
                _replace(projected, path, accepted)
            except (KeyError, IndexError, ValueError):
                hold(path, "existing_official_value_has_no_source_graph_destination")
            else:
                save(path, {"source_present": False}, "S24_existing_official_or_wiki_value_retained")
            continue
        if canonical_bytes(incoming) != canonical_bytes(accepted):
            save(path, incoming, "S24_existing_official_or_wiki_value_retained")
            _replace(projected, path, accepted)
    for field in ("reference_spell_id", "presentation"):
        if field not in spell and field in current:
            spell[field] = copy.deepcopy(current[field])
    for field in ("allow_on_self", "check_floor", "allowed_targets", "aim_at_target", "cast_at_position"):
        if field not in spell["targeting"] and field in current["targeting"]:
            spell["targeting"][field] = copy.deepcopy(current["targeting"][field])
    conjure = spell.get("execution", {}).get("conjure")
    accepted_conjure = current.get("execution", {}).get("conjure", {})
    if conjure is not None and "effect_asset_binding" not in conjure and "effect_asset_binding" in accepted_conjure:
        conjure["effect_asset_binding"] = accepted_conjure["effect_asset_binding"]
    if review_roles:
        errors = validate(projected, deps, current_entry.get("catalog"))
        if errors:
            for error in errors:
                hold("/formal_v1/source_graph", error, "ADAPTER_MISSING")
            return result
    for index, formula in enumerate(deps["formulas"]):
        accepted = current_entry["dependencies"]["formulas"][index]
        if canonical_bytes(formula) != canonical_bytes(accepted):
            normalized = copy.deepcopy(formula)
            expected = copy.deepcopy(accepted)
            try:
                for bound in ("minimum", "maximum"):
                    if bound in normalized:
                        normalized[bound] = _normal_expression(normalized[bound], world_curve=True)
                    if bound in expected:
                        expected[bound] = _normal_expression(expected[bound])
            except (KeyError, ValueError, TypeError, ArithmeticError):
                hold(f"/dependencies/formulas/{index}", "source_formula_expression_shape_is_invalid", "ADAPTER_MISSING")
                continue
            if canonical_bytes(normalized) == canonical_bytes(expected):
                save(f"/dependencies/formulas/{index}", formula,
                     "S5_world_curve_and_proven_redundant_sign_order_wrappers")
                deps["formulas"][index] = copy.deepcopy(accepted)
            else:
                hold(f"/dependencies/formulas/{index}",
                     "source_formula_coefficients_or_order_disagree_after_S5_normalization",
                     "SOURCE_CONFLICT_KEPT", "S24-source-precedence")
                if review_roles:
                    save(f"/dependencies/formulas/{index}", formula,
                         "unresolved_donor_AST_retained_for_review_current_Formula_not_replaced")
                    deps["formulas"][index] = copy.deepcopy(accepted)
    if remaining and not (review_roles and all(row["missing_class"] == "SOURCE_CONFLICT_KEPT"
                                               for row in remaining)):
        return result
    errors = validate(projected, deps, current_entry.get("catalog"))
    for error in errors:
        hold("/formal_v1", error, "ADAPTER_MISSING")
    if not errors and (not remaining or review_roles):
        result["bundle"], result["dependencies"] = projected, deps
    return result


def _source_expression(expression):
    """Decode a closed prepared Lua AST, preserving its ordered binary64 fold."""
    if not isinstance(expression, dict):
        raise ValueError("SOURCE_AST_NOT_OBJECT")
    if set(expression) == {"const"}:
        if not isinstance(expression["const"], str) or not Decimal(expression["const"]).is_finite():
            raise ValueError("SOURCE_AST_CONSTANT_INVALID")
        return copy.deepcopy(expression)
    if set(expression) == {"input"} and expression["input"] in {"level", "magic_level", "attack_skill",
            "attack_value", "attack_factor", "base_power", "shielding_skill", "shield_defense"}:
        return {"var": expression["input"]}
    if expression == {"helper": "crystal_base_damage_healing", "input": "level"}:
        return {"fn": "level_base_damage_healing", "args": [{"var": "level"}]}
    operators = {"add": "add", "subtract": "sub", "multiply": "mul", "divide": "div",
                 "negate": "neg", "floor": "floor", "ceil": "ceil", "sqrt": "sqrt",
                 "absolute": "abs", "minimum": "min", "maximum": "max"}
    if set(expression) != {"operator", "arguments"} or expression["operator"] not in operators or \
            not isinstance(expression["arguments"], list):
        raise ValueError("SOURCE_AST_OPERATOR_UNREPRESENTED")
    operation = operators[expression["operator"]]
    args = [_source_expression(child) for child in expression["arguments"]]
    if operation in {"add", "mul"} and len(args) >= 2:
        folded = args[0]
        for child in args[1:]:
            folded = {"op": operation, "args": [folded, child]}
        return folded
    if len(args) != (1 if operation in {"neg", "floor", "ceil", "sqrt", "abs"} else 2):
        raise ValueError("SOURCE_AST_ARITY_INVALID")
    return {"op": operation, "args": args}


def lower_combat_controller_roles(bundle, dependencies, current_entry, cue_aliases=None, existing_cues=None):
    """Review a donor BASE graph using existing IDs while retaining every held scope.

    This does not accept conflicting numbers: they remain SOURCE_CONFLICT_KEPT,
    the accepted Formula remains in the review graph and the actual donor AST is
    retained separately. The caller verifies pinned model/enum proofs for aliases.
    Neither this function nor the generic adapter enables source controllers.
    """
    result = {"bundle": None, "dependencies": None, "preserved_fields": [], "remaining": [],
              "partial_source_scope": [], "scope": "combat_roles_review", "runtime_activation": False}

    def held(path, reason, cls="ADAPTER_MISSING", owner="ordinary-combat"):
        result["remaining"].append({"path": path, "reason": reason, "missing_class": cls, "owner": owner})

    def exact_ref(value, family):
        if not isinstance(value, dict) or set(value) != {"family", "key", "revision"} or value["family"] != family:
            raise ValueError("CONTROLLER_REFERENCE_FAMILY_OR_SHAPE")
        return identity({key: value[key] for key in ("key", "revision")})

    def alias(token, kind):
        row = (cue_aliases or {}).get(token)
        if not isinstance(row, dict) or set(row) != {"alias", "source_numeric_value", "proof"} or \
                type(row["source_numeric_value"]) is not int or row["source_numeric_value"] <= 0 or \
                not isinstance(row["proof"], dict) or \
                set(row["proof"]) != {"revision", "path", "sha256"} or \
                not re.fullmatch(r"[0-9a-f]{40}", row["proof"].get("revision", "")) or \
                row["proof"]["revision"] not in {pin[2] for pin in DONORS.values()} or \
                not re.fullmatch(r"[0-9a-f]{64}", row["proof"].get("sha256", "")) or \
                row["proof"].get("path") != "src/utils/utils_definitions.hpp" or \
                not isinstance(row["alias"], str) or \
                not re.fullmatch(r"canary\.appearance:" + kind + r"/[a-z0-9_]+", row["alias"]) or \
                (existing_cues or {}).get(row["alias"]) != {"kind": kind, "value": row["source_numeric_value"]}:
            raise ValueError("CONTROLLER_CUE_ALIAS_REQUIRES_PINNED_ENUM_PROOF:" + str(token))
        result["preserved_fields"].append({"path": "/source_cues/" + token, "value": copy.deepcopy(row),
                                            "reason": "exact_pinned_enum_numeric_alias_to_existing_binding"})
        return row["alias"]

    try:
        source = copy.deepcopy(bundle)
        deps = copy.deepcopy(dependencies)
        if not isinstance(source, dict) or set(source) != {"spell"} or not isinstance(source["spell"], dict):
            raise ValueError("CONTROLLER_BUNDLE_SHAPE")
        spell, current = source["spell"], current_entry["bundle"]["spell"]
        if any(not isinstance(spell.get(key), dict) for key in ("execution", "requirements", "targeting")):
            raise ValueError("CONTROLLER_HEADER_SHAPE")
        identity(spell["identity"])
        if any(spell.get(key) != current.get(key) for key in ("name", "carrier", "reference_spell_id", "words")):
            raise ValueError("CONTROLLER_HEADER_IDENTITY_MISMATCH")
        if current["execution"].get("native_behavior") is not None or \
                any(item.get("requirements", {}).get("wheel_unlock") is True for item in (spell, current)):
            raise ValueError("CONTROLLER_NATIVE_OR_WHEEL_GATE_REMAINS_CLOSED")
        if set(deps) != {"abilities", "effects", "formulas"}:
            raise ValueError("CONTROLLER_DEPENDENCY_COLLECTION_SHAPE")
        controller = spell["execution"].get("native_behavior")
        if set(spell["execution"]) != {"native_behavior"} or not isinstance(controller, dict) or \
                set(controller) != {"key", "parameters"} or not isinstance(controller["parameters"], dict):
            raise ValueError("CONTROLLER_MODEL_REQUIRED")
        params = controller["parameters"]
        result["preserved_fields"].append({"path": "/spell/execution/native_behavior", "value": copy.deepcopy(controller),
                                            "reason": "complete_donor_controller_retained_no_whole_source_parity"})
        if controller["key"] == "equipment_attack" and params.get("source_model") in {"r60_equipment_source_v1", "r61-equipment_attack"}:
            expected = {"combat", "consumer_contract_pending", "default_cost_commit", "equipment", "harmony", "routes", "source_model"}
            if "chain" in params:
                expected.add("chain")
            if set(params) != expected:
                raise ValueError("EQUIPMENT_SOURCE_CONTROLLER_FIELDS_UNREPRESENTED")
            routes = params["routes"]
            if not isinstance(routes, list) or not routes:
                raise ValueError("EQUIPMENT_SOURCE_ROUTES_REQUIRED")
            ordinary_route = {"ability", "area_variant", "damage_type", "hit_order", "source_combat_index", "source_phase"}
            chain_route = {"ability", "damage_type", "source_combat_index"}
            if any(not isinstance(row, dict) or set(row) != (chain_route if "chain" in params else ordinary_route)
                   or type(row["source_combat_index"]) is not int or
                   ("hit_order" in row and type(row["hit_order"]) is not int) for row in routes):
                raise ValueError("EQUIPMENT_ROUTE_FIELDS_UNREPRESENTED")
            choices = [row for row in routes if row.get("damage_type") == "physical" and
                       row.get("source_combat_index") == 0 and row.get("source_phase", "primary") == "primary" and
                       row.get("area_variant", "default") == "default" and row.get("hit_order", 0) == 0]
            if len(choices) != 1:
                raise ValueError("EQUIPMENT_BASE_ROUTE_AMBIGUOUS")
            chosen = choices[0]
            ref = exact_ref(chosen["ability"], "Ability")
            selected = {}
            for section in deps:
                if not isinstance(deps[section], list):
                    raise ValueError("EQUIPMENT_DEPENDENCY_LIST_REQUIRED")
                ids = [canonical_bytes(identity(item["identity"])) for item in deps[section]]
                if len(set(ids)) != len(ids):
                    raise ValueError("EQUIPMENT_DUPLICATE_DEPENDENCY_IDENTITY")
            ability = next(item for item in deps["abilities"] if item["identity"] == ref)
            effect_ids = [exact_ref(value, "Effect") for value in ability["effects"]]
            selected["abilities"] = [ability]
            selected["effects"] = [item for item in deps["effects"] if item["identity"] in effect_ids]
            if len(selected["effects"]) != len(effect_ids) or any(item.get("operation") != "damage" or
                    item.get("damage_type") != "physical" for item in selected["effects"]):
                raise ValueError("EQUIPMENT_BASE_EFFECT_ROLE_MISMATCH")
            formula_ids = [exact_ref(item["formula"], "Formula") for item in selected["effects"]]
            selected["formulas"] = [item for item in deps["formulas"] if item["identity"] in formula_ids]
            if len(selected["formulas"]) != len(set(canonical_bytes(value) for value in formula_ids)):
                raise ValueError("EQUIPMENT_BASE_FORMULA_REFERENCE_MISMATCH")
            for item in selected["effects"]:
                for field, value in item.get("presentation", {}).items():
                    match = re.fullmatch(r"source:(effect|missile)/(const_(?:me|ani)_[a-z0-9_]+)", value)
                    if match:
                        item["presentation"][field] = alias(match[2].upper(), match[1])
            deps = selected
            result["preserved_fields"].append({"path": "/dependencies", "value": copy.deepcopy(dependencies),
                "reason": "complete_donor_route_graph_retained_selected_BASE_is_only_partial_projection"})
            spell["execution"] = {"ability": copy.deepcopy(chosen["ability"])}
            harmony = params["harmony"]
            if not isinstance(harmony, dict) or type(harmony.get("gain")) is not int or \
                    type(harmony.get("spend_all")) is not bool:
                raise ValueError("EQUIPMENT_HARMONY_ROLE_TYPED_SOURCE_REQUIRED")
            role = "builder" if harmony["gain"] == 1 and not harmony["spend_all"] else \
                "spender" if harmony["gain"] == 0 and harmony["spend_all"] else None
            if role is None or role != current.get("harmony_role"):
                raise ValueError("EQUIPMENT_HARMONY_ROLE_CONFLICT")
            spell["harmony_role"] = role
            result["preserved_fields"].append({"path": "/spell/harmony_role", "value":
                {"gain": harmony["gain"], "spend_all": harmony["spend_all"]},
                "reason": "source_builder_or_spender_role_maps_existing_accepted_header_role"})
            if "harmony_cost" in spell:
                if spell["harmony_cost"] is not True or role != "spender":
                    raise ValueError("EQUIPMENT_HARMONY_COST_SOURCE_SHAPE_UNREPRESENTED")
                result["preserved_fields"].append({"path": "/spell/harmony_cost", "value": spell.pop("harmony_cost"),
                    "reason": "private_source_charge_cost_retained_under_unprojected_harmony_lifecycle_scope"})
            result["partial_source_scope"].append({"scope": "physical_primary_default_route_only",
                "source_fields": ["equipment", "harmony", "combat", "chain", "routes_except_selected", "/spell/harmony_cost"],
                "missing_class": "RUNTIME_GAP", "owner": "equipment-harmony-owner",
                "reason": "source_equipment_element_and_harmony_lifecycle_not_projected_into_ordinary_BASE"})
        elif controller["key"] == "shared_conservation" and params.get("source_model") in {"r62/heal_friend", "r62/natures_embrace"}:
            expected = {"caster_pre_primary_blue", "combat_bindings", "combat_definitions", "execution_order",
                    "formula_input_binding", "formula_math_semantics", "helper_definitions", "party_selector", "primary_formula",
                    "secondary_attempted_even_primary_false", "secondary_formula", "self_refusal", "source_cast_return",
                    "source_costs_unchanged", "source_function_scopes", "source_model", "source_monk_spell_type", "source_sound_bindings"}
            if params["source_model"] == "r62/natures_embrace":
                expected.add("self_refusal_message")
            if set(params) != expected:
                raise ValueError("CONSERVATION_SOURCE_CONTROLLER_FIELDS_UNREPRESENTED")
            if any(deps.values()) or params.get("combat_bindings") != {"primary": 0, "secondary_shared_conservation": 1} or \
                    params.get("execution_order") != ["primary_execute", "secondary_helper", "return_primary_result"] or \
                    params.get("source_costs_unchanged") is not True or params.get("source_cast_return") != "combat_result":
                raise ValueError("CONSERVATION_SOURCE_PRIMARY_EXECUTION_NOT_EXACT")
            combat = params["combat_definitions"][0]
            if set(combat) != {"aggressive", "area", "block_armor", "callback_bindings", "chain_effect", "condition_declarations",
                    "dispel_condition", "health_type", "impact_effect", "projectile_effect", "source_combat_index",
                    "source_parameter_bindings", "use_charges"} or \
                    type(combat.get("source_combat_index")) is not int or combat.get("source_combat_index") != 0 or \
                    combat.get("health_type") != "COMBAT_HEALING" or \
                    combat.get("condition_declarations") != [] or combat.get("area") is not None or \
                    combat.get("dispel_condition") != "CONDITION_PARALYZE" or combat.get("aggressive") is not False or \
                    combat.get("block_armor") is not False or combat.get("use_charges") is not False or \
                    combat.get("projectile_effect") != "CONST_ANI_NONE" or combat.get("chain_effect") != "CONST_ME_NONE" or \
                    combat.get("callback_bindings") != [{"function": "onGetFormulaValues", "kind": "CALLBACK_PARAM_LEVELMAGICVALUE"}]:
                raise ValueError("CONSERVATION_PRIMARY_COMBAT_FIELDS_UNREPRESENTED")
            declarations = combat["source_parameter_bindings"]
            if not isinstance(declarations, dict) or set(declarations) != {"COMBAT_PARAM_AGGRESSIVE", "COMBAT_PARAM_DISPEL",
                    "COMBAT_PARAM_EFFECT", "COMBAT_PARAM_TYPE"} or \
                    type(declarations["COMBAT_PARAM_AGGRESSIVE"]) not in (bool, int) or \
                    declarations["COMBAT_PARAM_AGGRESSIVE"] not in (False, 0) or \
                    declarations["COMBAT_PARAM_DISPEL"] != combat["dispel_condition"] or \
                    declarations["COMBAT_PARAM_EFFECT"] != combat["impact_effect"] or \
                    declarations["COMBAT_PARAM_TYPE"] != combat["health_type"]:
                raise ValueError("CONSERVATION_PRIMARY_DECLARATIONS_DISAGREE")
            ast = params["primary_formula"]
            if set(ast) != {"evaluation", "maximum", "minimum", "pair_semantics", "sampling"} or \
                    ast["evaluation"] != "lua_Number_then_source_LuaCombat_binding_int32" or \
                    ast["pair_semantics"] != "raw_source_signed_callback_pair_no_absolute_value_projection" or \
                    ast["sampling"] != "source_normal_random_inclusive":
                raise ValueError("CONSERVATION_FORMULA_SEMANTICS_UNREPRESENTED")
            formulas = current_entry["dependencies"]["formulas"]
            effects = current_entry["dependencies"]["effects"]
            abilities = current_entry["dependencies"]["abilities"]
            if len(formulas) != 1 or len(abilities) != 1 or len(effects) != 2 or \
                    {item["operation"] for item in effects} != {"heal", "remove_condition"}:
                raise ValueError("CONSERVATION_EXISTING_ROLES_REQUIRED")
            formula = {"identity": copy.deepcopy(formulas[0]["identity"]), "kind": "player_expression", "inputs": "level_magic",
                       "minimum": _source_expression(ast["minimum"]), "maximum": _source_expression(ast["maximum"])}
            if "level_base_damage_healing" in str(formula) and not any(
                    "S5:" in row.get("resolution", "") and "world curve level_base_damage_healing" in row.get("resolution", "")
                    for row in current_entry.get("manifest", {}).get("entries", [])):
                raise ValueError("CONSERVATION_S5_WORLD_CURVE_FIELD_PROOF_REQUIRED")
            if not all(_nonnegative(formula[key]) for key in ("minimum", "maximum")):
                raise ValueError("CONSERVATION_RAW_HEAL_SIGN_NOT_PROVEN")
            heal = {"identity": copy.deepcopy(next(item for item in effects if item["operation"] == "heal")["identity"]),
                    "operation": "heal", "damage_type": "healing", "formula": {"family": "Formula", **formula["identity"]},
                    "presentation": {"impact_asset_binding": alias(combat["impact_effect"], "effect")}}
            if type(params.get("caster_pre_primary_blue")) is not bool:
                raise ValueError("CONSERVATION_CASTER_PRESENTATION_TYPE")
            if params["caster_pre_primary_blue"]:
                heal["presentation"].update({"caster_effect_asset_binding": alias("CONST_ME_MAGIC_BLUE", "effect"),
                                            "caster_effect_timing": "before_combat"})
            dispel = {"identity": copy.deepcopy(next(item for item in effects if item["operation"] == "remove_condition")["identity"]),
                      "operation": "remove_condition", "removed_condition": "paralyze"}
            ability = {"identity": copy.deepcopy(abilities[0]["identity"]), "kind": "spell", "needs_direction": False,
                       "needs_target": True, "range_tiles": 0,
                       "effects": [{"family": "Effect", **item["identity"]} for item in (heal, dispel)]}
            deps = {"abilities": [ability], "effects": [heal, dispel], "formulas": [formula]}
            spell["execution"] = {"ability": {"family": "Ability", **ability["identity"]}}
            result["partial_source_scope"].append({"scope": "primary_heal_and_paralyze_dispel_only",
                "source_fields": ["party_selector", "secondary_formula", "formula_input_binding", "self_refusal", "self_refusal_message"],
                "missing_class": "RUNTIME_GAP", "owner": "shared-conservation-owner",
                "reason": "source_secondary_party_stance_selection_and_specialized_input_controller_remain_unprojected"})
        else:
            raise ValueError("SOURCE_CONTROLLER_BASE_PROJECTION_NOT_IMPLEMENTED")
        lowered = _lower_formal_source_model(source, deps, current_entry, review_roles=True)
        for field in ("bundle", "dependencies"):
            result[field] = lowered[field]
        result["preserved_fields"].extend(lowered["preserved_fields"])
        result["remaining"].extend(lowered["remaining"])
    except (ValueError, KeyError, TypeError, StopIteration, ArithmeticError) as exc:
        reason = str(exc)
        if reason.startswith("CONTROLLER_CUE_ALIAS_REQUIRES_PINNED_ENUM_PROOF:"):
            held("/dependencies/effects/presentation", reason, "CONTRACT_GAP", "spell-presentation")
        else:
            held("/spell/execution/native_behavior", reason)
    return result
