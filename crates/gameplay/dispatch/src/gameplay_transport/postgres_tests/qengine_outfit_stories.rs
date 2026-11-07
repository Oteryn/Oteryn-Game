//! Complete outfit reward stories: real conversations, exact custody and parent ownership.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue,
    QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;

const LIBRARY: &str = "oteryn:quest-progress/quest/u11_80/the_secret_library/";
const FEATHERS: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/feathers";

#[test]
fn qengine_l2_battle_mage_whole_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-battle-mage-story")
}
#[test]
fn qengine_l2_festive_whole_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-festive-story")
}
#[test]
fn qengine_l2_elementalist_whole_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-elementalist-story")
}

#[test]
fn qengine_l3_demon_whole_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l3-demon-story")
}
#[test]
fn qengine_l3_glooth_engineer_high_votes_preserve_source_priority() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l3-glooth-engineer-story")
}
#[test]
fn qengine_l3_dark_trails_base_and_glooth_addons_follow_live_votes() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l3-dark-trails-story")
}

#[test]
fn qengine_l4_nobleman_paid_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l4-nobleman-story")
}

#[test]
fn qengine_l4_royal_costume_full_token_exchange_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l4-royal-costume-story")
}

#[test]
fn qengine_l4_pirate_full_source_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l4-pirate-story")
}

#[test]
fn qengine_l5_oriental_female_complete_source_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l5-oriental-story")
}

#[test]
fn qengine_l5_norseman_and_formorgar_complete_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l5-norseman-story")
}

#[test]
fn qengine_l6_druid_rook_complete_available_body_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l6-druid-rook-story")
}

#[test]
fn qengine_l6_citizen_rook_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l6-citizen-rook-story")
}

#[test]
fn qengine_l6_beggar_female_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l6-beggar-female-story")
}
#[test]
fn qengine_l6_beggar_male_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l6-beggar-male-story")
}

const ICE_ISLANDS: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/";

const ORC_POWDER: &str = "oteryn:quest-progress/quest/u7_6/explorer_society/the_orc_powder";

const SHATTERED: &str = "oteryn:quest-progress/quest/u7_8/the_shattered_isles/";

const ANNIHILATOR: &str = "oteryn:quest-progress/quest/u7_24/the_annihilator/reward";
const DEMON_HELMET: &str = "oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet";
const DEMON_OAK: &str = "oteryn:quest-progress/quest/u8_2/the_demon_oak/done";
const DARK: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/";
const VOTES: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Requirement {
    track: String,
    minimum: i64,
    maximum: i64,
}

fn requirements(callback: &RewardCallback) -> TestResult<Vec<Requirement>> {
    let json = serde_json::to_value(callback)?;
    Ok(serde_json::from_value(
        json.get("requires")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    )?)
}

type Counts = (i64, i64, i64, i64);
type StoryStep = (
    &'static str,
    Option<(u16, u16)>,
    Option<(u16, u16)>,
    Option<u8>,
);
async fn counts(h: &Harness) -> TestResult<Counts> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_account_appearance_unlock_receipts)").fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(item).fetch_one(&h.pool).await?)
}

async fn coin_worth(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity * CASE definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 END),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN('oteryn:item.tibia.i3031','oteryn:item.tibia.i3035','oteryn:item.tibia.i3043')").fetch_one(&h.pool).await?)
}

async fn commit_fixture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    ordinal: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(ordinal).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "fixture transition: {result:?}"
    );
    Ok(())
}

// Supply real admitted stacks/instances into closed bags. Books, masks and
// epaulettes are non-stackable; never fabricate illegal multi-quantity instances.
async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    item: &oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount,
    bags: &mut Vec<(sqlx::types::Uuid, u32)>,
    occurrence: &mut u64,
) -> TestResult {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::ItemStackClass;
    use crate::gameplay_transport::item_ref_admission::generation_item_facts;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let definition = |key: &str| TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    };
    let facts = generation_item_facts(native, &definition(&item.item)).map_err(debug)?;
    let maximum = match facts.stack {
        ItemStackClass::NonStackable => 1,
        ItemStackClass::Stackable { proven_maximum } => proven_maximum.unwrap_or(100),
        ItemStackClass::Unknown => return Err("unadmitted recipe item".into()),
    };
    let capacity = generation_item_facts(native, &definition("oteryn:item.tibia.i2854"))
        .map_err(debug)?
        .container_capacity
        .ok_or("bag capacity")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let main: sqlx::types::Uuid = sqlx::query_scalar("SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    // Reuse physical free slots after prior source hand-ins consumed their stacks.
    for (bag, used) in bags.iter_mut() {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=$1 UNION ALL SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=$1) entries")
            .bind(*bag).fetch_one(&h.pool).await?;
        *used = u32::try_from(count)?;
    }
    let mut remaining = u32::from(item.count) + if item.count > 1 { 1 } else { 0 };
    while remaining > 0 {
        let target = if let Some(index) = bags.iter().position(|(_, used)| *used < capacity) {
            index
        } else {
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?;
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    position: Position {
                        x: 33293,
                        y: 31677,
                        floor: -15,
                    },
                    item: "oteryn:item.tibia.i2854".into(),
                    count: 1,
                },
                *occurrence,
                true,
            )
            .await?;
            *occurrence += 1;
            let candidates: Vec<sqlx::types::Uuid> = sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=$1 AND i.definition_production_key='oteryn:item.tibia.i2854' ORDER BY i.item_instance_id LIMIT 20").bind(main).fetch_all(&h.pool).await?;
            let bag = candidates
                .into_iter()
                .find(|id| !bags.iter().any(|(known, _)| known == id))
                .ok_or("new recipe bag")?;
            bags.push((bag, 0));
            bags.len() - 1
        };
        let count = u16::try_from(remaining.min(maximum))?;
        if count == 0 {
            return Err("invalid native stack maximum".into());
        }
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: item.item.clone(),
                count,
            },
            *occurrence,
            true,
        )
        .await?;
        *occurrence += 1;
        let instance: sqlx::types::Uuid = sqlx::query_scalar("SELECT i.item_instance_id FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=$1 AND i.definition_production_key=$2 LIMIT 1").bind(main).bind(&item.item).fetch_one(&h.pool).await?;
        super::qengine_rewards::into_bag(
            owner,
            admitted,
            *instance.as_bytes(),
            *bags[target].0.as_bytes(),
            20000 + *occurrence,
        )
        .await?;
        bags[target].1 += 1;
        remaining -= u32::from(count);
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let (family, steps, look): (&str, &[StoryStep], u16) = match name {
        "qengine-rewards-l2-battle-mage-story" => (
            "battle-mage",
            &[
                ("base", None, None, Some(0)),
                ("book", Some((28792, 5)), None, Some(1)),
                ("epaulettes", Some((28793, 20)), None, Some(2)),
            ],
            1070,
        ),
        "qengine-rewards-l2-festive-story" => (
            "festive",
            &[
                ("base", None, None, Some(0)),
                ("mask-start", None, Some((25088, 3)), None),
                ("mask", Some((25088, 3)), None, Some(1)),
                ("feather-start", None, Some((25089, 50)), None),
                ("feather", Some((25089, 50)), None, Some(2)),
            ],
            929,
        ),
        "qengine-rewards-l2-elementalist-story" => (
            "elementalist",
            &[
                ("start", None, None, None),
                ("base", Some((12601, 20)), None, Some(0)),
                ("cap", Some((12599, 1)), None, Some(1)),
                ("staff", Some((12803, 1)), None, Some(2)),
            ],
            433,
        ),
        "qengine-rewards-l3-demon-story" => (
            "demon",
            &[
                ("base", None, None, Some(0)),
                ("shield", None, None, Some(1)),
                ("helmet", None, None, Some(2)),
            ],
            542,
        ),
        "qengine-rewards-l3-glooth-engineer-story" => (
            "glooth-engineer",
            &[
                ("base", None, None, Some(0)),
                ("second", None, None, Some(2)),
                ("first-high", None, None, Some(1)),
            ],
            618,
        ),
        "qengine-rewards-l3-dark-trails-story" => (
            "glooth-engineer",
            &[
                ("base", None, None, Some(0)),
                ("first-low", None, None, Some(1)),
                ("second", None, None, Some(2)),
            ],
            618,
        ),
        "qengine-rewards-l4-nobleman-story" => (
            "nobleman",
            &[
                ("first-start", None, None, None),
                ("first", None, None, Some(1)),
                ("second-start", None, None, None),
                ("second", None, None, Some(2)),
            ],
            140,
        ),
        "qengine-rewards-l4-royal-costume-story" => (
            "royal-costume",
            &[
                ("base", Some((22516, 15000)), Some((22721, 12500)), Some(0)),
                ("shield", Some((22516, 7500)), Some((22721, 6250)), Some(1)),
                ("crown", Some((22516, 7500)), Some((22721, 6250)), Some(2)),
            ],
            1456,
        ),
        "qengine-rewards-l4-pirate-story" => (
            "pirate",
            &[
                ("base", None, None, Some(0)),
                ("sabre-start", None, None, None),
                ("eye-patches", Some((6098, 100)), None, None),
                ("peg-legs", Some((6126, 100)), None, None),
                ("hooks", Some((6097, 100)), None, None),
                ("sabre", None, None, Some(1)),
                ("hat-start", None, None, None),
                ("hat", None, None, Some(2)),
            ],
            155,
        ),
        "qengine-rewards-l5-oriental-story" => (
            "oriental",
            &[
                ("female-first-start", None, None, None),
                ("female-comb", Some((5945, 1)), None, Some(1)),
                ("female-second-start", None, None, None),
                ("female-ape-fur", Some((5883, 100)), None, None),
                ("female-fish-fins", Some((5895, 100)), None, None),
                ("female-wings", Some((5891, 2)), None, None),
                ("female-blue-cloth", Some((5912, 100)), None, Some(2)),
            ],
            150,
        ),
        "qengine-rewards-l5-norseman-story" => (
            "norseman",
            &[
                ("base", Some((7289, 1)), None, Some(0)),
                ("first-shards", Some((7290, 5)), None, None),
                ("first-reward", None, None, Some(1)),
                ("second-shards", Some((7290, 10)), None, None),
                ("second-reward", None, None, Some(2)),
            ],
            252,
        ),
        "qengine-rewards-l6-druid-rook-story" => (
            "druid-rook",
            &[
                ("start", None, None, None),
                ("body", Some((5896, 50)), Some((5897, 50)), Some(1)),
            ],
            148,
        ),
        "qengine-rewards-l6-beggar-female-story" => (
            "beggar",
            &[
                ("start", None, None, None),
                ("cloth", Some((5913, 20)), None, None),
                ("leather", Some((5878, 50)), None, None),
                ("wings", Some((5894, 10)), None, None),
                ("blossoms", Some((5921, 30)), None, None),
                ("base", None, None, Some(0)),
                ("first-start", None, None, None),
                ("first-materials", Some((5883, 100)), None, None),
                ("dress", None, None, Some(1)),
                ("staff-start", None, None, None),
                ("staff", Some((6107, 1)), None, Some(2)),
            ],
            157,
        ),
        "qengine-rewards-l6-beggar-male-story" => (
            "beggar",
            &[
                ("start", None, None, None),
                ("cloth", Some((5913, 20)), None, None),
                ("leather", Some((5878, 50)), None, None),
                ("wings", Some((5894, 10)), None, None),
                ("blossoms", Some((5921, 30)), None, None),
                ("base", None, None, Some(0)),
                ("first-start", None, None, None),
                ("first-materials", Some((5883, 100)), None, None),
                ("beard-start", None, None, None),
                ("beard", None, None, Some(1)),
                ("staff-start", None, None, None),
                ("staff", Some((6107, 1)), None, Some(2)),
            ],
            153,
        ),
        "qengine-rewards-l6-citizen-rook-story" => (
            "citizen-rook",
            &[
                ("backpack-start", None, None, None),
                ("backpack-materials", Some((5878, 100)), None, None),
                ("backpack", None, None, Some(1)),
                ("hat-start", None, None, None),
                ("hat", None, None, Some(2)),
            ],
            136,
        ),
        _ => return Err("unknown outfit story".into()),
    };
    let outfit_family = if family == "druid-rook" {
        "druid".to_owned()
    } else if family == "citizen-rook" {
        "citizen".to_owned()
    } else {
        family.replace('-', "_")
    };
    // Independent story expectations determine earned bits, not the production callbacks.
    let expected_addons = steps
        .iter()
        .filter_map(|(_, _, _, addon)| *addon)
        .fold(0, |mask, addon| mask | addon);
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let mut callbacks = Vec::new();
    for (suffix, take, present, addon) in steps {
        let batch = if name.starts_with("qengine-rewards-l6-") {
            "l6"
        } else if name.starts_with("qengine-rewards-l5-") {
            "l5"
        } else if name.starts_with("qengine-rewards-l4-") {
            "l4"
        } else if name.starts_with("qengine-rewards-l3-") {
            "l3"
        } else {
            "l2"
        };
        let key = if family == "pirate" && *suffix == "sabre" {
            "oteryn:quest-reward/pirate-sabre".into()
        } else {
            format!("oteryn:quest-reward/{batch}-{family}-{suffix}")
        };
        let callback = all
            .iter()
            .find(|c| c.key == key)
            .cloned()
            .ok_or_else(|| format!("missing story reward {key}"))?;
        let amount =
            |item: &(u16, u16)| oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount {
                item: format!("oteryn:item.tibia.i{}", item.0),
                count: item.1,
            };
        let mut expected_take = take.iter().map(amount).collect::<Vec<_>>();
        if family == "citizen-rook" && *suffix == "hat" {
            expected_take = [(3374, 1), (5902, 50), (5890, 100)]
                .iter()
                .map(amount)
                .collect();
        }
        if family == "pirate" && *suffix == "hat" {
            expected_take = [(6101, 1), (6102, 1), (6100, 1), (6099, 1)]
                .iter()
                .map(amount)
                .collect();
        }
        if matches!(family, "royal-costume" | "druid-rook") {
            expected_take.extend(present.iter().map(amount));
        }
        assert_eq!(callback.take, expected_take);
        assert_eq!(
            callback.present,
            if matches!(family, "royal-costume" | "druid-rook") {
                vec![]
            } else {
                present.iter().map(amount).collect::<Vec<_>>()
            }
        );
        assert_eq!(callback.outfit.as_ref().map(|o| o.addons), *addon);
        if family == "beggar" && *suffix == "first-materials" {
            assert_eq!(
                serde_json::to_value(&callback)?["fee"],
                serde_json::json!({"gold":20000,"include_bank":true})
            );
            assert!(callback.requires_premium);
        }
        if family == "beggar" && *suffix == "first-start" {
            assert_eq!(callback.requires_outfits, vec!["oteryn:outfit.beggar"]);
        }
        if family == "nobleman" && addon.is_some() {
            let value = serde_json::to_value(&callback)?;
            assert_eq!(
                value.get("fee"),
                Some(&serde_json::json!({"gold":150000,"include_bank":false})),
                "exact source carried-money charge"
            );
        }
        if family == "battle-mage" && *suffix == "base" {
            assert_eq!(
                callback.achievement.as_deref(),
                Some("oteryn:achievement/battle_mage")
            );
        }
        let expected = if family == "battle-mage" && *suffix != "base" {
            vec![Requirement {
                track: format!("{LIBRARY}library_permission"),
                minimum: 7,
                maximum: 7,
            }]
        } else if family == "festive" && *suffix != "base" {
            vec![Requirement {
                track: FEATHERS.into(),
                minimum: 2,
                maximum: 2,
            }]
        } else if family == "demon" && *suffix != "base" {
            vec![Requirement {
                track: ANNIHILATOR.into(),
                minimum: 2,
                maximum: 2,
            }]
        } else if family == "pirate" && *suffix == "base" {
            vec![
                Requirement {
                    track: format!("{SHATTERED}access_to_laguna_island"),
                    minimum: 1,
                    maximum: 1,
                },
                Requirement {
                    track: format!("{SHATTERED}rays_mission4"),
                    minimum: 5,
                    maximum: 5,
                },
            ]
        } else if family == "norseman" && *suffix == "base" {
            (1..=4)
                .map(|n| Requirement {
                    track: format!("{ICE_ISLANDS}obelisk0{n}"),
                    minimum: 5,
                    maximum: 5,
                })
                .collect()
        } else if family == "oriental" && suffix.ends_with("first-start") {
            vec![Requirement {
                track: ORC_POWDER.into(),
                minimum: 35,
                maximum: i64::from(i32::MAX),
            }]
        } else if family == "glooth-engineer" {
            if *suffix == "base" {
                vec![Requirement {
                    track: format!("{DARK}mission18"),
                    minimum: 1,
                    maximum: 1,
                }]
            } else {
                let mut expected = vec![
                    Requirement {
                        track: format!("{DARK}outfit"),
                        minimum: 1,
                        maximum: 1,
                    },
                    Requirement {
                        track: VOTES.into(),
                        minimum: if *suffix == "first-low" { 3 } else { 6 },
                        maximum: if *suffix == "first-low" {
                            5
                        } else {
                            i64::from(i32::MAX)
                        },
                    },
                ];
                if *suffix == "first-high" {
                    expected.push(Requirement {
                        track: "oteryn:quest-progress/quest/u10_50/glooth_engineer_outfits/addon2"
                            .into(),
                        minimum: 1,
                        maximum: 1,
                    });
                }
                expected
            }
        } else {
            vec![]
        };
        assert_eq!(
            requirements(&callback)?,
            expected,
            "exact source prerequisites"
        );
        callbacks.push(callback);
    }
    if family == "oriental" {
        for callback in &callbacks {
            assert_eq!(
                serde_json::to_value(callback)?
                    .get("creation_sex")
                    .and_then(|v| v.as_str()),
                Some("female")
            );
        }
    }
    // Build one fixed catalogue for the whole story. Only external prerequisite
    // quests are arranged; the outfit's own stages run through actual NPC commands.
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    let mut readonly = None;
    for callback in &callbacks {
        let required = requirements(callback)?;
        if readonly.is_none() {
            readonly = required.first().cloned();
        }
        let keys = std::iter::once(callback.key.clone()).chain(
            callback.participants.iter().enumerate().map(|(i, _)| {
                oteryn_gameplay_quests::quest::reward_callbacks::participant_key(&callback.key, i)
            }),
        );
        for key in keys {
            let transition = canonical
                .transition(&key)
                .cloned()
                .ok_or("canonical transition")?;
            for effect in &transition.effects {
                tracks.insert(
                    effect.track.clone(),
                    canonical
                        .track(&effect.track)
                        .cloned()
                        .ok_or("effect owner track")?,
                );
            }
            transitions.push(transition);
        }
        for requirement in required {
            tracks.insert(
                requirement.track.clone(),
                canonical
                    .track(&requirement.track)
                    .cloned()
                    .ok_or("prerequisite track")?,
            );
        }
    }
    let mut clock_fixtures = BTreeMap::new();
    for callback in &callbacks {
        for operation in &callback.state_ops {
            if let oteryn_gameplay_quests::quest::reward_callbacks::StateCondition::ElapsedAtLeast { seconds } = operation.from {
                let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint").fetch_one(&h.pool).await?;
                let key=format!("oteryn:quest-transition/qengine2-story/clock-advance-{}",clock_fixtures.len());
                transitions.push(QuestTransition {key:key.clone(),quest:callback.quest.clone(),
                    effects:vec![QuestEffect {track:operation.track.clone(),from:QuestComparison::Any,
                        effect:QuestEffectKind::Set(now-i64::from(seconds)-1)}],completes:false,experience:None});
                clock_fixtures.insert(callback.key.clone(),(operation.track.clone(),seconds,key));
            }
        }
    }
    if family == "oriental" {
        let opposite = all
            .iter()
            .find(|c| c.key == "oteryn:quest-reward/l5-oriental-male-first-start")
            .ok_or("male source counterpart")?;
        transitions.push(
            canonical
                .transition(&opposite.key)
                .cloned()
                .ok_or("male transition")?,
        );
        for (i, _) in opposite.participants.iter().enumerate() {
            transitions.push(
                canonical
                    .transition(
                        &oteryn_gameplay_quests::quest::reward_callbacks::participant_key(
                            &opposite.key,
                            i,
                        ),
                    )
                    .cloned()
                    .ok_or("male participant")?,
            );
        }
    }
    let initial: Vec<(String, i64)> = if family == "battle-mage" {
        [
            ("small_islands/questline", 4),
            ("liquid_death/questline", 8),
            ("asuras/questline", 7),
            ("falcon_bastion/questline", 3),
            ("darashia/questline", 9),
            ("mo_ta/questline", 8),
            ("library_permission", 6),
        ]
        .into_iter()
        .map(|(key, value)| (format!("{LIBRARY}{key}"), value))
        .collect()
    } else if family == "festive" {
        vec![(FEATHERS.into(), 1)]
    } else if family == "norseman" {
        std::iter::once((format!("{ICE_ISLANDS}questline"), 39))
            .chain((1..=4).map(|n| (format!("{ICE_ISLANDS}obelisk0{n}"), 5)))
            .collect()
    } else if family == "oriental" {
        vec![(ORC_POWDER.into(), 35)]
    } else if family == "pirate" {
        vec![
            (format!("{SHATTERED}access_to_laguna_island"), 1),
            (format!("{SHATTERED}rays_mission4"), 5),
        ]
    } else if family == "demon" {
        vec![
            (ANNIHILATOR.into(), 1),
            (DEMON_HELMET.into(), 1),
            (DEMON_OAK.into(), 3),
        ]
    } else if family == "glooth-engineer" {
        vec![
            (format!("{DARK}mission18"), 1),
            (
                VOTES.into(),
                if name.contains("dark-trails") { 3 } else { 6 },
            ),
        ]
    } else {
        vec![]
    };
    let mut setup = BTreeMap::<String, Vec<QuestEffect>>::new();
    for (key, value) in initial {
        let track = tracks.get(&key).ok_or("setup track")?;
        setup
            .entry(track.quest.clone())
            .or_default()
            .push(QuestEffect {
                track: key,
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(value),
            });
    }
    let mut setup_keys = Vec::new();
    for (index, (quest, effects)) in setup.into_iter().enumerate() {
        let key = format!("oteryn:quest-transition/qengine2-story/setup-{index}");
        setup_keys.push(key.clone());
        transitions.push(QuestTransition {
            key,
            quest,
            effects,
            completes: false,
            experience: None,
        });
    }
    if let Some(required) = &readonly {
        let track = tracks.get(&required.track).ok_or("read track")?;
        assert!(required.minimum > track.min);
        for (key, value) in [
            ("lower", required.minimum - 1),
            ("restore", required.minimum),
        ] {
            transitions.push(QuestTransition {
                key: format!("oteryn:quest-transition/qengine2-story/{key}"),
                quest: track.quest.clone(),
                effects: vec![QuestEffect {
                    track: required.track.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                }],
                completes: false,
                experience: None,
            });
        }
    }
    if name == "qengine-rewards-l3-dark-trails-story" {
        transitions.push(QuestTransition {
            key: "oteryn:quest-transition/qengine2-story/vote-six".into(),
            quest: tracks.get(VOTES).ok_or("votes owner")?.quest.clone(),
            effects: vec![QuestEffect {
                track: VOTES.into(),
                from: QuestComparison::Eq(3),
                effect: QuestEffectKind::Set(6),
            }],
            completes: false,
            experience: None,
        });
    }
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks.values().cloned().collect(),
            transitions,
        )
        .map_err(debug)?,
    ));
    for (index, key) in setup_keys.iter().enumerate() {
        commit_fixture(owner, admitted, key, 9000 + u64::try_from(index)?).await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    // Earn ordinary carrying capacity through the real progression owner for
    // the largest source recipes, preserving every native weight check.
    if matches!(
        family,
        "battle-mage" | "royal-costume" | "pirate" | "oriental"
    ) {
        let carrying_level = if family == "royal-costume" { 200 } else { 50 };
        use crate::durability::character_progression::{
            ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
        };
        let binding = owner
            .progression_binding(admitted.game_session_id)
            .ok_or("progression")?;
        let target = binding
            .policy
            .thresholds
            .iter()
            .find(|t| t.level == carrying_level)
            .ok_or("carrying-level threshold")?
            .minimum_experience
            .get();
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let current = owner
            .root
            .read_character_progression(authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("progression state")?;
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_experience(
                owner.root,
                authority,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(super::fixture::id(105))
                        .map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(
                        target
                            .checked_sub(current.total_experience.get())
                            .ok_or("XP delta")?,
                    ),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
    }
    if family == "oriental" {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
            .with_wheel(owner.premium_coordinator);
        let before = counts(h).await?;
        let outcome = {
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            slot.commit_quest_reward(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: "oteryn:quest-reward/l5-oriental-male-first-start".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(9300).map_err(debug)?,
                    )),
                },
                owner.quest_catalogue.clone().ok_or("catalogue")?,
                crate::durability::reward_claim_mint::QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                },
                &owner.premium_coordinator,
            )
            .await
            .map_err(debug)?
        };
        assert!(
            matches!(
                outcome,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
            ),
            "immutable creation sex must guard the native commit: {outcome:?}"
        );
        assert_eq!(counts(h).await?, before);
        let participants: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_participants")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            participants, 0,
            "wrong-sex source cannot write the foreign owner"
        );
        let bypass = {
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            slot.commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key:
                        oteryn_gameplay_quests::quest::reward_callbacks::participant_key(
                            "oteryn:quest-reward/l5-oriental-female-first-start",
                            0,
                        ),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(9301).map_err(debug)?,
                    )),
                },
                owner.quest_catalogue.clone().ok_or("catalogue")?,
            )
            .await
        };
        assert!(matches!(bypass,Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)),"participant cannot bypass the source reward owner: {bypass:?}");
        assert_eq!(counts(h).await?, before);
    }
    let mut npcs = BTreeMap::new();
    if family == "citizen-rook" {
        let keys = ["oteryn:npc.amber", "oteryn:npc.norma"];
        let actors = super::classic_quests::install(owner, admitted, keys).await?;
        for (key, actor) in keys.into_iter().zip(actors) {
            npcs.insert(key.to_owned(), actor);
        }
    } else if family == "oriental" {
        let keys = ["oteryn:npc.ishina", "oteryn:npc.miraia"];
        let actors = super::classic_quests::install(owner, admitted, keys).await?;
        for (key, actor) in keys.into_iter().zip(actors) {
            npcs.insert(key.to_owned(), actor);
        }
    } else if family == "pirate" {
        let keys = [
            "oteryn:npc.raymond_striker",
            "oteryn:npc.duncan",
            "oteryn:npc.morgan",
            "oteryn:npc.ariella",
        ];
        let actors = super::classic_quests::install(owner, admitted, keys).await?;
        for (key, actor) in keys.into_iter().zip(actors) {
            npcs.insert(key.to_owned(), actor);
        }
    } else if family == "beggar" {
        let keys = ["oteryn:npc.hugo", "oteryn:npc.simon_the_beggar"];
        let actors = super::classic_quests::install(owner, admitted, keys).await?;
        for (key, actor) in keys.into_iter().zip(actors) {
            npcs.insert(key.to_owned(), actor);
        }
    } else if family == "festive" {
        let keys = ["oteryn:npc.the_first_dragon", "oteryn:npc.muriel"];
        let actors = super::classic_quests::install(owner, admitted, keys).await?;
        for (key, actor) in keys.into_iter().zip(actors) {
            npcs.insert(key.to_owned(), actor);
        }
    } else {
        let key = callbacks.first().ok_or("first stage")?.npc.clone();
        let [actor] = super::classic_quests::install(owner, admitted, [key.as_str()]).await?;
        npcs.insert(key, actor);
    }
    let mut checked_stale = false;
    let mut earned_base = false;
    let mut bags = Vec::new();
    let mut occurrence = 500;
    for (step_index, callback) in callbacks.iter().enumerate() {
        let npc = *npcs.get(&callback.npc).ok_or("story NPC actor")?;
        if name == "qengine-rewards-l3-dark-trails-story" && callback.key.ends_with("-second") {
            commit_fixture(
                owner,
                admitted,
                "oteryn:quest-transition/qengine2-story/vote-six",
                9300,
            )
            .await?;
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
        }
        if family == "norseman" && callback.key.ends_with("-base") {
            let before = counts(h).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "missing frost charm refuses the entire cross-owner exchange"
            );
            assert_eq!(counts(h).await?, before);
            let proofs: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_participants")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                proofs, 0,
                "foreign quest effects roll back when the later item participant refuses"
            );
            let changed:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key IN('oteryn:quest-progress/quest/u8_0/the_ice_islands/norseman_outfit','oteryn:quest-progress/outfit_quest/default_start')").fetch_one(&h.pool).await?;
            assert_eq!(changed, 0, "refusal cannot retain foreign source stages");
        }
        if let Some((timer, seconds, fixture_key)) = clock_fixtures.get(&callback.key) {
            let (stamp,now):(i64,i64)=sqlx::query_as("SELECT value,floor(extract(epoch FROM now()))::bigint FROM game_character_quest_tracks WHERE track_key=$1").bind(timer).fetch_one(&h.pool).await?;
            assert!(
                stamp > 0 && stamp <= now && now - stamp < i64::from(*seconds),
                "source start timestamp is fresh"
            );
            let before = counts(h).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "source wait cannot grant early"
            );
            assert_eq!(counts(h).await?, before);
            let before_stages:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key<>$1 ORDER BY track_key").bind(timer).fetch_all(&h.pool).await?;
            // Only elapsed wall time is arranged; all outfit stages run literally.
            commit_fixture(owner, admitted, fixture_key, 9800 + step_index as u64).await?;
            let after_stages:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key<>$1 ORDER BY track_key").bind(timer).fetch_all(&h.pool).await?;
            assert_eq!(
                after_stages, before_stages,
                "clock arrangement never seeds an outfit stage"
            );
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
        }
        let needed: Vec<_> = callback.take.iter().chain(&callback.present).collect();
        let mut missing = false;
        for item in &needed {
            missing |= quantity(h, &item.item).await? < i64::from(item.count);
        }
        if missing {
            let before = counts(h).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "missing exact carried recipe"
            );
            assert_eq!(counts(h).await?, before);
        }
        for (index, item) in needed.iter().enumerate() {
            if quantity(h, &item.item).await? < i64::from(item.count) {
                supply(owner, admitted, h, item, &mut bags, &mut occurrence).await?;
            }
            if (matches!(family, "royal-costume" | "druid-rook")
                || (matches!(family, "pirate" | "citizen-rook") && callback.key.ends_with("-hat")))
                && index == 0
            {
                let before = counts(h).await?;
                let silver = quantity(h, &item.item).await?;
                let (result, _, _) =
                    super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false)
                        .await?;
                assert_eq!(
                    result,
                    NpcIntentDisposition::Rejected,
                    "missing later ingredient must refuse the entire exchange"
                );
                assert_eq!(counts(h).await?, before);
                assert_eq!(
                    quantity(h, &item.item).await?,
                    silver,
                    "first ingredient cannot be consumed before the complete recipe is proven"
                );
            }
        }
        let required = requirements(callback)?;
        if !checked_stale && !required.is_empty() {
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            commit_fixture(
                owner,
                admitted,
                "oteryn:quest-transition/qengine2-story/lower",
                9100,
            )
            .await?;
            let before = counts(h).await?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?;
            let native = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or("native")?;
            let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                .with_wheel(owner.premium_coordinator);
            let outcome = {
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                slot.commit_quest_reward(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: callback.key.clone(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(9200).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                    crate::durability::reward_claim_mint::QuestRewardAdmission {
                        native,
                        weights: &weights,
                        fee_payment: None,
                    },
                    &owner.premium_coordinator,
                )
                .await
                .map_err(debug)?
            };
            assert!(
                matches!(
                    outcome,
                    QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
                ),
                "fresh parent state must beat stale conversation eligibility: {outcome:?}"
            );
            assert_eq!(
                counts(h).await?,
                before,
                "no consumption or value on failed parent prerequisite"
            );
            commit_fixture(
                owner,
                admitted,
                "oteryn:quest-transition/qengine2-story/restore",
                9101,
            )
            .await?;
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            checked_stale = true;
        }
        let mut held = BTreeMap::new();
        for item in &needed {
            held.insert(item.item.clone(), quantity(h, &item.item).await?);
        }
        if let Some(fee) = &callback.fee {
            let before = counts(h).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "unpaid source reward"
            );
            assert_eq!(
                counts(h).await?,
                before,
                "unpaid hand-in has no flags/value/consumption"
            );
            supply(
                owner,
                admitted,
                h,
                &oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount {
                    item: "oteryn:item.tibia.i3043".into(),
                    count: u16::try_from(fee.gold / 10000).map_err(debug)?,
                },
                &mut bags,
                &mut occurrence,
            )
            .await?;
        }
        let money_before = if callback.fee.is_some() {
            Some(coin_worth(h).await?)
        } else {
            None
        };
        if callback.requires_premium {
            let current = owner.premium_coordinator.take();
            let before = counts(h).await?;
            let coins = coin_worth(h).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            owner.premium_coordinator = current;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "absent premium owner fails closed"
            );
            assert_eq!(
                counts(h).await?,
                before,
                "premium refusal has no durable effects"
            );
            assert_eq!(coin_worth(h).await?, coins, "premium refusal cannot charge");
            for (item, count) in &held {
                assert_eq!(
                    quantity(h, item).await?,
                    *count,
                    "premium refusal cannot consume"
                );
            }
        }
        let before = counts(h).await?;
        let (result, catalogues, command) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, callback, true).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        let after = counts(h).await?;
        assert_eq!(after.0, before.0 + 1);
        assert_eq!(after.2, before.2);
        let new_facts = callback
            .outfit
            .as_ref()
            .map_or(0, |o| if o.addons != 0 && !earned_base { 2 } else { 1 });
        assert_eq!(after.3, before.3 + new_facts);
        earned_base |= callback.outfit.is_some();
        if let Some(before) = money_before {
            assert_eq!(
                coin_worth(h).await?,
                before - i64::try_from(callback.fee.as_ref().ok_or("fee")?.gold).map_err(debug)?,
                "exact source fee, retain all change"
            );
        }
        if let Some(key) = &callback.achievement {
            let earned: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
            )
            .bind(key)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                earned, 1,
                "source achievement rides the same committed reward"
            );
        }
        for item in &callback.take {
            assert_eq!(
                quantity(h, &item.item).await?,
                held[&item.item] - i64::from(item.count)
            );
        }
        for item in &callback.present {
            assert_eq!(quantity(h, &item.item).await?, held[&item.item]);
        }
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        for state in &callback.storage {
            let row: (String,i64) = sqlx::query_as("SELECT quest_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(fence.character_id.as_bytes().as_slice()).bind(&state.track).fetch_one(&h.pool).await?;
            assert_eq!(row, (tracks[&state.track].quest.clone(), state.set));
        }
        for state in &required {
            let row: (String,i64) = sqlx::query_as("SELECT quest_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(fence.character_id.as_bytes().as_slice()).bind(&state.track).fetch_one(&h.pool).await?;
            assert_eq!(
                row,
                (tracks[&state.track].quest.clone(), state.minimum),
                "read-only parent ownership and progress retained"
            );
        }
        if let Some(outfit) = &callback.outfit {
            assert!(catalogues.iter().any(|c| {
                c.outfits.iter().any(|o| {
                    o.key == outfit.outfit && o.permitted_addons & outfit.addons == outfit.addons
                })
            }));
        }
        let retained_money = if callback.fee.is_some() {
            Some(coin_worth(h).await?)
        } else {
            None
        };
        let replay = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            7,
            oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: npc,
                text: callback.keywords[0].clone(),
            })
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay)
                .map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(counts(h).await?, after, "literal replay has no new value");
        if let Some(money) = retained_money {
            assert_eq!(
                coin_worth(h).await?,
                money,
                "literal replay cannot pay twice"
            );
        }
    }
    assert_eq!(
        checked_stale,
        readonly.is_some(),
        "every independently verified source prerequisite has a stale-snapshot refusal proof"
    );
    if family == "nobleman" {
        let paid:(i64,i64,i64)=sqlx::query_as("SELECT count(*),coalesce(sum(fee_gold_units),0)::bigint,coalesce(sum(bank_debit_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=18").fetch_one(&h.pool).await?;
        assert_eq!(
            paid,
            (2, 300000, 0),
            "exactly two source prices, no bank debit"
        );
    }
    if family == "beggar" {
        let paid:(i64,i64,i64)=sqlx::query_as("SELECT count(*),coalesce(sum(fee_gold_units),0)::bigint,coalesce(sum(bank_debit_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=18").fetch_one(&h.pool).await?;
        assert_eq!(
            paid,
            (1, 20000, 0),
            "one exact source fee,carried payment retains bank"
        );
    }
    let observed = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("appearance")?;
    let key = format!("oteryn:outfit.{outfit_family}");
    let option = observed
        .catalogue
        .outfits
        .iter()
        .find(|o| o.key == key)
        .ok_or("whole outfit absent")?;
    assert_eq!(
        (option.look_type, option.permitted_addons),
        (u32::from(look), expected_addons)
    );
    let mut change = oteryn_protocol_oteryn::appearance::Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key: key,
            colours: [0; 4],
            addons: expected_addons,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    if expected_addons != 3 {
        change.selection.addons = 3;
        let command = admitted.continuity.next_command_id;
        let refused = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            25,
            change.encode().map_err(debug)?,
        )
        .await?;
        assert!(
            matches!(
                oteryn_protocol_oteryn::appearance::ChangeResult::decode(&refused)
                    .map_err(debug)?,
                oteryn_protocol_oteryn::appearance::ChangeResult::Rejected
            ),
            "unearned source addon must remain unavailable"
        );
        change.selection.addons = expected_addons;
    }
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        25,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        oteryn_protocol_oteryn::appearance::ChangeResult::decode(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
    ));
    let root: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE $1),(SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key LIKE $1)").bind(format!("oteryn:outfit.{outfit_family}/quest-grant/%")).fetch_one(&h.pool).await?;
    let facts = 1 + i64::from(expected_addons.count_ones());
    assert_eq!(
        root,
        (facts, facts),
        "one base plus exactly the source-earned immutable addon facts"
    );
    if family == "citizen-rook" {
        let value:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/outfit_quest/ref' AND quest_key='oteryn:quest.outfit_and_addon_quests'").fetch_one(&h.pool).await?;
        assert_eq!(
            value, 0,
            "shared active marker closes under its declared original owner"
        );
        let proofs:(i64,bool)=sqlx::query_as("SELECT count(*),bool_and(p.committed_character_revision=r.committed_character_revision) FROM game_quest_reward_participants p JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key)").fetch_one(&h.pool).await?;
        assert_eq!(
            proofs,
            (2, true),
            "counter increment and close each share the primary revision"
        );
    }
    if family == "norseman" {
        let (stage,stage_owner): (i64,String) = sqlx::query_as("SELECT value,quest_key FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_0/the_ice_islands/norseman_outfit'").fetch_one(&h.pool).await?;
        assert_eq!(stage, 3);
        assert_eq!(stage_owner, "oteryn:quest.outfit_and_addon_quests");
        let (questline,quest_owner):(i64,String) = sqlx::query_as("SELECT value,quest_key FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_0/the_ice_islands/questline'").fetch_one(&h.pool).await?;
        assert_eq!(questline, 44);
        assert_eq!(quest_owner, "oteryn:quest.the_ice_islands_quest");
        let proofs:(i64,bool)=sqlx::query_as("SELECT count(*),bool_and(p.committed_character_revision=r.committed_character_revision) FROM game_quest_reward_participants p JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key)").fetch_one(&h.pool).await?;
        assert_eq!(
            proofs,
            (3, true),
            "all three cross-owner outfit rewards commit at their primary revisions,without replay repeats"
        );
    }

    if family == "oriental" {
        let (value, owner, same_revision): (i64,String,bool) = sqlx::query_as("SELECT t.value,t.quest_key,t.committed_character_revision=r.committed_character_revision FROM game_character_quest_tracks t JOIN game_quest_reward_participants p ON p.character_id=t.character_id AND p.quest_key=t.quest_key AND t.track_key=ANY(p.track_keys) JOIN game_character_quest_receipts r ON r.character_id=p.character_id AND r.cause_id=p.cause_id AND r.cause_ordinal=p.cause_ordinal AND r.transition_key=p.transition_key WHERE t.track_key='oteryn:quest-progress/quest/u7_8/oriental_outfits/oriental_door'").fetch_one(&h.pool).await?;
        assert_eq!(value, 1);
        assert_eq!(owner, "oteryn:quest.oriental_outfits_quest");
        assert!(
            same_revision,
            "door and source introduction share one atomic revision"
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_participants")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 1, "literal replay cannot repeat foreign effects");
        let mut forged = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_quest_reward_participants(character_id,cause_id,cause_ordinal,transition_key,participant_index,quest_key,plan,pinned_content_revision,definition_hash,track_keys,values_before,values_after,committed_character_revision) SELECT character_id,cause_id,cause_ordinal,transition_key,2,'oteryn:quest.forged_participant',jsonb_set(plan,'{quest}','\"oteryn:quest.forged_participant\"'::jsonb),pinned_content_revision,definition_hash,track_keys,values_before,values_after,committed_character_revision FROM game_quest_reward_participants WHERE participant_index=0").execute(&mut *forged).await?;
        let error = forged
            .commit()
            .await
            .expect_err("a later transaction cannot attach a participant to an old source");
        let database = error
            .as_database_error()
            .ok_or("database constraint refusal")?;
        assert_eq!(database.code().as_deref(), Some("23514"));
        assert!(database.message().contains("exact same-transaction source"));
        let retained: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_participants")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            retained, 1,
            "forged late participant transaction rolled back"
        );
    }
    Ok(())
}
