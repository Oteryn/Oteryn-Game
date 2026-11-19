//! Crystal 201189f8 data-global/scripts/quests/hidden_threats/movement-cave_spider_room.lua:31-58.
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, wire, wire_with_feedback, world_with_cells};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position, StepBenefit, StepBuff},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
};
use oteryn_protocol_oteryn::boss_encounter::{CAPABILITY_BOSS_ENCOUNTER_V1, COMMAND_TYPE_BOSS_COOLDOWNS};

const QUEST: &str = "oteryn:quest.fixture_solo_room";
const TRACK: &str = "oteryn:quest-progress/fixture/solo-room";
const ENTRY: Position = Position {
    x: 33039,
    y: 32103,
    floor: -12,
};
const ROOM: Position = Position {
    x: 33023,
    y: 32106,
    floor: -12,
};
const EXIT: Position = Position {
    x: 33056,
    y: 32004,
    floor: -11,
};
const BYSTANDER: Position = Position {
    x: 33052,
    y: 32103,
    floor: -12,
};
const ROOM_VISITOR: Position = Position {
    x: 33024,
    ..ROOM
};
const AFTER_MS: u32 = 5 * 60 * 1_000;
const ROOM_OCCUPIED_TEXT: &str = "There's someone fighting with Rat.";
const LEVEL_TEXT: &str = "All the players need to be level 250 or higher.";
const COOLDOWN_TEXT: &str = "You have to wait 10 hours to face Rat again!";

#[test]
fn solo_quest_room_ejects_the_admitted_player_after_the_last_monster_dies() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    cave_case(admin, "solo_quest_room_ejection", CaveRoom::EmptyAtTimeout)
}

#[test]
fn cave_spider_refuses_occupied_entry_then_ejects_only_the_admitted_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    cave_case(admin, "cave_spider_room_cleanup", CaveRoom::OccupiedEntry)
}

#[test]
fn cave_spider_first_deadline_cleans_new_monsters_and_second_deadline_ejects_second_player()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    cave_case(admin, "cave_spider_room_reentry", CaveRoom::TwoAdmissions)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaveRoom {
    EmptyAtTimeout,
    OccupiedEntry,
    TwoAdmissions,
}

fn cave_case(admin: String, name: &'static str, cave: CaveRoom) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
                let result = Box::pin(exercise_world(&h, None, None, Some(cave))).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "solo quest room case panicked")??;
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct BossRoom {
    pub minimum_level: u32,
    pub player_level: Option<i64>,
    pub already_fought: bool,
    pub with_occupant: bool,
    pub reenter_before_timeout: bool,
    pub allowed: bool,
    pub source: &'static str,
}

pub(super) async fn exercise(h: &Harness, boss: Option<BossRoom>) -> TestResult {
    exercise_world(h, boss, None, None).await
}

pub(super) async fn exercise_step_buff(h: &Harness, buff: StepBuff) -> TestResult {
    exercise_world(h, None, Some(buff), None).await
}

async fn exercise_world(
    h: &Harness,
    boss: Option<BossRoom>,
    buff: Option<StepBuff>,
    cave: Option<CaveRoom>,
) -> TestResult {
    let keep_cave_monsters = cave == Some(CaveRoom::OccupiedEntry);
    let buff = buff.map(|buff| {
        // Crystal movements_strangely_glowing_mark.lua:18,28.
        let message = match buff.benefit {
            StepBenefit::Heal { .. } => "The mythic fires beneath your feet heal you.",
            StepBenefit::Speed { .. } => "The mythic fires beneath your feet gave you speed.",
        };
        (buff, message)
    });
    let mut source = boss.map_or(
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/hidden_threats/movement-cave_spider_room.lua:31-58",
        |room| room.source
    );
    if buff.is_some() {
        source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/grimvale/movements_strangely_glowing_mark.lua:1-28";
    }
    let after_ms = if boss.is_some() { 600_000 } else { AFTER_MS };
    let mut callback_json = serde_json::json!({
        "key":"oteryn:quest-callback/fixture/solo-room", "quest":QUEST, "trigger":"step_in",
        "selector":{"position":ENTRY,"appearance":100,"native":{"position":ENTRY,"appearance":100,"ordinal":1}},
        "requires_world_admission":true, "reversible":false, "passages":[], "offerings":[],
        "storage":[{"track":TRACK,"minimum":9,"maximum":9,"set":10}],
        "effects":{"encounter":{
            "minimum_level":1, "players":[{"from":ENTRY,"to":ROOM}],
            "monsters":[{"creature":"oteryn:creature.rat","position":{"x":33034,"y":32107,"floor":-12}}],
            "room_min":{"x":33011,"y":32097,"floor":-12},
            "room_max":{"x":33051,"y":32117,"floor":-12},
            "ejection":{"after_ms":after_ms,"destination":EXIT}
        }},
        "return_on_failure":true, "failure_teleport":EXIT, "sources":[source]
    });
    if let Some(room) = boss {
        callback_json["storage"] = serde_json::json!([]);
        callback_json["requires_world_admission"] = serde_json::json!(false);
        callback_json["effects"]["encounter"]["minimum_level"] =
            serde_json::json!(room.minimum_level);
        callback_json["effects"]["encounter"]["boss_cooldown_seconds"] = serde_json::json!(36_000);
        callback_json["effects"]["encounter"]["boss_feedback"] = serde_json::json!({
            "occupied":ROOM_OCCUPIED_TEXT,
            "below_level":(room.minimum_level > 1).then_some(LEVEL_TEXT),
            "on_cooldown":COOLDOWN_TEXT,
            "return_effect":11
        });
        callback_json["failure_teleport"] = serde_json::Value::Null;
        if room.reenter_before_timeout {
            // The fixture's adjacent western tile is reachable by STEP and
            // lies beyond the room boundary.
            callback_json["effects"]["encounter"]["room_min"]["x"] =
                serde_json::json!(ROOM.x);
        }
    }
    if let Some((buff, message)) = buff {
        callback_json["storage"] = serde_json::json!([]);
        callback_json["state_operations"] = serde_json::json!([{
            "track":TRACK,"from":{"kind":"elapsed_at_least","seconds":61},"effect":{"kind":"set_now"}
        }]);
        callback_json["effects"] = serde_json::json!({"step_buff":buff});
        callback_json["success_message"] = serde_json::json!(message);
        callback_json["return_on_failure"] = serde_json::json!(false);
        callback_json["failure_teleport"] = serde_json::Value::Null;
    }
    let callback: Callback = serde_json::from_value(callback_json.clone())?;
    let bindings: Bindings = serde_json::from_value(serde_json::json!({
        "schema":1,"quests":[{
            "quest":QUEST,
            "recipe_sha256":<sha2::Sha256 as sha2::Digest>::digest(source).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "tracks":[{"key":TRACK,"initial":9,"minimum":9,"maximum":if buff.is_some() { i64::MAX } else { 10 }}],
            "stages":[{"key":"s1","completes":false,"trigger":{"kind":"step_in","callback":callback_json}}],
            "sources":[source]
        }]
    }))?;
    let catalogue =
        bindings.attach(&QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(debug)?)?;
    let world = world_with_cells(&callback, ENTRY, &[EXIT, BYSTANDER, ROOM_VISITOR])?;
    prepare_current_session(h).await?;
    if let Some(level) = boss.and_then(|room| room.player_level) {
        // Arrange the progression snapshot before admitting the real STEP command.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let updated = sqlx::query(
            "UPDATE game_character_progression_state SET level=$2 WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .bind(level)
        .execute(&mut *tx)
        .await?;
        assert_eq!(updated.rows_affected(), 1);
        tx.commit().await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let start = local(Position {
        x: ENTRY.x - 1,
        ..ENTRY
    });
    let frame = activated.frame_binding().digest();
    let digest = generation.identity().server_artifact_digest();
    let client_digest = generation.identity().client_artifact_digest();
    let map = world.map_revision_digest();
    deps.combat_room = Some(
        activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                map,
                crate::content::NativeEntryStart {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
            )
            .ok_or("solo room bundle")?,
    );
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        digest,
        client_digest,
        frame,
        map,
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue));

    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    if boss.is_some() {
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[CAPABILITY_BOSS_ENCOUNTER_V1],
        ).ok_or("boss cooldown capability")?;
    }
    if boss.is_some() || buff.is_some() {
        owner.states.chat.admit(admitted.game_session_id, "Grimvale visitor", owner.owner_now());
    }
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    admitted.item_fence = Some(fence.item_fence());
    deps.runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digest.as_slice())
        .bind(client_digest.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            digest,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;

    if let Some((buff, message)) = buff {
        use oteryn_protocol_oteryn::chat::ChatLine;

        let session = admitted.game_session_id;
        {
            let runtime = deps.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .get_mut(&runtime, deps.actor, session)
                .ok_or("mark actor")?
                .set_health_for_test(1);
        }
        let mut first_clock = None;
        for (command, direction, now) in [
            (1, StepDirection::East, 1_000_000),
            (2, StepDirection::West, 31_000_000),
            (3, StepDirection::East, 33_000_000),
        ] {
            owner
                .states
                .test_owner_time
                .store(now + 1, Ordering::Relaxed);
            let (reply, mut feedback) = wire_with_feedback(
                &owner,
                &mut admitted,
                command,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
            )
            .await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
                StepDisposition::Moved
            );
            feedback.extend(owner.states.chat.drain(session));
            if command == 1 {
                assert!(matches!(feedback.as_slice(), [ChatLine::Private { text, .. }] if text == message));
            } else {
                assert!(feedback.is_empty(), "the mark must stay silent during cooldown");
            }
            assert_eq!(movement_effect_count(&owner, local(ENTRY), buff.magic_effect)?, 1);
            let clock = sqlx::query_as::<_, (i64, i64)>(
                "SELECT value, committed_character_revision::bigint FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
            ).bind(id(41).as_slice()).bind(TRACK).fetch_one(&h.pool).await?;
            assert!(clock.0 > 9);
            if let Some(first) = first_clock {
                assert_eq!(clock, first, "steps in cooldown must preserve the first receipt");
            } else {
                first_clock = Some(clock);
            }
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let current = states
                .get_mut(&runtime, deps.actor, session)
                .ok_or("mark actor")?;
            if command == 2 {
                current.set_health_for_test(1);
                continue;
            }
            assert_eq!(
                runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(ENTRY)
            );
            match buff.benefit {
                StepBenefit::Heal { amount } => {
                    let vitals = current.vitals();
                    let expected = if command == 1 {
                        (1 + u32::from(amount)).min(vitals.max_health)
                    } else {
                        1
                    };
                    assert_eq!(vitals.health, expected);
                }
                StepBenefit::Speed { .. } => {
                    assert_eq!(
                        current.owned_conditions().effective_speed(now).is_some(),
                        command == 1
                    );
                }
            }
        }
        return Ok(());
    }

    let cooldown = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT available_at FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid AND boss_key='oteryn:creature.rat'",
        ).bind(id(41).as_slice()).fetch_optional(&h.pool).await
    };
    let time_before: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    if boss.is_some_and(|room| room.already_fought) {
        sqlx::query("INSERT INTO game_character_boss_cooldowns(character_id,boss_key,available_at,admission,reset_epoch)
            VALUES(encode($1,'hex')::uuid,'oteryn:creature.rat',$2,encode($3,'hex')::uuid,0)")
            .bind(id(41).as_slice()).bind(time_before+33_000_000).bind(id(82).as_slice()).execute(&h.pool).await?;
    }
    let previous_cooldown = cooldown().await?;
    let mut eventual_bystander = None;
    let test_occupied_entry = keep_cave_monsters || boss.is_some_and(|room| room.with_occupant);
    if test_occupied_entry {
        let second_session = crate::foundation::GameSessionId::decode(&id(97)).map_err(debug)?;
        let other = {
            let mut runtime = deps.runtime.lock().await;
            let reserve = runtime
                .reserve_fresh_session(second_session)
                .map_err(debug)?;
            let other = runtime.commit_fresh_session(reserve).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(other, local(ROOM))
                .map_err(debug)?;
            other
        };
        owner
            .states
            .test_owner_time
            .store(1_000_001, Ordering::Relaxed);
        let (reply, feedback) = wire_with_feedback(
            &owner,
            &mut admitted,
            1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(cooldown().await?, previous_cooldown);
        assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), None);
        if boss.is_some() {
            use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeechMode};
            let mut feedback = feedback;
            feedback.extend(owner.states.chat.drain(admitted.game_session_id));
            assert!(matches!(feedback.as_slice(), [ChatLine::Local {
                text, mode: ChatSpeechMode::MonsterSay, position, ..
            }] if text == ROOM_OCCUPIED_TEXT && (position.x, position.y, position.floor) == (start.x, start.y, start.floor)), "{feedback:?}");
            assert_teleport_effect(&owner, start)?;
        }
        if keep_cave_monsters {
            assert_teleport_effect(&owner, local(ENTRY))?;
            assert_teleport_effect(&owner, local(EXIT))?;
            let awarded = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND value=10)",
            )
            .bind(id(41).as_slice())
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?;
            assert!(!awarded, "an occupied Cave Spider room grants no progress");
        }
        let mut runtime = deps.runtime.lock().await;
        let position = runtime.read_actor_position(deps.actor).map_err(debug)?;
        assert_eq!(
            position.position(),
            if keep_cave_monsters { local(EXIT) } else { start }
        );
        assert_eq!(
            runtime
                .read_actor_position(other)
                .map_err(debug)?
                .position(),
            local(ROOM)
        );
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        runtime.remove_test_actor(other).map_err(debug)?;
        if keep_cave_monsters {
            let map = owner.bundle_map().ok_or("room map")?;
            let doors = crate::map::floor::BundleDoors(map.facts.doors());
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime, owner.movement_cells(), deps.actor, admitted.game_session_id,
                position, start, false, &doors,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        } else {
            let reserve = runtime
                .reserve_fresh_session(second_session)
                .map_err(debug)?;
            let other = runtime.commit_fresh_session(reserve).map_err(debug)?;
            let bystander = local(BYSTANDER);
            runtime
                .initialize_pinned_test_position(other, bystander)
                .map_err(debug)?;
            eventual_bystander = Some((other, bystander));
        }
    }
    let (command, enter_at) = if test_occupied_entry {
        (2, 5_000_000)
    } else {
        (1, 1_000_000)
    };
    owner
        .states
        .test_owner_time
        .store(enter_at + 1, Ordering::Relaxed);
    let (reply, feedback) = wire_with_feedback(
        &owner,
        &mut admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    if let Some(room) = boss.filter(|room| !room.allowed) {
        use oteryn_protocol_oteryn::chat::ChatLine;
        let expected = if room.already_fought { COOLDOWN_TEXT } else { LEVEL_TEXT };
        let mut feedback = feedback;
        feedback.extend(owner.states.chat.drain(admitted.game_session_id));
        assert!(matches!(feedback.as_slice(), [ChatLine::Private { text, .. }] if text == expected), "{feedback:?}");
        assert_teleport_effect(&owner, start)?;
        let runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            start
        );
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        drop(runtime);
        assert_eq!(cooldown().await?, previous_cooldown);
        assert_boss_cooldown_on_wire(&owner, &mut admitted, previous_cooldown).await?;
        return Ok(());
    }
    if boss.is_some() {
        use oteryn_protocol_oteryn::chat::ChatLine;
        let mut feedback = feedback;
        feedback.extend(owner.states.chat.drain(admitted.game_session_id));
        assert!(!feedback.iter().any(|line| matches!(
            line,
            ChatLine::Local { text, .. } | ChatLine::Private { text, .. }
                if [ROOM_OCCUPIED_TEXT, LEVEL_TEXT, COOLDOWN_TEXT].contains(&text.as_str())
        )), "successful entry repeated refusal feedback: {feedback:?}");
        assert_teleport_effect(&owner, local(ROOM))?;
        let expiry = cooldown().await?.ok_or("recorded boss cooldown")?;
        let time_after: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!((time_before + 36_000_000..=time_after + 36_000_000).contains(&expiry));
        // Crystal actions_portal_minis_grimvale.lua:126-127 publishes the
        // newly set clock to Bosstiary. Check the existing cooldown query
        // before leaving this room and independently of raw SQL reads.
        assert_boss_cooldown_on_wire(&owner, &mut admitted, Some(expiry)).await?;
    }
    let read_progress = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
        ).bind(id(41).as_slice()).bind(TRACK).fetch_one(&h.pool).await
    };
    if boss.is_none() {
        assert_eq!(read_progress().await?, 10);
        assert_eq!(
            movement_effect_count(&owner, local(ENTRY), 11)?,
            if keep_cave_monsters { 2 } else { 1 },
            "Cave Spider emits one departure cue per committed teleport"
        );
        assert_teleport_effect(&owner, local(ROOM))?;
    }
    if cave == Some(CaveRoom::TwoAdmissions) {
        return two_cave_admissions(&owner, &deps, h, admitted, start, enter_at).await;
    }
    let (room_visitor, cave_creatures) = {
        let mut runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(ROOM)
        );
        let creatures = runtime.current_live_creature_census().map_err(debug)?;
        assert_eq!(creatures.len(), 1);
        let cave_creatures = if keep_cave_monsters {
            let [boss] = creatures.try_into().map_err(|_| "Cave Spider boss")?;
            // clearBossRoom scans current spectators. The two later monsters
            // share a native profile and differ only by their room positions.
            let rat = "oteryn:creature.rat";
            let health = runtime.companion_policy(rat).map_err(debug)?.maximum_health;
            let [inside, outside] = runtime.commit_quest_creature_spawn(&[
                (rat, health, local(ROOM_VISITOR)),
                (rat, health, local(BYSTANDER)),
            ]).map_err(debug)?.try_into().map_err(|_| "Cave Spider cleanup fixtures")?;
            Some((boss, inside, outside))
        } else if boss.is_none() {
            for creature in creatures {
                runtime.remove_encounter_actor(creature).map_err(debug)?;
            }
            None
        } else {
            None
        };
        let visitor_position = if keep_cave_monsters {
            Some(Position { y: ROOM.y + 1, ..ROOM })
        } else {
            eventual_bystander.map(|_| ROOM_VISITOR)
        };
        let visitor = if let Some(at) = visitor_position {
            let session = crate::foundation::GameSessionId::decode(&id(98)).map_err(debug)?;
            let reserve = runtime.reserve_fresh_session(session).map_err(debug)?;
            let visitor = runtime.commit_fresh_session(reserve).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(visitor, local(at))
                .map_err(debug)?;
            Some((visitor, if keep_cave_monsters { local(at) } else { local(EXIT) }))
        } else {
            None
        };
        (visitor, cave_creatures)
    };
    let due = enter_at + u64::from(after_ms) * 1_000;
    let mut next_due = None;
    assert_eq!(
        owner.states.quest_callbacks.next_encounter_due_us(),
        Some(due)
    );
    if boss.is_some_and(|room| room.reenter_before_timeout) {
        let session = admitted.game_session_id;
        owner
            .states
            .test_owner_time
            .store(enter_at + 2_000_001, Ordering::Relaxed);
        let step_command = admitted.continuity.next_command_id;
        let reply = wire(
            &owner,
            &mut admitted,
            step_command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::West),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        let old_boss = {
            let map = owner.bundle_map().ok_or("room map")?;
            let doors = crate::map::floor::BundleDoors(map.facts.doors());
            let mut runtime = deps.runtime.lock().await;
            let expected = runtime.read_actor_position(deps.actor).map_err(debug)?;
            assert_eq!(
                expected.position(),
                local(Position { x: ROOM.x - 1, ..ROOM })
            );
            let [old_boss] = runtime.current_live_creature_census().map_err(debug)?
                .try_into().map_err(|_| "first visit boss")?;
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime, owner.movement_cells(), deps.actor, session, expected, start, false, &doors,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
            old_boss
        };
        // Advance the fixture's persisted character cooldown independently of
        // its encounter clock, modeling the next eligible entrant.
        let expired = sqlx::query(
            "UPDATE game_character_boss_cooldowns SET available_at=0 WHERE character_id=encode($1,'hex')::uuid AND boss_key='oteryn:creature.rat'",
        )
        .bind(id(41).as_slice())
        .execute(&h.pool)
        .await?;
        assert_eq!(expired.rows_affected(), 1);
        let reentry = enter_at + 5_000_000;
        owner.states.test_owner_time.store(reentry + 1, Ordering::Relaxed);
        let step_command = admitted.continuity.next_command_id;
        let reply = wire(
            &owner,
            &mut admitted,
            step_command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        next_due = Some(reentry + u64::from(after_ms) * 1_000);
        // Crystal does not cancel the first addEvent when another player enters.
        assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), Some(due));
        let runtime = deps.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(deps.actor).map_err(debug)?.position(), local(ROOM));
        assert!(!runtime.contains_live_creature(old_boss));
        assert_eq!(runtime.current_live_creature_census().map_err(debug)?.len(), 1);
    }
    owner.states.test_owner_time.store(due, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        local(ROOM)
    );
    owner
        .states
        .test_owner_time
        .store(due + 1, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        local(EXIT)
    );
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), next_due);
    if let Some(next_due) = next_due {
        owner.states.test_owner_time.store(next_due + 1, Ordering::Relaxed);
        owner.drain_quest_encounter_deadlines().await;
        let runtime = deps.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(deps.actor).map_err(debug)?.position(), local(EXIT));
    }
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), None);
    if let Some((other, bystander)) = eventual_bystander {
        let runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(other)
                .map_err(debug)?
                .position(),
            bystander
        );
    }
    if let Some((visitor, expected)) = room_visitor {
        let runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime.read_actor_position(visitor).map_err(debug)?.position(),
            expected
        );
    }
    if boss.is_some() {
        let runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime.current_live_creature_census().map_err(debug)?.len(),
            1,
            "Crystal boss-room timeout clears the players and leaves the live boss"
        );
    }
    if let Some((boss, inside, outside)) = cave_creatures {
        let runtime = deps.runtime.lock().await;
        assert!(!runtime.contains_live_creature(boss));
        assert!(!runtime.contains_live_creature(inside));
        assert!(runtime.contains_live_creature(outside));
    }
    if boss.is_none() {
        assert_eq!(read_progress().await?, 10);
    }
    Ok(())
}

async fn assert_boss_cooldown_on_wire(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    expiry: Option<i64>,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    let (status, reply) = super::boss_encounter::exchange_kind(
        owner,
        admitted,
        command,
        COMMAND_TYPE_BOSS_COOLDOWNS,
        &[],
    ).await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    let (now, rows, cursor) = super::boss_encounter::page(&reply)?;
    assert!(cursor.is_empty());
    if let Some(expiry) = expiry {
        let expiry = u64::try_from(expiry)?;
        assert!(now < expiry);
        assert_eq!(rows, [("oteryn:creature.rat".into(), expiry)]);
    } else {
        assert!(rows.is_empty());
    }
    Ok(())
}

fn assert_teleport_effect(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    place: crate::foundation::MovementLocalPosition,
) -> TestResult {
    assert_eq!(movement_effect_count(owner, place, 11)?, 1);
    Ok(())
}

fn movement_effect_count(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    place: crate::foundation::MovementLocalPosition,
    effect_id: u16,
) -> TestResult<usize> {
    let map = owner.bundle_map().ok_or("room map")?;
    let effects = map.facts.movement_effects_since(0);
    let at = crate::map::overlay::TilePos {
        x: u16::try_from(place.x)?,
        y: u16::try_from(place.y)?,
        floor: -i8::try_from(place.floor)?,
    };
    Ok(effects
        .iter()
        .filter(|(_, position, effect)| *position == at && *effect == effect_id)
        .count())
}


async fn two_cave_admissions(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    h: &Harness,
    first: super::AdmittedSession,
    start: crate::foundation::MovementLocalPosition,
    first_enter_at: u64,
) -> TestResult {
    let first_actor = deps.actor;
    let first_due = first_enter_at + u64::from(AFTER_MS) * 1_000;
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), Some(first_due));
    let first_monster = {
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(first_actor).map_err(debug)?;
        assert_eq!(before.position(), local(ROOM));
        let [monster] = runtime.current_live_creature_census().map_err(debug)?
            .try_into().map_err(|_| "first room monster")?;
        let map = owner.bundle_map().ok_or("room map")?;
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let leave = crate::map::floor::prepare_quest_callback_teleport(
            &runtime, owner.movement_cells(), first_actor, first.game_session_id,
            before, local(BYSTANDER), false, &doors,
        ).map_err(debug)?;
        runtime.commit_source_step(leave).map_err(debug)?;
        monster
    };
    let (second_actor, mut second) =
        super::grizzly_boss::vacancy::second_player(owner, deps, h, start).await?;
    let second_enter_at = first_enter_at + 5_000_000;
    owner.states.test_owner_time.store(second_enter_at + 1, Ordering::Relaxed);
    let reply = wire(
        owner,
        &mut second,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let credited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks WHERE character_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND track_key=$3 AND value=10",
    )
    .bind(id(41).as_slice())
    .bind(id(101).as_slice())
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(credited, 2, "each admission credits its own character");
    let second_monster = {
        let runtime = owner.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(second_actor).map_err(debug)?.position(), local(ROOM));
        assert!(!runtime.contains_live_creature(first_monster));
        let [monster] = runtime.current_live_creature_census().map_err(debug)?
            .try_into().map_err(|_| "second room monster")?;
        monster
    };

    // Crystal clearBossRoom (data/libs/functions/functions.lua:226-238)
    // targets the first entrant but scans all current monsters in the room.
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), Some(first_due));
    owner.states.test_owner_time.store(first_due + 1, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(!runtime.contains_live_creature(second_monster));
        assert_eq!(runtime.read_actor_position(first_actor).map_err(debug)?.position(), local(BYSTANDER));
        assert_eq!(runtime.read_actor_position(second_actor).map_err(debug)?.position(), local(ROOM));
    }
    let second_due = second_enter_at + u64::from(AFTER_MS) * 1_000;
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), Some(second_due));
    owner.states.test_owner_time.store(second_due + 1, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    let runtime = owner.runtime.lock().await;
    assert_eq!(runtime.read_actor_position(first_actor).map_err(debug)?.position(), local(BYSTANDER));
    assert_eq!(runtime.read_actor_position(second_actor).map_err(debug)?.position(), local(EXIT));
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), None);
    Ok(())
}
