#!/usr/bin/env python3
"""Bind source-specific callback adapters to the actual pinned native definitions.

This is a bounded rule witness, not a ProjectV2Draft or an authority grant. Existing
source adapter specifications must still equal canonical authoring before a new
full-root digest is emitted. Runtime consumers also compare the active native
Creature, Behavior and parent Ability bodies and independently prove current scope.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
NATIVE = "content/creatures/definitions/spell-native-profiles.json"
OUTPUT = "crates/gameplay/content/src/content/native_monster_callback_witness.json"
SCHEMA = "OTERYN_NATIVE_MONSTER_CALLBACKS/v1"

# Game.createMonster uses the admitted current monster type. Fourteen defaults
# match pinned Canary47dfd51f global monster Lua; Lost Time and Thorn Minion retain
# the already accepted wiki/BR health receipts documented in
# docs/contracts/NATIVE_MONSTER_CALLBACK_SOURCE_V1.md. Root rebinding alone must
# never turn a new health rule into an approved callback spawn.
SPAWN_CHILD_HEALTH = {
    "corrupted_soul": 1100, "damned_soul": 800, "demon": 8200,
    "disgusting_ooze": 3650, "disruption": 12000, "energy_pulse": 5000,
    "frenzy": 8210, "glooth_generator": 12000, "greed": 1000,
    "lost_time": 4000, "minion_of_gaz_haragoth": 5500, "rage_of_mazoran": 4500,
    "shadow_fiend": 3200, "sin_devourer": 2700, "thorn_minion": 3600,
    "time_waster": 1000,
}


# Exact already-admitted native projections for the source event closure. These
# constants deliberately require review if body semantics change; a new canonical
# root alone must not silently bless a different Outburst event participant.
OUTBURST_EVENT_BODIES = {
    "outburst": ("249171c2bd7dbe88247c4e39e0f0a51408655662e0aa798c95a7319a99bd178b",
                 "2bcfed5fa053c8f42750169200faf29b358276b889836fed3898c892308b894e"),
    "spark_of_destruction": ("98274a47e53bdb6a3298dc1796f00f5041ab83bb5ad979dfd0459e37b5a0922b",
                             "fc910a65eb4c5b88f67d673c422e3cff0cce30f2ce11f570e8442eadcdc19e03"),
}
RELATED_EVENT_BODIES = {'frenzy': 'a701ef1f486ed97ea3c852dc0a273a6fa90314bb435d5ca1929ef207d98276bb',
 'disruption': 'be8121eefa83f708dd219e29026d9c4c24abca02936e95fa651f1e56f18a51aa',
 'charged_disruption': 'fc7951c606412085bdc1067dac22f60d742239f575a661fa043d0a7216d5a4bf',
 'overcharged_disruption': '28536acb6155efbee2de6d114b625de516e7eec24459235aae93d0e197f52b91',
 'greed': '3c037b421e87d39113c445e3c2ea2d93c140baa1fd9dcb9bc3de947a3aad1a39',
 'the_hunger': 'e061c09f8909595d2238e65a1027f34c31af0653eb19780e8b8ec55c9b3c5e07',
 'the_destruction': 'd53317b1db9a3ed04637309e49e8c2518af1a9850627ad2afd52062c099c30fd',
 'the_rage': 'd500c11f79f0a88c9fe0d57621b3988b3933ef63e53c7d07e6a6883ec432baf4',
 'damage_resonance': '0ba2300a6487909572dd68c01a397f97aed3a0b5009cbffa5664b7181c587f53',
 'world_devourer': '8d4b0f654943bf2a4b061af6748a6f00142489b6c949bd888db63fbb156c03da',
 'bone_capsule': '2b6943777e34a325f540ca19f4b35cbac91c8da955c7bb36eaf0f90d6a63ffcd',
 'anomaly': '0f36a51c9f3ca0c8bbf50a5c4aaf18b8529101aee6047bb81c56f43161fc6968',
 'rupture': 'f1ef28305514fe0541c570d37112b593dab73485efcb226476c9a65537adf6c0',
 'realityquake': 'fb63ab2e517016dee2571383c0c862760935013bd06c2f6e866a087cda99e78e',
 'eradicator': '3b4a23d253eeb6ccc09561a7ebcbb5724f94d74a70ede53fbdb1685c59a10eb6'}
EVENT_CONTEXT_BODIES = {'baby_dragon': '0e33419ba5c639038d911caf0114b32020a44df1150920eab99a577b6b3d80df',
 'dragonking_zyrtarch': '70d497b880bb5f73dea771268c613fe7916b88388166f2b5a746be6c7b2ad23f',
 'eradicator2': 'd000a0909f786b5f534fa87bf60575deaa99a4f621ef4b17d9ff42da817a016c',
 'possessed_tree': 'ebf709c51336e3e86a127314164e6bb796b8aa2a4228d7b83f42715ce602ebf9',
 'shadow_tentacle': '13af30920e799bfa0c1e8280c51b47d30155dab52241831d84284c44e2c17ee1',
 'soul_of_dragonking_zyrtarch': '32c9962c7700007b945b153d053901d669332d235a65290d418a6063dd94b00a',
 'the_enraged_thorn_knight': 'ba4c1c6befe855de45742be5a560c930297e19bea2ef99ef305ab4fe5eb420b1',
 'the_shielded_thorn_knight': 'a96af6ca2e313d01ee2a6aef4cff7b759704a3e1742ede277c989e032579a937',
 'thorn_steed': '04313fef00f83dd7c424783f2ed4543a7be1af8b4aa13f835dcc2e5736c6b588',
 'unbound_blightwalker': '391816f1cb44ac0476b1e5d77e111d78104e0aa6b91dcf3035d9f4798240ca87',
 'unbound_defiler': '2a9ce1ad4defbc3d88a93cf9b8c12f4127ece5d15192785cbb411f02950d9c95',
 'unbound_demon': '984c64cabfccd2939b8aef2344a5be0b5113f512d5a7629a1191bdc969d74e97',
 'unbound_demon_outcast': '840cde8edf5298eca51455c800cfe53f8274b6c7adf082828f980878210c4772'}
SOURCE_EVENTS = [
    {"name": "OutburstCharge", "repository": "opentibiabr/canary",
     "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
     "path": "data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_outburst_charge.lua",
     "sha256": "a8c4ba6e835176fa7d918d5c92fefed9f805d2e3bd2893a6811fec23740174fd"},
    {"name": "ChargingOutDeath", "repository": "opentibiabr/canary",
     "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
     "path": "data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_charging_out_death.lua",
     "sha256": "c91c47b3bb887d7fd55396fe9c9d68d4ee2287557ddac0b8df0a34e7b82391fa"},
]
SOURCE_EVENTS.extend([{'name': 'HeartMinionDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_heart_minion_death.lua',
  'sha256': '8b47831ca92d0d8ed2804c422994cf3529b90b6944cac997ec6171490a1e2de3'},
 {'name': 'DisruptionTransform',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_disruption_transform.lua',
  'sha256': '31f9c40a80d37eb29d861402e90aec076be8707426c51ca87e3910008133bb35'},
 {'name': 'ChargedDisruptionTransform',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_charged_disruption_transform.lua',
  'sha256': '38298c959ca72f0b2e22c3e38a46090846cb7974f2376c16820d0ce1f638d487'},
 {'name': 'HeartVortexHunger',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/movements_vortex_hunger.lua',
  'sha256': 'ac50a60c4041ae6b73285f75e18087895c5066c417bd8bfc44500c9894d0315b'},
 {'name': 'AnomalyVortexStep',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/movements_vortex_anomaly.lua',
  'sha256': '05f6da9012a82ac8589f8a45f74c72c1469b1a917d79711a006e839a6d94e275'},
 {'name': 'BoneCapsule',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_bone_capsule.lua',
  'sha256': 'b89ccd225bec2263b1396159eedf93fc84d10b713968633ed5d3f66acdca3c7f'},
 {'name': 'HeartBossDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_heart_boss_death.lua',
  'sha256': '37b640f71260ae4b5ca4f50c485a7cf61877a9717149e6a9d6fad6c6a702c3d6'}])


# Native12 controlled evidence enrichment. These attached source events share
# the existing callback witness; no synthetic attack/defence slot is created.
SOURCE_EVENTS.extend([
    {"name": "GazHaragothHeal", "repository": "opentibiabr/canary",
     "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
     "path": "data-otservbr-global/scripts/creaturescripts/monster/gaz_haragoth_heal.lua",
     "sha256": "ab715b9d27d98511d47707103860383766b579d71de567a6455aa0deabb3a12c"},
    {"name": "MinionGazDeath", "repository": "opentibiabr/canary",
     "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
     "path": "data-otservbr-global/scripts/creaturescripts/monster/minion_gaz_haragoth_vortex.lua",
     "sha256": "140e7db53054ac626d49e0b411395fe7b5e0a3d4bdb951de42be0853c77d6159"},
    {"name": "GazNightmareVortexStep", "repository": "opentibiabr/canary",
     "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
     "path": "data-otservbr-global/scripts/movements/roshamuul/strange_vortex_tp.lua",
     "sha256": "1e80473ce94e535c4aa56a5f784df35221fa354128b13406dd18d51e88e2c25e"},
])


SOURCE_EVENTS.extend([{'name': 'AnomalyTransform',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_anomaly_transform.lua',
  'sha256': '11645f1798c553d1a85ee8da0270a9c80b212efa57b18162cd6a46f6fb570b83'},
 {'name': 'AscendantBossesDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_bosses_kill.lua',
  'sha256': 'e0381b196597b2e936a0e763c69c547aac234af0d4b983854488a9c2ea6c9a7d'},
 {'name': 'ChargedAnomalyDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_charged_anomaly_death.lua',
  'sha256': 'f22081519337d45dd39b43ab9d31c9ef6de243e6bedb9e8b3bfe930e9a905758'},
 {'name': 'CorruptedSoul',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_corrupted_soul.lua',
  'sha256': '1e03253a5fdbe294e57b0a7066b7ec232dfac8191a7b62665be633d71421c0fa'},
 {'name': 'DisgustingOozeDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_disgusting_ooze_death.lua',
  'sha256': 'e6b56dc98370e669593e3bc98ef247b402883293401911ba9fc33fd9db853822'},
 {'name': 'EradicatorTransform',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_eradicator_transform.lua',
  'sha256': '2b441f012dccb8c8dc0f399b355e370c79035256c1945f0f27f9b073b1f0dced'},
 {'name': 'ForgottenKnowledgeBossDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_bosses_kill.lua',
  'sha256': 'a253bd0bd92d9acba483a5a181998f59a09d48aef91c50b20de2128661599907'},
 {'name': 'HealthForgotten',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_healthchange_forgotten.lua',
  'sha256': '3bf777975003746480e6fa7c1da5fc0b2351e61a7602c49076c9b1d44114cde7'},
 {'name': 'RathletonBossDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/hero_of_rathleton/creaturescripts_bosses_kill.lua',
  'sha256': '289f76f6149ece86b3c0f2fdbdb080e8a165bb2a9712ca8b81661d944b080831'},
 {'name': 'RuptureHeal',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_rupture_heal.lua',
  'sha256': '93a333558e110b20234d961a18f351b38ca9d15a24bf57a8c0d6ba9785b3dbe5'},
 {'name': 'RuptureResonance',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_rupture_resonance.lua',
  'sha256': '1b56eafa677fc7a5503214bbe5cb99e5a342b496c6f555efd225bfc8ab32b1ae'},
 {'name': 'SoulcatcherSummon',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_soulcatcher_summon.lua',
  'sha256': '4a88ea5b5b51a184f90ff45eb30fc8e0e3de3a2551984f1846c48e174a149e1d'},
 {'name': 'ThornKnightDeath',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_thorn_knight_death.lua',
  'sha256': '2c32df19ff50292362326b185a0cd16a8bfeaa4c2dc823e4131034c5ae36e919'},
 {'name': 'CorruptedSoulVortexStep',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/forgotten_knowledge/movements_dragonking_vortex.lua',
  'sha256': '77fd63a435fb71f7f717da088f3c4a07db321be62480ad7660f7ac479be86a78'},
 {'name': 'PossessedTree',
  'repository': 'opentibiabr/canary',
  'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
  'path': 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_possessed_tree.lua',
  'sha256': '094afc3b236833eca3017cf097c0658b07c146c7da991e17217d665e96f4c570'}])

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode()


def read_documents(root):
    paths = {"reference": "content/world/definitions/reference.json",
             "declarations": "content/world/definitions/declarations.json",
             "provenance": "content/world/provenance/sources.json"}
    return {name: json.loads((root / path).read_bytes()) for name, path in paths.items()}


def source_digest(documents):
    reference, declarations, provenance = (documents[name] for name in
                                          ("reference", "declarations", "provenance"))
    value = {"schema": "OTERYN_NATIVE_MONSTER_DEFINITIONS/v1",
             "records": reference["records"],
             "authoring_profiles": declarations["authoring_profiles"],
             "declarations": declarations["records"],
             "sources": provenance["sources"],
             "source_identity_bindings": provenance["source_identity_bindings"]}
    return hashlib.sha256(canonical(value)).hexdigest()


def build(root=ROOT, documents=None, native=None):
    documents = read_documents(root) if documents is None else documents
    native = json.loads((root / NATIVE).read_bytes()) if native is None else native
    declarations = documents["declarations"]
    profiles = {row["target"]["key"]: row for row in declarations["authoring_profiles"]}
    if len(profiles) != len(declarations["authoring_profiles"]):
        raise ValueError("duplicate canonical callback profile identity")
    # These are reviewed executable source specifications, not newly inferred
    # semantics. A changed root digest alone cannot author another callback rule.
    expected_data, reviewed_entries = {}, {}
    for name in ("source_encounter_seven_fixture.json", "source_callback_spawn_fixture.json",
                 "source_callback_cast_fixture.json", "smelly_encounter_source.json"):
        source = json.loads((root / "apps/game-server/src" / name).read_bytes())
        cases = source.get("cases", [])
        if isinstance(cases, dict):
            cases = cases.values()
        for case in cases:
            if "entry" in case:
                reviewed_entries[(case["caster"], case["phase"], case["index"])] = case["entry"]
        rows = source["profiles"]
        if isinstance(rows, list):
            rows = {row["target"]["key"]: row["data"] for row in rows}
        for key, value in rows.items():
            if key in expected_data and expected_data[key] != value:
                raise ValueError("conflicting callback source specifications: " + key)
            expected_data[key] = value
    threshold = root / ("docs/agents/evidence/monster-full-mechanics-20261004/lanes/"
                        "encounters/callback18/threshold-heal-expected-encounters.json")
    for key, value in json.loads(threshold.read_bytes()).items():
        if key in expected_data and expected_data[key] != value:
            raise ValueError("conflicting threshold callback source specifications: " + key)
        expected_data[key] = value
    for key, value in expected_data.items():
        if profiles.get(key, {}).get("data") != value:
            raise ValueError("source rule differs from reviewed callback specification: " + key)
    digest = source_digest(documents)
    if native.get("source_definitions_sha256") != digest:
        raise ValueError("native source-definition digest does not match actual canonical root")
    selected, entries, targets = {}, [], set()
    for record in native["records"]:
        creature = record["profile"]["target"]
        behavior = record.get("behavior")
        if behavior is None:
            continue
        definitions = record.get("attack_definitions", {})
        active = {row["target"]["key"]: row for row in definitions.get("profiles", [])}
        for phase in ("attacks", "defenses"):
            for index, entry in enumerate(behavior["data"]["profile"].get(phase, [])):
                ability = active.get(entry["ability"]["key"])
                details = (ability or {}).get("data", {}).get("profile", {}).get("details", {})
                encounter = details.get("encounter")
                if encounter is None:
                    continue
                # The accepted canonical union preserves active native profiles,
                # including unrelated movement differences (e.g. Hunger push_items).
                # That is not permission to change this callback's schedule/body.
                source_behavior = profiles.get(behavior["target"]["key"], {}).get("data", {}).get("profile", {})
                source_entries = source_behavior.get(phase, [])
                if index >= len(source_entries) or source_entries[index] != entry:
                    raise ValueError("native callback body differs from actual canonical source: "
                                     + behavior["target"]["key"])
                if profiles.get(ability["target"]["key"]) != ability:
                    raise ValueError("native callback body differs from actual canonical source: "
                                     + ability["target"]["key"])
                if encounter["key"] not in expected_data:
                    raise ValueError("callback has no reviewed executable source specification: "
                                     + encounter["key"])
                reviewed = reviewed_entries.get((creature["key"], phase, index))
                if reviewed is not None and reviewed != entry:
                    raise ValueError("source rule differs from reviewed callback schedule: "
                                     + ability["target"]["key"])
                # Native constructors consume the actual reviewed Encounter body,
                # not a fabricated entry-room Draft or a separately mutable fixture.
                encounter_profile = profiles.get(encounter["key"])
                if encounter_profile is None or encounter_profile["target"] != encounter:
                    raise ValueError("missing actual canonical callback Encounter")
                for row in (record["profile"], behavior, ability, encounter_profile):
                    key = row["target"]["key"]
                    if key in selected and selected[key] != row:
                        raise ValueError("conflicting native callback profile: " + key)
                    selected[key] = row
                targets.update((creature["key"], encounter["key"]))
                entries.append({"creature": creature, "phase": phase, "index": index,
                                "ability": entry["ability"], "encounter": encounter})
    spawn = json.loads((root / "apps/game-server/src/source_callback_spawn_fixture.json").read_bytes())
    child_keys = {"oteryn:creature." + name for name in SPAWN_CHILD_HEALTH}
    if set(spawn["targets"]) != child_keys:
        raise ValueError("spawn child source closure differs from reviewed callback targets")
    for key in sorted(child_keys):
        records = [row for row in native["records"] if row["profile"]["target"]["key"] == key]
        if len(records) != 1:
            raise ValueError("spawn child native body is missing or ambiguous: " + key)
        actual = records[0]["profile"]
        canonical_child = profiles.get(key, {})
        health = SPAWN_CHILD_HEALTH[key.removeprefix("oteryn:creature.")]
        if (actual["target"] != canonical_child.get("target")
                or actual["data"].get("kind") != "Creature"
                or actual["data"].get("profile", {}).get("health") != health
                or canonical_child.get("data", {}).get("profile", {}).get("health") != health):
            raise ValueError("spawn child health differs from donor/accepted rule: " + key)
        if key in selected and selected[key] != actual:
            raise ValueError("spawn child body conflicts with callback caster: " + key)
        selected[key] = actual
        targets.add(key)
    for name, expected in OUTBURST_EVENT_BODIES.items():
        key = "oteryn:creature." + name
        records = [row for row in native["records"] if row["profile"]["target"]["key"] == key]
        if len(records) != 1:
            raise ValueError("Outburst event body differs: missing or ambiguous " + key)
        record = records[0]
        actual = tuple(hashlib.sha256(canonical(record[part])).hexdigest()
                       for part in ("profile", "behavior"))
        if actual != expected:
            raise ValueError("Outburst event body differs from reviewed native source: " + key)
        rows = [record["profile"], record["behavior"]]
        rows.extend(record.get("attack_definitions", {}).get("profiles", []))
        for row in rows:
            target = row["target"]["key"]
            if target in selected and selected[target] != row:
                raise ValueError("Outburst event body differs: conflicting " + target)
            if row["data"]["kind"] != "Creature" and profiles.get(target) != row:
                raise ValueError("Outburst event body differs from actual canonical source: " + target)
            selected[target] = row
        targets.add(key)
    for name, expected in RELATED_EVENT_BODIES.items():
        key = "oteryn:creature." + name
        records = [row for row in native["records"] if row["profile"]["target"]["key"] == key]
        if len(records) != 1 or hashlib.sha256(canonical(records[0])).hexdigest() != expected:
            raise ValueError("event participant body changed: " + key)
        record = records[0]
        rows = [record["profile"], record["behavior"]]
        rows.extend(record.get("attack_definitions", {}).get("profiles", []))
        for row in rows:
            target = row["target"]["key"]
            if target in selected and selected[target] != row:
                raise ValueError("event participant body changed: conflicting " + target)
            # Preserve the reviewed admitted projection exactly. In particular,
            # native The Hunger push_items differs from canonical authoring; the
            # full-record hash above prevents silently blessing another rule.
            selected[target] = row
        targets.add(key)
    # These source-event children need their complete immutable native body,
    # not every unrelated spell flattened into the callback constructor slice.
    event_records = []
    for name, expected in EVENT_CONTEXT_BODIES.items():
        key = "oteryn:creature." + name
        records = [row for row in native["records"] if row["profile"]["target"]["key"] == key]
        if len(records) != 1 or hashlib.sha256(canonical(records[0])).hexdigest() != expected:
            raise ValueError("source event child native body changed: " + key)
        record = records[0]
        profile = record["profile"]
        if key in selected and selected[key] != profile:
            raise ValueError("source event child conflicts with existing callback body: " + key)
        selected[key] = profile
        targets.add(key)
        event_records.append(record)
    entries.sort(key=lambda row: (row["creature"]["key"], row["phase"], row["index"]))
    bindings = [row for row in documents["provenance"]["source_identity_bindings"]
                if row["target"]["key"] in targets]
    if targets != {row["target"]["key"] for row in bindings}:
        raise ValueError("native callback source binding closure is incomplete")
    sources = {(row["key"], row["revision"]) for row in documents["provenance"]["sources"]}
    if any((row["source_key"], row["source_revision"]) not in sources for row in bindings):
        raise ValueError("native callback source binding has no actual source")
    return {"schema": SCHEMA, "source_definitions_sha256": digest,
            "profiles": [selected[key] for key in sorted(selected)], "entries": entries,
            "source_events": SOURCE_EVENTS,
            "event_creature_records": sorted(event_records,key=lambda row:row["profile"]["target"]["key"]),
            "source_identity_bindings": sorted(bindings, key=lambda row:
                                                (row["target"]["key"], row["identity_namespace"],
                                                 row["external_id"]))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = (json.dumps(build(args.root), ensure_ascii=False,
                         sort_keys=True, indent=2) + "\n").encode()
    destination = args.root / OUTPUT
    legacy = args.root / "crates/gameplay/encounters/src/bosses/source_callbacks/native_callback_witness.json"
    if args.check:
        if legacy.exists():
            raise SystemExit("obsolete duplicate native callback witness; run build_native_callback_witness.py")
        if not destination.is_file() or destination.read_bytes() != output:
            raise SystemExit("stale native callback witness; run build_native_callback_witness.py")
    else:
        destination.write_bytes(output)
        legacy.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
