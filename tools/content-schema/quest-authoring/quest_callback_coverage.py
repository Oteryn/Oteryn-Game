"""Generate the bounded quest-chest execution inventory without executing donor Lua.

This reports implementation coverage, not admission of a deployed World. A candidate
still needs the exact map binding and an admitted native reward/backpack generation.
"""
import argparse
import hashlib
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

import lua_tables

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "docs/repository/QUEST_CALLBACK_COVERAGE.json"
QUALIFIED = {
    "oteryn:reward-claim.kv.bear_chest1": "quest_container_chests_bear_bag_once_and_replay",
    "oteryn:reward-claim.kv.desert2": "source_quest_rewards::the_desert_dungeon_rewards",
    "oteryn:reward-claim.kv.madmageroom3": "quest_container_chests_full_definition_charges_mint_one_instance",
    "oteryn:reward-claim.kv.naginata": "quest_reward_claims_naginata_use_once_and_replay",
    "oteryn:reward-claim.kv.nightmare_teddy_quest": "quest_reward_claims_nightmare_teddy_use_once_and_replay",
    "oteryn:reward-claim.kv.noblearmor1": "quest_reward_claims_noble_armor_use_once_and_replay",
    "oteryn:reward-claim.kv.noblearmor2": "quest_reward_claims_noble_armor_crown_helmet_use_once_and_replay",
    "oteryn:reward-claim.kv.orcfortress1": "quest_reward_claims_batch_one_orc_fortress_all_rewards_and_level_gate",
    "oteryn:reward-claim.kv.orcfortress2": "quest_reward_claims_batch_one_orc_fortress_all_rewards_and_level_gate",
    "oteryn:reward-claim.kv.orcfortress3": "quest_reward_claims_batch_one_orc_fortress_all_rewards_and_level_gate",
    "oteryn:reward-claim.kv.orcshaman": "quest_reward_claims_batch_one_orc_shaman_bag_exact_contents",
    "oteryn:reward-claim.kv.panpipe": "quest_reward_claims_batch_one_panpipe_bag_exact_contents",
    "oteryn:reward-claim.kv.platearmor": "quest_reward_claims_batch_one_plate_armor_coffin_once_and_replay",
    "oteryn:reward-claim.kv.poisondaggers": "quest_reward_claims_batch_two_poison_daggers_two_instances_once_and_replay",
    "oteryn:reward-claim.kv.powerbolt1": "quest_reward_claims_batch_two_power_bolts_book_and_ammunition",
    "oteryn:reward-claim.kv.powerbolt2": "quest_reward_claims_batch_two_power_bolts_book_and_ammunition",
    "oteryn:reward-claim.kv.purpletome1": "quest_reward_claims_batch_one_purple_tome_all_three_bookcases",
    "oteryn:reward-claim.kv.purpletome2": "quest_reward_claims_batch_one_purple_tome_all_three_bookcases",
    "oteryn:reward-claim.kv.purpletome3": "quest_reward_claims_batch_one_purple_tome_all_three_bookcases",
    "oteryn:reward-claim.kv.steelhelmet1": "quest_callbacks_source_chest_use_once_replay_and_second_use_refused",
    "oteryn:reward-claim.kv.timering3": "source_quest_rewards::time_ring_rewards",
    "oteryn:reward-claim.kv.triangletower": "source_quest_rewards::triangle_tower_rewards",
    "oteryn:reward-claim.kv.trollcave1": "source_quest_rewards::troll_cave_rewards",
    "oteryn:reward-claim.kv.weddingring1": "source_quest_rewards::wedding_ring_rewards",
    "oteryn:reward-claim.quest.key.id3002": "quest_container_chests_crystal_key3002_is_minted_once",
    "oteryn:reward-claim.quest.key.id4055": "quest_reward_claims_panpipe_key_4055_once_and_replay",
    "oteryn:reward-claim.quest.u6_1.emperors_cookies.rewards.cookies": "quest_container_chests_key_attributes_are_atomic",
    "oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.demon_helmet": "quest_demon_helmet_chests_demon_helmet_once_and_replay",
    "oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.demon_shield": "quest_demon_helmet_chests_demon_shield_once_and_replay",
    "oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.steel_boots": "quest_demon_helmet_chests_steel_boots_once_and_replay",
    "oteryn:reward-claim.quest.u7_2.parchment_room.bag": "quest_reward_claims_batch_two_parchment_room_key_bag",
    "oteryn:reward-claim.quest.u7_24.the_annihilator.reward": "quest_container_chests_annihilator_choices_share_storage_and_achievement",
    "oteryn:reward-claim.quest.u7_9.dreamers_challenge.bplong_sword": "quest_container_chests_authored_book_text_survives_replay",
}

# These source selectors ran through the production USE route in run7. This
# certifies granting/replay only; access and the complete quest remain separate.
QUALIFIED.update({
    f"oteryn:reward-claim.kv.{suffix}":
        "quest_catalogue_chests_full_charge_rewards_use_replay_and_refusal"
    for suffix in ("desert2", "draconia1", "edrongoblin1", "fireaxe1", "ghoulroom1",
                   "lifering", "ornamentedshield", "powerring2", "silveramulet", "stealhring1", "timering3",
                   "triangletower", "trollcave1", "weddingring1")
})
QUALIFIED["oteryn:reward-claim.quest.u7_3.the_medusa_quest"] = (
    "quest_catalogue_chests_medusa_exact_crystal_reward_use_and_replay")
QUALIFIED.update({f"oteryn:reward-claim.{suffix}":
    "quest_catalogue_chests_exact_partial_charges_use_replay_and_refusal"
    for suffix in ("kv.deeperfibula4",
                   "quest.u7_2.the_queen_of_the_banshees.reward.stone_skin_amulet",
                   "quest.u15_24.targuna.silver_amulet_chest",
                   "quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield")})
QUALIFIED.update({f"oteryn:reward-claim.{suffix}":
    "quest_catalogue_chests_exact_fluid_subtypes_use_replay_and_refusal"
    for suffix in ("kv.goblintemple2", "quest.u7_9.dreamers_challenge.chests_wine", "kv.present_box")})
# Phase A: 88 additional claims passed all native source placements through
# the shared real USE / mint / replay / refusal cohorts (106 placements total).
# Existing qualification locators stay unchanged for the 16 proven siblings.
for cohort, claims in (
    (1, (
        'oteryn:reward-claim.kv.adoreduh',
        'oteryn:reward-claim.kv.alawars_vault1',
        'oteryn:reward-claim.kv.alawars_vault2',
        'oteryn:reward-claim.kv.barbarianaxe1',
        'oteryn:reward-claim.kv.battleaxe',
        'oteryn:reward-claim.kv.bear_chest2',
        'oteryn:reward-claim.kv.bear_chest3',
        'oteryn:reward-claim.quest.key.id4502',
        'oteryn:reward-claim.quest.key.id4601',
    )),
    (2, (
        'oteryn:reward-claim.kv.behemothquest1',
        'oteryn:reward-claim.kv.behemothquest2',
        'oteryn:reward-claim.kv.behemothquest3',
        'oteryn:reward-claim.kv.behemothquest4',
        'oteryn:reward-claim.kv.blackknight1',
        'oteryn:reward-claim.kv.blackknight2',
        'oteryn:reward-claim.kv.bloodherb',
        'oteryn:reward-claim.kv.captain_iglues',
        'oteryn:reward-claim.quest.key.id5010',
        'oteryn:reward-claim.quest.u6_4.berserker_treasure.rewards.white_pearls',
    )),
    (3, (
        'oteryn:reward-claim.kv.circleroom1',
        'oteryn:reward-claim.kv.circleroom2',
        'oteryn:reward-claim.kv.combatknife',
        'oteryn:reward-claim.kv.crusaderhelmet',
        'oteryn:reward-claim.kv.crystalwand1',
        'oteryn:reward-claim.kv.crystalwand2',
        'oteryn:reward-claim.quest.u8_5.darashia_dragon',
    )),
    (4, (
        'oteryn:reward-claim.kv.darkarmor',
        'oteryn:reward-claim.kv.deadarcher',
        'oteryn:reward-claim.kv.deeperfibula1',
        'oteryn:reward-claim.kv.deeperfibula2',
        'oteryn:reward-claim.kv.deeperfibula3',
        'oteryn:reward-claim.kv.deeperfibula5',
        'oteryn:reward-claim.kv.demonaring1',
        'oteryn:reward-claim.kv.demonaring2',
        'oteryn:reward-claim.quest.key.id3980',
    )),
    (5, (
        'oteryn:reward-claim.kv.desert1',
        'oteryn:reward-claim.kv.devilhemet1',
        'oteryn:reward-claim.kv.devilhemet2',
        'oteryn:reward-claim.kv.devilhemet3',
        'oteryn:reward-claim.kv.doublehero1',
        'oteryn:reward-claim.kv.doublehero2',
        'oteryn:reward-claim.kv.doublet',
        'oteryn:reward-claim.kv.draconia2',
        'oteryn:reward-claim.quest.key.id3008',
        'oteryn:reward-claim.quest.key.id3610',
        'oteryn:reward-claim.quest.key.id3667',
    )),
    (6, (
        'oteryn:reward-claim.kv.dragoncorpse',
        'oteryn:reward-claim.kv.edrongoblin2',
        'oteryn:reward-claim.kv.explorerbrooch',
        'oteryn:reward-claim.quest.u7_1.dragon_tower.rewards.backpack1',
        'oteryn:reward-claim.quest.u7_1.dragon_tower.rewards.backpack2',
        'oteryn:reward-claim.quest.u7_5.elephant_tusk',
    )),
    (7, (
        'oteryn:reward-claim.kv.fanfare',
        'oteryn:reward-claim.kv.fireaxe2',
        'oteryn:reward-claim.kv.geomancer',
        'oteryn:reward-claim.kv.ghoulroom2',
        'oteryn:reward-claim.quest.key.id3520',
        'oteryn:reward-claim.quest.key.id3600',
        'oteryn:reward-claim.quest.u8_0.fishing_box',
    )),
    (8, (
        'oteryn:reward-claim.kv.goblintemple',
        'oteryn:reward-claim.kv.griffinshield1',
        'oteryn:reward-claim.kv.griffinshield2',
        'oteryn:reward-claim.kv.griffinshield3',
        'oteryn:reward-claim.kv.heavenblossom',
        'oteryn:reward-claim.kv.ironhamer',
        'oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.inukaya',
    )),
    (9, (
        'oteryn:reward-claim.kv.ironhelmet',
        'oteryn:reward-claim.kv.ironore',
        'oteryn:reward-claim.kv.isleofthemists',
        'oteryn:reward-claim.kv.katanacorpse1',
        'oteryn:reward-claim.kv.katanacorpse2',
        'oteryn:reward-claim.quest.key.id4603',
        'oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.formorgar_mines',
    )),
    (10, (
        'oteryn:reward-claim.kv.longsword1',
        'oteryn:reward-claim.kv.longsword2',
        'oteryn:reward-claim.kv.longsword3',
        'oteryn:reward-claim.kv.madmageroom1',
        'oteryn:reward-claim.kv.madmageroom2',
        'oteryn:reward-claim.kv.minohell1',
        'oteryn:reward-claim.kv.minohell2',
        'oteryn:reward-claim.kv.minohell3',
        'oteryn:reward-claim.quest.u8_5.lone_medusa',
    )),
    (11, (
        'oteryn:reward-claim.kv.minotaurleatherquest',
        'oteryn:reward-claim.kv.mintwallincyclops',
        'oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.helheim',
        'oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.okolnir',
    )),
    (12, (
        'oteryn:reward-claim.quest.key.id3800',
        'oteryn:reward-claim.quest.key.id3802',
    )),
):
    QUALIFIED.update({claim:
        f"quest_catalogue_chests_phase_a_{cohort:02}_use_replay_and_refusal"
        for claim in claims})

QUALIFIED["oteryn:reward-claim.quest.key.id3620"] = (
    "quest_catalogue_chests_mintwallin_prison_key3620_use_replay_and_refusal")

# Story and outfit reward sources: 44 additional claims qualified. Three shared
# USE/replay/refusal cohorts exercised 46 claims at all 55 source placements.
# This does not certify the surrounding story, arena, hand-in or outfit path.
for test_name, claims in (
    ('quest_catalogue_chests_botany_arena_beggar_children_cults_use_replay_and_refusal', (
        'oteryn:reward-claim.kv.petrified_scream_quest',
        'oteryn:reward-claim.kv.simon_the_beggars_favorite_staff',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_greenhorn_bp',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_greenhorn_present',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_greenhorn_weapons',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_scrapper_bp',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_scrapper_present',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_scrapper_weapons',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_warlord_bp',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_warlord_present',
        'oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_warlord_weapons',
        'oteryn:reward-claim.quest.u8_54.children_of_the_revolution.chest_tome_of_knowledge1',
        'oteryn:reward-claim.quest.u8_54.children_of_the_revolution.chest_tome_of_knowledge2',
        'oteryn:reward-claim.quest.u8_54.children_of_the_revolution.flask_of_extra_greasy_oil',
        'oteryn:reward-claim.quest.u8_6.an_interest_in_botany.botany_almanach',
    )),
    ('quest_catalogue_chests_dark_trails_dreamer_druid_ferumbras_knowledge_use_replay_and_refusal', (
        'oteryn:reward-claim.halls_of_hope.reward1',
        'oteryn:reward-claim.halls_of_hope.reward2',
        'oteryn:reward-claim.halls_of_hope.reward3',
        'oteryn:reward-claim.halls_of_hope.reward4',
        'oteryn:reward-claim.halls_of_hope.reward5',
        'oteryn:reward-claim.kv.ghostsilver_lantern_quest',
        'oteryn:reward-claim.kv.imbuing_crystal_quest',
        'oteryn:reward-claim.kv.painting_of_agirl_quest',
        'oteryn:reward-claim.quest.u10_50.dark_trails.reward05_gloth_and_belongings',
        'oteryn:reward-claim.quest.u10_50.dark_trails.reward10_gloth_and_belongings',
        'oteryn:reward-claim.quest.u10_50.dark_trails.reward_small_notebook',
        'oteryn:reward-claim.quest.u10_90.ferumbras_ascension.stone_coffins_bones',
        'oteryn:reward-claim.quest.u10_90.ferumbras_ascension.the_rite_of_eternal_night',
        'oteryn:reward-claim.quest.u10_90.ferumbras_ascension.vampire_teeth',
        'oteryn:reward-claim.quest.u7_8.druid_outfits.wolf_tooth_chain',
        'oteryn:reward-claim.quest.u7_9.dreamers_challenge.chest_book',
        'oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests3_small_diamond',
    )),
    ('quest_catalogue_chests_hidden_threats_hunter_knight_koshei_use_replay_and_refusal', (
        'oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.key_fragment01',
        'oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.key_fragment02',
        'oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.metal_file',
        'oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow',
        'oteryn:reward-claim.quest.u7_8.knight_outfits.ramsays_helmet_chest',
        'oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part1',
        'oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part2',
        'oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part3',
        'oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part4',
        'oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_the_deathless_gold',
        'oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_the_deathless_legs',
        'oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_the_deathless_platinum',
    )),
):
    QUALIFIED.update({claim: test_name for claim in claims})

FALCON_PREFIX = "oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer."
FALCON_SOURCES = {
    "coast": (6105, 23741, 33352, 31318, 7),
    "island": (6106, 23740, 33384, 31285, 7),
    "throne_hall": (6107, 23741, 33366, 31323, 5),
    "shortcut": (6108, 23741, 33374, 31340, 4),
    "lower_bastion": (6109, 23740, 33324, 31268, 8),
    "underground_bastion": (6110, 23740, 33308, 31304, 9),
}
QUALIFIED.update({FALCON_PREFIX + suffix:
    "falcon_chests_postgres_real_use_frozen_draw_refusal_restart_and_fences"
    for suffix in FALCON_SOURCES})


def falcon_supported(definition):
    key = definition["identity"]["key"]
    expected = FALCON_SOURCES.get(key.removeprefix(FALCON_PREFIX)) if key.startswith(FALCON_PREFIX) else None
    if expected is None:
        return False
    uid, appearance, x, y, z = expected
    pool = [{"item": {"family": "Item", "key": f"oteryn:item.tibia.i{item}",
                      "revision": "definition-r1"}, "count": count}
            for item, count in ((9081, 1), (28821, 1), (28823, 1), (9058, 1),
                                (6299, 1), (3052, 1), (3035, 10))]
    valid = (definition["claim"] == {"per": "character", "repeat": {"kind": "cooldown", "hours": 24}}
             and definition["data_holds"] == [] and definition["placements"] == [{
                 "appearance_tibia_id": appearance,
                 "source_binding": {"project_position": {"x": x, "y": y, "z": z},
                     "legacy_unique_ids": [{"server": "canary", "unique_id": uid},
                                           {"server": "crystalserver", "unique_id": uid}]},
                 "reward": {"items": [], "random_one_of": pool}}])
    if not valid:
        raise ValueError(f"qualified Falcon recipe changed: {key}")
    return True

KEY_LOWERING = "KEY_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED"
SOURCE_SHA256 = {
    "canary": "2b130d4196afc9700eb0b7c2f5594e331edee4d25811a0d2020fb9d8757d6cda",
    "crystalserver": "7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4",
}
CONTAINER_SOURCE = "b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757"
ACCEPTED_OVERRIDES = {
    6068: "Target Wiki2026-11-06 and both donor comments: two quantity1 daggers; retain numeric donor one in source_variant",
    6078: "PR2770 Q5/A: retain accepted Item12510 over donor3208",
    6218: "PR2770 Q4/A: retain accepted two100stacks over donor Container:addItem clamp",
    6226: "PR2770 Q4/A: retain accepted two100stacks over donor Container:addItem clamp",
    6335: "PR2770 Q6/A: retain Item9238 at32994,32405,12 over donor9237 at32629,32446,10",
    **{uid: "PR2770 Q3/A: retain accepted24hour repeat despite absent donor timerStorage"
       for uid in range(6105, 6111)},
}


def source(root, pack):
    path = f"{pack}/startup/tables/chest.lua"
    data = (root / path).read_bytes()
    fields = lua_tables.assignments(data.decode(), {"ChestUnique"})["ChestUnique"]["fields"]
    return path, hashlib.sha256(data).hexdigest(), {
        field["key"]: (field["line"], lua_tables.as_python(field["value"]))
        for field in fields
    }


def coverage(canary, crystal):
    sources = {
        "canary": source(canary, "data-otservbr-global"),
        "crystalserver": source(crystal, "data-global"),
    }
    if {server: digest for server, (_, digest, _) in sources.items()} != SOURCE_SHA256:
        raise ValueError("reference chest tables differ from the task's pinned revisions")
    sys.path.insert(0, str(ROOT / "tools/content-schema/reward-claim-authoring"))
    import charge_native
    import container_native
    import reward_claim_authoring as claims
    items = claims.load_items()
    quests = defaultdict(list)
    for path in sorted((ROOT / "content/interactions/reward_claims").glob("reward-claim*.json")):
        for record in json.loads(path.read_text())["records"]:
            definition = record["definition"]
            key = definition["identity"]["key"]
            variant = definition.get("source_variant", {})
            pending = variant.get("native_lowering", {}).get("pending_implementation", [])
            native = definition.get("native_contents", {})
            medusa = (
                key == "oteryn:reward-claim.quest.u7_3.the_medusa_quest"
                and native.get("source_difference") == "CRYSTAL_MEDUSA_WITHOUT_CANARY_TEXT"
                and definition.get("data_holds") == [{"category": "source",
                    "code": "SOURCE_WRITTEN_TEXT_CARRIER_UNRESOLVED", "placement_index": 0}]
            )
            reconciled = bool(native) and (
                charge_native.derive(definition, items, claims.stack_problem) == native
                or (definition['identity']['key'] in container_native.FLUID_CHOICES
                    and container_native.derive(definition, items, claims.stack_problem) == native))
            supported = falcon_supported(definition) or definition.get("readiness") == "ready" or (
                pending == [KEY_LOWERING] and not definition.get("data_holds")
                and not variant.get("achievement_grants")
                and not variant.get("charge_dispositions")
                and variant.get("source_data", {}).get("status") == "COMPLETE"
                and all(p["reward"].get("key_binding", "").startswith("canary:door-key/")
                        for p in definition["placements"])
            ) or (
                native.get("schema") == "OTERYN_NATIVE_CONTAINER_REWARD/v1"
                and native.get("source_sha256") == CONTAINER_SOURCE
                and native.get("revision") == "201189f871d23597b82caf88fd611454a2ddefb8"
                and (not definition.get("data_holds") or medusa or reconciled)
                and len(native.get("placements", [])) == len(definition["placements"])
            )
            status = "supported_unqualified" if supported else "not_playable"
            reason = "requires exact World binding and native Item admission" if supported else (
                ", ".join(pending) or definition["readiness"]
            )
            if key in QUALIFIED:
                status = "dispatch_pg_qualified"
                reason = QUALIFIED[key]
            if definition.get('quest', {}).get('key') == 'canary:quest/dawnport_quest':
                status = 'not_playable'
                reason = 'OWNER_EXCLUDED_DAWNPORT: Newhaven owns the tutorial and starting equipment'
            placements = []
            for placement in definition["placements"]:
                witnesses = []
                for legacy in placement["source_binding"]["legacy_unique_ids"]:
                    server = legacy["server"]
                    src_path, _, entries = sources[server]
                    found = entries.get(legacy["unique_id"])
                    witness = {"server": server, "unique_id": legacy["unique_id"]}
                    if found:
                        line, value = found
                        prefix = "canary" if server == "canary" else "crystal"
                        witness["citation"] = f"{prefix}:{src_path}:{line}"
                        authored = placement["reward"].get("items", [])
                        pairs = [[int(item["item"]["key"].rsplit(".i", 1)[1]), item["count"]]
                                 for item in authored]
                        witness["reward_matches_current_source"] = value.get("reward") == pairs
                        if server == "crystalserver" and legacy["unique_id"] in ACCEPTED_OVERRIDES:
                            witness["accepted_override"] = ACCEPTED_OVERRIDES[legacy["unique_id"]]
                    else:
                        witness["missing_in_current_source"] = True
                    witnesses.append(witness)
                placements.append({"position": placement["source_binding"]["project_position"],
                                   "sources": witnesses})
            inventory = {
                "claim": key, "status": status, "reason": reason, "placements": placements,
            }
            if key == "oteryn:reward-claim.kv.poisondaggers":
                inventory["target_correction"] = (
                    "Target Wiki2026-11-06 and both donor comments specify two daggers. "
                    "The native recipe mints two distinct quantity1 instances; "
                    "source_variant preserves the donor numeric recipe of one."
                )
            if native and any(p['reward'].get('written_text') for p in definition['placements']):
                inventory["qualification_limit"] = (
                    "Reward-grant support includes durable authored book text; "
                    "ordinary carried-book READ/TextWindow transport is not implemented. "
                    "LOOK and distance-reading admission are not changed. "
                    "Text retains the established source author's outer-LF normalization."
                )
            quests[definition.get("quest", {}).get("key", "unlinked")].append(inventory)
    counts = Counter(row["status"] for rows in quests.values() for row in rows)
    return {
        "task": "PARITY-QUEST-CALLBACKS-1+W15b",
        "source_revisions": {"canary": "47dfd51f", "crystalserver": "201189f871d23597b82caf88fd611454a2ddefb8"},
        "source_sha256": {server: digest for server, (_, digest, _) in sources.items()},
        "scope": "Once-only leaf/key/text chests, exact filled-container/shared-storage-choice recipes and six Falcon 24h random-cycle sources; levers and step-in scripts are separate owners.",
        "qualification": "Only dispatch_pg_qualified has a real source-placement PostgreSQL USE test. "
                         "That status certifies reward granting, not every later Item consumer. "
                         "Other supported shapes need per-World binding, Item admission and qualification. "
                         "No deployed World or full-quest completion is certified.",
        "counts": dict(sorted(counts.items())),
        "quests": [{"quest": quest, "chests": sorted(rows, key=lambda row: row["claim"])}
                   for quest, rows in sorted(quests.items())],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary", type=Path, required=True)
    parser.add_argument("--crystal", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    text = json.dumps(coverage(args.canary, args.crystal), indent=2, sort_keys=True) + "\n"
    if args.check:
        if OUTPUT.read_text() != text:
            raise SystemExit("quest callback coverage is stale")
    else:
        OUTPUT.write_text(text)


if __name__ == "__main__":
    main()
