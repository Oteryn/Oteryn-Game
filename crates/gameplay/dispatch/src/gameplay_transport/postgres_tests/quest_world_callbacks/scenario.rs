use super::*;

pub(crate) fn scenario(name: &str, trigger: Position, lever_fixture: bool) -> TestResult {
    scenario_case(name, trigger, lever_fixture, None)
}

#[allow(
    dead_code,
    reason = "per-quest stateless PG cases are independently picked after the common harness"
)]
pub(crate) fn stateless_scenario(
    name: &str,
    trigger: Position,
    expected_destination: Position,
) -> TestResult {
    scenario_case(name, trigger, false, Some(expected_destination))
}

fn scenario_case(
    name: &str,
    trigger: Position,
    lever_fixture: bool,
    stateless_destination: Option<Position>,
) -> TestResult {
    let m07_dreamer = name.starts_with("m07/dreamer-lever-");
    let qm48 = name == "qm48/mammoth";
    let m01b_explore = name.starts_with("m01b/");
    let small_ruby = name.starts_with("l6-small-ruby");
    let shared_gate = name.starts_with("l6-tomb-gate-");
    let shared_switch = name.starts_with("l6-tomb-switch-");
    let tomb_native = name.starts_with("l6-tomb-native-");
    let carried_step = name.starts_with("l6-tomb-carried-") || tomb_native;
    let mead = name == "l6/waterskin-of-mead";
    let spirit = name == "qengine-rewards-questpc4-spirit";
    let native_tower = name.starts_with("qengine-rewards-questpc4-native-tower-");
    let native_switch = name.starts_with("l6-tomb-switch-native-");
    let native_upper = name.starts_with("qengine-rewards-questpc4-paradox-upper-");
    let native_story = name.starts_with("qengine-rewards-questpc4-paradox-story-");
    let diprath_room = name.starts_with("l6-diprath-room-");
    let native_signs = name.starts_with("l6-native-signs-");
    let native_flames = name.starts_with("l6-native-flames-");
    let native_room = name.starts_with("l6-native-room-");
    let native_altar = name.starts_with("l6-floor-assembly-native");
    let native_gate = name.starts_with("l6-tomb-gate-native-");
    let native_crop = m07_dreamer || qm48
        || m01b_explore
        || small_ruby
        || mead
        || spirit
        || tomb_native
        || native_tower
        || native_switch
        || native_gate
        || native_altar
        || native_room
        || native_flames
        || native_signs
        || diprath_room
        || native_story
        || native_upper;
    let start = if m07_dreamer {
        Position { x:32762,y:32289,floor:-14 }
    } else if qm48 {
        Position {
            x: trigger.x,
            y: trigger.y - 1,
            ..trigger
        }
    } else if m01b_explore {
        Position {
            x: trigger.x + 1,
            ..trigger
        }
    } else if native_upper {
        super::super::l6_paradox_upper::start(name)
    } else if native_story {
        super::super::l6_paradox_story::start()
    } else if diprath_room {
        super::super::l6_diprath_room::start()?
    } else if native_signs {
        super::super::l6_native_signs::start()?
    } else if native_flames {
        super::super::l6_native_flames::start()?
    } else if native_room {
        super::super::l6_native_rooms::start(name)?
    } else if native_altar {
        super::super::l6_floor_native::start()
    } else if native_gate {
        super::super::l6_shared_gate_native::start(name)?
    } else if native_tower {
        super::super::l6_paradox_native::start(name)
    } else if native_switch {
        super::super::l6_shared_switch_native::start(name)?
    } else if tomb_native {
        Position {
            x: trigger.x,
            y: trigger.y - 1,
            ..trigger
        }
    } else if spirit {
        Position {
            x: 33081,
            y: 31159,
            floor: -8,
        }
    } else if mead {
        Position {
            x: trigger.x,
            y: trigger.y + 1,
            ..trigger
        }
    } else {
        Position {
            x: trigger.x - 1,
            ..trigger
        }
    };
    let reward_fluids = name == "l6-reward-fluids";
    let ancient_instruments = name == "l6-ancient-instruments";
    let floor_profiles = name.starts_with("l6-treasure-reward-")
        || matches!(
            name,
            "l6-floor-profiles" | "l6-ruby-profile" | "l6-stag-item-profile"
        );
    let room_raise = name.starts_with("l6-room-raise-");
    let floor_assembly = name.starts_with("l6-floor-assembly");
    let reusable_tool = name.starts_with("l6-reusable-tool");
    let monk = name.starts_with("monk-guidance");
    let qengine_reward = name.starts_with("qengine-rewards-");
    let outfit_world = name.starts_with("l1-") && name.ends_with("-wardrobe");
    let classic_npc = qengine_reward
        || matches!(
            name,
            "engnpc-m04"
                | "catalogue-npc"
                | "catalogue-npc-end"
                | "catalogue-npc-calendar"
                | "catalogue-npc-xp"
                | "nx6-wayfarer"
                | "classic-quests-npc"
                | "classic-rottin-disrepair"
                | "threatened-dreams-npc"
                | "conditional-quest-greetings"
        );
    let rotten_purchase = name == "rotten-blood-access-purchase";
    let target_swap = name == "fish-napping-key-target-swap";
    let mut callback = if native_signs || diprath_room {
        callback("ancient-tombs-diprath-sign-1")?
    } else if native_flames {
        callback("ancient-tombs-morguthis-seven-flame-gate")?
    } else if native_room {
        super::super::l6_native_rooms::callback(name)?
    } else {
        callback(if name == "triangle-tower-wall" {
            "triangle-tower-lever"
        } else if ancient_instruments {
            "ancient-tombs-diprath-sign-1"
        } else if classic_npc || small_ruby || reward_fluids {
            "behemoth"
        } else if rotten_purchase {
            "rotten-blood/sacrifice"
        } else if monk {
            "monk-guidance"
        } else if target_swap
            || reusable_tool
            || floor_profiles
            || floor_assembly
            || room_raise
            || carried_step
            || shared_switch
            || shared_gate
        {
            "fish-napping-key"
        } else {
            name.strip_suffix("-expiry").unwrap_or(name)
        })?
    };
    if shared_gate {
        let unique = name.rsplit('-').next().ok_or("gate UID")?.parse()?;
        callback.key = "oteryn:quest-callback/l6-shared-gate-fixture".into();
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn;
        callback.selector.position = Some(trigger);
        callback.selector.unique = Some(unique);
        callback.selector.action = None;
        callback.selector.appearance = Some(100);
        callback.lever = None;
        callback.storage.clear();
        callback.offerings.clear();
        callback.passages.clear();
        callback.item_on_target = None;
        callback.effects = Default::default();
        callback.teleport = Some(super::super::l6_shared_gates::spec(unique)?.2);
        callback.return_on_failure = true;
    }
    if shared_switch {
        callback.key = "oteryn:quest-callback/l6-shared-switch-fixture".into();
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use;
        callback.selector.position = Some(trigger);
        callback.selector.unique = None;
        callback.selector.action = Some(name.rsplit('-').next().ok_or("switch action")?.parse()?);
        callback.selector.appearance = Some(2772);
        callback.lever = Some([2772, 2773]);
        callback.reversible = false;
        callback.storage.clear();
        callback.offerings.clear();
        callback.passages.clear();
        callback.item_on_target = None;
        callback.teleport = None;
        callback.effects = Default::default();
    }
    if carried_step {
        // Geometry supplies the exact source UID and both floor endpoints only.
        // Production callback selection and physical branch ownership are unchanged.
        let unique = name.rsplit('-').next().ok_or("tomb UID")?.parse::<u16>()?;
        callback.key = "oteryn:quest-callback/l6-carried-step-fixture".into();
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn;
        callback.selector.position = Some(trigger);
        callback.selector.unique = Some(unique);
        callback.selector.action = None;
        callback.selector.appearance = Some(100);
        callback.item_on_target = None;
        callback.storage.clear();
        callback.passages.clear();
        callback.offerings.clear();
        callback.lever = None;
        callback.effects = Default::default();
        callback.teleport = stateless_destination;
        if tomb_native {
            callback.key = "oteryn:quest-callback/l6-native-step-fixture".into();
        }
    }
    if name.ends_with("-expiry") {
        callback
            .effects
            .portals
            .as_mut()
            .ok_or("portal pair")?
            .lifetime_us = Some(60_000_000);
    }
    if reusable_tool {
        callback = super::super::l6_reusable_tools::fixture(callback);
    }
    if floor_profiles || floor_assembly {
        callback.key = "oteryn:quest-callback/l6-floor-profile-fixture".into();
        callback.selector.unique = None;
        callback.selector.action = if name.ends_with("-unbound") {
            Some(40041)
        } else if floor_assembly {
            Some(40040)
        } else {
            None
        };
        callback.selector.appearance = Some(100);
        callback.selector.position = None;
        callback.item_on_target = None;
        callback.storage.clear();
        callback.teleport = None;
    }
    if ancient_instruments {
        // The fixture supplies source-verified native positions and appearances;
        // all matching and durable effects still come from the production registry.
        callback.key = "oteryn:quest-callback/l6-instrument-fixture".into();
        callback.selector.unique = None;
        callback.selector.position = Some(trigger);
        callback.selector.action = None;
        callback.selector.appearance = Some(3255);
    }
    if target_swap {
        callback
            .item_on_target
            .as_mut()
            .ok_or("item")?
            .target_replacement = Some(7777);
    }
    if lever_fixture {
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use;
        callback.selector.unique = Some(25012);
        callback.selector.appearance = Some(2772);
        callback.lever = Some([2772, 2773]);
    }
    if stateless_destination.is_some() {
        assert_eq!(callback.selector.position, Some(trigger));
    }
    if name.starts_with("qengine-rewards-questpc4-admission-") {
        callback = super::super::l6_world_admission::callback(name)?;
    }
    if name.starts_with("qengine-rewards-questpc4-puzzle-predicate") {
        callback = super::super::l6_world_puzzles::predicate_fixture(
            callback,
            trigger,
            name.ends_with("-refuse"),
        );
    }
    if name == "qengine-rewards-questpc4-puzzle-presence" {
        callback = super::super::l6_world_puzzles::presence_fixture(callback, trigger);
    }
    if name == "qengine-rewards-questpc4-puzzle-ladder" {
        callback = super::super::l6_world_puzzles::ladder_fixture(callback, trigger);
    }
    let world = world(&callback, trigger)?;
    run(async |admin| {
        let h = if qengine_reward || outfit_world || name == "srv7-reliable-ram" {
            Harness::create_quest_reward(
                admin,
                "quest_rewards",
                !name.ends_with("pirate") && !name.contains("-male-"),
            )
            .await
        } else {
            Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false).await
        }
        .map_err(|e| format!("callback harness: {e}"))?;
        if small_ruby || carried_step || native_tower || name == "triangle-tower-wall" {
            let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
                .fetch_one(&h.pool)
                .await?;
            if native_tower {
                eprintln!("POSTGRES_DATABASE_READY {name} native_scene_pending version={version}");
            } else if carried_step {
                eprintln!("POSTGRES_DATABASE_READY l6_carried_step version={version}");
            } else if name == "triangle-tower-wall" {
                eprintln!("POSTGRES_DATABASE_READY triangle_tower_wall version={version}");
            } else {
                eprintln!("POSTGRES_DATABASE_READY l6_small_ruby_native_pickup version={version}");
            }
        }
        prepare_current_session(&h)
            .await
            .map_err(|e| format!("callback session: {e}"))?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|e| format!("callback authority: {e:?}"))?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let world = if qm48 {
            super::super::qm48_mammoth::world(
                generation.native_gameplay().ok_or("mammoth policies")?,
            )?
        } else if m01b_explore {
            super::super::m01b_explore::world(
                generation
                    .native_gameplay()
                    .ok_or("native explore policies")?,
                name,
            )?
        } else if native_upper {
            super::super::l6_paradox_upper::world(
                generation
                    .native_gameplay()
                    .ok_or("native upper policies")?,
                name,
            )?
        } else if native_story {
            super::super::l6_paradox_story::world(
                generation
                    .native_gameplay()
                    .ok_or("native story policies")?,
            )?
        } else if diprath_room {
            super::super::l6_diprath_room::world(
                generation
                    .native_gameplay()
                    .ok_or("Diprath native policies")?,
            )?
        } else if native_signs {
            super::super::l6_native_signs::world(
                generation.native_gameplay().ok_or("native sign policies")?,
            )?
        } else if native_flames {
            super::super::l6_native_flames::world(
                generation
                    .native_gameplay()
                    .ok_or("native flame policies")?,
            )?
        } else if native_room {
            super::super::l6_native_rooms::world(
                generation.native_gameplay().ok_or("native room policies")?,
                name,
            )?
        } else if native_altar {
            super::super::l6_floor_native::world(
                generation
                    .native_gameplay()
                    .ok_or("native altar policies")?,
            )?
        } else if native_gate {
            super::super::l6_shared_gate_native::world(
                generation.native_gameplay().ok_or("native gate policies")?,
                name,
            )?
        } else if native_tower {
            super::super::l6_paradox_native::world(
                generation
                    .native_gameplay()
                    .ok_or("native tower policies")?,
                name,
            )?
        } else if native_switch {
            super::super::l6_shared_switch_native::world(
                generation
                    .native_gameplay()
                    .ok_or("native switch policies")?,
                name,
            )?
        } else if tomb_native {
            super::super::l6_tomb_native::world(
                generation.native_gameplay().ok_or("native Tomb policies")?,
                callback.selector.unique.ok_or("source UID")?,
            )?
        } else if small_ruby {
            super::super::l6_world_pickups::small_ruby(
                generation
                    .native_gameplay()
                    .ok_or("native pickup policies")?,
            )?
        } else if spirit {
            super::super::l6_spirit::world(
                generation
                    .native_gameplay()
                    .ok_or("native Spirit policies")?,
            )?
        } else if m07_dreamer {
            super::super::m07_dreamer_levers::world(generation.native_gameplay().ok_or("native Dreamer policies")?)?
        } else if mead {
            super::super::l6_mead::world(
                generation.native_gameplay().ok_or("native Mead policies")?,
            )?
        } else {
            world
        };
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame_binding = if native_crop {
            world.frame_binding_digest(deps.admitted.world_id)
        } else {
            activated.frame_binding().digest()
        };
        let bundle_room = if native_crop {
            Some(
                activated
                    .qualified_room()
                    .bundle_view(
                        cells.clone(),
                        frame_binding,
                        world.map_revision_digest(),
                        crate::content::NativeEntryStart {
                            x: i32::from(start.x),
                            y: i32::from(start.y),
                            floor: -i16::from(trigger.floor),
                        },
                    )
                    .ok_or("qualified native Bundle room")?,
            )
        } else {
            None
        };
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame_binding,
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(trigger.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            if name.starts_with("qengine-rewards-l12-") {
                128
            } else if native_story || (native_upper && name.ends_with("-complete")) {
                6
            } else if classic_npc {
                5
            } else if monk {
                4
            } else if callback.effects.encounter.is_some() {
                16
            } else {
                2
            },
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(start);
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let premium = if name.contains("l6-beggar-")
            || name.contains("l7-knight-")
            || name.contains("l8-")
            || name.contains("l9-")
            || name.contains("l10-documents-")
            || name.contains("l11-")
            || name.contains("l12-")
            || spirit
            || name.starts_with("qengine-rewards-questpc4-paradox")
        {
            use crate::gameplay_transport::spell_premium_coordinator::{
                SpellPremiumCoordinator, purchase_fixture,
            };
            let (consumer, clock, activation, _) =
                purchase_fixture::current(super::super::fixture::id(40));
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            Some(SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            ))
        } else {
            None
        };
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        if mead {
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
        }
        let mead_premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = premium.as_ref();
        if mead {
            owner.premium_coordinator = Some(&mead_premium);
        }
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        if let Some(room) = bundle_room.as_ref() {
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(room), Some(room));
        }
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        if qm48 {
            owner.quest_catalogue =
                Some(std::sync::Arc::new(super::super::qm48_mammoth::catalogue(
                    owner.quest_catalogue.as_ref().ok_or("mammoth catalogue")?,
                    &callback,
                )?));
        }
        if mead {
            owner.quest_catalogue = Some(std::sync::Arc::new(super::super::l6_mead::catalogue(
                owner.quest_catalogue.as_ref().ok_or("Mead catalogue")?,
                &callback,
            )?));
        }
        if name.starts_with("qengine-rewards-questpc4-admission-")
            || name.starts_with("qengine-rewards-questpc4-puzzle-predicate")
            || lever_fixture
            || target_swap
            || name.ends_with("-expiry")
            || name == "triangle-tower-lever"
        {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_binding.as_slice()).execute(&h.pool).await?;
        let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(|e| format!("callback admission equipment: {e:?}"))?;
        let native = generation.native_gameplay().ok_or("native")?;
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
            .map_err(|e| format!("callback starter items: {e:?}"))?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        if mead {
            owner.admit_premium(id(40), admitted.game_session_id);
        }
        owner.admit_quest_session(&admitted).await;
        if name == "barbarian-test-mead" {
            super::super::qm47_mead::exercise(&mut owner, &admitted, &h, fence, &callback).await?;
        } else if m01b_explore {
            super::super::m01b_explore::exercise(&mut owner, &mut admitted, &h, &callback).await?;
        } else if m07_dreamer {
            super::super::m07_dreamer_levers::exercise(&mut owner,&mut admitted,&h,&callback).await?;
        } else if mead {
            super::super::l6_mead::exercise(&mut owner, &mut admitted, &h, fence, &callback)
                .await?;
        } else if reward_fluids {
            super::super::l6_reward_fluids::exercise(&owner, admitted, &h, &authority).await?;
        } else if small_ruby {
            super::super::l6_world_pickups::exercise(&owner, &mut admitted, &h, 0).await?;
            if name.ends_with("-reset") {
                let mut next = super::super::l6_world_pickups::cycle(
                    &owner,
                    admitted,
                    &h,
                    &authority,
                    bundle_room.as_ref().ok_or("native reset room")?,
                )
                .await?;
                drop(owner);
                let reset_world = super::super::l6_world_pickups::small_ruby(
                    generation
                        .native_gameplay()
                        .ok_or("native pickup policies")?,
                )?;
                let reset_cells = reset_world
                    .movement_cells(activated.qualified_room().movement_cells())
                    .map_err(debug)?;
                let reset_frame = reset_world.frame_binding_digest(deps.admitted.world_id);
                let reset_room = activated
                    .qualified_room()
                    .bundle_view(
                        reset_cells.clone(),
                        reset_frame,
                        reset_world.map_revision_digest(),
                        crate::content::NativeEntryStart {
                            x: origin.x,
                            y: origin.y,
                            floor: origin.floor,
                        },
                    )
                    .ok_or("reset Bundle room")?;
                let reset_pin = ChannelContentPin::from_activation(
                    deps.admitted.world_id,
                    2,
                    generation.identity().server_artifact_digest(),
                    generation.identity().client_artifact_digest(),
                    reset_frame,
                    reset_world.map_revision_digest(),
                    (origin.x, origin.y, origin.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    deps.admitted.world_id,
                    deps.admitted.channel_id,
                    crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                    1,
                    2,
                    2,
                    "runtime-scope-assignment:2",
                    2,
                    reset_pin,
                )
                .map_err(debug)?;
                let reserved = runtime
                    .reserve_fresh_session(next.game_session_id)
                    .map_err(debug)?;
                deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(deps.actor, origin)
                    .map_err(debug)?;
                next.runtime_actor = Some(deps.actor);
                deps.admitted = next;
                deps.room = reset_room.clone();
                *deps.runtime.lock().await = runtime;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(generation);
                owner.movement_cells = (&reset_cells).into();
                owner.rooms =
                    crate::gameplay_transport::SeamRooms::new(Some(&reset_room), Some(&reset_room));
                owner.reset_epoch = h
                    .root
                    .active_world_reset_epoch(owner.world_id)
                    .await?
                    .ok_or("active reset epoch")?;
                deps.initialize(&owner).await;
                owner.admit_progression_session(&mut next).await;
                super::super::l6_world_pickups::exercise(&owner, &mut next, &h, 1).await?;
                super::super::l6_world_pickups::refuse_old_epoch(&owner, next, &h).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
        } else if let Some(source_id) = super::super::l6_treasure_rewards::source_id(name) {
            super::super::l6_treasure_rewards::exercise(
                &mut owner,
                &mut admitted,
                &h,
                fence,
                source_id,
            )
            .await?;
        } else if qm48 {
            super::super::qm48_mammoth::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "l6-stag-item-profile" {
            super::super::l6_stag_item::exercise(&mut owner, &mut admitted, &h, fence).await?;
        } else if name == "l6-ruby-profile" {
            super::super::l6_ruby_profile::exercise(&owner, &h).await?;
        } else if floor_profiles {
            super::super::l6_floor_profiles::exercise(&mut owner, &mut admitted, &h, fence).await?;
        } else if diprath_room {
            super::super::l6_diprath_room::exercise(&mut owner, &mut admitted, &h, name).await?;
        } else if native_signs {
            super::super::l6_native_signs::exercise(&mut owner, &mut admitted, &h, name).await?;
        } else if native_flames {
            super::super::l6_native_flames::exercise(&mut owner, &mut admitted, &h, name).await?;
        } else if native_room {
            super::super::l6_native_rooms::exercise(&mut owner, &mut admitted, &h, name).await?;
        } else if room_raise {
            super::super::l6_room_raise::exercise(&mut owner, &mut admitted, &h, name).await?;
        } else if native_altar {
            super::super::l6_floor_native::exercise(&mut owner, &mut admitted, &h, fence, name)
                .await?;
        } else if floor_assembly {
            super::super::l6_floor_assembly::exercise(&mut owner, &mut admitted, &h, fence, name)
                .await?;
        } else if native_gate {
            super::super::l6_shared_gate_native::exercise(&mut owner, &mut admitted, &h, name)
                .await?;
        } else if shared_gate {
            super::super::l6_shared_gates::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                trigger,
                name,
            )
            .await?;
        } else if native_switch {
            super::super::l6_shared_switch_native::exercise(
                &owner,
                &mut admitted,
                &h,
                trigger,
                name,
            )
            .await?;
        } else if shared_switch && name.contains("-treasure-") {
            super::super::l6_shared_switch_treasure::exercise(
                &mut owner,
                &mut admitted,
                &h,
                trigger,
                name,
                deps.actor,
            )
            .await?;
        } else if shared_switch && name.contains("-timer-") {
            super::super::l6_shared_switch_timers::exercise(
                &mut owner,
                &mut admitted,
                &h,
                trigger,
                name,
            )
            .await?;
        } else if shared_switch {
            super::super::l6_shared_switches::exercise(&owner, &mut admitted, &h, trigger).await?;
        } else if carried_step {
            if name.contains("-present-") {
                let recipe = oteryn_gameplay_quests::quest::carried_step::RECIPES
                    .iter()
                    .find(|r| Some(r.unique) == callback.selector.unique)
                    .ok_or("source key recipe")?;
                super::input_item(
                    &owner,
                    fence.item_fence(),
                    &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                        position: trigger,
                        item: recipe.item.into(),
                        count: 1,
                    },
                    71,
                    true,
                )
                .await?;
            }
            super::stateless::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                stateless_destination.ok_or("source branch destination")?,
            )
            .await?;
            super::super::l6_carried_step::verify(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                name.contains("-present-"),
                tomb_native,
            )
            .await?;
        } else if ancient_instruments {
            super::super::l6_ancient_tombs::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                trigger,
            )
            .await?;
        } else if name == "triangle-tower-wall" {
            super::super::triangle_tower::exercise(
                &mut owner,
                &mut admitted,
                &world,
                &callback,
                trigger,
            )
            .await?;
        } else if name == "engnpc-m04" {
            super::super::engnpc_m04_completion::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name == "catalogue-npc-xp" {
            super::super::npc_program::exercise_xp(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "catalogue-npc-calendar" {
            super::super::npc_program::exercise_calendar(&mut owner, &mut admitted, &h).await?;
        } else if name == "catalogue-npc-end" {
            super::super::npc_program::exercise_end(&mut owner, &mut admitted).await?;
        } else if name == "catalogue-npc" {
            super::super::npc_program::exercise(&mut owner, &mut admitted, &h, &authority).await?;
        } else if name == "srv7-reliable-ram" {
            super::super::srv7_reliable_ram::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &callback,
                trigger,
            )
            .await?;
        } else if outfit_world {
            super::super::qengine_access::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                trigger,
            )
            .await?;
        } else if qengine_reward {
            super::super::qengine_rewards::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name,
            )
            .await?;
        } else if callback.effects != Default::default()
            || callback.quest == "oteryn:quest.waterfall_quest" && stateless_destination.is_none()
        {
            super::super::qengine_access::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                trigger,
            )
            .await?;
        } else if let Some(expected_destination) = stateless_destination {
            super::stateless::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                expected_destination,
            )
            .await?;
        } else if name.starts_with("dryad-garden-") {
            super::super::srvwld_dryad_garden::exercise(
                &owner,
                &mut admitted,
                deps.actor,
                &callback,
                trigger,
            )
            .await?;
        } else if name == "srv6-schrodinger-island" {
            super::super::srv6_island_title::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &callback,
                origin,
            )
            .await?;
        } else if name == "nx6-wayfarer" {
            super::super::nx6_wayfarer::exercise(&mut owner, &mut admitted, &h, &authority).await?;
        } else if name == "conditional-quest-greetings" {
            super::super::conditional_quest_greetings::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name == "threatened-dreams-npc" {
            super::super::threatened_dreams::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "classic-rottin-disrepair" {
            super::super::classic_quests::exercise_rottin(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if classic_npc {
            super::super::classic_quests::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if matches!(
            name,
            "outlaw-camp-entrance"
                | "banshees-first-seal-return"
                | "banshees-second-seal"
                | "banshees-final-battle-exit"
        ) {
            super::super::classic_quests::exercise_step(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &world,
                &callback,
                trigger,
                origin,
            )
            .await?;
        } else if monk {
            super::super::monk_shrines::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &world,
                name.ends_with("full-backpack"),
                name.ends_with("capacity"),
            )
            .await?;
        } else if callback.key == oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE {
            super::rotten_blood::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
                rotten_purchase,
            )
            .await?;
        } else if reusable_tool {
            super::super::l6_reusable_tools::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                if name.ends_with("multitool") {
                    "oteryn:item.tibia.i9598"
                } else {
                    "oteryn:item.tibia.i3304"
                },
            )
            .await?;
        } else if callback.item_on_target.is_some()
            || name == "outlaw-camp-door"
            || name == "travelling-trader-delivery"
        {
            super::actions::exercise(
                &owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
                origin,
            )
            .await?;
        } else if !lever_fixture && !callback.offerings.is_empty() {
            for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
            }
            let east = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, east.clone()).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(trigger)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 0);
            offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
            wire(
                &owner,
                &mut admitted,
                2,
                1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                    oteryn_protocol_oteryn::world_spatial::StepDirection::West,
                ),
            )
            .await?;
            wire(&owner, &mut admitted, 3, 1, east).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 4);
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
        } else if name == "lions-rock-inner-sanctum-entry" {
            super::lions_rock::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                origin,
            )
            .await?;
        } else if name == "unnatural-selection-dance-2" {
            super::unnatural_selection::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
            )
            .await?;
        } else if name == "liquid-black-shortcut" {
            // The source shortcut has no text. Exercise the generic authored refusal message
            // without changing that source binding in production content.
            let mut message_callback = callback.clone();
            message_callback.failure_message = Some("Finish the visitor mission first.".into());
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![message_callback])
                    .is_ok()
            );
            owner
                .states
                .chat
                .admit(admitted.game_session_id, "Callback", owner.owner_now());
            let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, step.clone()).await?;
            assert!(owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
                matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == "Finish the visitor mission first.")));
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            for (n, key) in ["npc_1", "action_2", "action_1", "npc_2"]
                .into_iter()
                .enumerate()
            {
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: format!(
                        "oteryn:quest-transition/quest/u9_4/liquid_black_quest/visitor/{key}"
                    ),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(100 + u64::try_from(n)?)
                            .map_err(debug)?,
                    )),
                };
                assert!(matches!(
                    slot.commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request,
                        owner.quest_catalogue.clone().ok_or("catalogue")?
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::Committed(_)
                ));
            }
            drop(slot);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            wire(&owner, &mut admitted, 2, 1, step).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&5));
        } else {
            let key =
                oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
                    .ok_or("key")?;
            let target = |revision| {
                let mut placement = world.base().digest().to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                WorldObjectTarget {
                    placement,
                    expected_revision: revision,
                }
            };
            if lever_fixture {
                for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                    offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
                }
            }
            let result = wire(
                &owner,
                &mut admitted,
                1,
                2,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                    .map_err(debug)?,
            )
            .await?;
            let expected = if lever_fixture {
                UseDisposition::Rejected
            } else {
                UseDisposition::Committed
            };
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
                expected
            );
            if lever_fixture {
                let untouched: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(untouched, 0);
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert!(!copy.tracks().contains_key(&callback.storage[0].track));
                offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let context = {
                    let runtime = deps.runtime.lock().await;
                    crate::durability::quest_world_callback::QuestWorldContext {
                        map_revision: crate::map::overlay::map_revision(world.base()),
                        content_revision: format!(
                            "sha256:{}",
                            runtime
                                .content_pin()
                                .server_artifact_digest()
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>()
                        ),
                        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                        reset_epoch: owner.reset_epoch,
                        actor_placement: deps.actor.placement_identity(),
                        movement_revision: runtime
                            .read_actor_position(deps.actor)
                            .map_err(debug)?
                            .revision(),
                    }
                };
                owner
                    .states
                    .quest_callbacks
                    .interrupt_after_commit
                    .store(true, std::sync::atomic::Ordering::Release);
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await;
                assert!(
                    result.is_err(),
                    "an unknown result must close without an acknowledgement"
                );
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                let burned: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(burned, 4);
                // Another session must not settle or project this disconnected player's
                // pending callback when its own command stream opens.
                let mut other = admitted;
                other.game_session_id =
                    crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
                other.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
                other.item_fence = None;
                {
                    let mut runtime = deps.runtime.lock().await;
                    let reservation = runtime
                        .reserve_fresh_session(other.game_session_id)
                        .map_err(debug)?;
                    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                    runtime
                        .initialize_pinned_test_position(
                            actor,
                            local(Position {
                                x: trigger.x - 2,
                                ..trigger
                            }),
                        )
                        .map_err(debug)?;
                    other.runtime_actor = Some(actor);
                }
                assert!(
                    wire(&owner, &mut other, 1, 999, Vec::new())
                        .await?
                        .is_empty()
                );
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                // The next real dispatch reconciles the original USE before allowing input.
                let replay = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&replay)
                        .map_err(debug)?,
                    UseDisposition::Committed
                );
                assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(2).map_err(debug)?,
                    )),
                };
                let catalogue = owner.quest_catalogue.clone().ok_or("catalogue")?;
                let permit = owner.spell_lane_permit().await.ok_or("lane")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                assert!(matches!(
                    slot.commit_world_callback(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request.clone(),
                        catalogue.clone(),
                        &permit,
                        context.clone(),
                        None,
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::AlreadyCommitted(_)
                ));
                // A generic quest caller cannot bypass the physical altar proof.
                assert!(matches!(slot.commit_quest_transition(owner.root, owner.character, owner.holder,
                    fence, request.clone(), catalogue.clone()).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)));
                let mut stale = fence;
                stale.character_lease_generation += 1;
                let fresh_request = QuestTransitionRequest {
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(999).map_err(debug)?,
                    )),
                    ..request
                };
                assert!(matches!(slot.commit_world_callback(owner.root, owner.character, owner.holder,
                    stale, fresh_request, catalogue, &permit, context,
                    None,
                ).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
                drop(slot);
                drop(permit);
                let (proofs, receipts, remaining): (i64, i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_quest_callback_consumptions), \
                     (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
                     (SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$2)")
                    .bind(&callback.key).bind(&callback.offerings[0].item).fetch_one(&h.pool).await?;
                assert_eq!((proofs, receipts, remaining), (4, 1, 0));
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    local(callback.teleport.ok_or("teleport")?)
                );
            }
            if !lever_fixture {
                for passage in &callback.passages {
                    assert!(world.enterable(crate::map::overlay::TilePos {
                        x: passage.position.x,
                        y: passage.position.y,
                        floor: passage.position.floor
                    }));
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(1))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&result)
                        .map_err(debug)?,
                    if callback.reversible {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::NothingToUse
                    }
                );
                if callback.reversible {
                    for passage in &callback.passages {
                        assert!(!world.enterable(crate::map::overlay::TilePos {
                            x: passage.position.x,
                            y: passage.position.y,
                            floor: passage.position.floor,
                        }));
                    }
                }
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
