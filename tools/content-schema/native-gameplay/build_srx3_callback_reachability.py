#!/usr/bin/env python3
"""Reproduce the W01 callback/body census; unavailable is not a runtime verdict.

Pinned source evidence is intentionally frozen. --source-root verifies it against
an independently fetched Canary checkout; no network or installation is needed.
Companion offense remains W01-relevant; lifecycle coverage alone is not offense evidence.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path("docs/agents/evidence/srx3-monster-callback-reachability.json")
PINNED_SOURCES = {'canary:creature/druid_familiar': {'git_blob': '2c4f3a55ab2e8e2383da2e82c9c2a6bfa49fedb5',
                                    'path': 'data-otservbr-global/monster/familiars/druid_familiar.lua',
                                    'repository': 'opentibiabr/canary',
                                    'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                                    'schedule_evidence': [{'line': 68, 'text': 'monster.attacks = {'},
                                                          {'line': 69,
                                                           'text': '\t{ name = "melee", interval = '
                                                                   '2000, chance = 100, minDamage = 0, '
                                                                   'maxDamage = -300 },'},
                                                          {'line': 70,
                                                           'text': '\t{ name = "combat", interval = '
                                                                   '2000, chance = 20, type = '
                                                                   'COMBAT_EARTHDAMAGE, minDamage = '
                                                                   '-230, maxDamage = -260, range = 5, '
                                                                   'shootEffect = CONST_ANI_EARTH, '
                                                                   'effect = CONST_ME_STONES, target = '
                                                                   'true },'},
                                                          {'line': 71,
                                                           'text': '\t{ name = "combat", interval = '
                                                                   '2000, chance = 25, type = '
                                                                   'COMBAT_EARTHDAMAGE, minDamage = '
                                                                   '-90, maxDamage = -150, length = 2, '
                                                                   'spread = 0, effect = '
                                                                   'CONST_ME_GROUNDSHAKER, target = '
                                                                   'false },'},
                                                          {'line': 72,
                                                           'text': '\t{ name = "summon challenge", '
                                                                   'interval = 2000, chance = 40, '
                                                                   'target = false },'},
                                                          {'line': 73, 'text': '}'},
                                                          {'line': 75, 'text': 'monster.defenses = {'},
                                                          {'line': 76, 'text': '\tdefense = 55,'},
                                                          {'line': 77, 'text': '\tarmor = 55,'},
                                                          {'line': 78,
                                                           'text': '\t--\tmitigation = ???,'},
                                                          {'line': 79,
                                                           'text': '\t{ name = "combat", interval = '
                                                                   '2000, chance = 75, type = '
                                                                   'COMBAT_HEALING, minDamage = 600, '
                                                                   'maxDamage = 600, effect = '
                                                                   'CONST_ME_MAGIC_GREEN, target = '
                                                                   'false },'},
                                                          {'line': 80, 'text': '}'}],
                                    'sha256': 'cca0a4d6cb4c07b6ddad7edd7151b0a795d30b389ed41953af6115036ead5016'},
 'canary:creature/knight_familiar': {'git_blob': '83c619695910876e8dbb1f24287fa755b5d9d0c8',
                                     'path': 'data-otservbr-global/monster/familiars/knight_familiar.lua',
                                     'repository': 'opentibiabr/canary',
                                     'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                                     'schedule_evidence': [{'line': 68, 'text': 'monster.attacks = {'},
                                                           {'line': 69,
                                                            'text': '\t{ name = "melee", interval = '
                                                                    '2000, chance = 100, minDamage = 0, '
                                                                    'maxDamage = -200 },'},
                                                           {'line': 70,
                                                            'text': '\t{ name = "sudden death rune", '
                                                                    'interval = 2000, chance = 17, '
                                                                    'minDamage = -300, maxDamage = '
                                                                    '-350, range = 7, target = false '
                                                                    '},'},
                                                           {'line': 71,
                                                            'text': '\t{ name = "combat", interval = '
                                                                    '2000, chance = 15, type = '
                                                                    'COMBAT_DEATHDAMAGE, minDamage = '
                                                                    '-200, maxDamage = -250, range = 6, '
                                                                    'radius = 2, effect = '
                                                                    'CONST_ME_MORTAREA, target = true '
                                                                    '},'},
                                                           {'line': 72,
                                                            'text': '\t{ name = "combat", interval = '
                                                                    '3000, chance = 25, type = '
                                                                    'COMBAT_DEATHDAMAGE, minDamage = '
                                                                    '-180, maxDamage = -250, range = 5, '
                                                                    'radius = 3, effect = '
                                                                    'CONST_ME_MORTAREA, target = true '
                                                                    '},'},
                                                           {'line': 73,
                                                            'text': '\t{ name = "combat", interval = '
                                                                    '3000, chance = 25, type = '
                                                                    'COMBAT_ICEDAMAGE, minDamage = '
                                                                    '-180, maxDamage = -250, range = 5, '
                                                                    'radius = 3, effect = '
                                                                    'CONST_ME_ICEAREA, target = true '
                                                                    '},'},
                                                           {'line': 74,
                                                            'text': '\t{ name = "combat", interval = '
                                                                    '2000, chance = 15, type = '
                                                                    'COMBAT_ICEDAMAGE, minDamage = '
                                                                    '-200, maxDamage = -250, range = 6, '
                                                                    'radius = 2, effect = '
                                                                    'CONST_ME_ICETORNADO, target = true '
                                                                    '},'},
                                                           {'line': 75,
                                                            'text': '\t{ name = "ice strike", interval '
                                                                    '= 2000, chance = 17, minDamage = '
                                                                    '-300, maxDamage = -350, range = 5, '
                                                                    'target = true },'},
                                                           {'line': 76, 'text': '}'},
                                                           {'line': 78, 'text': 'monster.defenses = {'},
                                                           {'line': 79, 'text': '\tdefense = 55,'},
                                                           {'line': 80, 'text': '\tarmor = 55,'},
                                                           {'line': 81,
                                                            'text': '\t--\tmitigation = ???,'},
                                                           {'line': 82,
                                                            'text': '\t{ name = "combat", interval = '
                                                                    '2000, chance = 75, type = '
                                                                    'COMBAT_HEALING, minDamage = 300, '
                                                                    'maxDamage = 300, effect = '
                                                                    'CONST_ME_MAGIC_GREEN, target = '
                                                                    'false },'},
                                                           {'line': 83, 'text': '}'}],
                                     'sha256': '0a7630ef4ad7b7e124d28c3a0e5ee6815e647a199d658f98bce2526a026ca527'},
 'canary:creature/monk_familiar': {'git_blob': '93a6e898ffa1fc62e921e9c4d7ed5b23c7f90c17',
                                   'path': 'data-otservbr-global/monster/familiars/monk_familiar.lua',
                                   'repository': 'opentibiabr/canary',
                                   'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                                   'schedule_evidence': [{'line': 68, 'text': 'monster.attacks = {'},
                                                         {'line': 69,
                                                          'text': '\t{ name = "melee", interval = 2000, '
                                                                  'chance = 100, minDamage = 0, '
                                                                  'maxDamage = -200 },'},
                                                         {'line': 70,
                                                          'text': '\t{ name = "combat", interval = '
                                                                  '2000, chance = 18, type = '
                                                                  'COMBAT_HOLYDAMAGE, minDamage = -200, '
                                                                  'maxDamage = -270, range = 5, radius '
                                                                  '= 3, shootEffect = CONST_ANI_HOLY, '
                                                                  'effect = CONST_ME_HOLYAREA, target = '
                                                                  'true },'},
                                                         {'line': 71,
                                                          'text': '\t{ name = "combat", interval = '
                                                                  '2000, chance = 18, type = '
                                                                  'COMBAT_FIREDAMAGE, minDamage = -200, '
                                                                  'maxDamage = -270, range = 5, radius '
                                                                  '= 3, shootEffect = CONST_ANI_FIRE, '
                                                                  'effect = CONST_ME_FIREATTACK, target '
                                                                  '= true },'},
                                                         {'line': 72,
                                                          'text': '\t{ name = "combat", interval = '
                                                                  '2000, chance = 18, type = '
                                                                  'COMBAT_FIREDAMAGE, minDamage = -170, '
                                                                  'maxDamage = -230, range = 5, radius '
                                                                  '= 5, shootEffect = CONST_ANI_FIRE, '
                                                                  'effect = CONST_ME_FIREATTACK, target '
                                                                  '= true },'},
                                                         {'line': 73,
                                                          'text': '\t{ name = "combat", interval = '
                                                                  '2000, chance = 18, type = '
                                                                  'COMBAT_HOLYDAMAGE, minDamage = -170, '
                                                                  'maxDamage = -230, range = 5, radius '
                                                                  '= 5, shootEffect = CONST_ANI_HOLY, '
                                                                  'effect = CONST_ME_HOLYAREA, target = '
                                                                  'true },'},
                                                         {'line': 74, 'text': '}'},
                                                         {'line': 76, 'text': 'monster.defenses = {'},
                                                         {'line': 77, 'text': '\tdefense = 55,'},
                                                         {'line': 78, 'text': '\tarmor = 55,'},
                                                         {'line': 79,
                                                          'text': '\t{ name = "combat", interval = '
                                                                  '2000, chance = 75, type = '
                                                                  'COMBAT_HEALING, minDamage = 450, '
                                                                  'maxDamage = 450, effect = '
                                                                  'CONST_ME_MAGIC_GREEN, target = false '
                                                                  '},'},
                                                         {'line': 80, 'text': '}'}],
                                   'sha256': '2bd4dab7451a89f91d9f6853189a1ddc3185eabe80e58a1e8e48a5c9ac16034c'},
 'canary:creature/paladin_familiar': {'git_blob': '181e96f394c48c4dd1f6675252cbe62b93392361',
                                      'path': 'data-otservbr-global/monster/familiars/paladin_familiar.lua',
                                      'repository': 'opentibiabr/canary',
                                      'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                                      'schedule_evidence': [{'line': 68, 'text': 'monster.attacks = {'},
                                                            {'line': 69,
                                                             'text': '\t{ name = "melee", interval = '
                                                                     '2000, chance = 100, minDamage = '
                                                                     '0, maxDamage = -200 },'},
                                                            {'line': 70,
                                                             'text': '\t{ name = "combat", interval = '
                                                                     '2000, chance = 18, type = '
                                                                     'COMBAT_HOLYDAMAGE, minDamage = '
                                                                     '-200, maxDamage = -270, range = '
                                                                     '5, radius = 3, shootEffect = '
                                                                     'CONST_ANI_HOLY, effect = '
                                                                     'CONST_ME_HOLYAREA, target = true '
                                                                     '},'},
                                                            {'line': 71,
                                                             'text': '\t{ name = "combat", interval = '
                                                                     '2000, chance = 18, type = '
                                                                     'COMBAT_FIREDAMAGE, minDamage = '
                                                                     '-200, maxDamage = -270, range = '
                                                                     '5, radius = 3, shootEffect = '
                                                                     'CONST_ANI_FIRE, effect = '
                                                                     'CONST_ME_FIREATTACK, target = '
                                                                     'true },'},
                                                            {'line': 72,
                                                             'text': '\t{ name = "combat", interval = '
                                                                     '2000, chance = 18, type = '
                                                                     'COMBAT_FIREDAMAGE, minDamage = '
                                                                     '-170, maxDamage = -230, range = '
                                                                     '5, radius = 5, shootEffect = '
                                                                     'CONST_ANI_FIRE, effect = '
                                                                     'CONST_ME_FIREATTACK, target = '
                                                                     'true },'},
                                                            {'line': 73,
                                                             'text': '\t{ name = "combat", interval = '
                                                                     '2000, chance = 18, type = '
                                                                     'COMBAT_HOLYDAMAGE, minDamage = '
                                                                     '-170, maxDamage = -230, range = '
                                                                     '5, radius = 5, shootEffect = '
                                                                     'CONST_ANI_HOLY, effect = '
                                                                     'CONST_ME_HOLYAREA, target = true '
                                                                     '},'},
                                                            {'line': 74, 'text': '}'},
                                                            {'line': 76, 'text': 'monster.defenses = {'},
                                                            {'line': 77, 'text': '\tdefense = 55,'},
                                                            {'line': 78, 'text': '\tarmor = 55,'},
                                                            {'line': 79,
                                                             'text': '\t--\tmitigation = ???,'},
                                                            {'line': 80,
                                                             'text': '\t{ name = "combat", interval = '
                                                                     '2000, chance = 75, type = '
                                                                     'COMBAT_HEALING, minDamage = 450, '
                                                                     'maxDamage = 450, effect = '
                                                                     'CONST_ME_MAGIC_GREEN, target = '
                                                                     'false },'},
                                                            {'line': 81, 'text': '}'}],
                                      'sha256': 'e8674b2495283c6314a248d1e7389e688d3e8e15d242373a9d72407766bfcd84'},
 'canary:creature/rat': {'git_blob': '205888f9a55b66f6e05fbd2dc653b33d401ffcd1',
                         'path': 'data-otservbr-global/monster/mammals/rat.lua',
                         'repository': 'opentibiabr/canary',
                         'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                         'schedule_evidence': [{'line': 85, 'text': 'monster.attacks = {'},
                                               {'line': 86,
                                                'text': '\t{ name = "melee", interval = 2000, chance = '
                                                        '100, minDamage = 0, maxDamage = -8 },'},
                                               {'line': 87, 'text': '}'},
                                               {'line': 89, 'text': 'monster.defenses = {'},
                                               {'line': 90, 'text': '\tdefense = 5,'},
                                               {'line': 91, 'text': '\tarmor = 1,'},
                                               {'line': 92, 'text': '\tmitigation = 0.07,'},
                                               {'line': 93, 'text': '}'}],
                         'sha256': 'd4bfd4bc61f28e8b22b218b7cb36f5d715d72b79cb9cb7e667a052a3792a2f37'},
 'canary:creature/skeleton': {'git_blob': '22b05bff44dddfdd71e743fa7ecf7884cc1b6b01',
                              'path': 'data-otservbr-global/monster/undeads/skeleton.lua',
                              'repository': 'opentibiabr/canary',
                              'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                              'schedule_evidence': [{'line': 91, 'text': 'monster.attacks = {'},
                                                    {'line': 92,
                                                     'text': '\t{ name = "melee", interval = 2000, '
                                                             'chance = 100, minDamage = 0, maxDamage = '
                                                             '-20 },'},
                                                    {'line': 93,
                                                     'text': '\t{ name = "combat", interval = 2000, '
                                                             'chance = 15, type = COMBAT_LIFEDRAIN, '
                                                             'minDamage = -7, maxDamage = -13, range = '
                                                             '1, target = false },'},
                                                    {'line': 94, 'text': '}'},
                                                    {'line': 96, 'text': 'monster.defenses = {'},
                                                    {'line': 97, 'text': '\tdefense = 10,'},
                                                    {'line': 98, 'text': '\tarmor = 2,'},
                                                    {'line': 99, 'text': '\tmitigation = 0.23,'},
                                                    {'line': 100, 'text': '}'}],
                              'sha256': 'dd9df80a1a5f392996e23ca78a6227562ce0fb960cf3632c97544117f2881ff1'},
 'canary:creature/sorcerer_familiar': {'git_blob': '31d61dfbf280cee1f30a0758f99828f06bf61f63',
                                       'path': 'data-otservbr-global/monster/familiars/sorcerer_familiar.lua',
                                       'repository': 'opentibiabr/canary',
                                       'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                                       'schedule_evidence': [{'line': 68, 'text': 'monster.attacks = {'},
                                                             {'line': 69,
                                                              'text': '\t{ name = "melee", interval = '
                                                                      '2000, chance = 100, minDamage = '
                                                                      '0, maxDamage = -280 },'},
                                                             {'line': 70,
                                                              'text': '\t{ name = "combat", interval = '
                                                                      '2000, chance = 25, type = '
                                                                      'COMBAT_LIFEDRAIN, minDamage = '
                                                                      '-90, maxDamage = -150, length = '
                                                                      '2, spread = 0, effect = '
                                                                      'CONST_ME_MAGIC_RED, target = '
                                                                      'false },'},
                                                             {'line': 71,
                                                              'text': '\t{ name = "combat", interval = '
                                                                      '2000, chance = 25, type = '
                                                                      'COMBAT_ENERGYDAMAGE, minDamage = '
                                                                      '-190, maxDamage = -210, length = '
                                                                      '2, spread = 0, effect = '
                                                                      'CONST_ME_ENERGYHIT, target = '
                                                                      'false },'},
                                                             {'line': 72,
                                                              'text': '\t{ name = "summon challenge", '
                                                                      'interval = 2000, chance = 40, '
                                                                      'target = false },'},
                                                             {'line': 73, 'text': '}'},
                                                             {'line': 75,
                                                              'text': 'monster.defenses = {'},
                                                             {'line': 76, 'text': '\tdefense = 55,'},
                                                             {'line': 77, 'text': '\tarmor = 55,'},
                                                             {'line': 78,
                                                              'text': '\t{ name = "combat", interval = '
                                                                      '2000, chance = 75, type = '
                                                                      'COMBAT_HEALING, minDamage = 600, '
                                                                      'maxDamage = 600, effect = '
                                                                      'CONST_ME_MAGIC_GREEN, target = '
                                                                      'false },'},
                                                             {'line': 79, 'text': '}'}],
                                       'sha256': 'c9bbdc224311d112fbe0f9bbce9bbbf405ad2795a7cc90026e87e02d708b6aa7'}}

EXPECTED_MISSING = {"canary:creature/rat": 1, "canary:creature/skeleton": 2,
    "canary:creature/druid_familiar": 5, "canary:creature/knight_familiar": 8,
    "canary:creature/monk_familiar": 6, "canary:creature/paladin_familiar": 6,
    "canary:creature/sorcerer_familiar": 5}


def code_reference(path, anchor, following=12):
    raw = (ROOT / path).read_bytes()
    lines = raw.decode().splitlines()
    found = [i for i, line in enumerate(lines) if anchor in line]
    if not found:
        raise ValueError("missing reviewed source caller anchor: " + path + ":" + anchor)
    start = found[0]
    return {"path": path, "file_sha256": hashlib.sha256(raw).hexdigest(),
            "lines": [{"line": i + 1, "text": lines[i]}
                      for i in range(start, min(start + following, len(lines)))]}


def runtime_role_trace(native):
    refs = {
        "compiled_familiar_flag": code_reference("crates/gameplay/content/src/content/native_gameplay.rs", "is_familiar: details.summoning.is_familiar", 3),
        "actual_player_master_at_birth": code_reference("crates/gameplay/entities/src/carrier/runtime_actor_companion.rs", "master: Some(CompanionMaster", 6),
        "familiar_acquisition": code_reference("crates/gameplay/spell/src/spell/companion_lifecycle.rs", 'let p = parameters(spell, "familiar_summon")?', 12),
        "familiar_physical_birth": code_reference("crates/gameplay/spell/src/spell/companion_lifecycle.rs", "Some(runtime.prepare_companion_spawn(", 13),
        "animate_dead_acquisition": code_reference("crates/gameplay/dispatch/src/gameplay_transport/native_companion_item_cast.rs", "let spawn = crate::spell::companion_lifecycle::prepare_animate_dead(", 16),
        "skeleton_exact_owner_birth": code_reference("crates/gameplay/spell/src/spell/companion_lifecycle.rs", "pub fn prepare_animate_dead(", 100),
        "current_role": code_reference("crates/gameplay/monster-ai/src/monster_summon.rs", "pub fn current_source_creature_role(", 34),
        "ordinary_ai_excludes_companions": code_reference("crates/gameplay/monster-ai/src/ai_think/native_profile.rs", "pub fn admit_native_creature(", 23),
        "ordinary_melee_excludes_companions": code_reference("crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs", "fn qualified_melee<'a>(", 19),
        "builtin_melee_fallback": code_reference("crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs", "let legacy = record.monster_melee.as_ref().filter", 34),
        "rat_fixture_not_production": code_reference("apps/game-server/src/qualification_wild_spawn.rs", "//! Explicit Game-owned test spawn", 16),
        "familiar_defense_caller": code_reference("crates/gameplay/dispatch/src/gameplay_transport/composed_admission/vitals.rs", "self.tick_familiar_defenses(", 2),
        "familiar_defense_owner": code_reference("crates/gameplay/dispatch/src/gameplay_transport/familiar_defense.rs", "let companions=runtime.owned_companions", 11),
        "master_target_store": code_reference("crates/gameplay/dispatch/src/gameplay_transport/attack.rs", "pub struct ChannelAttackStates", 8),
        "master_target_read": code_reference("crates/gameplay/dispatch/src/gameplay_transport/attack.rs", "let previous = entry.state.target();", 12),
        "ordinary_target_role_guard": code_reference("crates/gameplay/dispatch/src/gameplay_transport/attack.rs", "snapshot.state.master.is_none() && !snapshot.state.policy.is_familiar", 4),
    }
    records = {r["profile"]["target"]["key"]: r for r in native["records"]}
    roles = {}
    for key in EXPECTED_MISSING:
        profile = records[key]["profile"]
        roles[key] = {"identity": profile["target"], "summoning": profile["data"]["profile"]["details"]["summoning"],
                      "monster_melee_fallback": records[key].get("monster_melee"),
                      "ordinary_role_does_not_prove_familiar_offense": True}
    return {"code_references": refs, "actual_profile_roles": roles,
            "other_branches_checked": [
                {"branch": "claude/happy-davinci-ijz3wy-srvpkg4", "head": "5e60c197", "finding": "no companion/familiar offensive controller or HP producer found; matching attack.rs/shared_experience.rs lines classify target and harm"},
                {"branch": "claude/happy-davinci-ijz3wy-questpc", "head": "6c09e0365110db6683f938dd08d5d584744cbf37", "finding": "same read-only offensive caller search: none found; unpublished owner-PC work cannot be inferred"}],
            "ledger": {"path": "docs/architecture/reviews/MECHANICS-STATUS-2026-11-05.md", "row": "C13", "status": "DONE", "limitation": "cited lifecycle/owner-end tests do not demonstrate companion offensive HP"},
            "remaining_offense": {"familiar_attacks": 25, "skeleton_attacks": 2,
                                  "not_duplicate_on_either_checked_branch": True,
                                  "status": "W01_RELEVANT_RUNTIME_GAP"}}


def build(native, authoring, witness):
    bodies = {p["target"]["key"]: p for r in native["records"]
              for p in r.get("attack_definitions", {}).get("profiles", [])
              if p["target"]["family"] == "Ability"}
    known = {e["ability"]["key"] for e in witness["entries"]}
    missing, empty = [], []
    for record in native["records"]:
        local = {p["target"]["key"]: p for p in record.get("attack_definitions", {}).get("profiles", [])
                 if p["target"]["family"] == "Ability"}
        creature = record["profile"]["target"]["key"]
        for phase in ("attacks", "defenses"):
            for index, entry in enumerate(record.get("behavior", {}).get("data", {}).get("profile", {}).get(phase, [])):
                ability = entry["ability"]["key"]
                if ability not in local:
                    missing.append({"creature": creature, "phase": phase, "index": index,
                                    "ability": entry["ability"]})
                else:
                    details = local[ability]["data"]["profile"].get("details", {})
                    if not any(details.get(k) for k in ("effects", "variants", "encounter")):
                        empty.append({"creature": creature, "ability": ability})
    if dict(collections.Counter(row["creature"] for row in missing)) != EXPECTED_MISSING:
        raise ValueError("missing-body schedule census changed; review real production reachability")
    reasons = {}
    for reason in sorted({r.get("reason") for r in authoring if r["status"] == "unavailable"}):
        rows = [r for r in authoring if r["status"] == "unavailable" and r["reason"] == reason]
        reasons[reason] = {"count": len(rows), "actual_native_body_count": sum(r["ability"]["key"] in bodies for r in rows)}
    return {"schema": "OTERYN_SRX3_CALLBACK_REACHABILITY/v1",
        "source_definitions_sha256": native["source_definitions_sha256"],
        "authoring_rows": len(authoring),
        "authoring_status_counts": dict(collections.Counter(r["status"] for r in authoring)),
        "unavailable_reasons": reasons,
        "unavailable_is_not_a_runtime_completion_or_stop_verdict": True,
        "no_effects_parent_split": dict(collections.Counter("existing_callback_parent" if r["ability"]["key"] in known else "actual_variant_parent" for r in authoring if r.get("reason") == "donor_ability_has_no_effects")),
        "present_native_schedule_bodies_without_effects_variants_or_encounter": empty,
        "missing_schedule_bodies": missing,
        "pinned_current_donor_sources": PINNED_SOURCES,
        "runtime_role_trace": runtime_role_trace(native),
        "production_routes": {
            "rat": "builtin monster_melee fallback exists in real native loop; qualification_wild_spawn is explicitly TEST ONLY and is not claimed as a production alias spawn",
            "skeleton": "actual animate-dead acquisition binds a current player master; ordinary native loop refuses that role; controlled companion offense is still missing",
            "familiars": "actual familiar acquisition binds current player master; Familiar role refused by ordinary native AI/melee; dedicated familiar_defense consumes five defense slots, not the twenty-five offensive slots",
            "remaining_gap": "twenty-seven controlled companion offensive slots remain W01-relevant; neither C13 DONE nor role exclusion proves their execution; no offense completion claim"}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-root", type=Path)
    args = parser.parse_args()
    if args.source_root:
        for source in PINNED_SOURCES.values():
            raw = (args.source_root / source["path"]).read_bytes()
            if hashlib.sha256(raw).hexdigest() != source["sha256"]:
                raise ValueError("pinned donor source changed: " + source["path"])
    native = json.loads((ROOT / "content/creatures/definitions/spell-native-profiles.json").read_bytes())
    authoring = [json.loads(row) for row in (ROOT / "content/abilities/source-imports/monster-attack-authoring.jsonl").read_text().splitlines()]
    witness = json.loads((ROOT / "crates/gameplay/content/src/content/native_monster_callback_witness.json").read_bytes())
    raw = (json.dumps(build(native, authoring, witness), indent=2, sort_keys=True) + "\n").encode()
    path = ROOT / OUTPUT
    if args.check:
        if not path.exists() or path.read_bytes() != raw:
            raise ValueError("stale W01 callback reachability report")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
    print("W01_CALLBACK_REACHABILITY", len(raw), hashlib.sha256(raw).hexdigest())


if __name__ == "__main__":
    main()
