//! Catalogue-wide reward qualification through the production USE and durable owners.
//! This proves the reward at its source selector, not the quest's access path.
use super::TestResult;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) type CatalogueActivation = (
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
);

pub(super) struct CatalogueChest {
    pub claim: String,
    pub position: (u16, u16, i8),
    pub unique: u16,
    pub servable: bool,
    pub appearance: u16,
    pub root: String,
    pub quantity: u32,
    pub contents: Vec<(String, i64, i32)>,
    pub texts: Vec<(String, String)>,
    pub key_number: Option<i32>,
    pub achievement: bool,
    pub charges: Vec<(String, i32)>,
    pub fluids: Vec<(String, i32)>,
}

fn string(value: &Value) -> TestResult<String> {
    Ok(value.as_str().ok_or("catalogue string")?.into())
}

fn cases() -> TestResult<Vec<CatalogueChest>> {
    cases_including(|_| false)
}

fn cases_including(include: impl Fn(&str) -> bool) -> TestResult<Vec<CatalogueChest>> {
    let coverage: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    let mut supported: BTreeSet<_> = coverage["quests"]
        .as_array()
        .ok_or("coverage quests")?
        .iter()
        .flat_map(|quest| quest["chests"].as_array().into_iter().flatten())
        .filter(|chest| chest["status"] != "not_playable")
        .map(|chest| string(&chest["claim"]))
        .collect::<TestResult<_>>()?;
    // Random repeat sources have their own complete production USE scenarios;
    // the generic once-only recipe harness cannot represent their cycle owner.
    for chest in oteryn_gameplay_quests::quest::falcon::CHESTS {
        assert!(
            supported.remove(&chest.key()),
            "Falcon source needs cycle coverage"
        );
    }
    let mut found = BTreeSet::new();
    let mut served = BTreeSet::new();
    let mut cases = Vec::new();
    for shard in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
        let shard: Value = serde_json::from_str(shard)?;
        for row in shard["records"].as_array().ok_or("claim records")? {
            let definition = &row["definition"];
            let claim = string(&definition["identity"]["key"])?;
            // A new source slice must reach the production binder before its
            // coverage report can claim qualification. The broad sweep still
            // requires every previously supported claim exactly as before.
            if include(&claim) {
                supported.insert(claim.clone());
            }
            if !supported.contains(&claim) {
                continue;
            }
            assert!(found.insert(claim.clone()), "duplicate claim {claim}");
            for (index, placement) in definition["placements"]
                .as_array()
                .ok_or("claim placements")?
                .iter()
                .enumerate()
            {
                let binding = &placement["source_binding"];
                let position = &binding["project_position"];
                let ids = binding["legacy_unique_ids"]
                    .as_array()
                    .ok_or("source unique ids")?;
                let crystal = ids
                    .iter()
                    .find(|source| source["server"] == "crystalserver");
                let unique = crystal.or_else(|| ids.first()).ok_or("source selector")?["unique_id"]
                    .as_u64()
                    .ok_or("source unique id")?;
                if crystal.is_some() {
                    served.insert(claim.clone());
                }
                let reward = &placement["reward"];
                let native = definition["native_contents"]["placements"].get(index);
                let (root, quantity, contents) = if let Some(native) = native {
                    let children = native["contents"]
                        .as_array()
                        .ok_or("native contents")?
                        .iter()
                        .map(|child| {
                            Ok((
                                string(&child["item"]["key"])?,
                                child["count"].as_i64().ok_or("child count")?,
                                i32::try_from(child["key_number"].as_i64().unwrap_or(0))?,
                            ))
                        })
                        .collect::<TestResult<Vec<_>>>()?;
                    (string(&native["root"]["key"])?, 1, children)
                } else {
                    let items = reward["items"].as_array().ok_or("leaf items")?;
                    assert_eq!(items.len(), 1, "unsupported leaf shape {claim}");
                    (
                        string(&items[0]["item"]["key"])?,
                        u32::try_from(items[0]["count"].as_u64().ok_or("leaf count")?)?,
                        Vec::new(),
                    )
                };
                let mut texts = Vec::new();
                if let Some(native) = native {
                    if let Some(text) = native["root_text"].as_str() {
                        texts.push((root.clone(), text.into()));
                    }
                    for child in native["contents"].as_array().ok_or("native contents")? {
                        if let Some(text) = child["text"].as_str() {
                            texts.push((string(&child["item"]["key"])?, text.into()));
                        }
                    }
                }
                let key_number = reward["key_binding"]
                    .as_str()
                    .filter(|_| reward.get("container").is_none())
                    .map(|binding| -> TestResult<i32> {
                        Ok(binding
                            .rsplit('/')
                            .next()
                            .ok_or("key binding")?
                            .parse::<i32>()?)
                    })
                    .transpose()?;
                cases.push(CatalogueChest {
                    claim: claim.clone(),
                    position: (
                        u16::try_from(position["x"].as_u64().ok_or("source x")?)?,
                        u16::try_from(position["y"].as_u64().ok_or("source y")?)?,
                        -i8::try_from(position["z"].as_u64().ok_or("source z")?)?,
                    ),
                    unique: u16::try_from(unique)?,
                    servable: crystal.is_some(),
                    appearance: u16::try_from(
                        placement["appearance_tibia_id"]
                            .as_u64()
                            .ok_or("source appearance")?,
                    )?,
                    root,
                    quantity,
                    contents,
                    texts,
                    key_number,
                    achievement: native.is_some_and(|native| !native["achievement"].is_null()),
                    charges: expected_charges(&claim),
                    fluids: expected_fluids(&claim),
                });
            }
        }
    }
    assert_eq!(found, supported, "every supported claim needs a scenario");
    assert_eq!(
        served, supported,
        "every supported claim needs a native-source placement"
    );
    Ok(cases)
}

#[test]
fn quest_catalogue_chests_every_supported_source_placement_use_replay_and_refusal() -> TestResult {
    execute(cases()?)?;
    super::falcon_chests::falcon_chests_postgres_real_use_frozen_draw_refusal_restart_and_fences()
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

// Small cohorts use the same production dispatch scenario as the catalogue
// sweep. Keep every placement and every already-qualified sibling so an early
// checkpoint describes a whole reward set, rather than a partial failed sweep.
// Access puzzles, NPC exchanges and outfits remain separate qualification work.
fn phase_a_cohort(quests: &[&str]) -> TestResult {
    let coverage: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    let mut found = BTreeSet::new();
    let mut claims = BTreeSet::new();
    let mut placements = 0;
    for quest in coverage["quests"].as_array().ok_or("coverage quests")? {
        let title = quest["quest"]
            .as_str()
            .ok_or("quest title")?
            .rsplit('/')
            .next()
            .ok_or("quest slug")?;
        if !quests.contains(&title) {
            continue;
        }
        assert!(found.insert(title), "duplicate quest {title}");
        for chest in quest["chests"].as_array().ok_or("quest chests")? {
            assert_ne!(chest["status"], "not_playable", "held reward in {title}");
            assert!(
                claims.insert(string(&chest["claim"])?),
                "duplicate cohort claim"
            );
            placements += chest["placements"]
                .as_array()
                .ok_or("coverage placements")?
                .len();
        }
    }
    assert_eq!(
        found,
        quests.iter().copied().collect(),
        "every cohort quest must be authored"
    );
    assert!(!claims.is_empty(), "a cohort must exercise actual rewards");
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| claims.contains(&case.claim))
        .collect();
    assert_eq!(
        selected.len(),
        placements,
        "every source placement must execute"
    );
    assert_eq!(
        selected
            .iter()
            .map(|case| &case.claim)
            .collect::<BTreeSet<_>>(),
        claims.iter().collect(),
        "every reward must execute"
    );
    execute(selected)
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

fn expected_charges(claim: &str) -> Vec<(String, i32)> {
    let ids: &[u16] = match claim.strip_prefix("oteryn:reward-claim.") {
        Some("kv.deeperfibula4") => &[3082],
        Some("quest.u7_2.the_queen_of_the_banshees.reward.stone_skin_amulet") => &[3081],
        Some("quest.u15_24.targuna.silver_amulet_chest") => &[3054],
        Some("quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield") => &[3048, 3085],
        Some("kv.ornamentedshield") => &[3083],
        _ => &[],
    };
    ids.iter()
        .map(|id| (format!("oteryn:item.tibia.i{id}"), 1))
        .collect()
}

fn expected_fluids(claim: &str) -> Vec<(String, i32)> {
    match claim.strip_prefix("oteryn:reward-claim.") {
        Some("kv.goblintemple2") => vec![("oteryn:item.tibia.i2874".into(), 9)],
        Some("quest.u7_9.dreamers_challenge.chests_wine") => {
            vec![("oteryn:item.tibia.i2874".into(), 2)]
        }
        Some("kv.present_box") => vec![
            ("oteryn:item.tibia.i2881".into(), 1),
            ("oteryn:item.tibia.i2882".into(), 1),
        ],
        _ => Vec::new(),
    }
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

pub(super) async fn assert_charges(
    pool: &sqlx::PgPool,
    root: [u8; 16],
    chest: &CatalogueChest,
) -> TestResult {
    let actual: Vec<(String, i32, i64, bool, bool, bool)> = sqlx::query_as(
        "SELECT i.definition_production_key,t.charges,t.state_revision::bigint, \
            t.remaining_ms IS NULL,t.deadline_at IS NULL,t.xmin=r.xmin \
         FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) \
         JOIN game_reward_claim_mint_receipts r ON r.item_instance_id=encode($1,'hex')::uuid \
         WHERE i.item_instance_id=r.item_instance_id OR i.item_instance_id IN \
            (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=r.item_instance_id) \
         ORDER BY i.definition_production_key",
    )
    .bind(root.as_slice())
    .fetch_all(pool)
    .await?;
    let expected: Vec<_> = chest
        .charges
        .iter()
        .map(|(key, count)| (key.clone(), *count, 1, true, true, true))
        .collect();
    assert_eq!(
        actual, expected,
        "exact source charge state, minted atomically and stable on replay"
    );
    let (reserved, committed): (Value, Value) = sqlx::query_as(
        "SELECT v.charges_plan,r.charges_plan FROM game_reward_claim_mint_receipts r \
         JOIN game_reward_claim_mint_reservations v USING(game_session_id,command_id) \
         WHERE r.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(root.as_slice())
    .fetch_one(pool)
    .await?;
    let mut expected_plan = Vec::new();
    for (key, charges) in &chest.charges {
        let ordinal = if *key == chest.root {
            0
        } else {
            chest
                .contents
                .iter()
                .position(|(child, _, _)| child == key)
                .ok_or("charged child absent")?
                + 1
        };
        expected_plan.push(serde_json::json!({"ordinal": ordinal, "charges_remaining": charges}));
    }
    expected_plan.sort_by_key(|value| value["ordinal"].as_u64());
    assert_eq!(committed, Value::Array(expected_plan));
    assert_eq!(
        reserved, committed,
        "reservation binds the exact complete charge plan"
    );
    if !chest.charges.is_empty() {
        let shape: (bool, bool, bool, bool, bool) = sqlx::query_as(
            "SELECT game_reward_claim_charges_plan_valid(r.charges_plan,r.contents_plan), \
                game_reward_claim_charges_plan_valid(r.charges_plan#-'{0,charges_remaining}',r.contents_plan), \
                game_reward_claim_charges_plan_valid(jsonb_set(r.charges_plan,'{0,charges_remaining}','2'),r.contents_plan), \
                game_reward_claim_charges_plan_valid(jsonb_set(r.charges_plan,'{0,ordinal}','-1'),r.contents_plan), \
                game_reward_claim_charges_plan_valid(r.charges_plan||r.charges_plan,r.contents_plan) \
             FROM game_reward_claim_mint_receipts r WHERE r.item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(root.as_slice())
        .fetch_one(pool)
        .await?;
        assert_eq!(shape, (true, false, false, false, false));
        for mutation in [
            "UPDATE game_reward_claim_mint_reservations SET charges_plan='[]'::jsonb WHERE item_instance_id=encode($1,'hex')::uuid",
            "UPDATE game_item_timed_states SET charges=2,state_revision=state_revision+1 WHERE item_instance_id=encode($1,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)",
        ] {
            let changed = sqlx::query(mutation)
                .bind(root.as_slice())
                .execute(pool)
                .await;
            assert!(
                matches!(changed, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("23514")),
                "charge initialization never permits later plan substitution or unfenced inflation"
            );
        }
    }
    Ok(())
}

/// The binder admits a claim only when every source placement is present.
/// Preserve all sibling selectors even while exercising one clicked chest.
pub(super) fn catalogue_source_world(
    chest: &CatalogueChest,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    use std::collections::BTreeMap;
    let mut placements = Vec::new();
    for shard in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
        let shard: Value = serde_json::from_str(shard)?;
        for row in shard["records"].as_array().ok_or("claim records")? {
            let definition = &row["definition"];
            if definition["identity"]["key"] != chest.claim {
                continue;
            }
            for source in definition["placements"]
                .as_array()
                .ok_or("claim placements")?
            {
                let binding = &source["source_binding"];
                let unique = binding["legacy_unique_ids"]
                    .as_array()
                    .ok_or("source ids")?
                    .iter()
                    .find(|id| id["server"] == "crystalserver")
                    .ok_or("native source")?;
                let position = &binding["project_position"];
                placements.push((
                    u16::try_from(position["x"].as_u64().ok_or("source x")?)?,
                    u16::try_from(position["y"].as_u64().ok_or("source y")?)?,
                    -i8::try_from(position["z"].as_u64().ok_or("source z")?)?,
                    u16::try_from(unique["unique_id"].as_u64().ok_or("source uid")?)?,
                    u16::try_from(
                        source["appearance_tibia_id"]
                            .as_u64()
                            .ok_or("source appearance")?,
                    )?,
                ));
            }
        }
    }
    assert!(
        !placements.is_empty(),
        "every catalogue claim has placed sources"
    );
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut appearances = BTreeMap::new();
    let mut tiles = BTreeMap::<(i8, u16, u16), sector::Tile>::new();
    for (x, y, floor, unique, appearance) in placements {
        let palette = if let Some(index) = appearances.get(&appearance) {
            *index
        } else {
            let index = u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: bundle::Family::Terrain,
                id: index,
                terrain: Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                }),
            });
            appearances.insert(appearance, index);
            index
        };
        for tile_x in x.saturating_sub(2)..=x {
            tiles
                .entry((floor, y, tile_x))
                .or_insert_with(|| sector::Tile {
                    x: tile_x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items: vec![sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: sector::Attrs::default(),
                    }],
                });
        }
        tiles
            .get_mut(&(floor, y, x))
            .ok_or("source tile")?
            .items
            .push(sector::Item {
                palette,
                depth: 0,
                attrs: sector::Attrs {
                    unique: Some(unique),
                    ..Default::default()
                },
            });
    }
    let coordinates: Vec<_> = tiles.keys().copied().collect();
    data.manifest.world = bundle::Extent {
        min_x: coordinates
            .iter()
            .map(|(_, _, x)| *x)
            .min()
            .ok_or("source minimum x")?,
        min_y: coordinates
            .iter()
            .map(|(_, y, _)| *y)
            .min()
            .ok_or("source minimum y")?,
        max_x: coordinates
            .iter()
            .map(|(_, _, x)| *x)
            .max()
            .ok_or("source maximum x")?
            + 1,
        max_y: coordinates
            .iter()
            .map(|(_, y, _)| *y)
            .max()
            .ok_or("source maximum y")?
            + 1,
        floors: coordinates
            .iter()
            .map(|(floor, _, _)| *floor)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    let mut grouped = BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    for ((floor, y, x), tile) in tiles {
        grouped
            .entry((floor, x / 32, y / 32))
            .or_default()
            .push(tile);
    }
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((floor, sx, sy), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(super::fixture::debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: chest.position.0 - 1,
                y: chest.position.1,
                floor: chest.position.2,
            },
        },
        crate::foundation::WorldId::decode(&super::fixture::id(42))
            .map_err(super::fixture::debug)?,
        crate::foundation::ChannelId::decode(&super::fixture::id(43))
            .map_err(super::fixture::debug)?,
        |_| None,
    )
    .map_err(super::fixture::debug)
}

fn execute(cases: Vec<CatalogueChest>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            // Every case has an isolated durable world/session, using the same
            // immutable production content generation. Qualify that content once;
            // no mutable character, command or receipt state is shared.
            let world = crate::foundation::WorldId::decode(&super::fixture::id(42))
                .map_err(|error| format!("{error:?}"))?;
            let activation =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")
                    .map_err(|error| error.to_string())?;
            let generation = activation.0.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native generation")?;
            for case in cases {
                for key in std::iter::once(case.root.as_str())
                    .chain(case.contents.iter().map(|(key, _, _)| key.as_str()))
                {
                    let definition = crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(), production_key: key.into(), revision_ref: "definition-r1".into(),
                    };
                    let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(native, &definition)
                        .map_err(|error| format!("{} {key}: {error:?}", case.claim))?;
                    if matches!(facts.stack, crate::durability::item_transfer::ItemStackClass::Unknown) {
                        return Err(format!("{} {key}: native Item admission missing", case.claim));
                    }
                }
                let started = std::time::Instant::now();
                super::quest_container_chests::catalogue_scenario(&case, &activation)
                    .map_err(|error| format!("{} {:?}: {error}", case.claim, case.position))?;
                println!(
                    "QUEST_CATALOGUE_CHEST POSTGRES_CASE_COMPLETED {} {:?} native_source={} duration={:?}",
                    case.claim,
                    case.position,
                    case.servable,
                    started.elapsed()
                );
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "catalogue chest scenario panicked")??;
    Ok(())
}
