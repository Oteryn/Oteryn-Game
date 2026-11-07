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
        _ => return Err("unknown outfit story".into()),
    };
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let mut callbacks = Vec::new();
    for (suffix, take, present, addon) in steps {
        let key = format!("oteryn:quest-reward/l2-{family}-{suffix}");
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
        assert_eq!(callback.take, take.iter().map(amount).collect::<Vec<_>>());
        assert_eq!(
            callback.present,
            present.iter().map(amount).collect::<Vec<_>>()
        );
        assert_eq!(callback.outfit.as_ref().map(|o| o.addons), *addon);
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
    // Build one fixed catalogue for the whole story. Only external prerequisite
    // quests are arranged; the outfit's own stages run through actual NPC commands.
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    let mut readonly = None;
    for callback in &callbacks {
        let required = requirements(callback)?;
        for key in callback
            .storage
            .iter()
            .map(|s| s.track.as_str())
            .chain(required.iter().map(|r| r.track.as_str()))
        {
            tracks.insert(
                key.to_owned(),
                canonical.track(key).cloned().ok_or("canonical track")?,
            );
        }
        if readonly.is_none() {
            readonly = required.first().cloned();
        }
        transitions.push(
            canonical
                .transition(&callback.key)
                .cloned()
                .ok_or("canonical transition")?,
        );
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
    // The creation fixture starts at level two. Earn normal level-50 capacity
    // through the real progression owner before carrying the heavy native books.
    if family == "battle-mage" {
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
            .find(|t| t.level == 50)
            .ok_or("level50 threshold")?
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
    let mut npcs = BTreeMap::new();
    if family == "festive" {
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
    let mut bags = Vec::new();
    let mut occurrence = 500;
    for callback in &callbacks {
        let npc = *npcs.get(&callback.npc).ok_or("story NPC actor")?;
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
        for item in &needed {
            if quantity(h, &item.item).await? < i64::from(item.count) {
                supply(owner, admitted, h, item, &mut bags, &mut occurrence).await?;
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
            let physical = owner.spell_lane_permit().await.ok_or("physical lane")?;
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
                        fee_payment: None,
                        permit: &physical,
                        native,
                        weights: &weights,
                    },
                )
                .await
                .map_err(debug)?
            };
            drop(physical);
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
        let before = counts(h).await?;
        let (result, catalogues, command) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, callback, true).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        let after = counts(h).await?;
        assert_eq!(after.0, before.0 + 1);
        assert_eq!(after.2, before.2);
        assert_eq!(after.3, before.3 + i64::from(callback.outfit.is_some()));
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
    }
    assert_eq!(checked_stale, family != "elementalist");
    let observed = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("appearance")?;
    let key = format!("oteryn:outfit.{}", family.replace('-', "_"));
    let option = observed
        .catalogue
        .outfits
        .iter()
        .find(|o| o.key == key)
        .ok_or("whole outfit absent")?;
    assert_eq!(
        (option.look_type, option.permitted_addons),
        (u32::from(look), 3)
    );
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key: key,
            colours: [0; 4],
            addons: 3,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
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
    let root: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE $1),(SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key LIKE $1)").bind(format!("oteryn:outfit.{}/quest-grant/%",family.replace('-',"_"))).fetch_one(&h.pool).await?;
    assert_eq!(root, (3, 3), "one base and two immutable addon facts");
    Ok(())
}
