use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::death_ml_loss_composed_fixtures::{current_fence, production_generation_with_source};

use super::fixture::{Harness, debug, id};

use super::{Dependencies, TestResult, prepare_current_session};

use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};

use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};

use crate::foundation::{ChannelContentPin, CharacterLease, MovementFacing, MovementLocalPosition};

use crate::gameplay_transport::SeamRooms;

use crate::gameplay_transport::actor_spell::native_combat_cast::NativeCastDispatch;

use crate::gameplay_transport::spell_access_facts::UnavailableAccessOwner;

use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};

use oteryn_protocol_oteryn::actor_spell_v2::{ParameterSpellCastIntent, PrivateSpellEffect};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

use serde_json::json;

use std::num::NonZeroU32;

pub(super) const ROPE_AFTER_STEP: u8 = 5;

fn at(x: i32, y: i32, floor: i16) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor }
}

pub(super) async fn scenario(admin: String, case: u8, use_bundle: bool) -> TestResult {
    run(admin, case, use_bundle, None).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RopeUse {
    Climb,
    Pull,
    PullFromFullLanding,
    PullCreature,
    PullCreatureFromSouth,
    PullPlayer,
    CoveredObject,
    CoveredHandle,
    GroundHandle,
    GroundHandleExplicit,
    Jam,
    DistantObject,
    DistantHandle,
}

impl RopeUse {
    fn climbs(self) -> bool {
        matches!(self, Self::Climb | Self::CoveredObject | Self::CoveredHandle) || self.ground_handle()
    }

    fn pulls(self) -> bool {
        matches!(self, Self::Pull | Self::PullFromFullLanding | Self::PullCreature | Self::PullCreatureFromSouth | Self::PullPlayer)
    }

    fn ground_handle(self) -> bool {
        matches!(self, Self::GroundHandle | Self::GroundHandleExplicit)
    }

    fn covered(self) -> bool {
        matches!(self, Self::CoveredObject | Self::CoveredHandle)
    }

    fn is_distant(self) -> bool {
        matches!(self, Self::DistantObject | Self::DistantHandle)
    }
}

pub(super) async fn physical_rope(admin: String, tool: &str, appearance: u32) -> TestResult {
    run(admin, 3, true, Some((tool, appearance, RopeUse::Climb))).await
}

pub(super) async fn agent_rope(
    admin: String,
    tool: &str,
    appearance: u32,
    action: RopeUse,
) -> TestResult {
    run(admin, 3, true, Some((tool, appearance, action))).await
}

async fn run(admin: String, case: u8, use_bundle: bool, tool: Option<(&str, u32, RopeUse)>) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin, &format!("vertical_move_{case}"), false, false)
            .await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_source(
        world,
        "content-1",
        Some(include_bytes!(
            "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    )?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let bundle = use_bundle.then(|| {
        use crate::map::spell_tiles::tests::vertical_movement;
        let channel = crate::foundation::ChannelId::decode(&id(43)).expect("fixture channel");
        match tool {
            Some((_, _, action)) if action.pulls() => vertical_movement::world_with_rope_hole(world, channel),
            Some((_, appearance, action)) => {
                let bundle = vertical_movement::world_with_rope_spot(
                    world, channel, appearance, action.is_distant(), action.covered(), action.ground_handle(),
                );
                if action.ground_handle() {
                    let gate = oteryn_world_bundle::bundle::placement_key(-8, 10, 10, 1)
                        .expect("rope doorway placement");
                    assert!(matches!(bundle.facts().doors().toggle(gate, 0, false, 20, true),
                        Some(crate::map::door::DoorUse::Toggled { open: true, .. })));
                }
                bundle
            }
            None => vertical_movement::world(world, channel),
        }
    });
    let bundle_room = bundle.as_ref().map(|bundle| {
        pin.qualified_room()
            .bundle_view(
                bundle
                    .movement_cells(pin.qualified_room().movement_cells())
                    .expect("bundle movement cells"),
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 10,
                    y: 10,
                    floor: 6,
                },
            )
            .expect("bundle room")
    });
    let room = bundle_room.as_ref().unwrap_or(pin.qualified_room());
    let start = if use_bundle {
        match case {
            _ if tool.is_some_and(|(_, _, action)| action == RopeUse::PullCreatureFromSouth) => at(11, 11, 7),
            _ if tool.is_some_and(|(_, _, action)| action.pulls()) => at(10, 10, 7),
            0 => at(10, 10, 6),
            1 => at(10, 9, 5),
            _ => at(10, 10, 8),
        }
    } else {
        match case {
            0 => at(1, 5, -6),
            1 => at(1, 4, -5),
            2 => at(-4, -3, -6),
            ROPE_AFTER_STEP => at(6, 14, -8),
            _ => at(7, 14, -8),
        }
    };
    let channel_pin = ChannelContentPin::from_activation(
        world,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content(channel_pin, room.clone(), native.spell_book().clone())?;
    let (actor, session) = (deps.actor, deps.admitted.game_session_id);
    let premium = if use_bundle {
        use crate::gameplay_transport::spell_premium_coordinator::{
            SpellPremiumCoordinator, purchase_fixture,
        };
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
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
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.premium_coordinator = premium.as_ref();
    owner.movement_cells = (room.movement_cells()).into();
    // No qualified room: the same accessor path a bundle World uses.
    owner.rooms = SeamRooms::new(None, Some(room));
    let mut admitted = deps.admitted;
    owner.admit_progression_session(&mut admitted).await;
    if use_bundle {
        owner.admit_premium(id(40), session);
    }
    if use_bundle || case == ROPE_AFTER_STEP {
        owner
            .states
            .chat
            .admit(session, "Movement", owner.owner_now());
    }
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_turn(
                before,
                if case == 1 {
                    MovementFacing::South
                } else {
                    MovementFacing::North
                },
            )
            .map_err(debug)?;
    }
    let formula = native.training_formula().ok_or("training")?;
    let build = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: build.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    h.root
        .commit_character_experience(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(98_800),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        states
            .initialize(
                &runtime,
                actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    level: 20,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("caster")?;
        assert!(states.load_owned_build(&runtime, actor, session, &build));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    if let (Some((tool, _, action)), Some(bundle)) = (tool, &bundle) {
        use crate::gameplay_transport::connection::UseCommand;
        use crate::gameplay_transport::item_view::{
            ItemKey, SessionItemView, inventory_from_backpack,
        };
        use oteryn_protocol_oteryn::item_view::decode_character_inventory;
        use oteryn_protocol_oteryn::world_object::{
            MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        };

        let pull = action.pulls();
        let distant = action.is_distant();
        let repetitions = if action == RopeUse::Jam { 10 } else { 1 };
        let mut entries = vec![(tool, 1); repetitions];
        if action.ground_handle() || pull {
            entries.push(("oteryn:item.tibia.i3031", if pull { 7 } else { 1 }));
        }
        super::item_look_fixtures::seed_main_backpack(&h, &entries).await?;
        if action.ground_handle() || pull {
            use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
            let fence = current_fence(&owner, session).await?.item_fence();
            let backpack = h.root.read_character_backpack(&authority, fence.character_id)
                .await?.ok_or("rope backpack")?;
            let coin = backpack.entries.iter().find(|entry| entry.item.definition.production_key.ends_with(".i3031"))
                .ok_or("ground target coin")?.item.item_instance_id;
            if pull {
                let mut runtime = deps.runtime.lock().await;
                position_fixture_actor(&mut runtime, room.movement_cells(), actor, session, at(11, 10, 8))?;
            }
            assert_eq!(owner.move_loop_item(
                actor,
                UseCommand { game_session_id: session, command_id: 10_001, item_fence: Some(fence) },
                LoopMove { item: ItemKey::Instance(coin), count: None,
                    target: LoopTarget::Ground(oteryn_protocol_oteryn::item_view::WorldTilePosition {
                        x: if pull { 11 } else { 10 }, y: 10, floor: -8,
                    }), slot: None },
            ).await, LoopMoveResult::Committed);
            if pull {
                let mut runtime = deps.runtime.lock().await;
                position_fixture_actor(&mut runtime, room.movement_cells(), actor, session, start)?;
            }
        }
        let filled = if action == RopeUse::PullFromFullLanding {
            use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

            let fence = current_fence(&owner, session).await?.item_fence();
            let offering = Offering {
                position: Position { x: 11, y: 11, floor: -7 },
                item: "oteryn:item.tibia.i3264".into(),
                count: 1,
            };
            let limit = u64::try_from(crate::durability::ground_cell_limit::GROUND_CELL_ROOT_LIMIT)?;
            for n in 0..limit {
                super::quest_world_callbacks_fixtures::offering(&owner, fence, &offering, 2_000 + n).await?;
            }
            i64::try_from(limit)?
        } else {
            0
        };
        let creature = if matches!(action, RopeUse::PullCreature | RopeUse::PullCreatureFromSouth) {
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            let mut runtime = deps.runtime.lock().await;
            native.install_companion_policies(&mut runtime).map_err(debug)?;
            Some(runtime
                .commit_quest_creature_spawn(&[("canary:creature/rat", 20, at(11, 10, 8))])
                .map_err(debug)?
                .into_iter()
                .next()
                .ok_or("rat beneath hole")?)
        } else {
            None
        };
        let peer = if action == RopeUse::PullPlayer {
            let session = crate::foundation::GameSessionId::decode(&id(73)).map_err(debug)?;
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime.initialize_pinned_test_position(peer, at(11, 10, 8)).map_err(debug)?;
            owner.states.spell_states.lock().await.initialize(
                &runtime,
                peer,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                owner.owner_now(),
            ).ok_or("pulled player")?;
            Some((peer, session))
        } else {
            None
        };
        let index = native.item_index().ok_or("item index")?;
        let definition = index
            .definition_ref("Item", tool, "definition-r1")
            .ok_or("tool reference")?;
        let digest = bundle.base().digest();
        let target_x = if distant { 12 } else if pull { 11 } else { 10 };
        let spot = oteryn_world_bundle::bundle::placement_key(if pull { -7 } else { -8 }, target_x, 10, u8::from(action.covered()))
            .ok_or("rope spot placement")?;
        let mut view = SessionItemView::resume(Default::default()).with_map_view();
        let map_target = if matches!(action, RopeUse::DistantHandle | RopeUse::CoveredHandle) {
            use crate::gameplay_transport::world_map::{MapUpdate, SessionMapView};
            use oteryn_protocol_oteryn::world_map::{MapOrigin, decode_world_map_snapshot};

            let mut map_view = SessionMapView::default();
            let update = crate::gameplay_transport::bundle_map_position::observe(
                &owner, actor, &mut map_view, &mut view,
            )
            .await
            .ok_or("bundle map view")?
            .map_err(debug)?;
            let Some(MapUpdate::Snapshot(snapshot)) = update else {
                return Err("initial bundle map snapshot".into());
            };
            let map = decode_world_map_snapshot(&snapshot.payload).map_err(debug)?;
            let handle = map.tiles.iter()
                .find(|tile| tile.position.x == i32::from(target_x) && tile.position.y == 10 && tile.position.floor == 8)
                .and_then(|tile| tile.items.iter().filter(|item| item.appearance_id == 3031).find_map(|item| match item.origin {
                    MapOrigin::Handle(handle) => Some(handle),
                    _ => None,
                }))
                .ok_or("visible covering item handle")?;
            MapUseTarget::Item(handle)
        } else {
            let mut placement = digest.to_vec();
            placement.extend_from_slice(&spot.to_be_bytes());
            MapUseTarget::Object(WorldObjectTarget { placement, expected_revision: 0 })
        };
        if pull {
            let map = owner.bundle_map().ok_or("active rope hole map")?;
            let facts = map.facts.pinned();
            let ground = crate::gameplay_transport::loop_move::loop_ground(&owner, map.overlay.base(), Some(index))
                .await.ok_or("rope hole Ground view")?;
            let source = crate::gameplay_transport::world_map::MapViewSource {
                overlay: ground.overlay(),
                facts: &facts,
                content_generation: pin.identity().client_artifact_digest(),
                reset_epoch: owner.reset_epoch,
            };
            let MapUseTarget::Object(object) = &map_target else {
                return Err("rope hole fixture must select an exact object".into());
            };
            let target = crate::gameplay_transport::world_map::resolve_map_target(&source, object, |_| None::<()>)
                .map_err(debug)?;
            assert_eq!(target.placement_key, spot);
            assert_eq!(ground.ground_entry(&id(72)).map(|(pos, _)| (pos.x, pos.y, pos.floor)), Some((11, 10, -8)));
        }
        let explicit_inventory = if action == RopeUse::Jam || (pull && tool == "oteryn:item.tibia.i3003") {
            let character = current_fence(&owner, session).await?.character_id;
            let backpack = h.root.read_character_backpack(&authority, character).await?;
            let items = inventory_from_backpack(backpack.as_ref(), |definition| {
                index.definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
            })
            .ok_or("qualified backpack items")?;
            let [snapshot, _] = view.snapshot(items, None).map_err(debug)?;
            Some(decode_character_inventory(&snapshot.payload).map_err(debug)?)
        } else {
            None
        };
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session).await?.0;
        let mut previous_command = 0;
        for offset in 0..repetitions {
            let item = id(71 + u8::try_from(offset)?);
            let source = if let Some(inventory) = &explicit_inventory {
                let handle = inventory.entries.iter()
                    .find(|entry| view.resolve(entry.handle) == Some(ItemKey::Instance(item)))
                    .ok_or("Secret Agent inventory handle")?
                    .handle;
                UseTarget::Item(handle)
            } else {
                UseTarget::ItemByDefinition(definition)
            };
            let command_id = (previous_command + 1..=previous_command + 10_000)
                .find(|&command_id| {
                    oteryn_gameplay_items::secret_agent::jams(
                        *session.as_bytes(), command_id, item,
                    ) == !(action.climbs() || pull)
                })
                .ok_or("Secret Agent test roll")?;
            previous_command = command_id;
            let intent = UseIntent {
                target: source,
                use_with: None,
                use_with_position: None,
                use_with_map_item: Some(map_target.clone()),
            };
            if action.ground_handle() {
                assert_eq!(command_id, admitted.continuity.next_command_id);
                use crate::gameplay_transport::capabilities::SelectedCapabilities;
                use crate::gameplay_transport::connection::FirstEntryOutcome;
                admitted.runtime_actor = Some(actor);
                admitted.first_entry = FirstEntryOutcome::Positioned;
                admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                        .ok_or("rope capability offer")?;
                let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
                    .map_err(debug)?;
                let (_, disposition, _) = super::item_craft_fixtures::exchange_with_map_item(
                    &owner,
                    admitted,
                    command_id,
                    &payload,
                    (action == RopeUse::GroundHandleExplicit).then_some(definition),
                    0,
                    Some((10, 10, 8, 3031)),
                ).await?;
                assert_eq!(disposition, UseDisposition::Committed, "{tool}");
                continue;
            }
            let outcome = crate::gameplay_transport::item_craft::dispatch(
                &owner,
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: crate::gameplay_transport::item_fence_of(current),
                },
                intent,
                Some(&view),
            )
            .await
            .ok_or("rope dispatch")?;
            assert_eq!(
                outcome.disposition,
                if distant { UseDisposition::TooFar } else { UseDisposition::Committed },
                "{tool}"
            );
        }
        let expected = if action.climbs() { at(10, 11, 7) } else { start };
        assert_eq!(deps.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
                   expected, "{tool}");
        if action.ground_handle() {
            let gate = oteryn_world_bundle::bundle::placement_key(-8, 10, 10, 1)
                .ok_or("rope doorway placement")?;
            assert_eq!(bundle.facts().doors().revision(gate), 2, "climbing closes the vacated gate");
            let moved: Vec<Vec<u8>> = sqlx::query_scalar(
                "SELECT spatial_position FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
            ).bind(id(72).as_slice()).fetch_all(&h.pool).await?;
            assert_eq!(moved.len(), 1);
            let cell = crate::durability::spell_items_abi::decode_ground_cell(&moved[0]).map_err(debug)?;
            assert_eq!((cell.x, cell.y, cell.z), (11, 10, -8));
            let moves: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
                .fetch_one(&h.pool).await?;
            assert_eq!(moves, 2, "one initial drop and one doorway relocation");
        }
        if let Some(creature) = creature {
            let runtime = deps.runtime.lock().await;
            assert_eq!(
                runtime.read_actor_position(creature).map_err(debug)?.position(),
                if action == RopeUse::PullCreatureFromSouth { at(10, 10, 7) } else { at(11, 11, 7) },
            );
        }
        if let Some((peer, session)) = peer {
            use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
            let runtime = deps.runtime.lock().await;
            assert_eq!(runtime.read_actor_position(peer).map_err(debug)?.position(), at(11, 11, 7));
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, peer, session).ok_or("moved player state")?;
            let until = state.cooldowns.groups.get(STAIRHOP_COOLDOWN_KEY).ok_or("missing stair-hop cooldown")?;
            let now = owner.owner_now();
            assert!(now < *until && *until <= now.checked_add(STAIRHOP_MICROS).map_err(debug)?);
        }
        if pull {
            let (position, quantity): (Vec<u8>, i64) = sqlx::query_as(
                "SELECT g.spatial_position, i.quantity FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid",
            ).bind(id(72).as_slice()).fetch_one(&h.pool).await?;
            let cell = crate::durability::spell_items_abi::decode_ground_cell(&position).map_err(debug)?;
            let actor_pulled = creature.is_some() || peer.is_some();
            let expected = if actor_pulled {
                (11, 10, -8)
            } else if action == RopeUse::PullFromFullLanding {
                (10, 10, -7)
            } else {
                (11, 11, -7)
            };
            assert_eq!((cell.x, cell.y, cell.z), expected, "{tool}");
            assert_eq!(quantity, 7, "{tool}");
            let moves: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
                .fetch_one(&h.pool).await?;
            assert_eq!(moves, filled + if actor_pulled { 1 } else { 2 });
        }
        let schedules: Vec<(String, i64)> = sqlx::query_as(
            "SELECT definition_production_key, duration_millis FROM game_spell_item_temporal_schedules WHERE target_production_key=$1",
        )
        .bind(tool).fetch_all(&h.pool).await?;
        if action == RopeUse::Jam {
            use oteryn_protocol_oteryn::chat::ChatLine;
            let id: u32 = tool.rsplit(".i").next().ok_or("tool id")?.parse()?;
            assert_eq!(schedules, vec![(format!("oteryn:item.tibia.i{}", id + 1), 60_000); repetitions]);
            let progress: Option<i64> = sqlx::query_scalar(
                "SELECT progress FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/bad_timing'",
            ).fetch_optional(&h.pool).await?;
            let granted: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key='oteryn:achievement/bad_timing')",
            ).fetch_one(&h.pool).await?;
            let feedback = owner.states.chat.drain(session);
            assert_eq!(
                (progress, granted, feedback.iter().filter(|line|
                    matches!(line, ChatLine::Private { text, .. } if text == "Oh no! Your tool is jammed and can't be used for a minute.")
                ).count()),
                (Some(i64::try_from(repetitions)?), true, repetitions),
            );
        } else {
            assert!(schedules.is_empty());
            if distant {
                assert!(owner.states.chat.drain(session).is_empty());
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
    if case >= 3 && !use_bundle {
        seed_rope_blockers(&h, &deps, room, native, case == 4).await?;
    }
    let name = if case >= 3 { "Magic Rope" } else { "Levitate" };
    let spell = (1..=u32::try_from(deps.spells.source_len())?)
        .filter_map(NonZeroU32::new)
        .find(|index| {
            deps.spells
                .source_indexed(*index)
                .is_some_and(|(spell, enabled)| enabled && spell.name == name)
        })
        .ok_or("source spell")?;
    let intent = ParameterSpellCastIntent {
        intent: SpellCastIntent {
            spell,
            target: SpellTarget::None,
            aim_at_target: false,
        },
        parameter: (case < 3).then(|| if case == 1 { "down" } else { "up" }.into()),
    };
    let clock = (case == ROPE_AFTER_STEP)
        .then(|| super::srx3_native_source_cast_fixtures::NativeOwnerClock::new(&owner));
    let command_id = if let Some(clock) = &clock {
        clock.set(owner.owner_now().get());
        let (step, wait) = owner.paced_step(actor, session, StepDirection::East).await;
        assert_eq!(step.disposition, StepDisposition::Moved);
        assert!(wait.is_some_and(|wait| !wait.is_zero()));
        assert_eq!(
            deps.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
            at(7, 14, -8)
        );
        2
    } else {
        1
    };
    let before = owner.observe_vitals(actor, session).await.ok_or("vitals")?;
    if use_bundle || case == ROPE_AFTER_STEP {
        let text = match case {
            0 => "exani hur \"up",
            1 => "exani hur \"down",
            _ => "exani tera",
        };
        let (chat, cast) = owner
            .complete_chat_intent(
                actor,
                session,
                command_id,
                &oteryn_protocol_oteryn::chat::ChatIntent::Say {
                    mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                    text: text.into(),
                },
            )
            .await
            .ok_or("spoken cast")?;
        assert_eq!(
            chat.disposition,
            oteryn_protocol_oteryn::chat::ChatDisposition::Ok,
            "case {case}: {cast:?}"
        );
        assert_eq!(
            cast.ok_or("chat spell outcome")?.disposition,
            SpellCastDisposition::Cast
        );
    }
    let dispatch = owner
        .cast_parameters(actor, session, command_id, &intent, &UnavailableAccessOwner)
        .await;
    assert!(
        matches!(dispatch.cast, NativeCastDispatch::Outcome(_)),
        "case {case}"
    );
    let result = dispatch.result.ok_or("parameter result")?;
    let disposition = if case == 2 {
        SpellCastDisposition::Rejected
    } else {
        SpellCastDisposition::Cast
    };
    assert_eq!(result.disposition, disposition, "case {case}");
    let feedback = result.feedback.as_ref().ok_or("source feedback")?;
    assert_eq!(
        feedback.text,
        if case == 2 {
            "Sorry, not possible."
        } else {
            ""
        }
    );
    assert_eq!(
        feedback.effect,
        if case == 2 {
            PrivateSpellEffect::Poff
        } else {
            PrivateSpellEffect::Teleport
        }
    );
    let destination = if use_bundle {
        match case {
            0 => at(10, 9, 5),
            1 => at(10, 10, 6),
            _ => at(10, 11, 7),
        }
    } else {
        match case {
            0 => at(1, 4, -5),
            1 => at(1, 5, -6),
            3 | ROPE_AFTER_STEP => at(7, 13, -7),
            4 => at(7, 15, -7),
            _ => start,
        }
    };
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        destination
    );
    let after = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("paid vitals")?;
    assert_eq!(
        after.1.mana,
        before.1.mana
            - match case {
                2 => 0,
                3 | 4 | ROPE_AFTER_STEP => 20,
                _ => 50,
            }
    );
    // All candidates reject the projectile barrier; the source south fallback ignores it.
    let replay = owner
        .cast_parameters(actor, session, command_id, &intent, &UnavailableAccessOwner)
        .await;
    assert_eq!(replay.result, Some(result));
    assert_eq!(owner.observe_vitals(actor, session).await, Some(after));
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        destination
    );
    drop(clock);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn seed_rope_blockers(
    h: &Harness,
    deps: &Dependencies,
    room: &crate::content::QualifiedNativeEntryRoom,
    native: &crate::content::native_gameplay::NativeGameplayState,
    all: bool,
) -> TestResult {
    let policy = native
        .item_policy_for_source_id(10181)
        .ok_or("projectile barrier")?;
    assert_eq!(
        oteryn_gameplay_items::native_world_items::source_barrier_collision(policy),
        Some((false, true))
    );
    let definition = &policy.record().production_definition;
    let targets = {
        let runtime = deps.runtime.lock().await;
        let points = [
            (7, 15),
            (7, 13),
            (8, 14),
            (6, 14),
            (6, 15),
            (8, 15),
            (6, 13),
            (8, 13),
        ];
        points
            .iter()
            .take(if all { 8 } else { 1 })
            .map(|&(x, y)| {
                crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
                    room,
                    &runtime,
                    at(x, y, -7),
                )
                .map_err(Into::into)
            })
            .collect::<TestResult<Vec<_>>>()?
    };
    let cost = serde_json::to_vec(&json!({
        "version":1,"before_revision":0,"after_revision":1,
        "caster_before":vec![1;32],"caster_after":vec![2;32],
        "mana_before":100,"mana_after":50,"soul_before":5,"soul_after":4,
        "cooldowns_before":[],"cooldowns_after":[["spell:fixture",1000000]]
    }))?;
    let mut tx = h.pool.begin().await?;
    // Fixture custody only. Casts read these actual durable rows under the production fences.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,999,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,1,'Spell','fixture:vertical_blocker','r1',$7,sha256($8),$8,$9,sha256($9),$10,1)")
        .bind(id(120).as_slice()).bind(id(121).as_slice()).bind(id(50).as_slice())
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(native.source_digest().as_slice()).bind(b"vertical-blocker".as_slice()).bind(&cost)
        .bind(i32::try_from(targets.len())?).execute(&mut *tx).await?;
    for (ordinal, target) in targets.iter().enumerate() {
        let item = id(130 + u8::try_from(ordinal)?);
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(item.as_slice()).bind(id(42).as_slice()).bind(&definition.production_key)
            .bind(&definition.revision_ref).bind(id(120).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid) VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,'Item',$4,$5,0,1,0,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10,$11,$12,false,true,false)")
            .bind(id(120).as_slice()).bind(i32::try_from(ordinal + 1)?).bind(item.as_slice())
            .bind(&definition.production_key).bind(&definition.revision_ref)
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position)
            .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context)
            .bind(native.source_digest().as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,$9)")
            .bind(item.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(&target.spatial_position).bind(b"vertical-blocker".as_slice())
            .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context)
            .bind(i64::try_from(ordinal + 1)?).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,7776000001,$3,sha256($3))")
        .bind(id(121).as_slice()).bind(id(120).as_slice()).bind(b"vertical-blocker".as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn position_fixture_actor(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    destination: MovementLocalPosition,
) -> TestResult {
    let open = std::collections::BTreeSet::new();
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        runtime, cells, actor, session, before, destination, false, &open,
    ).map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
