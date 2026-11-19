//! Catalogue-wide reward qualification through the production USE and durable owners.
//! This proves the reward at its source selector, not the quest's access path.
use super::quest_catalogue_chests_fixtures::{CatalogueChest, cases, cases_including, catalogue_source_world, execute, expected_charges, expected_fluids, phase_a_cohort, standalone_claim_cohort, string};

use super::TestResult;
use serde_json::Value;
use std::collections::BTreeSet;

const CATALOGUE_SLICE_COUNT: usize = 8;

fn catalogue_slice(index: usize, cases: &[CatalogueChest]) -> std::ops::Range<usize> {
    // cases() emits every placement for a claim consecutively.
    let mut boundaries = vec![0];
    boundaries.extend(
        cases
            .windows(2)
            .enumerate()
            .filter(|(_, pair)| pair[0].claim != pair[1].claim)
            .map(|(index, _)| index + 1),
    );
    boundaries.push(cases.len());
    let claims = boundaries.len() - 1;
    boundaries[index * claims / CATALOGUE_SLICE_COUNT]
        ..boundaries[(index + 1) * claims / CATALOGUE_SLICE_COUNT]
}

fn run_catalogue_slice(index: usize) -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let mut cases = cases()?;
    let range = catalogue_slice(index, &cases);
    execute(cases.drain(range).collect())
}

#[test]
fn quest_catalogue_chests_slices_cover_every_supported_source_placement() -> TestResult {
    let cases = cases()?;
    let falcon_claims = oteryn_gameplay_quests::quest::falcon::CHESTS.map(|chest| chest.key());
    let expected: BTreeSet<_> = cases
        .iter()
        .map(|case| case.claim.as_str())
        .chain(falcon_claims.iter().map(String::as_str))
        .collect();
    assert_eq!(expected.len(), 334);

    let mut seen_claims = BTreeSet::new();
    let mut seen_placements = vec![false; cases.len()];
    for index in 0..CATALOGUE_SLICE_COUNT {
        let range = catalogue_slice(index, &cases);
        assert!(!range.is_empty(), "slice {index} is empty");
        let mut slice_claims = BTreeSet::new();
        for placement in range {
            assert!(
                !seen_placements[placement],
                "placement {placement} appears twice"
            );
            seen_placements[placement] = true;
            slice_claims.insert(cases[placement].claim.as_str());
        }
        for claim in slice_claims {
            assert!(seen_claims.insert(claim), "claim {claim} spans slices");
        }
    }
    for claim in &falcon_claims {
        assert!(
            seen_claims.insert(claim.as_str()),
            "duplicate Falcon claim {claim}"
        );
    }
    assert!(seen_placements.into_iter().all(|visited| visited));
    assert_eq!(seen_claims, expected);
    Ok(())
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_1_of_8()
-> TestResult {
    run_catalogue_slice(0)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_2_of_8()
-> TestResult {
    run_catalogue_slice(1)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_3_of_8()
-> TestResult {
    run_catalogue_slice(2)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_4_of_8()
-> TestResult {
    run_catalogue_slice(3)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_5_of_8()
-> TestResult {
    run_catalogue_slice(4)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_6_of_8()
-> TestResult {
    run_catalogue_slice(5)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_7_of_8()
-> TestResult {
    run_catalogue_slice(6)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal_slice_8_of_8()
-> TestResult {
    run_catalogue_slice(7)?;
    super::falcon_chests_fixtures::run_falcon_chests()
}

#[test]
fn quest_catalogue_chests_dragahs_spellbook_use_replay_and_refusal() -> TestResult {
    let cases: Vec<_> = cases()?
        .into_iter()
        .filter(|chest| {
            chest.claim == "oteryn:reward-claim.quest.u7_8.the_shattered_isles.dragahs_spellbook"
        })
        .collect();
    assert_eq!(cases.len(), 1);
    assert!(cases.iter().all(|chest| chest.servable));
    execute(cases)
}

#[test]
fn quest_catalogue_chests_mixed_source_namespaces_use_replay_and_refusal() -> TestResult {
    let cases: Vec<_> = cases()?
        .into_iter()
        .filter(|chest| {
            chest.claim == "oteryn:reward-claim.quest.u8_6.wrath_of_the_emperor.chest_items"
        })
        .collect();
    assert_eq!(cases.len(), 5);
    assert_eq!(cases.iter().filter(|chest| chest.servable).count(), 4);
    execute(cases)
}

#[test]
fn quest_catalogue_chests_mixed_binding_requires_all_crystal_sources() -> TestResult {
    let claim = "oteryn:reward-claim.quest.u8_6.wrath_of_the_emperor.chest_items";
    let cases: Vec<_> = cases()?
        .into_iter()
        .filter(|chest| chest.claim == claim)
        .collect();
    let native = cases
        .iter()
        .find(|chest| chest.servable)
        .ok_or("Crystal source")?;
    let world = catalogue_source_world(native)?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|_| true,
    )?;
    let served = report
        .served
        .iter()
        .find(|row| row.key.as_str() == claim)
        .ok_or("Crystal claim")?;
    assert_eq!(served.placements.len(), 4);
    let unbound: Vec<_> = report.unbound.iter().filter(|row| row.claim == claim).collect();
    assert_eq!(unbound.len(), 1);
    assert_eq!(unbound[0].index, 0);
    assert_eq!(
        unbound[0].reason,
        crate::content::world_reward_claims::UnboundReason::NoCrystalUniqueId
    );
    let incomplete = super::quest_callbacks_fixtures::source_world(
        native.position,
        native.unique,
        native.appearance,
        0,
    )?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        incomplete.base().as_ref(),
        &|_| true,
    )?;
    assert!(!report.served.iter().any(|row| row.key.as_str() == claim));
    assert_eq!(
        report
            .unbound
            .iter()
            .filter(|row| {
                row.claim == claim
                    && row.reason == crate::content::world_reward_claims::UnboundReason::NoEntry
            })
            .count(),
        3,
        "one present Crystal source cannot substitute for the other three"
    );
    let foreign = cases
        .iter()
        .find(|chest| !chest.servable)
        .ok_or("Canary-only source")?;
    let world = catalogue_source_world(foreign)?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|_| true,
    )?;
    assert!(!report.served.iter().any(|row| row.key.as_str() == claim));
    assert!(report.unbound.iter().any(|row| row.claim == claim
        && row.reason == crate::content::world_reward_claims::UnboundReason::NoEntry));
    Ok(())
}

#[test]
fn quest_catalogue_chests_a_m_unqualified_source_placements_use_replay_and_refusal() -> TestResult {
    // Frozen run-8 claims remain exercised after qualification promotes their
    // coverage status. New claims join a later batch with their own PG evidence.
    let claims: BTreeSet<_> = [
        "oteryn:reward-claim.halls_of_hope.reward1",
        "oteryn:reward-claim.halls_of_hope.reward2",
        "oteryn:reward-claim.halls_of_hope.reward3",
        "oteryn:reward-claim.halls_of_hope.reward4",
        "oteryn:reward-claim.halls_of_hope.reward5",
        "oteryn:reward-claim.kv.adoreduh",
        "oteryn:reward-claim.kv.alawars_vault1",
        "oteryn:reward-claim.kv.alawars_vault2",
        "oteryn:reward-claim.kv.barbarianaxe1",
        "oteryn:reward-claim.kv.battleaxe",
        "oteryn:reward-claim.kv.bear_chest2",
        "oteryn:reward-claim.kv.bear_chest3",
        "oteryn:reward-claim.kv.behemothquest1",
        "oteryn:reward-claim.kv.behemothquest2",
        "oteryn:reward-claim.kv.behemothquest3",
        "oteryn:reward-claim.kv.behemothquest4",
        "oteryn:reward-claim.kv.blackknight1",
        "oteryn:reward-claim.kv.blackknight2",
        "oteryn:reward-claim.kv.bloodherb",
        "oteryn:reward-claim.kv.captain_iglues",
        "oteryn:reward-claim.kv.circleroom1",
        "oteryn:reward-claim.kv.circleroom2",
        "oteryn:reward-claim.kv.combatknife",
        "oteryn:reward-claim.kv.crusaderhelmet",
        "oteryn:reward-claim.kv.crystalwand1",
        "oteryn:reward-claim.kv.crystalwand2",
        "oteryn:reward-claim.kv.darkarmor",
        "oteryn:reward-claim.kv.deadarcher",
        "oteryn:reward-claim.kv.deeperfibula1",
        "oteryn:reward-claim.kv.deeperfibula2",
        "oteryn:reward-claim.kv.deeperfibula3",
        "oteryn:reward-claim.kv.deeperfibula5",
        "oteryn:reward-claim.kv.demonaring1",
        "oteryn:reward-claim.kv.demonaring2",
        "oteryn:reward-claim.kv.desert1",
        "oteryn:reward-claim.kv.devilhemet1",
        "oteryn:reward-claim.kv.devilhemet2",
        "oteryn:reward-claim.kv.devilhemet3",
        "oteryn:reward-claim.kv.doublehero1",
        "oteryn:reward-claim.kv.doublehero2",
        "oteryn:reward-claim.kv.doublet",
        "oteryn:reward-claim.kv.draconia2",
        "oteryn:reward-claim.kv.dragoncorpse",
        "oteryn:reward-claim.kv.edrongoblin2",
        "oteryn:reward-claim.kv.explorerbrooch",
        "oteryn:reward-claim.kv.fanfare",
        "oteryn:reward-claim.kv.fireaxe2",
        "oteryn:reward-claim.kv.geomancer",
        "oteryn:reward-claim.kv.ghostsilver_lantern_quest",
        "oteryn:reward-claim.kv.ghoulroom2",
        "oteryn:reward-claim.kv.goblintemple",
        "oteryn:reward-claim.kv.griffinshield1",
        "oteryn:reward-claim.kv.griffinshield2",
        "oteryn:reward-claim.kv.griffinshield3",
        "oteryn:reward-claim.kv.heavenblossom",
        "oteryn:reward-claim.kv.imbuing_crystal_quest",
        "oteryn:reward-claim.kv.ironhamer",
        "oteryn:reward-claim.kv.ironhelmet",
        "oteryn:reward-claim.kv.ironore",
        "oteryn:reward-claim.kv.isleofthemists",
        "oteryn:reward-claim.kv.katanacorpse1",
        "oteryn:reward-claim.kv.katanacorpse2",
        "oteryn:reward-claim.kv.longsword1",
        "oteryn:reward-claim.kv.longsword2",
        "oteryn:reward-claim.kv.longsword3",
        "oteryn:reward-claim.kv.madmageroom1",
        "oteryn:reward-claim.kv.madmageroom2",
        "oteryn:reward-claim.kv.minohell1",
        "oteryn:reward-claim.kv.minohell2",
        "oteryn:reward-claim.kv.minohell3",
        "oteryn:reward-claim.kv.minotaurleatherquest",
        "oteryn:reward-claim.kv.mintwallincyclops",
        "oteryn:reward-claim.kv.painting_of_agirl_quest",
        "oteryn:reward-claim.kv.petrified_scream_quest",
        "oteryn:reward-claim.kv.simon_the_beggars_favorite_staff",
        "oteryn:reward-claim.quest.key.id3008",
        "oteryn:reward-claim.quest.key.id3520",
        "oteryn:reward-claim.quest.key.id3600",
        "oteryn:reward-claim.quest.key.id3610",
        "oteryn:reward-claim.quest.key.id3667",
        "oteryn:reward-claim.quest.key.id3800",
        "oteryn:reward-claim.quest.key.id3802",
        "oteryn:reward-claim.quest.key.id3980",
        "oteryn:reward-claim.quest.key.id4502",
        "oteryn:reward-claim.quest.key.id4601",
        "oteryn:reward-claim.quest.key.id4603",
        "oteryn:reward-claim.quest.key.id5010",
        "oteryn:reward-claim.quest.u10_50.dark_trails.reward05_gloth_and_belongings",
        "oteryn:reward-claim.quest.u10_50.dark_trails.reward10_gloth_and_belongings",
        "oteryn:reward-claim.quest.u10_50.dark_trails.reward_small_notebook",
        "oteryn:reward-claim.quest.u10_90.ferumbras_ascension.stone_coffins_bones",
        "oteryn:reward-claim.quest.u10_90.ferumbras_ascension.the_rite_of_eternal_night",
        "oteryn:reward-claim.quest.u10_90.ferumbras_ascension.vampire_teeth",
        "oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.key_fragment01",
        "oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.key_fragment02",
        "oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.metal_file",
        "oteryn:reward-claim.quest.u6_4.berserker_treasure.rewards.white_pearls",
        "oteryn:reward-claim.quest.u7_1.dragon_tower.rewards.backpack1",
        "oteryn:reward-claim.quest.u7_1.dragon_tower.rewards.backpack2",
        "oteryn:reward-claim.quest.u7_5.elephant_tusk",
        "oteryn:reward-claim.quest.u7_8.druid_outfits.wolf_tooth_chain",
        "oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow",
        "oteryn:reward-claim.quest.u7_8.knight_outfits.ramsays_helmet_chest",
        "oteryn:reward-claim.quest.u7_9.dreamers_challenge.chest_book",
        "oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests3_small_diamond",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_greenhorn_bp",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_greenhorn_present",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_greenhorn_weapons",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_scrapper_bp",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_scrapper_present",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_scrapper_weapons",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_warlord_bp",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_warlord_present",
        "oteryn:reward-claim.quest.u8_0.barbarian_arena.reward_warlord_weapons",
        "oteryn:reward-claim.quest.u8_0.fishing_box",
        "oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.formorgar_mines",
        "oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.helheim",
        "oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.inukaya",
        "oteryn:reward-claim.quest.u8_0.shardsof_ancient_winters.okolnir",
        "oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part1",
        "oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part2",
        "oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part3",
        "oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_amulet_part4",
        "oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_the_deathless_gold",
        "oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_the_deathless_legs",
        "oteryn:reward-claim.quest.u8_1.koshei_the_deathless.koshei_the_deathless_platinum",
        "oteryn:reward-claim.quest.u8_5.darashia_dragon",
        "oteryn:reward-claim.quest.u8_5.lone_medusa",
        "oteryn:reward-claim.quest.u8_54.children_of_the_revolution.chest_tome_of_knowledge1",
        "oteryn:reward-claim.quest.u8_54.children_of_the_revolution.chest_tome_of_knowledge2",
        "oteryn:reward-claim.quest.u8_54.children_of_the_revolution.flask_of_extra_greasy_oil",
        "oteryn:reward-claim.quest.u8_6.an_interest_in_botany.botany_almanach",
    ]
    .into_iter()
    .collect();
    let coverage: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    // Lane ownership comes from the coverage quest title, including its leading
    // article. Claim slugs can describe an unrelated item or donor storage key.
    let owned_claims: BTreeSet<_> = coverage["quests"]
        .as_array()
        .ok_or("coverage quests")?
        .iter()
        .filter(|quest| {
            quest["quest"]
                .as_str()
                .and_then(|title| title.rsplit('/').next())
                .and_then(|title| title.bytes().next())
                .is_some_and(|initial| (b'a'..=b'm').contains(&initial))
        })
        .flat_map(|quest| quest["chests"].as_array().into_iter().flatten())
        .filter(|chest| chest["status"] != "not_playable")
        .map(|chest| string(&chest["claim"]))
        .collect::<TestResult<_>>()?;
    assert!(
        claims.iter().all(|claim| owned_claims.contains(*claim)),
        "every frozen A..M claim must retain supported title ownership"
    );
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| claims.contains(case.claim.as_str()))
        .collect();
    assert_eq!(
        selected
            .iter()
            .map(|case| case.claim.as_str())
            .collect::<BTreeSet<_>>(),
        claims,
        "every A..M run-8 claim needs every source placement exercised"
    );
    execute(selected)
}

#[test]
fn standalone_claims_bear_berserk_berserker_black_knight_dead_archer() -> TestResult {
    standalone_claim_cohort(
        &[
            "bear_room_quest",
            "berserk_potion_quest",
            "berserker_treasure_quest",
            "black_knight_quest",
            "dead_archer_quest",
        ],
        "standalone_claims_bear_berserk_berserker_black_knight_dead_archer",
    )
}

#[test]
fn standalone_claims_dragon_corpse_tower_edron_emperor_fanfare() -> TestResult {
    standalone_claim_cohort(
        &[
            "dragon_corpse_quest",
            "dragon_tower_quest",
            "edron_goblin_quest",
            "emperor_s_cookies_quest",
            "fanfare_quest",
        ],
        "standalone_claims_dragon_corpse_tower_edron_emperor_fanfare",
    )
}

#[test]
fn standalone_claims_fire_axe_fishing_frost_geomancer_ghoul() -> TestResult {
    standalone_claim_cohort(
        &[
            "fire_axe_quest",
            "fishing_box_quest",
            "frost_dragon_quest",
            "geomancer_quest",
            "ghoul_room_quest",
        ],
        "standalone_claims_fire_axe_fishing_frost_geomancer_ghoul",
    )
}

#[test]
fn standalone_claims_goblin_inukaya_iron_helmet_katana_lion() -> TestResult {
    standalone_claim_cohort(
        &[
            "goblin_temple_quest",
            "inukaya_quest",
            "iron_helmet_quest",
            "katana_quest",
            "lion_trophy_quest",
        ],
        "standalone_claims_goblin_inukaya_iron_helmet_katana_lion",
    )
}

#[test]
fn standalone_claims_longsword_mad_mage_medusa_shield_minotaur() -> TestResult {
    standalone_claim_cohort(
        &[
            "longsword_quest",
            "mad_mage_room_quest",
            "medusa_shield_quest",
            "minotaur_hell_quest",
        ],
        "standalone_claims_longsword_mad_mage_medusa_shield_minotaur",
    )
}

#[test]
fn quest_catalogue_chests_phase_a_01_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "adorned_uh_rune_quest",
        "alawar_s_vault_quest",
        "barbarian_axe_quest",
        "battle_axe_quest",
        "bear_room_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_02_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "behemoth_quest",
        "berserker_treasure_quest",
        "black_knight_quest",
        "blood_herb_quest",
        "captain_iglues_treasure_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_03_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "circle_room_quest",
        "combat_knife_quest",
        "crusader_helmet_quest",
        "crystal_wand_quest",
        "darashia_dragon_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_04_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "dark_armor_quest",
        "dead_archer_quest",
        "deeper_fibula_quest",
        "demon_helmet_quest",
        "demona_ring_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_05_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "desert_dungeon_quest",
        "devil_helmet_quest",
        "double_hero_quest",
        "doublet_quest",
        "draconia_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_06_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "dragon_corpse_quest",
        "dragon_tower_quest",
        "edron_goblin_quest",
        "elephant_tusk_quest",
        "explorer_brooch_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_07_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "fanfare_quest",
        "fire_axe_quest",
        "fishing_box_quest",
        "geomancer_quest",
        "ghoul_room_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_08_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "goblin_temple_quest",
        "griffin_shield_quest",
        "heaven_blossom_quest",
        "inukaya_quest",
        "iron_hammer_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_09_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "iron_helmet_quest",
        "iron_ore_quest",
        "isle_of_the_mists_quest",
        "katana_quest",
        "lion_trophy_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_10_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "lone_medusa_quest",
        "longsword_quest",
        "mad_mage_room_quest",
        "medusa_shield_quest",
        "minotaur_hell_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_11_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&[
        "minotaur_leather_quest",
        "mintwallin_cyclops_quest",
        "berserk_potion_quest",
        "frost_dragon_quest",
    ])
}

#[test]
fn quest_catalogue_chests_phase_a_12_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["emperor_s_cookies_quest", "life_ring_quest"])
}

// These story and outfit sources have the same native reward contract. Their
// reward receipts do not certify the missing NPC, arena or access continuation.
#[test]
fn quest_catalogue_chests_botany_arena_beggar_children_cults_use_replay_and_refusal() -> TestResult
{
    phase_a_cohort(&[
        "an_interest_in_botany_quest",
        "barbarian_arena_quest",
        "beggar_outfits_quest",
        "children_of_the_revolution_quest",
        "cults_of_tibia_quest",
    ])
}

#[test]
fn quest_catalogue_chests_dark_trails_dreamer_druid_ferumbras_knowledge_use_replay_and_refusal()
-> TestResult {
    phase_a_cohort(&[
        "dark_trails_quest",
        "dreamer_s_challenge_quest",
        "druid_outfits_quest",
        "ferumbras_ascension_quest",
        "forgotten_knowledge_quest",
    ])
}

#[test]
fn quest_catalogue_chests_hidden_threats_hunter_knight_koshei_use_replay_and_refusal() -> TestResult
{
    phase_a_cohort(&[
        "hidden_threats_quest",
        "hunter_outfits_quest",
        "knight_outfits_quest",
        "koshei_the_deathless_quest",
    ])
}

#[test]
fn quest_catalogue_chests_mintwallin_prison_key3620_use_replay_and_refusal() -> TestResult {
    // TibiaWiki Key 3620 identifies the Mintwallin barracks drawer; the donor's
    // inherited Black Knight section label is not this source's quest identity.
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| case.claim == "oteryn:reward-claim.quest.key.id3620")
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].position, (32411, 32155, -15));
    assert_eq!(selected[0].unique, 5009);
    assert_eq!(selected[0].appearance, 2434);
    assert_eq!(selected[0].root, "oteryn:item.tibia.i2969");
    assert_eq!(selected[0].key_number, Some(3620));
    execute(selected)
}

#[test]
fn quest_catalogue_chests_medusa_exact_crystal_reward_use_and_replay() -> TestResult {
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| case.claim == "oteryn:reward-claim.quest.u7_3.the_medusa_quest")
        .collect();
    assert_eq!(selected.len(), 1);
    execute(selected)
}

#[test]
fn quest_catalogue_chests_full_charge_rewards_use_replay_and_refusal() -> TestResult {
    let claims: BTreeSet<_> = [
        "desert2",
        "draconia1",
        "edrongoblin1",
        "fireaxe1",
        "ghoulroom1",
        "lifering",
        "powerring2",
        "silveramulet",
        "stealhring1",
        "timering3",
        "triangletower",
        "trollcave1",
        "weddingring1",
    ]
    .into_iter()
    .map(|suffix| format!("oteryn:reward-claim.kv.{suffix}"))
    .collect();
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| claims.contains(&case.claim))
        .collect();
    assert_eq!(
        selected
            .iter()
            .map(|case| &case.claim)
            .collect::<BTreeSet<_>>(),
        claims.iter().collect(),
        "every newly admitted full-charge claim needs a real USE scenario"
    );
    execute(selected)
}

#[test]
fn quest_catalogue_chests_exact_partial_charges_use_replay_and_refusal() -> TestResult {
    let selected: Vec<_> = cases_including(|claim| !expected_charges(claim).is_empty())?
        .into_iter()
        .filter(|case| !case.charges.is_empty())
        .collect();
    assert_eq!(
        selected.len(),
        5,
        "all five exact partial-charge source claims need USE proof"
    );
    execute(selected)
}

#[test]
fn quest_catalogue_chests_exact_fluid_subtypes_use_replay_and_refusal() -> TestResult {
    let selected: Vec<_> = cases_including(|claim| !expected_fluids(claim).is_empty())?
        .into_iter()
        .filter(|case| !case.fluids.is_empty())
        .collect();
    assert_eq!(
        selected.len(),
        6,
        "milk, four wine bags and water present need USE proof"
    );
    execute(selected)
}
