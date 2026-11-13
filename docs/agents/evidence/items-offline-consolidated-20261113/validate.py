#!/usr/bin/env python3
"""Check catalogue/UI integrity, namespace separation and private-data exclusion."""
import collections
import copy
import hashlib
import json
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
PINS = {
    "client6470-flags.json": "d345130bed5f2b7a419bd929120dbe6672d2848aaaa717f96cc508d0f9386d69",
    "client6470-five-tags.json": "49bde9e4057d7fbcc2452b08bdf2fe6b084e078e180d4165f73658d908cb0378",
    "legacy-armors-amulets.json": "d8fbfc7bd4e0017f3e8db68cd9e28296e0664fbf9821f88d8b33f771fa6469fe",
    "client-ui-relations.json": "ded2d16a63c3e57480cc2d84fa5a8c9c55a58bd06614c3faeed9af5b81ce599f",
    "proficiency-profiles.json": "4c79c4508e2577c04d1ef38a6fd048b4d5a5ebcab133a5b862dff6608d218278",
    "ui2623-observations.json": "0c4e4c99a8c4018bf9bce486279dc328e8bc6d988cf0e99196dfcd600992e73e",
}
CATEGORIES = {"Creature Products":980,"Decoration":462,"Food":169,"Gold":3,
              "Helmets and Hats":128,"Legs":66,"Others":603,"Potions":36,
              "Boots":67,"Containers":109}
FIELD_COUNTS = {"Weight":2623,"Tradeable In Market":2620,"Armor":242,"Attack":4,
                "Augments":46,"Body Position":400,"Capacity":84,"Classification":240,
                "Damage Reflection":1,"Defence":4,"Imbuement Effect":72,
                "Imbuement Slots":169,"Mantra":15,"Protection":150,
                "Required Level":143,"Skill Boost":120,"Total Expire Time":5,
                "Vocations":149,"Weapon Type":4}
SHA = re.compile(r"[0-9a-f]{64}\Z")

def require(value, reason):
    if not value:
        raise ValueError(reason)

def privacy(value):
    if isinstance(value, dict):
        for key, child in value.items():
            require((key not in {"description_display_lines", "ui_description"} or child == "EXCLUDED") and
                    (key != "Description" or isinstance(child, (dict, int)) or child in
                     ("ABSENT_IN_ALL_OBSERVED_BASIC_PAGES", "OBSERVED_VALUE",
                      "UNKNOWN_LITERAL_GLYPHS")), "private narrative")
            privacy(child)
    elif isinstance(value, list):
        for child in value:
            privacy(child)
    elif isinstance(value, str):
        require(not value.startswith(("/home/", "/workspace/", "C:\\", "C:/")),
                "private absolute path")
        require(not re.search(r"(?:^|[\\/])[^ ]*\.(?:png|apng|jpe?g|bmp)(?:$|[?#])",
                              value, re.I), "image path")
        require(not any(v in value for v in
                        ("Bearer ", "bridge-access.json", "server.key", "server.crt")),
                "secret/config marker")

def check_flags(doc):
    rows = doc["rows"]
    require(len(rows)==6470 and len({r[0] for r in rows})==6470, "local row identity")
    require(len({r[1] for r in rows})==6228, "Cyclopedia aliases retained")
    require(doc["source"]["immutable_prior_publication"]["frozen_git_commit_sha"] ==
            "54c930ab3771d28d7d5584beed6b3e69125bf5a7", "immutable source head")
    fields, sets = doc["known_flag_field_dictionary"], doc["known_flag_sets"]
    require(len(fields)==54 and len(set(fields))==54, "flag dictionary")
    require(not any(f.startswith("npcsaledata.") for f in fields), "NPC payload exclusion")
    for group in sets:
        require(len({p[0] for p in group})==len(group), "duplicate sparse field")
        require(all(type(p[0]) is int and 0 <= p[0] < len(fields) for p in group),
                "flag index")
    for row in rows:
        require(len(row)==9 and SHA.fullmatch(row[2]) and isinstance(row[3],str),
                "local row shape")
        require(type(row[4]) is int and 0 <= row[4] < len(sets), "flag set reference")
        require(type(row[5]) is int and 0 <= row[5] < len(doc["Description_custody_rows"]),
                "Description custody reference")
    require(sum(len(sets[r[4]]) for r in rows)==59546, "decoded flag instance count")
    require(sum(r[6] for r in rows)==14235, "NPC cohort occurrence count")
    return {r[0]:r for r in rows}

def check_tags(doc, local):
    rows = doc["rows"]
    require(len(rows)==6470 and {r[0] for r in rows}==set(local), "tag cohort identity")
    counts = collections.Counter()
    for row in rows:
        require(len(row)==10 and row[1:3]==local[row[0]][1:3], "tag provenance match")
        mask = row[3]
        require(type(mask) is int and 0 <= mask < 32, "presence mask")
        for bit in range(5):
            present, value = bool(mask & (1 << bit)), row[4+bit]
            require(present == (value is not None), "presence must not default values")
            if present:
                counts[60+bit] += 1
                if bit==2:
                    require(isinstance(value,list) and value and
                            all(type(v) is int and v>=0 for v in value),
                            "ordered repeated tag62")
                else:
                    require(type(value) is int and value>=0, "unsigned raw scalar")
        require(row[9] == (1 if mask & 2 else 0), "profile join state")
    require(dict(counts)=={60:636,61:667,62:747,63:925,64:672}, "tag coverage")
    require(sum(len(r[6] or []) for r in rows)==884, "tag62 occurrence count")
    profiles = doc["referenced_profile_rows"]
    require(len(profiles)==431 and len({r[0] for r in profiles})==431, "profile dictionary")
    require(all(r[1]=="EXACT_CLIENT_PROFILE_ID_MATCH" and SHA.fullmatch(r[3])
                for r in profiles), "profile join qualification")
    require({r[5] for r in rows if r[3]&2}=={r[0] for r in profiles}, "profile references")

def check_ui(doc):
    require(doc["proof_columns"]==["opaque_proof_id","sha256"], "opaque proof format")
    proofs, rows = doc["proof_dictionary"], doc["rows"]
    require(len(rows)==2623 and len(proofs)==10528, "UI/proof count")
    require(all(re.fullmatch(r"proof:\d+", p[0]) and SHA.fullmatch(p[1])
                for p in proofs), "proof identifiers")
    categories, labels = doc["category_dictionary"], collections.Counter()
    coverage, keys, references, descriptions = collections.Counter(), set(), 0, 0
    for row in rows:
        require(len(row)==8, "UI row shape")
        category, ordinal, capture, name, props, states, custody, refs = row
        require(type(category) is int and 0 <= category < len(categories), "UI category")
        require(type(ordinal) is int and ordinal>0, "physical ordinal")
        key = (category,ordinal)
        require(key not in keys, "physical duplicates must remain separate ordinals")
        keys.add(key)
        coverage[categories[category]] += 1
        require(isinstance(name,str) and isinstance(props,dict), "UI literals")
        require("Description" not in props, "private description")
        labels.update(props.keys())
        descriptions += custody["state"] != "NOT_RECORDED_IN_PROPERTIES"
        for value in capture.values():
            require(not isinstance(value,str) or value.startswith("capture:"),
                    "opaque capture identifiers")
        for role, index in refs:
            require(type(index) is int and 0 <= index < len(proofs), "proof reference")
            references += 1
    require(dict(coverage)==CATEGORIES, "UI category coverage")
    require(dict(labels)==FIELD_COUNTS, "UI field coverage")
    require(references==12630 and descriptions==645, "UI custody/proof occurrence counts")
    require(doc["boundaries"]["server_id_binding"]=="NOT_QUALIFIED", "server namespace boundary")
    return coverage

def check_profiles(doc,tags):
    profiles,levels,perks=doc["profile_rows"],doc["level_rows"],doc["perk_rows"]
    require(len(profiles)==444 and len(levels)==2216 and len(perks)==3680,"profile census")
    by_id={r[0]:r for r in profiles}
    require(len(by_id)==444,"profile identity")
    for ref in tags["referenced_profile_rows"]:
        require(ref[0] in by_id and ref[2]==by_id[ref[0]][2] and
                ref[3]==by_id[ref[0]][5],"profile version/raw hash match")
    level_ids={(r[0],r[1]):r for r in levels}
    require(len(level_ids)==2216,"profile level identity")
    observed=collections.Counter()
    fields=doc["raw_perk_field_dictionary"]
    held=[]
    for profile,level,ordinal,values,rawhash,status,unit in perks:
        require((profile,level) in level_ids and profile in by_id,"perk parent")
        require(0<=values<len(doc["raw_perk_value_sets"]) and
                0<=rawhash<len(doc["raw_perk_hash_dictionary"]),"perk dictionary reference")
        observed[profile,level]+=1
        raw={fields[k]:v for k,v in doc["raw_perk_value_sets"][values]}
        require(type(raw["Type"]) is int,"raw perk Type")
        require(raw["Type"]<=32 if status==0 else raw["Type"]==33,
                "unknown perk must remain held")
        if status!=0:
            held.append((profile,level,ordinal,raw))
    require(all(observed[k]==r[2] for k,r in level_ids.items()),"level perk coverage")
    require(held==[(499,3,0,{"Type":33,"Value":0.05})],"Type33 raw value hold")

def check_relations(doc,ui,local):
    require(len(doc["rows"])==797,"retained relation census")
    lookup={(ui["category_dictionary"][r[0]],r[1]):r for r in ui["rows"]}
    for category,ordinal,status,ids,icon_hash in doc["rows"]:
        require(0<=category<len(doc["category_codes"]) and
                0<=status<len(doc["status_codes"]),"qualification code")
        key=(doc["category_codes"][category],ordinal)
        require(key in lookup,"UI relation ancestry")
        require(icon_hash in {ui["proof_dictionary"][ref[1]][1] for ref in lookup[key][7]},
                "UI icon proof binding")
        require(len(set(ids))==len(ids),"candidate set duplicate")
    index=json.loads((HERE/"catalogue-index.json").read_text())
    require(len(index["rows"])==6470 and {r[0] for r in index["rows"]}==set(local),
            "consolidated index cohort")
    for name,pin in index["inputs"].items():
        blob=(HERE/name).read_bytes()
        require(len(blob)==pin["bytes"] and hashlib.sha256(blob).hexdigest()==pin["sha256"],
                "index input hash")

def check_legacy(doc):
    require(len(doc["rows"])==314 and len(doc["proof_dictionary"])==1052,
            "legacy scoped observation census")
    require([v["observations"] for v in doc["inputs"]]==[33,138,51,87,5],
            "legacy overlapping source scopes")
    observed=collections.Counter()
    references=0
    for row in doc["rows"]:
        require(len(row)==11 and row[4] is None,"legacy global ordinal must be unknown")
        observed[row[0]]+=1
        require("Description" not in row[6],"legacy private Description")
        for value in row[3].values():
            require(not isinstance(value,str) or value.startswith("legacy-capture:"),
                    "legacy opaque capture")
        for role,index in row[9]:
            require(type(index) is int and 0<=index<len(doc["proof_dictionary"]),
                    "legacy proof reference")
            references+=1
    require([observed[i] for i in range(5)]==[33,138,51,87,5],"legacy input distribution")
    require(references==1345,"legacy proof occurrence count")
    require(all(re.fullmatch(r"legacy-proof:\d+",r[0]) and SHA.fullmatch(r[1])
                for r in doc["proof_dictionary"]),"legacy opaque proof")

def validate():
    docs={}
    for name, expected in PINS.items():
        blob=(HERE/name).read_bytes()
        require(hashlib.sha256(blob).hexdigest()==expected, "payload SHA: "+name)
        docs[name]=json.loads(blob)
        privacy(docs[name])
    local=check_flags(docs["client6470-flags.json"])
    check_tags(docs["client6470-five-tags.json"],local)
    check_ui(docs["ui2623-observations.json"])
    check_legacy(docs["legacy-armors-amulets.json"])
    check_profiles(docs["proficiency-profiles.json"],docs["client6470-five-tags.json"])
    check_relations(docs["client-ui-relations.json"],docs["ui2623-observations.json"],local)
    for name in ("gaps.json","client-ui-relations.json","catalogue-index.json"):
        if (HERE/name).is_file():
            privacy(json.loads((HERE/name).read_text()))
    return docs,local

def self_test(docs,local):
    failures=0
    original=docs["client6470-five-tags.json"]
    cases=[]
    d=copy.deepcopy(original); d["rows"].pop(); cases.append(d)
    d=copy.deepcopy(original); d["rows"][0][4]=0; cases.append(d)
    d=copy.deepcopy(original); d["rows"][0][3]=1; cases.append(d)
    d=copy.deepcopy(original); d["rows"][0][2]="0"*64; cases.append(d)
    for d in cases:
        try: check_tags(d,local)
        except ValueError: failures+=1
    for value in [{"Description":"private"},{"x":"folder/proof.png"},{"x":"/home/mole/x"},
                  {"x":"Bearer private"}]:
        try: privacy(value)
        except ValueError: failures+=1
    require(failures==8,"negative privacy/provenance/presence checks")
    print("8 negative checks PASS")

if __name__=="__main__":
    docs,local=validate()
    if "--self-test" in sys.argv: self_test(docs,local)
    print("6470 local rows, 5 raw tags, 444 profiles/3680 perks, 2623 UI + 314 legacy observations PASS")
