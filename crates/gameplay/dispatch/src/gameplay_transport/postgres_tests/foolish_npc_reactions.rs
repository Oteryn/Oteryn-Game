//! Native NPC_TALK receipts with real displayed looks, inventory and creature state.
use super::quest_world_callbacks::{self, tool_fixture};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
};
use oteryn_gameplay_quests::quest::{
    authored_bindings::{Bindings, BoundQuest, Stage, Track, Trigger},
    reward_callbacks::RewardCallback,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::{
    npc_service::{NpcIntentDisposition, NpcTalkIntent},
    world_object::{UseDisposition, UseIntent, UseTarget},
    world_spatial_entities::EntityRef,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::{num::NonZeroU64, sync::atomic::Ordering};

const QUEST: &str = "oteryn:quest.what_a_foolish_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline";
const SCARED: &str = "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/scared_kazzan";
const WYDA: &str = "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/cookie_delivery/wyda";
const ORC_GREET: &str = "oteryn:quest-progress/storage/30006";
const KAZZAN_NPC: &str = "oteryn:npc.kazzan";
const WYDA_NPC: &str = "oteryn:npc.wyda";
const ORC_NPC: &str = "oteryn:npc.the_orc_king";
const COOKIE: &str = "oteryn:item.tibia.i130";
const DISGUISE: &str = "oteryn:item.tibia.i144";
const SCREAM: &str = "WAAAAAHHH!!!";
const COOKIE_PROMPT: &str = "Offer the cookie to Wyda?";

#[test]
fn kazzan_greeting_requires_the_current_mummy_look() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("kazzan-mummy", move || {
        scenario(admin, KAZZAN_NPC)
    })
}

#[test]
fn wyda_cookie_hand_in_consumes_cookie_and_applies_delayed_soulfire() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("wyda-soulfire", move || scenario(admin, WYDA_NPC))
}

#[test]
fn the_orc_king_first_greeting_spawns_public_guards_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("orc-king-guards", move || {
        scenario(admin, ORC_NPC)
    })
}

fn scenario(admin: String, npc: &'static str) -> TestResult {
    let at = Position {
        x: 32588,
        y: 31644,
        floor: -3,
    };
    let npc_at = Position { x: at.x + 1, ..at };
    let geometry: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/npc-map",
        "quest": QUEST, "trigger": "use",
        "selector": { "position": at, "appearance": 100 },
        "reversible": false, "passages": [], "storage": [],
        "offerings": [], "return_on_failure": false, "sources": ["fixture map"]
    }))?;
    let world = quest_world_callbacks::world(&geometry, at)?;
    let (case, keyword, stage, track, reply, source, look, take, effects) = match npc {
        KAZZAN_NPC => (
            "kazzan_mummy",
            "hi",
            35,
            SCARED,
            SCREAM,
            "data-global/npc/kazzan.lua:53-67",
            Some(65),
            serde_json::json!([]),
            serde_json::json!({ "end_conversation_delay_ms": 1000 }),
        ),
        WYDA_NPC => (
            "wyda_soulfire",
            "yes",
            31,
            WYDA,
            "FEEL MY WRATH!",
            "data-global/npc/wyda.lua:52-54,87-103",
            None,
            serde_json::json!([{ "item": COOKIE, "count": 1 }]),
            serde_json::json!({
                "fire_damage": {"ticks": 60, "interval_ms": 2000, "amount": 10},
                "end_conversation_delay_ms": 0
            }),
        ),
        ORC_NPC => (
            "orc_king_guards",
            "hi",
            31,
            ORC_GREET,
            "Kill them my guards!",
            "data-global/npc/the_orc_king.lua:48-59,118-124",
            None,
            serde_json::json!([]),
            serde_json::json!({
                "nearby_creatures": [
                    "oteryn:creature.slime", "oteryn:creature.slime", "oteryn:creature.slime",
                    "oteryn:creature.orc_warlord", "oteryn:creature.orc_warlord",
                    "oteryn:creature.orc_leader", "oteryn:creature.orc_leader", "oteryn:creature.orc_leader"
                ],
                "end_conversation_delay_ms": 0
            }),
        ),
        _ => return Err("unexpected reaction fixture".into()),
    };
    let mut storage = vec![serde_json::json!({"track": track, "minimum": 0, "maximum": 0, "set": 1})];
    if npc != ORC_NPC {
        storage.push(serde_json::json!({
            "track": QUESTLINE, "minimum": stage, "maximum": stage, "set": stage, "preserve": true
        }));
    }
    let callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key": format!("oteryn:quest-reward/fixture/{case}"),
        "quest": QUEST, "npc": npc, "keywords": [keyword], "topic": u32::from(npc == WYDA_NPC), "next_topic": 0,
        "storage": storage,
        "requires_temporary_look": look, "take": take, "npc_after_commit": effects,
        "reply": [reply], "sources": [format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:{source}")]
    }))?;
    let mut stages = vec![Stage {
        key: case.into(),
        completes: false,
        trigger: Trigger::NpcDialogue {
            callback: Box::new(callback),
            alternatives: Vec::new(),
        },
    }];
    if npc == WYDA_NPC {
        // Crystal 201189f8 data-global/npc/wyda.lua:64-68 opens topic 1.
        let offer: RewardCallback = serde_json::from_value(serde_json::json!({
            "key": "oteryn:quest-reward/fixture/wyda-offer",
            "quest": QUEST, "npc": npc, "keywords": ["cookie"], "topic": 0, "next_topic": 1,
            "storage": [
                {"track": QUESTLINE, "minimum": 31, "maximum": 31, "set": 31, "preserve": true},
                {"track": WYDA, "minimum": 0, "maximum": 0, "set": 0, "preserve": true}
            ],
            "take": [], "reply": [COOKIE_PROMPT],
            "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/wyda.lua:64-68"]
        }))?;
        stages.push(Stage {
            key: "wyda_cookie_offer".into(),
            completes: false,
            trigger: Trigger::NpcDialogue {
                callback: Box::new(offer),
                alternatives: Vec::new(),
            },
        });
    }
    let bindings = Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: QUEST.into(),
            recipe_sha256: "1".repeat(64),
            occurrence: None,
            admission: None,
            tracks: vec![
                Track {
                    key: QUESTLINE.into(),
                    initial: stage,
                    minimum: 0,
                    maximum: 40,
                },
                Track {
                    key: track.into(),
                    initial: 0,
                    minimum: 0,
                    maximum: 1,
                },
            ],
            stages,
            sources: vec![source.into()],
        }],
    };
    tool_fixture::run_bound_world(
        admin,
        case,
        world,
        at,
        bindings,
        async |owner, admitted, h, fence| {
            let [entity] =
                super::classic_quests::install_positions(owner, &admitted, [npc], [npc_at]).await?;
            let _clock = owner.freeze_owner_clock_for_test();
            if npc == WYDA_NPC {
                quest_world_callbacks::input_item(
                    owner,
                    fence.item_fence(),
                    &Offering {
                        position: at,
                        item: COOKIE.into(),
                        count: 1,
                    },
                    81,
                    true,
                )
                .await?;
            }
            let ordinary = talk(owner, admitted, fence, entity, 1, "hi").await?;
            assert!(ordinary.lines.iter().all(|line| line != SCREAM));
            if npc == KAZZAN_NPC {
                // Crystal 201189f8 action_disguise.lua:1-13 sets a ten-second look
                // through ordinary USE of a carried, reusable disguise.
                let rule = serde_json::from_value(serde_json::json!({
                    "key": "quest.fixture.mummy-disguise",
                    "item": DISGUISE,
                    "use_text": "Mummy disguise applied.",
                    "restore_text": "",
                    "use_effect": 0,
                    "speech_type": 36,
                    "appearance": 65,
                    "timed_look_ms": 10000,
                    "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/what_a_foolish_quest/action_disguise.lua:1-13"]
                }))?;
                owner.states.fixture_item_disguises.set(vec![rule])
                    .map_err(|_| "disguise fixture already installed")?;
                quest_world_callbacks::input_item(
                    owner,
                    fence.item_fence(),
                    &Offering {
                        position: at,
                        item: DISGUISE.into(),
                        count: 1,
                    },
                    82,
                    true,
                ).await?;
                let item: Vec<u8> = sqlx::query_scalar(
                    "SELECT uuid_send(item_instance_id) FROM game_item_instances
                     WHERE definition_production_key=$1 AND lifecycle=1",
                ).bind(DISGUISE).fetch_one(&h.pool).await?;
                let item: [u8; 16] = item.try_into().map_err(|_| "disguise item UUID")?;

                for (farewell, use_id, greeting, active) in [(2, 3, 9, false), (10, 11, 12, true)] {
                    talk(owner, admitted, fence, entity, farewell, "bye").await?;
                    owner.states.test_owner_time.store(use_id * 2_000_000 + 1, Ordering::Relaxed);
                    let used = owner.use_quest_tool(
                        admitted.runtime_actor.ok_or("actor")?,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: use_id,
                            item_fence: Some(fence.item_fence()),
                        },
                        UseIntent {
                            target: UseTarget::Item(NonZeroU64::MIN),
                            use_with: None,
                            use_with_map_item: None,
                            use_with_position: None,
                        },
                        Some(ItemKey::Instance(item)),
                        None,
                    ).await.map_err(|()| "disguise USE unavailable")?.ok_or("disguise USE outcome")?;
                    assert_eq!(used.disposition, UseDisposition::Committed);
                    let greeted = talk(owner, admitted, fence, entity, greeting, "hi").await?;
                    assert_eq!(greeted.lines.iter().any(|line| line == SCREAM), active);
                    if active {
                        assert_eq!(greeted.lines, [SCREAM]);
                    }
                }
                let retained: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_item_container_entries
                     WHERE item_instance_id=encode($1,'hex')::uuid
                       AND character_id=encode($2,'hex')::uuid",
                ).bind(item.as_slice())
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_one(&h.pool).await?;
                assert_eq!(retained, 1, "the disguise remains in the Character's backpack");
            } else if npc == WYDA_NPC {
                let offer = talk(owner, admitted, fence, entity, 2, "cookie").await?;
                assert_eq!(offer.lines, [COOKIE_PROMPT]);
                let delivered = talk(owner, admitted, fence, entity, 3, "yes").await?;
                assert_eq!(delivered.lines, [reply]);
                assert!(delivered.inventory_changed);
                let actor = admitted.runtime_actor.ok_or("actor")?;
                let session = admitted.game_session_id;
                let (applied_at, starting_health) = {
                    let runtime = owner.runtime.lock().await;
                    let states = owner.states.spell_states.lock().await;
                    let state = states.get(&runtime, actor, session).ok_or("player state")?;
                    let now = owner.owner_now().get();
                    let fire = state
                        .owned_conditions()
                        .active_at(
                            crate::domain::condition::ConflictKey::Element(
                                crate::domain::condition::DotElement::Fire,
                            ),
                            now,
                        )
                        .ok_or("active Soulfire")?;
                    assert_eq!(fire.remaining_duration_ms(now), Some(120_000));
                    (now, state.vitals().health)
                };

                // Crystal 201189f8 data-global/npc/wyda.lua:52-54 delays the
                // first 10 damage until 2 seconds after applying the condition.
                let first_tick = applied_at + 2_000_000;
                let _ = owner
                    .tick_spell_periodic_at(actor, session, SemanticTimeMicros::from_micros(first_tick - 1))
                    .await;
                {
                    let runtime = owner.runtime.lock().await;
                    let states = owner.states.spell_states.lock().await;
                    let state = states.get(&runtime, actor, session).ok_or("player state")?;
                    assert_eq!(state.vitals().health, starting_health);
                }
                let (_, vitals) = owner
                    .tick_spell_periodic_at(actor, session, SemanticTimeMicros::from_micros(first_tick))
                    .await
                    .ok_or("Soulfire tick did not publish vitals")?;
                assert_eq!(vitals.health, starting_health - 10);
                let cookies: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
            ).bind(COOKIE).fetch_one(&h.pool).await?;
                assert_eq!(cookies, 0);
            } else {
                assert_eq!(ordinary.lines, [reply]);
                let _ = talk(owner, admitted, fence, entity, 3, "hi").await?;
                let runtime = owner.runtime.lock().await;
                let actors = runtime.current_live_creature_census().map_err(debug)?;
                for (key, count) in [("slime", 3), ("orc_warlord", 2), ("orc_leader", 3)] {
                    let key = format!("oteryn:creature.{key}");
                    assert_eq!(
                        actors
                            .iter()
                            .filter(|actor| runtime
                                .matches_live_creature_identity(**actor, key.as_bytes()))
                            .count(),
                        count
                    );
                }
            }
            let copy = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(track), Some(&1));
            Ok(())
        },
    )
}

pub(super) struct TalkResult {
    pub(super) lines: Vec<String>,
    pub(super) inventory_changed: bool,
}

pub(super) async fn talk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    fence: CurrentCharacterGameplayFence,
    npc: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<TalkResult> {
    owner
        .states
        .test_owner_time
        .store(command_id * 2_000_000, std::sync::atomic::Ordering::Relaxed);
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
    assert!(!outcome.unknown, "{text}");
    Ok(TalkResult {
        lines: outcome
            .conversation
            .map_or_else(Vec::new, |conversation| conversation.lines),
        inventory_changed: outcome.inventory_changed,
    })
}
