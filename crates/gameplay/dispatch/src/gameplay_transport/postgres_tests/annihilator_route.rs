//! Continuous source-region Annihilator route with genuine Fresh participants and D88 levels.
use super::fixture::{debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, FirstEntryOutcome, TestResult};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn annihilator_source_route_genuine_party_entry() -> TestResult {
    run(Route::Entry)
}

#[test]
fn annihilator_source_route_native_combat_clearance() -> TestResult {
    run(Route::Combat)
}

#[test]
fn annihilator_source_route_earned_reward_exit_and_reentry() -> TestResult {
    run(Route::Complete)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Entry,
    Combat,
    Complete,
    ObjectiveAtomicity,
    ObjectiveAtomicityStale,
    ElvenbaneExploration,
    ElvenbaneCombat,
    ElvenbaneDefenders,
    ElvenbaneComplete,
}

pub(super) fn native_objective_atomicity() -> TestResult {
    run(Route::ObjectiveAtomicity)
}

pub(super) fn native_objective_stale_capture() -> TestResult {
    run(Route::ObjectiveAtomicityStale)
}

pub(super) fn elvenbane_source_combat() -> TestResult {
    run(Route::ElvenbaneCombat)
}

pub(super) fn elvenbane_source_defenders() -> TestResult {
    run(Route::ElvenbaneDefenders)
}

pub(super) fn elvenbane_source_complete() -> TestResult {
    run(Route::ElvenbaneComplete)
}

pub(super) fn elvenbane_source_exploration() -> TestResult {
    run(Route::ElvenbaneExploration)
}

fn run(route: Route) -> TestResult {
    let combat = route != Route::Entry;
    let start = if matches!(
        route,
        Route::ObjectiveAtomicity | Route::ObjectiveAtomicityStale
    ) {
        Position {
            x: 32591,
            y: 31645,
            floor: -5,
        }
    } else if matches!(
        route,
        Route::ElvenbaneExploration
            | Route::ElvenbaneCombat
            | Route::ElvenbaneDefenders
            | Route::ElvenbaneComplete
    ) {
        Position {
            x: 32599,
            y: 31652,
            floor: -7,
        }
    } else {
        Position {
            x: if route == Route::Complete {
                33213
            } else {
                33215
            },
            y: 31671,
            floor: -13,
        }
    };
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        let started = std::time::Instant::now();
        let h =
            crate::bestiary_postgres_harness::Harness::create(admin, "annihilator_route", false)
                .await?;
        let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("ANNIHILATOR_ROUTE POSTGRES_CASE_OBSERVED server_version_num={version}");
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = source_world(
            generation.native_gameplay().ok_or("native gameplay")?,
            start,
            matches!(
                route,
                Route::ElvenbaneCombat | Route::ElvenbaneDefenders | Route::ElvenbaneComplete
            ),
        )?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut currents = Vec::new();
        let mut operations = Vec::new();
        for index in 0..4_u8 {
            let (current, operation) =
                crate::world_party_runtime_postgres_cases::committed_fresh_player_with_operation(
                    &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                        root: h.root.clone(),
                        node: h.node.clone(),
                        recovery: &h.recovery,
                    },
                    60 + index,
                    80 + index,
                    70 + index,
                    now,
                )
                .await?;
            currents.push(current);
            operations.push(operation);
        }
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let cells = world
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(deps.admitted.world_id),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: start.x.into(),
                    y: start.y.into(),
                    floor: i16::from(-start.floor),
                },
            )
            .ok_or("source bundle combat room")?;
        deps.combat_room = Some(room.clone());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let local = super::quest_world_callbacks::local;
        let content = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                i16::from(-start.floor),
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
            16,
            content,
        )
        .map_err(debug)?;
        generation
            .native_gameplay()
            .ok_or("native gameplay")?
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        if matches!(
            route,
            Route::ElvenbaneCombat | Route::ElvenbaneDefenders | Route::ElvenbaneComplete
        ) {
            // Same source projection/activation owner as serve.rs2445-2471,
            // before Fresh players can block source spawn placement.
            let sources = world
                .activation_spawns(|key| {
                    let policy = runtime.companion_policy(key).ok()?;
                    Some((
                        u64::try_from(policy.maximum_health).ok()?,
                        policy.blocked_by_nearby_players,
                    ))
                })
                .ok_or("source defender policies")?;
            assert_eq!(
                sources
                    .iter()
                    .map(|(points, _)| points.len())
                    .sum::<usize>(),
                6,
                "all six source declarations must survive bundle compilation"
            );
            assert_eq!(
                runtime
                    .begin_activation_spawn_sources(sources)
                    .map_err(debug)?,
                6
            );
            let progress = runtime.realize_activation_spawn_window(0).map_err(debug)?;
            assert_eq!(
                (progress.realized, progress.remaining),
                (6, 0),
                "normal native source activation"
            );
        }
        let mut admitted = Vec::new();
        for (index, current) in currents.iter().enumerate() {
            let session = current.current_game_session_id();
            let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
            runtime
                .bind_attacker_lease(actor, session, current.current_character_lease())
                .map_err(debug)?;
            let at = start;
            runtime
                .initialize_pinned_test_position(actor, local(at))
                .map_err(debug)?;
            admitted.push(AdmittedSession {
                game_session_id: session,
                world_id: deps.admitted.world_id,
                channel_id: deps.admitted.channel_id,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::Positioned,
                controller: Some(crate::gameplay_transport::connection::ControllerBinding {
                    transport: current.current_transport().ok_or("current transport")?,
                    account_id: id(80 + u8::try_from(index)?),
                }),
                continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
                item_fence: None,
            });
        }
        deps.actor = admitted[0].runtime_actor.ok_or("initiator")?;
        deps.admitted = admitted[0];
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner_from(&h.root, &h.node, &authority);
        owner.active_generation = Some(generation);
        owner.test_step_wait_receipts = Some(true);
        owner.movement_cells = (&cells).into();
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let journal =
            oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
        owner.quest_log_content = Some(
            crate::gameplay_transport::quest_log::QuestLogLines::new(journal.content)
                .ok_or("journal line bounds")?,
        );
        owner.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
        let offered =
            crate::gameplay_transport::connection::FreshAdmissionAuthority::offered_capabilities(
                &owner,
            );
        let mut pending =
            vec![oteryn_protocol_oteryn::world_object::CAPABILITY_WORLD_INTERACTION_V1];
        if matches!(route, Route::ElvenbaneComplete) {
            pending.push(oteryn_protocol_oteryn::quest_log::CAPABILITY_QUEST_LOG_V1);
        }
        if combat {
            pending.push(oteryn_protocol_oteryn::attack::CAPABILITY_ATTACK_V1);
        }
        if matches!(
            route,
            Route::ElvenbaneExploration
                | Route::ElvenbaneCombat
                | Route::ElvenbaneDefenders
                | Route::ElvenbaneComplete
        ) {
            pending.push(oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1);
            pending.push(oteryn_protocol_oteryn::world_spatial::CAPABILITY_PACED_MOVEMENT_V1);
        }
        let mut supported = std::collections::BTreeSet::new();
        while let Some(id) = pending.pop() {
            if supported.insert(id)
                && let Some(capability) = offered.iter().find(|capability| capability.id == id)
            {
                pending.extend_from_slice(capability.requires);
            }
        }
        let supported: Vec<_> = supported.into_iter().collect();
        let selected = crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            offered, &supported,
        )
        .ok_or("native world interaction capabilities must be offered")?;
        assert!(selected.contains(oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1));
        assert!(
            selected
                .contains(oteryn_protocol_oteryn::world_object::CAPABILITY_WORLD_INTERACTION_V1)
        );
        for (index, member) in admitted.iter_mut().enumerate() {
            member.continuity.selected_capabilities = selected;
            owner.admit_progression_session(member).await;
            let source_level = if combat { 2000 } else { 100 };
            level(&owner, *member, source_level, 180 + u8::try_from(index)?).await?;
            if combat {
                super::party_xp::initialize_build(
                    &owner,
                    member.game_session_id,
                    "knight",
                    220 + u8::try_from(index)?,
                )
                .await?;
            }
            let fence = owner
                .current_quest_fence(member.game_session_id)
                .await
                .map_err(|_| "current fence")?
                .ok_or("session")?;
            let loaded =
                crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
                    owner.root,
                    owner.character,
                    fence.character_id,
                )
                .await;
            let facts = match loaded {
                crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
                    facts,
                    ..
                }
                | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
                    facts,
                    ..
                } => facts,
                _ => return Err("durable cast facts unavailable".into()),
            };
            assert_eq!(facts.level, source_level);
            if combat {
                let native = generation.native_gameplay().ok_or("native")?;
                owner
                    .root
                    .initialize_admission_equipment(
                        owner.character,
                        owner.holder,
                        &fence.item_fence(),
                        native.source_digest(),
                        &operations[index],
                    )
                    .await
                    .map_err(debug)?;
            }
            let build = owner
                .root
                .read_character_build_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            if combat {
                super::annihilator_combat::prepare_supplies(
                    &owner,
                    member,
                    &build,
                    index == 0
                        && !matches!(
                            route,
                            Route::ObjectiveAtomicity | Route::ObjectiveAtomicityStale
                        ),
                )
                .await?;
            }
            let stamina = owner
                .root
                .read_hunting_stamina(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            let monk = owner
                .load_monk_state(fence.character_id)
                .await
                .ok_or("durable monk state")?;
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        member.runtime_actor.ok_or("actor")?,
                        member.game_session_id,
                        facts,
                        monk,
                        owner.owner_now()
                    )
                    .is_some()
            );
            let mut states = owner.states.spell_states.lock().await;
            let actor = member.runtime_actor.ok_or("actor")?;
            assert!(states.load_owned_build(&runtime, actor, member.game_session_id, &build));
            states.load_stamina_at(
                &runtime,
                actor,
                member.game_session_id,
                stamina,
                owner.owner_now().get(),
            );
            if combat {
                states
                    .get_mut(&runtime, actor, member.game_session_id)
                    .ok_or("state")?
                    .enable_owned_training(
                        &build,
                        generation
                            .native_gameplay()
                            .and_then(|n| n.training_formula())
                            .ok_or("training")?,
                    )
                    .map_err(debug)?;
            }
        }

        use crate::map::boot::BundleCollisionIndexExt;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return Err("native bundle collision index".into());
        };
        let map = index.map();
        if matches!(
            route,
            Route::ObjectiveAtomicity | Route::ObjectiveAtomicityStale
        ) {
            let result = super::native_objective_atomic::scenario(
                &mut owner,
                &mut admitted[0],
                map,
                &h.pool,
                route == Route::ObjectiveAtomicityStale,
            )
            .await;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return result;
        }
        if matches!(
            route,
            Route::ElvenbaneCombat | Route::ElvenbaneDefenders | Route::ElvenbaneComplete
        ) {
            let result = super::elvenbane_combat::scenario(
                &mut owner,
                &mut admitted[0],
                &h.pool,
                matches!(route, Route::ElvenbaneDefenders | Route::ElvenbaneComplete),
                route == Route::ElvenbaneComplete,
            )
            .await;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return result;
        }
        if route == Route::ElvenbaneExploration {
            let result =
                super::elvenbane_route::exploration(&mut owner, &mut admitted[0], &h.pool).await;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return result;
        }
        // Fresh admissions share the same source approach tile, as normal login
        // admissions share a configured start. Each walks through the real door
        // to a distinct staging tile; no runtime relocation supplies entry.
        for (index, member) in admitted.iter_mut().enumerate() {
            if route == Route::Complete {
                super::annihilator_finish::outer_gate(&mut owner, member, map).await?;
            }
            let target = target(map, 33216, 31671)?;
            assert_eq!(
                use_target(&mut owner, member, &target).await?,
                UseDisposition::Committed
            );
            for _ in 0..(9 - index) {
                step(
                    &mut owner,
                    member,
                    oteryn_protocol_oteryn::world_spatial::StepDirection::East,
                )
                .await?;
            }
        }
        let lever = target(map, 33226, 31671)?;
        assert_eq!(
            use_target(&mut owner, &mut admitted[0], &lever).await?,
            UseDisposition::Committed
        );
        let runtime = owner.runtime.lock().await;
        for (member, x) in admitted.iter().zip([33222, 33221, 33220, 33219]) {
            let at = runtime
                .read_actor_position(member.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position();
            assert_eq!((at.x, at.y, at.floor), (x, 31659, 13));
        }
        assert_eq!(runtime.visible_entities().creatures.len(), 6);
        drop(runtime);
        if combat {
            super::annihilator_combat::clear_corridor(&mut owner, &mut admitted[0]).await?;
        }
        if route == Route::Complete {
            super::annihilator_finish::complete(&mut owner, &mut admitted[0], map, &h.pool).await?;
        }
        eprintln!(
            "ANNIHILATOR_ROUTE POSTGRES_CASE_COMPLETED members=4 creatures=6 elapsed_ms={}",
            started.elapsed().as_millis()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

pub(super) fn target(
    map: &crate::map::boot::BundleMap,
    x: u16,
    y: u16,
) -> TestResult<WorldObjectTarget> {
    let key = crate::map::view::placement_key(crate::map::overlay::TilePos { x, y, floor: -13 }, 1)
        .ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

pub(super) async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    direction: oteryn_protocol_oteryn::world_spatial::StepDirection,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::*;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    let command = member.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        member,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved,
        "source step {direction:?} from {origin:?}"
    );
    Ok(())
}
pub(super) async fn use_target(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    member.item_fence = Some(
        owner
            .current_quest_fence(member.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("current session")?
            .item_fence(),
    );
    let command = member.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        member,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

async fn level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: AdmittedSession,
    level: u32,
    seed: u8,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let mut fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("session")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    fence.expected_character_revision = slot
        .cursor(owner.root, owner.character)
        .await
        .map_err(debug)?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await?
        .ok_or("progression")?;
    let binding = owner
        .progression_binding(member.game_session_id)
        .ok_or("progression policy")?;
    let total = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == level)
        .ok_or("source level threshold")?
        .minimum_experience
        .get();
    let result = owner
        .root
        .commit_character_experience(
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(seed)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(
                    total - before.total_experience.get(),
                ),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await;
    slot.observe_party_experience(&result);
    assert!(matches!(
        result.map_err(debug)?,
        ExperienceCommitOutcome::Committed(_)
    ));
    Ok(())
}

fn source_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    start: Position,
    elvenbane_combat: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::bundle;
    use oteryn_world_bundle_compiler::{compile, project, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut project = source::project_regions(&root, &[(start.x / 256, start.y / 256)])?;
    if elvenbane_combat {
        let goals = oteryn_gameplay_quests::quest::native_kill_goals::load().map_err(debug)?;
        for source in &mut project.families.spawns {
            source.points.retain(|point| {
                goals.iter().any(|goal| {
                    goal.quest == oteryn_gameplay_quests::quest::kill_objectives::QUEST
                        && goal.matches(
                            &point.creature,
                            i32::from(point.cell.0),
                            i32::from(point.cell.1),
                            i16::from(point.cell.2),
                        )
                })
            });
        }
        project
            .families
            .spawns
            .retain(|source| !source.points.is_empty());
        assert_eq!(
            project
                .families
                .spawns
                .iter()
                .map(|source| source.points.len())
                .sum::<usize>(),
            6,
            "compile the six actual pinned source declarations, retaining cells/policy/respawn facts"
        );
    }
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let pins = crate::map::BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    let (xs, ys) = if start.floor == -13 {
        (33212..=33236, 31650..=31673)
    } else {
        (32580..=32600, 31635..=31655)
    };
    assert!(
        !compiled
            .diagnostics
            .iter()
            .any(|d| d.floor == start.floor && xs.contains(&d.x) && ys.contains(&d.y)),
        "unresolved source quest geometry"
    );
    let pins = crate::map::BundlePins {
        digest: compiled.digest,
        ..pins
    };
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&compiled.bytes,&crate::map::boot::BootPins{map_revision:crate::map::overlay::map_revision(&base),bundle:pins,start:crate::map::overlay::TilePos{x:start.x,y:start.y,floor:start.floor}},crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key| {
        use crate::content::{ReferenceItemField,ReferenceItemType};
        // Match the production node's boot_bundle_items (serve.rs): an indexed
        // static Item without a native profile retains the owner's sealed defaults.
        let index = native.item_index()?;
        let revision = index.revision(key)?;
        let reference = index.definition_ref("Item", key, revision)?;
        let profile = native.item_policy(key, revision).map(|p| p.record());
        Some(crate::map::facts::ItemDefinition {
            reference,
            solid: profile.and_then(|p| p.attributes.blocks_movement),
            blocks_projectile: profile.and_then(|p| p.attributes.blocks_projectile).unwrap_or(true),
            pickupable: profile.is_some_and(|p| matches!(&p.semantics.physical, ReferenceItemField::Known(p) if p.pickupable == ReferenceItemField::Known(true))),
            door: profile.is_some_and(|p| matches!(&p.semantics.classification, ReferenceItemField::Known(c) if c.item_type == ReferenceItemField::Known(ReferenceItemType::Door))),
        })
    }).map_err(debug)
}
