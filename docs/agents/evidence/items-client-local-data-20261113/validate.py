#!/usr/bin/env python3
"""Validate the pinned prepared projection, without private assets, GUI or network."""
import collections
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
PINS = {
    "client-cyclopedia-coverage.json": "1eab503e0f1ded5c828fb8a424278458fe83c6146ea7794d2e0c9f8f42c9a8cc",
    "client-npc-tuples.json": "dbb9a784bcb6d4dd654cce06db665fe547ccf89c516f14b2d9c0d6903d8bb811",
    "field-comparison-candidates.json": "ba6d7be6d8aaaac0ef629e05aa6c46561822251a443153869c8df5805fe01378",
    "qualified-correction-deltas.json": "3e3c25922175c3cd662b9b05345ec4fcd00fa14521d685521ce8bc46c929c919",
    "item-names-description-custody.json": "213fa25696b34b6aa0ebf4d7e48318861b1d72da53f1127c4799f695611de262",
    "alias-taxonomy-matrix.json": "d53e36c8b6303e48351b60c0e0d762f1cd903cb79e3bd143449936b38da80c12",
    "static-dataset-readiness.json": "5a57afd9040cc3392f84975c3bdb137a8cd4251e3e2b36fc04d513488502c76c",
    "weapon-comparison-supplement.json": "5165efcef1901170d5ad6f76e2a950879d5d84dfee932e038d2f2840bc57703f",
}
NPC_FIELDS = ["appearance_object_id", "flags_message_ordinal1", "flags40_entry_ordinal1",
              "raw_tuple_sha256", "field_presence_bits", "npc_name", "location",
              "sale_price_raw", "buy_price_raw", "currency_object_ref", "quest_currency_label"]


def require(value, reason):
    if not value:
        raise ValueError(reason)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def privacy(value):
    if isinstance(value, dict):
        for key, child in value.items():
            require(key != "Description", "private Description literal field")
            privacy(child)
    elif isinstance(value, list):
        for child in value:
            privacy(child)
    elif isinstance(value, str):
        require(not value.startswith("/") and not re.match(r"^[A-Za-z]:[\\/]", value),
                "absolute filesystem path")
        require(not any(marker in value for marker in
                        ("/home/", "/workspace/", "Bearer ", "bridge-access.json", "server.key")),
                "private path or secret marker")


def decode_columns(doc):
    require(doc["field_order"] == NPC_FIELDS and doc["row_count"] == 15350,
            "NPC field/count contract")
    require(len(doc["columns"]) == len(NPC_FIELDS), "NPC column count")
    columns = []
    for field, column in zip(NPC_FIELDS, doc["columns"]):
        if column["encoding"] == "LITERAL":
            require(set(column) == {"encoding", "values"}, "literal column fields")
            values = column["values"]
        else:
            require(column["encoding"] == "DICTIONARY" and
                    set(column) == {"encoding", "dictionary", "indices"}, "dictionary fields")
            require(all(type(i) is int and 0 <= i < len(column["dictionary"])
                        for i in column["indices"]), "dictionary index")
            require(all(isinstance(v, (str, dict)) for v in column["dictionary"]),
                    "only repeated strings/objects may use dictionaries")
            values = [column["dictionary"][i] for i in column["indices"]]
        require(len(values) == 15350, "NPC column length")
        if field in {"appearance_object_id", "flags_message_ordinal1", "flags40_entry_ordinal1",
                     "sale_price_raw", "buy_price_raw"}:
            require(column["encoding"] == "LITERAL" and all(type(v) is int for v in values),
                    "raw numeric columns must remain literal")
        columns.append(values)
    return [dict(zip(NPC_FIELDS, values)) for values in zip(*columns)]


def validate_npc(doc):
    rows = decode_columns(doc)
    require(doc["original_whitelist_sha256"] ==
            "6c391258de7652fb5b9320b92236d164de3bb8a9ce20ecd7f4d8104b4094d34d"
            and doc["raw_export_sha256"] ==
            "8220257a595dd320e36e533268b0e8bc390fc78eb95193ef54878d0661002bd7",
            "NPC original source pins")
    require(len({(r["appearance_object_id"], r["flags_message_ordinal1"],
                  r["flags40_entry_ordinal1"]) for r in rows}) == 15350, "NPC ancestry deduplication")
    require(len({r["appearance_object_id"] for r in rows}) == 2679, "NPC object census")
    for row in rows:
        require(re.fullmatch(r"[0-9a-f]{64}", row["raw_tuple_sha256"]), "raw tuple SHA")
        require(len(row["field_presence_bits"]) == 6 and
                set(row["field_presence_bits"]) <= {"0", "1"} and
                row["field_presence_bits"].startswith("1111"), "six presence bits")
        currency = row["currency_object_ref"]
        if row["field_presence_bits"][4] == "0":
            require(currency == {"state": "FIELD_ABSENT"}, "absent currency is not gp")
        else:
            require(currency["state"] == "OBSERVED_RAW_CLIENT_APPEARANCE_REFERENCE"
                    and currency["display_currency_binding"] == "UNKNOWN"
                    and currency["server_id_binding"] == "UNKNOWN", "unqualified currency binding")
        quest = row["quest_currency_label"]
        require(set(quest) <= {"state", "byte_count", "character_count", "source_custody",
                               "source_selector", "utf8_sha256"}, "private quest label leaked")
        require(quest["state"] in {"FIELD_ABSENT", "OBSERVED_VALUE_RETAINED_PRIVATELY"},
                "quest custody state")
        require((quest["state"] == "FIELD_ABSENT") == (row["field_presence_bits"][5] == "0"),
                "quest field presence/custody mismatch")
    require(sum(r["sale_price_raw"] == 0 for r in rows) == 6688 and
            sum(r["buy_price_raw"] == 0 for r in rows) == 6859, "known numeric zeros")
    require(sum(r["field_presence_bits"][4] == "1" for r in rows) == 135 and
            sum(r["field_presence_bits"][5] == "1" for r in rows) == 157,
            "currency reference/quest custody census")
    return {"rows": len(rows), "appearance_objects": 2679,
            "explicit_sale_zeros": 6688, "explicit_buy_zeros": 6859}


def decode_table(columns, arrays):
    require(len(set(columns)) == len(columns) and
            all(len(row) == len(columns) for row in arrays), "table shape")
    return [dict(zip(columns, row)) for row in arrays]


def validate_coverage(doc):
    rows = decode_table(doc["record_columns"], doc["record_arrays"])
    aliases = decode_table(doc["alias_columns"], doc["alias_arrays"])
    require(len(rows) == 6470 and len({r["appearance_object_id"] for r in rows}) == 6470,
            "appearance row census")
    require(len({r["cyclopedia_type"] for r in rows}) == 6228 and len(aliases) == 157,
            "client type/alias census")
    groups = collections.defaultdict(list)
    for row in rows:
        groups[row["cyclopedia_type"]].append(row["appearance_object_id"])
        require(re.fullmatch(r"[0-9a-f]{64}", row["raw_record_sha256"]), "record SHA")
    require({r["cyclopedia_type"]: r["appearance_object_ids"] for r in aliases} ==
            {key: values for key, values in groups.items() if len(values) > 1},
            "aliases must preserve distinct appearance rows")
    require(sum(r["NPC_raw_tuple_count"] for r in rows) == 14235 and
            sum(r["NPC_raw_tuple_count"] > 0 for r in rows) == 2571,
            "Cyclopedia subset NPC census")
    return {"appearances": 6470, "client_types": 6228, "alias_groups": 157}


def validate_populations(appearance_ids, npc_rows):
    inside = sum(row["appearance_object_id"] in appearance_ids for row in npc_rows)
    require(inside == 14235 and len(npc_rows) - inside == 1115,
            "global/Cyclopedia NPC populations must remain separate")
    return {"Cyclopedia_subset_tuples": inside, "outside_Cyclopedia_tuples": len(npc_rows) - inside}


def validate_candidates(doc):
    require(doc["total_observations"] == 2623 and len(doc["rows"]) == 115,
            "candidate coverage")
    require(doc["source_layers"]["canonical_records"] == 34111 and
            doc["source_layers"]["selected_records"] == 6090, "separate source layers")
    require(doc["source_pin"] == "5efd67da164ac9dc33c8c066434b228cebb86c2b",
            "historical comparison source pin")
    count = zero = 0
    for row in doc["rows"]:
        require(row["identity_status"] == "NAME_ONLY_CANDIDATE", "identity promotion")
        require(row["proofs"] and row["physical_ui_ordinal"] > 0, "candidate provenance")
        for field in row["fields"]:
            require(field["field"] in {"Armor", "Mantra", "Skill Boost", "Weight"},
                    "non-narrative candidate field whitelist")
            require(set(field["layers"]) == {"canonical", "selected"}, "layer collapse")
            for layer in field["layers"].values():
                require("DIFFERENT" not in layer["comparison_status"] or
                        layer["comparison_status"].endswith("CANDIDATE_ONLY"), "bug promotion")
                zero += layer["source_state"]["state"] == "KNOWN_ZERO"
            count += 1
    require(count == 116 and zero == 30 and doc["known_zero_layer_field_instances_preserved"] == 30,
            "candidate/known-zero census")
    return {"rows": 115, "fields": 116, "known_zero_layer_instances": 30}


def validate_corrections(doc):
    require(len(doc["records"]) == doc["record_count"] == 9 and doc["group_count"] == 8,
            "correction record/group census")
    require(not doc["existing_PR_or_source_mutated"], "history mutation")
    for row in doc["records"]:
        require(not (row["category"] == "Decoration" and row["physical_ui_ordinal"] == 116),
                "rejected Decoration116 candidate")
        require(re.fullmatch(r"[0-9a-f]{64}", row["qualification_sha256"]),
                "correction qualification receipt")
    return {"records": 9, "groups": 8, "empty_arrays_scope": "OBSERVED_INITIAL_PANEL_ONLY"}


def decode_names(doc):
    expected = ["appearance_object_id", "cyclopedia_type", "raw_record_sha256", "name_exact_utf8",
                "name_raw_sha256", "name_utf8_bytes", "name_unicode_codepoints",
                "description_presence", "description_state", "description_raw_sha256",
                "description_raw_bytes", "description_utf8_qualified",
                "description_unicode_codepoints", "description_line_count", "description_count_state"]
    require(doc["field_order"] == expected and doc["row_count"] == 6470
            and len(doc["columns"]) == len(expected), "name custody column contract")
    values = []
    for column in doc["columns"]:
        if column["encoding"] == "literal_values":
            require(set(column) == {"encoding", "values"}, "name literal fields")
            decoded = column["values"]
        else:
            require(column["encoding"] == "dictionary_indices" and
                    set(column) == {"encoding", "dictionary", "indices"}, "name dictionary fields")
            require(all(type(i) is int and 0 <= i < len(column["dictionary"])
                        for i in column["indices"]), "name dictionary index")
            decoded = [column["dictionary"][i] for i in column["indices"]]
        require(len(decoded) == 6470, "name column length")
        values.append(decoded)
    return [dict(zip(expected, row)) for row in zip(*values)]


def validate_names(doc, coverage):
    rows = decode_names(doc)
    observed = {row["appearance_object_id"]: row for row in
                decode_table(coverage["record_columns"], coverage["record_arrays"])}
    require(len({row["appearance_object_id"] for row in rows}) == 6470
            and {row["appearance_object_id"] for row in rows} == set(observed), "name row identity")
    states = collections.Counter()
    for row in rows:
        name = row["name_exact_utf8"]
        require(isinstance(name, str) and name and
                hashlib.sha256(name.encode("utf-8")).hexdigest() == row["name_raw_sha256"]
                and len(name.encode("utf-8")) == row["name_utf8_bytes"]
                and len(name) == row["name_unicode_codepoints"], "exact UTF8 name custody")
        other = observed[row["appearance_object_id"]]
        require(row["cyclopedia_type"] == other["cyclopedia_type"] and
                row["raw_record_sha256"] == other["raw_record_sha256"], "name coverage source join")
        state = row["description_state"]
        states[state] += 1
        if state == "FIELD_ABSENT":
            require(row["description_presence"] is False and all(row[k] is None for k in
                    ("description_raw_sha256", "description_raw_bytes", "description_utf8_qualified",
                     "description_unicode_codepoints", "description_line_count")) and
                    row["description_count_state"] == "METADATA_NOT_APPLICABLE_FIELD_ABSENT",
                    "absent Description must not become zero/known")
        else:
            require(row["description_presence"] is True and
                    re.fullmatch(r"[0-9a-f]{64}", row["description_raw_sha256"]), "Description custody")
            if state == "FIELD_PRESENT_ENCODING_UNKNOWN":
                require(row["description_utf8_qualified"] is False and
                        row["description_unicode_codepoints"] is None and
                        row["description_line_count"] is None and
                        row["description_count_state"] == "UNKNOWN_TEXT_ENCODING",
                        "unknown encoding must not acquire text counts")
            else:
                require(state == "FIELD_PRESENT_UTF8_QUALIFIED" and
                        row["description_utf8_qualified"] is True and
                        row["description_count_state"] == "KNOWN", "Description text state")
    require(states == {"FIELD_ABSENT": 3697, "FIELD_PRESENT_UTF8_QUALIFIED": 2771,
                       "FIELD_PRESENT_ENCODING_UNKNOWN": 2}, "Description state census")
    return {"exact_names": 6470, "Description_custody_states": dict(states)}


def validate_alias_matrix(doc, names, coverage):
    by_id = {row["appearance_object_id"]: row for row in decode_names(names)}
    source = {row["appearance_object_id"]: row for row in
              decode_table(coverage["record_columns"], coverage["record_arrays"])}
    expected = {row["cyclopedia_type"]: row["appearance_object_ids"] for row in
                decode_table(coverage["alias_columns"], coverage["alias_arrays"])}
    require(len(doc["aliases"]) == 157 and
            {row["cyclopedia_type"]: row["appearance_object_ids"] for row in doc["aliases"]} ==
            expected, "alias matrix distinct identities")
    for group in doc["aliases"]:
        members = group["members"]
        require([row["appearance_object_id"] for row in members] == group["appearance_object_ids"],
                "alias member ancestry")
        for member in members:
            name = by_id[member["appearance_object_id"]]
            original = source[member["appearance_object_id"]]
            require(all(member[k] == name[k] for k in
                        ("raw_record_sha256", "name_raw_sha256", "description_presence",
                         "description_state", "description_raw_sha256")), "alias name custody join")
            require(member["market_category"] == original["market_category"] and
                    member["NPC_raw_tuple_occurrences"] == original["NPC_raw_tuple_count"],
                    "alias market/NPC source join")
        require(group["literal_name_variants"] ==
                len({by_id[m["appearance_object_id"]]["name_exact_utf8"] for m in members}) and
                group["description_raw_presence_hash_variants"] ==
                len({(m["description_presence"], m["description_raw_sha256"]) for m in members}) and
                group["market_category_presence_value_variants"] ==
                len({(m["market_category_presence"], m["market_category"]) for m in members}) and
                group["NPC_ordered_semantic_sequence_variants"] ==
                len({m["NPC_ordered_semantic_sequence_sha256"] for m in members}), "alias variants")
    require(len(doc["wholly_market_absent_cyclopedia_types"]) == 1056 and
            len(doc["mixed_market_presence_cyclopedia_types"]) == 40 and
            len(doc["invalid_description_encoding_receipts"]) == 2, "taxonomy/custody census")
    markets = collections.defaultdict(list)
    for row in source.values():
        markets[row["cyclopedia_type"]].append(row["market_category"] is not None)
    require(set(doc["wholly_market_absent_cyclopedia_types"]) ==
            {key for key, values in markets.items() if not any(values)} and
            set(doc["mixed_market_presence_cyclopedia_types"]) ==
            {key for key, values in markets.items() if any(values) and not all(values)},
            "taxonomy presence sets must match source, including category zero")
    return {"groups": 157, "wholly_market_absent_types": 1056, "mixed_market_presence_types": 40}


def validate_static(doc):
    require(doc["raw_current_counts"] == {"creatures": 845, "bestiary_classes": 21, "bosses": 453,
                                         "quest_lines": 104, "achievements": 378, "houses": 995},
            "raw current static counts")
    require(doc["profile_admission"]["runtime_admitted"] is False and
            doc["profile_admission"]["gates_modified"] is False and
            doc["source_profile_admitted"] is False and doc["any_UI_tab_complete"] is False,
            "raw static data must not gain admission/UI qualification")
    require(doc["proficiency_metadata"]["records"] == 444 and
            doc["proficiency_metadata"]["matched_ref_occurrences"] == 667 and
            doc["map_metadata"]["mask_pixels_or_UI_parity_qualified"] is False,
            "static structural readiness boundary")
    return {"raw_counts": doc["raw_current_counts"], "runtime_admitted": False}


def validate_weapons(doc):
    require(len(doc["rows"]) == 4 and doc["observed_field_instances"] == 12 and
            [r["physical_ui_ordinal"] for r in doc["rows"]] == [113, 116, 725, 729],
            "weapon bounded observation scope")
    counts = {"canonical": collections.Counter(), "selected": collections.Counter()}
    for row in doc["rows"]:
        require(row["identity_status"] == "NAME_ONLY_CANDIDATE", "weapon identity promotion")
        for name, layer in row["layers"].items():
            if layer["state"] == "NOT_SELECTED":
                counts[name]["NOT_SELECTED"] += 3
                continue
            require(len(layer["fields"]) == 3 and
                    layer["raw_weapon_states"]["attack"]["state"] == "KNOWN" and
                    layer["raw_weapon_states"]["defense"]["state"] == "KNOWN", "weapon raw states")
            raw = layer["raw_weapon_states"]
            require(raw["weapon_type"]["state"] == "KNOWN" and
                    raw["weapon_type"]["value"] in {"AXE", "SWORD"} and
                    raw["elemental"] in ({"state": "UNKNOWN"}, {"state": "KNOWN", "value": []})
                    and raw["extra_defense"] in
                    ({"state": "UNKNOWN"}, {"state": "KNOWN", "value": 0}), "bounded melee route")
            patterns = layer["raw_equipment_patterns"]
            require(patterns["state"] == "KNOWN", "weapon equipment route")
            two_handed = bool(patterns["value"]) and all(
                p["primary_slot"] == {"state": "KNOWN", "value": "WEAPON"}
                and p["additional_reserved_slots"]["state"] == "KNOWN"
                and "SHIELD" in p["additional_reserved_slots"]["value"] for p in patterns["value"])
            formatted = {"Attack": str(raw["attack"]["value"]),
                         "Defence": str(raw["defense"]["value"]),
                         "Weapon Type": {"AXE": "axe", "SWORD": "sword"}[raw["weapon_type"]["value"]]
                         + (", two-handed" if two_handed else "")}
            require(layer["two_handed_source_projection"] == two_handed and
                    {f["field"]: f["source_formatter_literal"] for f in layer["fields"]} == formatted,
                    "weapon exact bounded source projection")
            for field in layer["fields"]:
                expected = ("LITERAL_EQUAL_CANDIDATE_ONLY" if field["observed_literal"] ==
                            field["source_formatter_literal"] else "LITERAL_DIFFERENT_CANDIDATE_ONLY")
                require(field["status"] == expected, "weapon candidate status")
                counts[name][expected] += 1
    require(counts == {"canonical": {"LITERAL_EQUAL_CANDIDATE_ONLY": 8,
                                     "LITERAL_DIFFERENT_CANDIDATE_ONLY": 4},
                       "selected": {"LITERAL_EQUAL_CANDIDATE_ONLY": 6,
                                    "LITERAL_DIFFERENT_CANDIDATE_ONLY": 3, "NOT_SELECTED": 3}},
            "weapon layer census")
    return {"observed_fields": 12, "counts": {k: dict(v) for k, v in counts.items()}}


def validate(root=HERE):
    documents = {}
    for name, expected in PINS.items():
        require(digest(root / name) == expected, "prepared member hash: " + name)
        doc = json.loads((root / name).read_text())
        privacy(doc)
        documents[name] = doc
    coverage_doc = documents["client-cyclopedia-coverage.json"]
    ids = {row["appearance_object_id"] for row in
           decode_table(coverage_doc["record_columns"], coverage_doc["record_arrays"])}
    populations = validate_populations(ids, decode_columns(documents["client-npc-tuples.json"]))
    names = documents["item-names-description-custody.json"]
    return {"members": 8, "coverage": validate_coverage(coverage_doc),
            "npc": validate_npc(documents["client-npc-tuples.json"]),
            "NPC_populations": populations,
            "candidates": validate_candidates(documents["field-comparison-candidates.json"]),
            "corrections": validate_corrections(documents["qualified-correction-deltas.json"]),
            "names": validate_names(names, coverage_doc),
            "alias_matrix": validate_alias_matrix(documents["alias-taxonomy-matrix.json"], names,
                                                   coverage_doc),
            "static_readiness": validate_static(documents["static-dataset-readiness.json"]),
            "weapons": validate_weapons(documents["weapon-comparison-supplement.json"]),
            "private_originals_rechecked_here": False, "overall_static_audit": "PARTIAL",
            "server_id_bindings": 0}


if __name__ == "__main__":
    print(json.dumps(validate(), sort_keys=True))
