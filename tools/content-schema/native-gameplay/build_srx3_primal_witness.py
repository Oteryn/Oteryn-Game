#!/usr/bin/env python3
"""Generate the bounded W10 callback witness from canonical native source inputs.

No WorldProject is fabricated. The actual native loader's complete source digest
binds the bounded records to the full canonical project; exact typed equality at
runtime binds every consumed Creature/Behavior/Ability/Effect/Formula record.
Crystal callback bytes are read-only compatibility evidence, never executed.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import build_native_callback_witness as canonical_witness
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
CANARY = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
KEYS = ('fungosaurus', 'emerald_tortoise_primal', 'gore_horn_primal', 'gorerilla_primal', 'headpecker_primal', 'hulking_prehemoth_primal', 'mantosaurus_primal', 'nighthunter_primal', 'noxious_ripptor_primal', 'sabretooth_primal', 'stalking_stalk_primal', 'sulphider_primal', 'the_primal_menace')
CALLBACKS = {'data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua': '468add6e4f49914100f38068be11075c86a7baa566f31fb0b20066d855c3a973', 'data-global/scripts/systems/hazard_primal.lua': '395f109a693fdaa0391801c8c1aae701d0e21cfe1cf701ff6b5ec373063e75b3', 'data-global/lib/quests/the_primal_ordeal.lua': '18e68a281c1b1245dd1f01fd70fe2947c2ae5ac7011c39be686fe027bc4bda0a'}
REVIEWED_PROFILES = {'oteryn:creature.fungosaurus': '27e372cade8dd1c166cc13f7fc61f4a8ed4623b63f30ac4976a19744d3cf71a9', 'oteryn:creature.emerald_tortoise_primal': '48a315ebf9a08f77a0f6e7033c33729415066496b0cea0f55b8999632380fc5b', 'oteryn:creature.gore_horn_primal': '456475d3a9688505217a9991601c49b5cc2b0f18b616b86a19c023da1aa20590', 'oteryn:creature.gorerilla_primal': '6adc13dba672e514e976f5abdbfe822117f47974eeffe0035d2431455ae9a989', 'oteryn:creature.headpecker_primal': 'ebd5c783ca6e34a978f2e23319516909d982134d6e52c6c451d0147290c67fc6', 'oteryn:creature.hulking_prehemoth_primal': '48925e9bb3fdc1160540f437ec43df066171fbda3a03ada187784d9ed60bf6d5', 'oteryn:creature.mantosaurus_primal': 'd18235eba9484c5ae5e98df713ced9e82f2d2e7dd5b7ad5c7615caeb5993de76', 'oteryn:creature.nighthunter_primal': '7430bf6b8c348b0352e2ccf90ba9753593e0ae12ffcb7c3f85b7ab4bb67efaf8', 'oteryn:creature.noxious_ripptor_primal': '992517585fe7562cfefc6f9d9b6acda966e648b6a4c4d4949b6863afcbaff5d6', 'oteryn:creature.sabretooth_primal': 'b2d27e7bd853180de68719ffae6eef09b2ca761322585897374219e80dff1fef', 'oteryn:creature.stalking_stalk_primal': '8796d2e24a066f0e6dba0ade3e2170593b61fd397706062598417d41b412cdfc', 'oteryn:creature.sulphider_primal': '6dfb25b16a7a70c2ed0cd58bf80233e1456757cc0de31c4b5d5d97d0f33d2b0c', 'oteryn:creature.the_primal_menace': 'cbcea77726286580896d408e22e4d4328b0998afe1412b4f0847823c87fcf3a5'}
OUTPUT = ROOT / "tools/content-schema/native-gameplay/srx3-primal-witness.json"


def build(documents=None):
    source = json.loads((ROOT / "content/creatures/definitions/spell-native-profiles.json").read_bytes())
    digest = source["source_definitions_sha256"]
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        raise ValueError("native full source digest missing")
    records = []
    bindings = []
    for slug in KEYS:
        key = "oteryn:creature." + slug
        matches = [row for row in source["records"] if row["profile"]["target"]["key"] == key]
        if len(matches) != 1:
            raise ValueError("missing or duplicated native participant: " + key)
        record = matches[0]
        actual = hashlib.sha256(json.dumps(record, sort_keys=True, separators=(",", ":"),
                                           ensure_ascii=False).encode()).hexdigest()
        if actual != REVIEWED_PROFILES[key]:
            raise ValueError("native Primal Ordeal body changed; source qualification review required: " + key)
        if record["profile"]["target"]["revision"] != "definition-r1" or not record.get("behavior"):
            raise ValueError("changed native source participant: " + key)
        canonical = [row for path in (ROOT / "content/creatures/definitions").glob("creatures-*.json")
                     for row in json.loads(path.read_bytes())["records"]
                     if row["definition"]["identity"]["key"] == key]
        if len(canonical) != 1:
            raise ValueError("canonical participant absent or duplicated: " + key)
        exact = [binding for binding in canonical[0]["source_bindings"]
                 if binding["source_key"] == "oteryn:source.canary"
                 and binding["source_revision"] == CANARY
                 and binding["identity_namespace"] == "canary/monster-file"
                 and binding["disposition"] == "EXACT"
                 and binding["target"] == record["profile"]["target"]]
        if len(exact) != 1:
            raise ValueError("source binding changed: " + key)
        wanted = exact[0]
        bindings.append(wanted)
        records.append(record)
    documents = canonical_witness.read_documents(ROOT) if documents is None else documents
    if canonical_witness.source_digest(documents) != digest:
        raise ValueError("native source-definition digest does not match actual canonical root")
    profiles = {row["target"]["key"]: row for row in documents["declarations"]["authoring_profiles"]}
    required = {reference["key"]: reference for row in records
                for reference in row["profile"]["data"]["profile"].get("encounters", [])}
    if set(required) != {"oteryn:encounter.the_primal_menace"}:
        raise ValueError("reviewed Primal Encounter reference closure changed")
    encounters = []
    for key, target in sorted(required.items()):
        profile = profiles.get(key)
        if profile is None or profile["target"] != target or profile["data"]["kind"] != "Encounter":
            raise ValueError("actual canonical Primal Encounter missing")
        if profile["data"]["profile"]["scope"] != "Instance" or profile["data"]["profile"]["encounter_type"] != "Boss":
            raise ValueError("Primal Encounter source scope/type changed")
        encounters.append(profile)
    return {"schema": "OTERYN_SRX3_PRIMAL_CALLBACK_WITNESS/v1",
            "mechanics_source_revision": PIN, "source_definitions_sha256": digest,
            "source_bindings": bindings, "callbacks": CALLBACKS, "records": records,
            "encounter_profiles": encounters}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-root", type=Path)
    args = parser.parse_args()
    if args.source_root is not None:
        for path, expected in CALLBACKS.items():
            if hashlib.sha256((args.source_root / path).read_bytes()).hexdigest() != expected:
                raise ValueError("pinned callback changed: " + path)
    raw = (json.dumps(build(), sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("stale W10 source witness; run this generator")
    else:
        OUTPUT.write_bytes(raw)


if __name__ == "__main__":
    main()
