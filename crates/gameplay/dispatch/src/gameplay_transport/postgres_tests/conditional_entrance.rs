//! Conditional entrances through ordinary STEP and literal USE over PostgreSQL.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{conditional_entrance_scenario, input_item, local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_authority::ReconciledCharacterAuthority,
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack, isle_of_evil,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::{
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
        encode_use_intent,
    },
    world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    },
};
use serde_json::json;

const GATE: Position = Position {
    x: 32791,
    y: 32331,
    floor: -10,
};
const LANDING: Position = Position { x: 32792, ..GATE };
const HOLY_TIBLE: &str = "oteryn:item.tibia.i2836";
const ISLE_REFUSAL: &str = "You better talk to Rapanaio before leave the safety of the ship!";
const ISLE_ADMITTED: &str = "You hear evil laughter from afar.";
const SCOUT_POINTS: &str = "oteryn:quest-progress/quest/u11_50/dangerous_depths/scouts/points";
const DANGEROUS_DEPTHS: &str = "oteryn:quest.dangerous_depths_quest";
const WARZONE_ACCESS: [&str; 3] = [
    "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone1_access",
    "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone2_access",
    "oteryn:quest-progress/quest/u9_60/bigfoots_burden/warzone3_access",
];
const NEW_FRONTIER_08: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission08";
const WRATH_11: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission11";
const VISITOR: &str = "oteryn:quest-progress/quest/u9_4/liquid_black_quest/visitor";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Case {
    Storage,
    Completion,
    Flags,
    Carried,
    Door,
    Isle { travel: bool },
    Warzone { access: bool },
    Bounac,
    Level,
    Premium,
    Occupied,
    CalendarStep,
    CalendarUse,
    PvpLock,
    Stair,
}

impl Case {
    fn is_use(self) -> bool {
        matches!(self, Self::CalendarUse | Self::PvpLock | Self::Door | Self::Bounac)
    }

    pub(super) fn callback(self, source: Position) -> TestResult<Callback> {
        let predicate = match self {
            // Inquisition movements_entrance.lua:1-32, any exact throne value.
            Self::Storage => json!({"kind":"any","predicates":[
                {"kind":"storage","range":{"source":{"kind":"track","track":VISITOR},"minimum":1,"maximum":1}},
                {"kind":"storage","range":{"source":{"kind":"track","track":VISITOR},"minimum":10,"maximum":10}}
            ]}),
            // Zalamon actions_zalamon_door.lua:5-17 reads both owning quests.
            Self::Door => json!({"kind":"all","predicates":[
                storage_range(NEW_FRONTIER_08, 2, i64::MAX),
                storage_range(WRATH_11, i64::MIN, 1)
            ]}),
            // Dangerous Depths movements_energy_entrance.lua:18-29 lowers the
            // Scouts threshold to exactly ten with all three access flags.
            Self::Warzone { .. } => json!({"kind":"any","predicates":[
                storage_range(SCOUT_POINTS, 15, i64::MAX),
                {"kind":"all","predicates":[
                    storage_range(SCOUT_POINTS, 10, i64::MAX),
                    storage_range(WARZONE_ACCESS[0], 1, 1),
                    storage_range(WARZONE_ACCESS[1], 1, 1),
                    storage_range(WARZONE_ACCESS[2], 1, 1)
                ]}
            ]}),
            // Isle of Evil movements_isle_enter.lua:18-24. Either native writer
            // is sufficient, and admission leaves the player on the gangway.
            Self::Isle { .. } => json!({"kind":"any","predicates":[
                {"kind":"storage","range":{
                    "source":{"kind":"track","track":isle_exit_track(false)},"minimum":1,"maximum":i64::MAX}},
                {"kind":"storage","range":{
                    "source":{"kind":"track","track":isle_exit_track(true)},"minimum":1,"maximum":1}}
            ]}),
            // Soul War moveevent-teleport_entrance_reward.lua:9-22.
            Self::Completion => json!({"kind":"storage","range":{
                "source":{"kind":"soul_war_completed"},"minimum":1,"maximum":1
            }}),
            // Soul War moveevent-soul_war_entrances.lua:60-88.
            Self::Flags => json!({"kind":"distinct_flags","minimum":5,"flags":
                (["Malice","Hatred","Spite","Cruelty","Greed"].map(|boss| json!({
                    "label":boss,"range":{"source":{"kind":"soul_war_boss","boss":boss},"minimum":1,"maximum":1}
                })))
            }),
            // Pits of Inferno movements_holy_tible_tile.lua:14-20.
            Self::Carried => json!({"kind":"carried","item":HOLY_TIBLE,"count":1}),
            // Feaster of Souls actions_entrances.lua:17-33.
            // Bounac action-bounac_entrance.lua:2-10 uses the same threshold on USE.
            Self::Level | Self::Bounac => json!({"kind":"level","minimum":250}),
            // Spike movements_spike_teleport.lua:35-46.
            Self::Premium => json!({"kind":"premium"}),
            // Demon Oak movements_entrance.lua:21-44, only players count.
            Self::Occupied => json!({"kind":"unoccupied","minimum":LANDING,"maximum":LANDING}),
            // Chyllfroest movements_prohibit_entry.lua:9-15 and actions_reward.lua:4-8.
            Self::CalendarStep => json!({"kind":"calendar","start":[4,1],"end":[5,1]}),
            Self::CalendarUse => json!({"kind":"calendar","start":[5,2],"end":[3,31]}),
            // Thieves Guild actions_climbing_vine.lua:13-18.
            Self::PvpLock => json!({"kind":"no_pvp_lock"}),
            // Bounac action-bounac_entrance.lua:24-25 intercepts the native
            // stairs without an actor predicate.
            Self::Stair => serde_json::Value::Null,
        };
        let mut refusal = json!({"message":"Entrance refused by the fixture."});
        if !self.is_use() {
            refusal["push_back"] = json!({"kind":"from","dx":0,"dy":0});
        }
        let trigger = if self.is_use() { "use" } else { "step_in" };
        let mut row = json!({
            "key":"oteryn:quest-callback/qgate-test","quest":"oteryn:quest.the_pits_of_inferno_quest",
            "trigger":trigger,"selector":{"position":source,"appearance":100},
            "teleport":LANDING,"storage":[],"passages":[],"offerings":[],
            "reversible":false,"return_on_failure":false,
            "effects":{"conditional_entrance":{"guards":[{"when":predicate,"refusal":refusal}]}},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_pits_of_inferno_quest/movements_holy_tible_tile.lua:14-20"]
        });
        if self == Self::Premium {
            // The Spike donor registers AID 4226. A native floor change at
            // the fixture tile would otherwise skip that StepIn altogether.
            row["selector"] = json!({"action":4226,"appearance":293});
            let entrance = &mut row["effects"]["conditional_entrance"];
            entrance["intercept_teleport"] = json!(true);
            entrance["guards"][0]["refusal"] = json!({
                "message":"Only premium accounts can use this teleporter.",
                "push_back":{"kind":"fixed","position":Position { x: source.x - 1, ..source }},
                "effect":11,"effect_at":"both"
            });
            row["sources"] = json!(["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_spike_tasks/movements_spike_teleport.lua:2-3,35-46,54-55"]);
        }
        if self == Self::Stair {
            row["selector"]["position"] = serde_json::Value::Null;
            row["selector"]["appearance"] = json!(293);
            row["selector"]["native"] = json!({
                "position": source, "ordinal": 1, "appearance": 293
            });
            row["effects"]["conditional_entrance"] = json!({
                "guards":[], "intercept_teleport":true
            });
        }
        if self == Self::Door {
            // Zalamon actions_zalamon_door.lua:5-17, exact UID and transform.
            row["sources"] = json!(["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/children_of_the_revolution/actions_zalamon_door.lua:5-17"]);
            row["selector"]["appearance"] = json!(9874);
            row["selector"]["unique"] = json!(3170);
            row["teleport"] = serde_json::Value::Null;
            row["effects"]["transform"] = json!(9875);
            row["effects"]["conditional_entrance"]["teleport_to_source"] = json!(true);
        }
        if self == Self::Bounac {
            row["quest"] = json!("oteryn:quest.the_order_of_the_lion_quest");
            row["selector"]["action"] = json!(59602);
            row["sources"] = json!(["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_order_of_lion_quest/action-bounac_entrance.lua:2-10"]);
            row["effects"]["conditional_entrance"]["guards"][0]["refusal"] =
                json!({"message":"You need at least level 250.","message_kind":"status","effect":3,"effect_at":"source"});
        }
        if matches!(self, Self::Warzone { .. }) {
            row["quest"] = json!(DANGEROUS_DEPTHS);
            row["sources"] = json!(["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dangerous_depth/movements_energy_entrance.lua:18-29"]);
            row["effects"]["conditional_entrance"]["guards"][0]["refusal"]["message"] =
                json!("You can not use this portal yet.");
            row["effects"]["conditional_entrance"]["guards"][0]["refusal"]["push_back"] =
                json!({"kind":"from","dx":0,"dy":1});
        }
        if matches!(self, Self::Isle { .. }) {
            row["quest"] = json!(isle_of_evil::QUEST);
            row["teleport"] = serde_json::Value::Null;
            row["sources"] = json!(["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_isle_of_evil_quest/movements_isle_enter.lua:18-24"]);
            let entrance = &mut row["effects"]["conditional_entrance"];
            entrance["guards"][0]["refusal"]["message"] = json!(ISLE_REFUSAL);
            entrance["guards"][0]["refusal"]["message_kind"] = json!("monster_say");
            entrance["on_admit_speech"] = json!({
                "message": ISLE_ADMITTED, "message_kind": "monster_say"
            });
        }
        Ok(serde_json::from_value::<Callback>(row)?)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "uses the shared PostgreSQL scenario's current Character and Channel owners"
    )]
    pub(super) async fn exercise(
        self,
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut AdmittedSession,
        h: &Harness,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        callback: &Callback,
    ) -> TestResult {
        let session = admitted.game_session_id;
        let origin = local(Position {
            x: GATE.x - 1,
            ..GATE
        });
        if self == Self::Stair {
            assert!(self.attempt(owner, admitted, callback, 1).await?);
            return Ok(());
        }
        if let Self::Warzone { access } = self {
            let current = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
            let mut tracks = WARZONE_ACCESS
                .iter()
                .map(|key| current.track(key).cloned().ok_or("native Warzone track"))
                .collect::<Result<Vec<_>, _>>()?;
            tracks.push(QuestTrack {
                key: SCOUT_POINTS.into(),
                quest: DANGEROUS_DEPTHS.into(),
                initial: -1,
                min: -1,
                max: i64::from(i32::MAX),
            });
            owner.quest_catalogue = Some(std::sync::Arc::new(QuestStateCatalogue::new(
                current.content_revision().to_owned(),
                tracks,
                Vec::new(),
            ).map_err(debug)?));
            for flag in &WARZONE_ACCESS[..2] {
                set_fixture_track(owner, h, fence, flag, 1).await?;
            }
            // Fourteen is insufficient without all three flags. Nine remains
            // insufficient with all three and then succeeds at ten.
            if access {
                set_fixture_track(owner, h, fence, WARZONE_ACCESS[2], 1).await?;
            }
            set_fixture_track(owner, h, fence, SCOUT_POINTS, if access { 9 } else { 14 }).await?;
        }
        let occupant = if self == Self::Occupied {
            let mut runtime = owner.runtime.lock().await;
            let visitor = GameSessionId::decode(&id(51)).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(visitor).map_err(debug)?;
            let visitor_actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(visitor_actor, local(LANDING))
                .map_err(debug)?;
            Some((visitor, visitor_actor))
        } else {
            None
        };
        if self == Self::Flags {
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at) VALUES(encode($1,'hex')::uuid,true,15,1)")
                .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        }
        if self == Self::PvpLock {
            sqlx::query("INSERT INTO game_character_pvp_state (character_id,world_id,pz_block_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '30 seconds')")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
        }
        let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
        if matches!(self, Self::CalendarStep | Self::CalendarUse) {
            let in_season: bool = sqlx::query_scalar(
                "SELECT to_char(transaction_timestamp() AT TIME ZONE 'UTC','MMDD') BETWEEN '0401' AND '0501'",
            ).fetch_one(&h.pool).await?;
            let allowed = in_season == (self == Self::CalendarStep);
            let result = self.attempt(owner, admitted, callback, 1).await?;
            assert_eq!(result, allowed);
            let expected = if allowed { local(LANDING) } else { origin };
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                expected
            );
            return Ok(());
        }

        let attempted = self.attempt(owner, admitted, callback, 1).await?;
        assert!(!attempted);
        let warzone = matches!(self, Self::Warzone { .. });
        let refusal_position = if warzone {
            local(Position {
                x: GATE.x - 1,
                y: GATE.y + 1,
                ..GATE
            })
        } else {
            origin
        };
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            refusal_position
        );
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            before, after,
            "an entrance refusal creates no Quest receipt"
        );

        match self {
            Self::Storage => {
                let catalogue = owner.quest_catalogue.clone().ok_or("quest catalogue")?;
                let seeded = owner.revision_sequencer.acquire(fence.character_id).await
                    .commit_quest_transition(owner.root, owner.character, owner.holder, fence, QuestTransitionRequest {
                        transition_key: "oteryn:quest-transition/quest/u9_4/liquid_black_quest/visitor/npc_1".into(),
                        cause: QuestCause::Command(CommandRef::new(session, CommandId::new(901).map_err(debug)?)),
                    }, catalogue).await.map_err(debug)?;
                assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
            }
            Self::Door => {
                set_fixture_track(owner, h, fence, NEW_FRONTIER_08, 2).await?;
                set_fixture_track(owner, h, fence, WRATH_11, 2).await?;
                assert!(!self.attempt(owner, admitted, callback, 2).await?);
                set_fixture_track(owner, h, fence, WRATH_11, 1).await?;
            }
            Self::Isle { travel } => {
                set_fixture_track(owner, h, fence, &isle_exit_track(travel), 1).await?;
            }
            Self::Warzone { access } => {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let reply = wire(
                    owner, admitted, 2, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    encode_step_intent(StepDirection::North),
                ).await?;
                assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
                set_fixture_track(owner, h, fence, SCOUT_POINTS, if access { 10 } else { 15 }).await?;
            }
            Self::Completion => {
                sqlx::query("INSERT INTO game_character_soul_war(character_id,megalomania_completed) VALUES(encode($1,'hex')::uuid,true)")
                    .bind(id(41).as_slice()).execute(&h.pool).await?;
            }
            Self::Flags => {
                sqlx::query("UPDATE game_character_soul_war SET defeated_bosses=31 WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).execute(&h.pool).await?;
            }
            Self::Carried => {
                input_item(
                    owner,
                    fence.item_fence(),
                    &Offering {
                        position: GATE,
                        item: HOLY_TIBLE.into(),
                        count: 1,
                    },
                    1,
                    true,
                )
                .await?;
            }
            Self::Level | Self::Bounac => {
                super::qengine_outfit_stories::earn_level(owner, admitted, authority, 250).await?;
                owner.states.level_refresh.mark([fence.character_id]);
                owner.refresh_level_up_vitals(actor, session).await;
            }
            Self::Premium => {
                owner.admit_premium(id(40), session);
                assert!(owner.session_premium_gate(session).await.premium_current());
            }
            Self::Occupied => {
                let (other_session, other_actor) = occupant.ok_or("grove fixture occupant")?;
                owner
                    .runtime
                    .lock()
                    .await
                    .remove_terminal_session(other_session, other_actor)
                    .map_err(debug)?;
            }
            Self::PvpLock => {
                // Expire both the durable deadline and the restored semantic deadline.
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(31))
                    .ok_or("PvP deadline")?;
                sqlx::query("UPDATE game_character_pvp_state SET pz_block_until=NULL,revision=revision+1 WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).execute(&h.pool).await?;
            }
            Self::CalendarStep | Self::CalendarUse | Self::Stair => {
                return Err("calendar handled by the complementary window cases".into());
            }
        }

        let next_command = if self == Self::Door || warzone { 3 } else { 2 };
        assert!(self.attempt(owner, admitted, callback, next_command).await?);
        if self == Self::Bounac {
            let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(before, after, "admitted Bounac travel creates no Quest receipt");
        }
        let landing = if matches!(self, Self::Door | Self::Isle { .. }) {
            GATE
        } else {
            LANDING
        };
        if matches!(self, Self::Isle { .. }) {
            let map = owner.bundle_map().ok_or("fixture map")?;
            let speech = map.facts.movement_speech_since(0);
            for (message, x) in [(ISLE_REFUSAL, GATE.x - 1), (ISLE_ADMITTED, GATE.x)] {
                assert!(speech.iter().any(|(_, line)| {
                    line.speaker == actor && line.text == message && line.talk_type == 36
                        && line.position == TilePos { x, y: GATE.y, floor: GATE.floor }
                }), "expected speech at x={x}: {message}");
            }
        }
        if self == Self::Door {
            let map = owner.bundle_map().ok_or("fixture map")?;
            let key = placement_key(
                TilePos {
                    x: GATE.x,
                    y: GATE.y,
                    floor: GATE.floor,
                },
                1,
            )
            .ok_or("door placement")?;
            assert_eq!(map.facts.object_revision(key), 1);
            let at = TilePos {
                x: GATE.x,
                y: GATE.y,
                floor: GATE.floor,
            };
            let base_id = *map
                .overlay
                .base()
                .tile(at.x, at.y, at.floor)
                .and_then(|tile| tile.ids().get(1))
                .ok_or("door base appearance")?;
            assert_eq!(
                map.facts
                    .base_entry(at, 1, base_id)
                    .ok_or("door projection")?
                    .appearance_id,
                9875
            );
        }
        if self == Self::Carried {
            let remaining: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
                .bind(HOLY_TIBLE).fetch_one(&h.pool).await?;
            assert_eq!(remaining, 1);
        }
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(landing)
        );
        Ok(())
    }

    async fn attempt(
        self,
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut AdmittedSession,
        callback: &Callback,
        command: u64,
    ) -> TestResult<bool> {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        if self.is_use() {
            let map = owner.bundle_map().ok_or("fixture map")?;
            let pos = callback.selector.position.ok_or("fixture selector")?;
            let key = placement_key(
                TilePos {
                    x: pos.x,
                    y: pos.y,
                    floor: pos.floor,
                },
                1,
            )
            .ok_or("fixture placement")?;
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let payload = encode_use_intent(&WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })
            .map_err(debug)?;
            let reply = wire(owner, admitted, command, COMMAND_TYPE_USE_INTENT, payload).await?;
            Ok(decode_use_result(&reply).map_err(debug)? == UseDisposition::Committed)
        } else {
            let reply = wire(
                owner,
                admitted,
                command,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                encode_step_intent(StepDirection::East),
            )
            .await?;
            assert_eq!(
                decode_step_result(&reply).map_err(debug)?,
                StepDisposition::Moved
            );
            let actor = admitted.runtime_actor.ok_or("fixture actor")?;
            let position = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            let destination = if matches!(self, Self::Isle { .. }) { GATE } else { LANDING };
            Ok(position == local(destination))
        }
    }
}

fn storage_range(track: &str, minimum: i64, maximum: i64) -> serde_json::Value {
    json!({"kind":"storage","range":{"source":{"kind":"track","track":track},"minimum":minimum,"maximum":maximum}})
}

async fn set_fixture_track(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    track: &str,
    value: i64,
) -> TestResult {
    let track = owner.quest_catalogue.as_deref()
        .and_then(|catalogue| catalogue.track(track))
        .ok_or_else(|| format!("native gate fixture track {track}"))?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks
            (character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
         WHERE character_id=encode($1,'hex')::uuid
         ON CONFLICT(character_id,track_key) DO UPDATE
         SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&track.key)
    .bind(&track.quest)
    .bind(value)
    .execute(&h.pool)
    .await?;
    Ok(())
}

fn isle_exit_track(travel: bool) -> String {
    if travel {
        format!("{}rapanaio-travel", isle_of_evil::KV_PREFIX)
    } else {
        format!("{}mission07", isle_of_evil::PREFIX)
    }
}

fn scenario(case: Case) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("conditional-entrance", move || {
        conditional_entrance_scenario(GATE, case)
    })
}

#[test]
fn conditional_step_checks_literal_storage_range_alternatives() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Storage)
}

#[test]
fn conditional_step_reads_native_megalomania_completion() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Completion)
}

#[test]
fn conditional_step_counts_each_native_miniboss_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Flags)
}

#[test]
fn conditional_step_reads_carried_item_presence_without_consuming_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Carried)
}

#[test]
fn conditional_step_reads_live_level_after_progression_changes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Level)
}

#[test]
fn conditional_step_uses_spike_action_before_native_floor_routing() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Premium)
}

#[test]
fn conditional_step_refuses_until_other_player_leaves_the_grove() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Occupied)
}

#[test]
fn conditional_step_and_use_evaluate_the_annual_calendar() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::CalendarStep)?;
    scenario(Case::CalendarUse)
}

#[test]
fn conditional_use_refuses_vine_while_pvp_locked() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::PvpLock)
}

#[test]
fn conditional_step_keeps_the_original_down_stair_selector() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Stair)
}

#[test]
fn conditional_use_checks_bounac_level_without_writing_quest_progress() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Bounac)
}

#[test]
fn conditional_use_moves_through_and_transforms_the_exact_door() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Door)
}

#[test]
fn conditional_step_accepts_either_native_isle_ship_exit_track() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Isle { travel: false })?;
    scenario(Case::Isle { travel: true })
}

#[test]
fn conditional_step_requires_ten_with_all_warzone_flags_and_fifteen_otherwise() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Warzone { access: false })?;
    scenario(Case::Warzone { access: true })
}
