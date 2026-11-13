"""Validate the five pinned, narrative-free static evidence projections."""
import base64
import collections
import hashlib
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent
MAIN = "3e168ce6afe374485da20ee0ac13d3749558e2f8"
PINS = {
    "achievements378-metadata.json": (239968, "982274568551ef5ae0ce4321300c69221b1ad44932d7e99e47e431f36bcfdb76"),
    "creatures845-classes21-bosses453-summary.json": (3662, "fc5860f648520f1d39f41479470ade073b7f764d88e950c717876c79d958029a"),
    "quests104-title-custody.json": (25653, "d6da9d45b893a21f74e7b5842809727826291114cf30964123da2cb8c6f470b6"),
    "houses995-source-metadata-columns.json": (279106, "2173775fd705b8a6754f9625f159995c60586a713f5c8ed30f062d06a04687d3"),
    "maps473-structure-summary.json": (2537, "3731869e639d80e54216d45093c2dacb43ddc3b1cc844f672ab718edb258c51b"),
}

def require(condition, message):
    if not condition:
        raise ValueError(message)

def sha(value):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value), "invalid SHA256")

def privacy(value):
    """Reject machine/capture paths and narrative-bearing fields, not custody."""
    forbidden = {"description", "description_display_lines", "restriction_text", "title",
                 "source_name_exact", "mission_text", "password", "token", "server_key",
                 "raw_bytes", "look_bytes", "image_bytes", "engine_entrance"}
    if isinstance(value, dict):
        for key, child in value.items():
            # The Houses boundary engine_entrance is an explicit exclusion, not coordinates.
            if key == "source_name_exact":
                require(child == {"SOURCE_VALUE_EQUAL": 995}, "house name literal exported")
            elif key == "engine_entrance":
                require(child == "NOT_SELECTED_NOT_COMPARED", "engine entrance exported")
            else:
                require(key.lower() not in forbidden, "forbidden literal field: " + key)
            privacy(child)
    elif isinstance(value, list):
        for child in value:
            privacy(child)
    elif isinstance(value, str):
        require(not re.search(r"(?i)(/home/|/workspace/|[a-z]:[\\/]|bearer |bridge-access|server\.key|\.(?:png|apng|webp)(?:$|[?#]))", value),
                "private path, capture or secret marker")

def achievements(x):
    rows = x["record_arrays"]
    require(len(rows) == 378 and len(x["record_columns"]) == 9, "achievement coverage")
    require([r[0] for r in rows] == list(range(1, 379)), "achievement order")
    require(len({r[1] for r in rows}) == 378 and len({r[3] for r in rows}) == 378, "achievement IDs/names")
    names = collections.Counter()
    evaluations = collections.Counter()
    raw = collections.Counter()
    for r in rows:
        require(len(r) == 9, "achievement row width")
        sha(r[2])
        require(isinstance(r[3], str) and r[3], "achievement name")
        custody = r[4]
        require(custody["field_presence"] is True and custody["text_state"] == "FIELD_PRESENT_UTF8_KNOWN_NONEMPTY", "achievement custody state")
        sha(custody["raw_sha256"])
        require(custody["utf8_bytes"] >= custody["unicode_codepoints"] > 0, "achievement custody sizes")
        require(r[6] == "HELD_OLD_PARSER_GRADE_LABEL_NOT_CURRENT15_33_QUALIFIED", "grade inference")
        raw[str(r[5])] += 1
        names[r[7]] += 1
        require(len(r[8]) == (1 if r[7] == "UNIQUE_EXACT_NAME_CANDIDATE" else 0), "achievement candidate count")
        for key, state, source in r[8]:
            require(key.startswith("oteryn:achievement/"), "achievement candidate namespace")
            evaluations[state] += 1
            sha(source["sha256"])
            if state == "EXACT_UTF8_HASH_BYTES_EQUAL_CANDIDATE_ONLY":
                require(source["sha256"] == custody["raw_sha256"] and source["utf8_bytes"] == custody["utf8_bytes"], "achievement false equality")
            else:
                require(state == "EXACT_UTF8_DIFFERENT_CANDIDATE_ONLY" and source["sha256"] != custody["raw_sha256"], "achievement difference")
    require(dict(names) == {"UNIQUE_EXACT_NAME_CANDIDATE": 368, "NO_EXACT_NAME_CANDIDATE": 10}, "achievement name counts")
    require(dict(evaluations) == {"EXACT_UTF8_HASH_BYTES_EQUAL_CANDIDATE_ONLY": 367, "EXACT_UTF8_DIFFERENT_CANDIDATE_ONLY": 1}, "achievement comparison counts")
    require(dict(raw) == {"1": 284, "2": 85, "3": 9}, "raw field4 histogram")
    require(x["counts"]["serverID_bindings"] == 0 and x["counts"]["semantic_grade_points_flags_comparisons_performed"] == 0, "achievement semantic promotion")
    require(x["source_header"]["source_commit"] == MAIN, "achievement source pin")

def quests(x):
    rows = x["rows"]
    require(len(rows) == 104 and len({r[0] for r in rows}) == 104, "quest coverage")
    require([r[1] for r in rows] == list(range(104)), "quest order")
    new = []
    for r in rows:
        require(len(r) == 9 and type(r[0]) is int, "quest row shape")
        sha(r[2]); sha(r[3])
        require(r[4] >= r[5] > 0, "quest title custody")
        if r[6] == 0:
            sha(r[7])
            require(r[2] == r[7] and r[8] is True, "old quest equality")
        else:
            require(r[6] == 1 and r[7] is None and r[8] is False, "new quest state")
            new.append(r[0])
    require(sorted(new) == [192, 195], "new quest IDs")
    require(x["record_contract"]["profile_admitted"] is False and x["record_contract"]["runtime_crosswalk"] == "NOT_QUALIFIED", "quest runtime promotion")
    require(x["counts"]["exact_old_raw_record_equal"] == 102 and x["counts"]["removed_old_source_IDs"] == [], "quest baseline counts")

def houses(x):
    rows = x["rows"]
    require(len(rows) == 995 and len({r[0] for r in rows}) == 995, "house ID coverage")
    require(len({r[2] for r in rows}) == 995, "house key uniqueness")
    require(x["source_main"] == MAIN and x["catalogue_key_prefix"] == "oteryn:content.house.", "house source namespace")
    require(x["hash_dictionary_encoding"] == "SHA256_BYTES_BASE64URL_NO_PADDING", "house hash encoding")
    hashes = x["sha256_dictionary"]
    for h in hashes:
        require(isinstance(h, str) and re.fullmatch(r"[A-Za-z0-9_-]{43}", h), "house hash dictionary")
        require(len(base64.urlsafe_b64decode(h + "=")) == 32, "house SHA256 length")
    comparisons = x["comparison_field_dictionary"]
    require(len(comparisons) == 12 and x["comparison_state_dictionary"] == ["SOURCE_VALUE_EQUAL", "SOURCE_VALUE_DIFFERENT"], "house states")
    changed = []
    for r in rows:
        require(len(r) == 18, "house row width")
        require(0 <= r[3] < len(x["kind_dictionary"]) and 0 <= r[4] < len(x["town_dictionary"]), "house enums")
        require(all(type(v) is int and v >= 0 for v in r[5:8]), "house known numerics")
        require(len(r[8]) == 3 and len(r[9]) == 6 and all(type(v) is int for v in r[8] + r[9]), "house marker/footprint")
        require(all(type(v) is int and 0 <= v < len(hashes) for v in r[10:16]), "house hash references")
        require(r[11] == r[12], "house static record mismatch")
        require(len(r[17]) == 12 and all(v in (0, 1) for v in r[17]), "house comparison shape")
        require(all(v == 0 for i, v in enumerate(r[17]) if i != comparisons.index("raw_layout_record")), "house qualified metadata mismatch")
        different = r[13] != r[14]
        require(r[17][comparisons.index("raw_layout_record")] == int(different), "house layout status")
        if different:
            changed.append(r[0])
    require(changed == [55015], "house layout difference")
    require(sum(r[6] == 0 for r in rows) == 6, "known zero beds lost")
    require(x["boundaries"]["engine_entrance"] == "NOT_SELECTED_NOT_COMPARED", "engine entrance scope")

def creatures(x):
    require(x["source_head"] == MAIN, "creature source pin")
    expected = {"creatures": (798, 21, 26), "bosses": (386, 8, 59)}
    for kind, counts in expected.items():
        c = x["counts"][kind]
        require(tuple(c[k] for k in ["UNIQUE_NAME_CANDIDATE", "AMBIGUOUS_NAME_CANDIDATES", "NO_NAME_CANDIDATE"]) == counts, "creature name populations")
    require(sum(expected["creatures"]) == 845 and sum(expected["bosses"]) == 453, "creature cohort totals")
    require(x["counts"]["classes"]["EXACT_LABEL_PRESENT_IN_SOURCE_CLASS_SET"] == 21 and x["exact_class_label_sets_equal"] is True, "class labels")
    for values in x["raw_field_distributions"]["creatures"].values():
        require(sum(values.values()) == 845, "creature raw histogram")
    require(sum(x["raw_field_distributions"]["bosses"]["f4"].values()) == 453, "boss raw histogram")
    require(x["runtime_admitted"] is False and x["gates_modified"] is False and x["UI_tab_complete"] is False, "creature admission")
    require("UNKNOWN" in x["field_scope"]["boss_f4"] and "UNKNOWN" in x["field_scope"]["creature_f4_f5_f6_f7"], "unqualified raw semantics")

def maps(x):
    c = x["counts"]
    require(x["source_head"] == MAIN and c["areas"] == 473, "map source/coverage")
    require(c["source_provenance_statuses"] == {"EXACT_RAW_RECORD_PROVENANCE_MATCH": 463, "SAME_CLIENT_SOURCE_ID_OLD_RAW_RECORD_DIFFERENT": 2, "NO_OLD_SOURCE_ID_PROVENANCE_CANDIDATE": 8}, "map provenance states")
    require(sum(c["source_provenance_statuses"].values()) == c["areas"], "map populations")
    require(c["subarea_image_refs"] == c["mask_file_exists"] == 212 and c["markers_counted_only"] == 1297, "map structural counts")
    require(c["dangling_child_ids"] == 0 and c["multiply_parented_child_ids"] == 1, "map ancestry")
    require(x["client_inputs"] == 0 and x["repo_writes"] == 0, "map mutation scope")

CHECKS = dict(zip(PINS, [achievements, creatures, quests, houses, maps]))

def validate_payloads(payloads):
    require(set(payloads) == set(PINS), "exact five payloads required")
    for name, x in payloads.items():
        privacy(x)
        CHECKS[name](x)

def load(directory=ROOT):
    payloads = {}
    for name, (size, digest) in PINS.items():
        data = (directory / name).read_bytes()
        require(len(data) == size and hashlib.sha256(data).hexdigest() == digest, "prepared pin drift: " + name)
        payloads[name] = json.loads(data)
    require(sum(v[0] for v in PINS.values()) == 550926, "payload size total")
    return payloads

def main():
    validate_payloads(load())
    print(json.dumps({"result": "PASS", "members": 5, "bytes": 550926,
                      "achievements": 378, "creatures": 845, "classes": 21, "bosses": 453,
                      "quests": 104, "houses": 995, "maps": 473,
                      "layout_changed_source_ids": [55015], "quest_new_source_ids": [192, 195],
                      "overall_static_audit": "PARTIAL", "runtime_admitted": False,
                      "private_originals_rechecked_in_Cloud": False}))

if __name__ == "__main__":
    main()
