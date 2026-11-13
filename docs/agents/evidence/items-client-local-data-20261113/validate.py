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
    return {"members": 4, "coverage": validate_coverage(coverage_doc),
            "npc": validate_npc(documents["client-npc-tuples.json"]),
            "NPC_populations": populations,
            "candidates": validate_candidates(documents["field-comparison-candidates.json"]),
            "corrections": validate_corrections(documents["qualified-correction-deltas.json"]),
            "private_originals_rechecked_here": False, "overall_static_audit": "PARTIAL",
            "server_id_bindings": 0}


if __name__ == "__main__":
    print(json.dumps(validate(), sort_keys=True))
