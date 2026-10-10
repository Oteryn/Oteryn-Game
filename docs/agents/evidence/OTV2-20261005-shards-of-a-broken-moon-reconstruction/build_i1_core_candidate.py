"""Build Shards Item core candidates through the existing admission semantics.

Requires exact pinned Crystal XML in an external file. Never edits served content.
Candidate 54610 still requires its separate source-held identity successor admission.
"""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]
PIN = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
XML_BLOB = "5530bb76d896d31fb75d6cad23969e1cc463b1c5"
XML_SHA256 = "13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece"
IDS = [54262, 54610, 54638]


def build(donor_xml):
    sys.path.insert(0, str(REPO / "tools/content-schema/item-authoring"))
    sys.path.insert(0, str(REPO / "tools/content-migration"))
    from engine_items import load_appearance_objects
    from quest_reward_item_semantics import CLIENT, CLIENT_BYTES, CLIENT_SHA, item_facts, enrich

    client = (REPO / CLIENT).read_bytes()
    if len(client) != CLIENT_BYTES or hashlib.sha256(client).hexdigest() != CLIENT_SHA:
        raise ValueError("CLIENT_PIN")
    xml = donor_xml.read_bytes()
    blob = hashlib.sha1(b"blob " + str(len(xml)).encode() + b"\0" + xml).hexdigest()
    if blob != XML_BLOB or hashlib.sha256(xml).hexdigest() != XML_SHA256:
        raise ValueError("DONOR_XML_PIN")
    appearances = load_appearance_objects(client)
    rows = {}
    for row in ET.fromstring(xml).findall("item"):
        if row.get("id") in {str(item_id) for item_id in IDS}:
            item_id = int(row.get("id"))
            if item_id in rows:
                raise ValueError("DUPLICATE_DONOR_ROW")
            rows[item_id] = dict(item_id=item_id, name=row.get("name"), article=row.get("article"),
                                plural=row.get("plural"), attrs={a.get("key"): a.get("value")
                                for a in row.findall("attribute")})
    entries = {e["item_id"]: e for e in json.loads((ROOT / "item-successor-admission-candidate.json").read_bytes())["items"]}
    definitions, witnesses = [], []
    for item_id in IDS:
        entry = entries[item_id]
        if entry["xml"]["crystal-summer"] != rows[item_id]:
            raise ValueError("DONOR_ROW_MISMATCH")
        facts = item_facts(entry, appearances[item_id])
        if facts != entry["derived_item_facts"]:
            raise ValueError("DERIVED_FACTS_MISMATCH")
        template_id = 54262 if item_id == 54610 else item_id
        source_path = entries[template_id]["current_record"]["path"]
        definition = copy.deepcopy(next(r["definition"] for r in json.loads((REPO / source_path).read_bytes())["records"]
                                        if r["definition"]["identity"]["key"] == f"oteryn:item.tibia.i{template_id}"))
        if item_id == 54610:
            definition["identity"]["key"] = entry["item_key"]
            definition["semantics"] = {name: {"state": "UNKNOWN"} for name in definition["semantics"]}
            definition["semantics"]["physical"] = {"state": "KNOWN", "value": {
                "weight": {"state": "KNOWN", "value": int(rows[item_id]["attrs"]["weight"])}}}
        before = copy.deepcopy(definition)
        enrich(definition, facts, entry["xml"])
        if not definition["materializable"] or definition["stack_class"] != "NonStackable":
            raise ValueError("CORE_CANDIDATE_SHAPE")
        for group in ["temporal", "use_transform", "equipment"]:
            if definition["semantics"][group] != {"state": "UNKNOWN"}:
                raise ValueError("UNSOURCED_BEHAVIOR")
        definitions.append(dict(definition=definition))
        witnesses.append((entry, appearances[item_id], facts, before))
    result = dict(schema="OTERYN_SHARDS_I1_CORE_CANDIDATE/v1", status="EVIDENCE_ONLY_NOT_ADMITTED",
                  source_pin=PIN, client=dict(path=CLIENT, sha256=CLIENT_SHA, bytes=CLIENT_BYTES),
                  source_xml=dict(path="data/items/items.xml", git_blob=XML_BLOB, sha256=XML_SHA256),
                  records=definitions, identity_admission_54610="REQUIRED_BEFORE_APPLICATION",
                  runtime_use="NOT_QUALIFIED")
    return result, witnesses


def negative_checks(witnesses):
    from quest_reward_item_semantics import item_facts, enrich
    entry, appearance, facts, definition = witnesses[1]
    def refused(operation, code):
        try:
            operation()
        except ValueError as error:
            if str(error) != code:
                raise
        else:
            raise AssertionError("Expected refusal: " + code)
    mutant = copy.deepcopy(entry)
    mutant["client_name"] = "wrong name"
    refused(lambda: item_facts(mutant, appearance), "CLIENT_NAME_MISMATCH")
    mutant = copy.deepcopy(entry)
    mutant["xml"]["crystal-summer"]["item_id"] = 34017
    refused(lambda: item_facts(mutant, appearance), "XML_ID_MISMATCH")
    mutant, client = copy.deepcopy(entry), copy.deepcopy(appearance)
    mutant["client_flags"]["flags.take"] = False
    client["flags"]["flags.take"] = False
    refused(lambda: item_facts(mutant, client), "NON_PORTABLE")
    incompatible = copy.deepcopy(definition)
    incompatible["stack_class"] = "StackCapable"
    refused(lambda: enrich(incompatible, facts, entry["xml"]), "EXISTING_STACK_CLASS_CONFLICT")
    incompatible = copy.deepcopy(definition)
    incompatible["semantics"]["physical"]["value"]["pickupable"] = {"state": "KNOWN", "value": False}
    refused(lambda: enrich(incompatible, facts, entry["xml"]),
            "EXISTING_FACT_CONFLICT:oteryn:item.tibia.i54610:physical.pickupable")
    print("Five negative admission/semantic cases refused by the existing pipeline.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--donor-xml", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result, witnesses = build(args.donor_xml)
    negative_checks(witnesses)
    data = (json.dumps(result, indent=2) + "\n").encode()
    target = ROOT / "i1-item-definitions-candidate.json"
    if args.check:
        if target.read_bytes() != data:
            raise SystemExit("I1 core candidate drift")
    else:
        target.write_bytes(data)
    print("Three core candidates qualified; no identity admission or served-content mutation.")


if __name__ == "__main__":
    main()
