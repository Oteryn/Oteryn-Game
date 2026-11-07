//! W16 real source STEP regression; geometry qualification does not itself admit an encounter.
use super::fixture::{debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
#[path = "rakesh_source_portal/framed.rs"]
mod framed;
#[path = "rakesh_source_portal/omission.rs"]
mod omission;
#[path = "rakesh_source_portal/pickup.rs"]
mod pickup;
#[path = "rakesh_source_portal/retirement.rs"]
mod retirement;
#[path = "rakesh_source_portal/sequential.rs"]
mod sequential;
#[path = "rakesh_source_portal/settlement.rs"]
mod settlement;
#[path = "rakesh_source_portal/stale_join.rs"]
mod stale_join;

#[test]
fn srv7_rakesh_real_source_portal_holds_without_instance_corpse_owner() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-rakesh-portal", scenario)
}
#[test]
fn srv7_rakesh_real_source_step_enters_current_allocated_instance() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-rakesh-live-entry", positive_scenario)
}
fn positive_scenario() -> TestResult {
    scenario_with_entry(true, false)
}
fn scenario() -> TestResult {
    scenario_with_entry(false, false)
}
#[test]
fn srv7_rakesh_real_framed_source_step_switches_instance() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-rakesh-framed-entry", || {
        scenario_with_entry(true, true)
    })
}
fn scenario_with_entry(expect_instance: bool, framed_entry: bool) -> TestResult {
    run(async |admin| {
        super::reset();
        let h =
            super::fixture::Harness::create_with_admission_seed(admin, "srv7_rakesh", true, false)
                .await?;
        prepare_current_session(&h).await?;
        assert_current_progression(&h, &id(41)).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let (controller, pin) = if framed_entry {
            super::raid_live::hare_production_activation::activate(
                world,
                "OTERYN_SRV7_RAKESH_CLIENT_ARTIFACT",
            )?
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        };
        let active = controller.active().ok_or("current native generation")?;
        let native = active.native_gameplay().ok_or("current native")?;
        let index = native.item_index().ok_or("Item index")?;
        let bytes = std::fs::read(
            std::env::var_os("OTERYN_SRV7_RAKESH_BUNDLE").ok_or("actual compiler Bundle path")?,
        )?;
        let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        let pins = crate::map::BundlePins {
            digest: bundle.digest,
            project_format_version: bundle.manifest.identity.project_format_version.clone(),
            world_schema_version: bundle.manifest.identity.world_schema_version.clone(),
            content_revision: bundle.manifest.identity.content_revision.clone(),
            production: false,
        };
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let start = crate::map::overlay::TilePos {
            x: 33038,
            y: 32964,
            floor: -8,
        };
        let map=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start},world,channel,|key|{
            let revision=index.revision(key)?;
            let p=native.item_policy(key,revision).map(|p|p.record());
            Some(crate::map::facts::ItemDefinition{
                reference:index.definition_ref("Item",key,revision)?,
                solid:p.and_then(|p|p.attributes.blocks_movement),
                blocks_projectile:p.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
                pickupable:p.is_some_and(|p|matches!(&p.semantics.physical,crate::content::ReferenceItemField::Known(p) if p.pickupable==crate::content::ReferenceItemField::Known(true))),door:false,
            })
        }).map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                map.movement_cells(pin.qualified_room().movement_cells())
                    .map_err(debug)?,
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 33038,
                    y: 32964,
                    floor: 8,
                },
            )
            .ok_or("real Rakesh source bundle room view")?;
        let source_template = if expect_instance {
            Some(
                controller
                    .qualify_rakesh_source_portal_template(
                        &room,
                        room.movement_cells(),
                        room.map_revision_digest(),
                        "content-1",
                    )
                    .map_err(debug)?,
            )
        } else {
            None
        };
        let child_room = if expect_instance {
            Some(
                controller
                    .qualify_rakesh_source_geometry(
                        &room,
                        room.movement_cells(),
                        room.map_revision_digest(),
                    )
                    .map_err(debug)?,
            )
        } else {
            None
        };
        let content_pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (33038, 32964, 8),
        );
        // The same production activation issuer used by the inventory/movement owners.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
            .bind(content_pin.server_artifact_digest().as_slice()).bind(content_pin.client_artifact_digest().as_slice())
            .bind(content_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::with_content(
            content_pin.clone(),
            room.clone(),
            native.spell_book().clone(),
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = room.movement_cells();
        if let Some(template) = source_template.as_ref() {
            owner
                .activity_runtimes
                .register_template(template)
                .map_err(|_| "register actual fully qualified Source gameplay template")?;
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        owner.admit_progression_session(&mut admitted).await;
        assert_eq!(
            crate::map::floor::table_of(owner.movement_cells).and_then(|table| table.teleport(
                crate::map::overlay::TilePos {
                    x: 33039,
                    y: 32964,
                    floor: -8
                }
            )),
            Some(crate::map::overlay::TilePos {
                x: 33045,
                y: 32988,
                floor: -8
            }),
            "actual source teleport pair/floor binding"
        );
        assert_eq!(
            owner.movement_cells.index().lookup(
                owner.movement_cells.scope(),
                crate::content::LogicalCell {
                    x: 33039,
                    y: 32964,
                    z: 8
                }
            ),
            Ok(crate::content::CollisionClass::Walkable),
            "actual source34111 origin physics must be qualified before warp regression"
        );
        assert!(
            crate::gameplay_transport::source_portal_hold::OriginPortalBinding::qualify(
                owner.movement_cells
            )
            .is_some(),
            "actual current source origin binding"
        );
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("real native portal actor fence")?;
        admitted.item_fence = Some(current.item_fence());
        let (crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        }
        | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
            facts,
            ..
        }) = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            &authority,
            current.character_id,
        )
        .await
        else {
            return Err("real native portal actor build".into());
        };
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(current.character_id.as_bytes())
                            .map_err(debug)?,
                        current.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        deps.actor,
                        admitted.game_session_id,
                        facts,
                        (0, 0),
                        owner.owner_now()
                    )
                    .is_some()
            );
        }

        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 33038,
                    y: 32964,
                    floor: 8,
                },
            )
            .map_err(debug)?;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        if let Some(instance_room) = child_room.as_ref() {
            use crate::foundation::{
                ChannelRuntimeV1, RuntimeScopeRefV1, ScopeOwnershipGeneration,
            };
            use crate::gameplay_transport::activity_runtime::{
                ActivityRuntimeAssets, ActivityTemplateKey,
            };
            use crate::gameplay_transport::capabilities::SelectedCapabilities;
            let wire = oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1;
            let framed_capabilities = [4, 6, 18, wire];
            let direct_capabilities = [4, 6, wire];
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                if framed_entry {
                    &framed_capabilities
                } else {
                    &direct_capabilities
                },
            )
            .ok_or("actual offered movement/Activity capabilities")?;
            owner
                .register_current_runtime()
                .await
                .map_err(|_| "register actual source owner")?;
            owner
                .activity_runtimes
                .remember(admitted)
                .map_err(|_| "retain actual source connection")?;
            // Existing control assignment/readiness writers supply the genuine
            // destination. Runtime code never creates its own grants or lease.
            let scope = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
            let assignment = h
                .root
                .read_runtime_scope_assignment(scope)
                .await
                .map_err(debug)?
                .ok_or("current actual allocated Instance")?;
            assert_eq!(assignment.holder, Some(h.node.fact()));
            let start = instance_room.entry_start();
            let instance_pin = ChannelContentPin::from_activation(
                world,
                active.activation_sequence(),
                instance_room.compiled().server_digest(),
                instance_room.compiled().client_digest(),
                instance_room.frame_binding().digest(),
                instance_room.map_revision_digest(),
                (start.x, start.y, start.floor),
            );
            let mut destination = ChannelRuntimeV1::from_committed_scope_assignment(
                scope,
                h.node.fact().node_id(),
                h.node.fact().registration_revision(),
                assignment.ownership_generation,
                assignment.source_revision,
                &assignment.decision_identity,
                32,
                instance_pin.clone(),
            )
            .map_err(debug)?;
            let policy = native
                .policies()
                .policies()
                .iter()
                .find(|policy| policy.definition_key == "oteryn:creature.rakesh_moonfang")
                .ok_or("actual source Rakesh profile")?
                .as_ref()
                .clone();
            assert_eq!(policy.maximum_health, 25_500, "no low-HP/creature alias");
            assert_eq!(
                policy.reward_boss,
                Some(false),
                "Crystal201 rakesh_moonfang.lua:49 keeps actual ordinary corpse loot"
            );
            destination
                .install_companion_policies(
                    crate::foundation::CompiledCreaturePolicies::from_active_artifact(
                        instance_pin.server_artifact_digest(),
                        vec![policy.clone()],
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            destination
                .realize_native_qualification_spawn(
                    instance_pin.clone(),
                    &policy.definition_key,
                    MovementLocalPosition {
                        x: 33042,
                        y: 32993,
                        floor: -8,
                    },
                )
                .map_err(debug)?;
            let reference = crate::world_runtime::ReferenceContentGeneration::from_content(
                instance_room.door(),
            )
            .map_err(debug)?;
            let generation =
                ScopeOwnershipGeneration::new(assignment.ownership_generation).map_err(debug)?;
            let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
                scope, generation, reference,
            );
            let door = crate::world_runtime::bind_native_entry_door(
                instance_room.door(),
                &door_fence,
                scope,
                generation,
            )
            .map_err(debug)?;
            let cells = instance_room.movement_cells();
            // Published source has no extra level/mission/solo restriction.
            // Level1 is the existing Character/storage floor;15 is the existing
            // Activity storage ceiling, with one actual portal landing per STEP.
            sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds)VALUES('oteryn:boss.rakesh_moonfang','content-1','oteryn:creature.rakesh_moonfang',1,15,1,7200,600,'[{\"x\":33039,\"y\":32964,\"floor\":8}]','oteryn:room.rakesh_moonfang_source_portal','oteryn:reward.rakesh_moonfang',true,'source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105','[{\"x\":33045,\"y\":32988,\"floor\":-8}]','{\"x\":33040,\"y\":32962,\"floor\":8}','{\"x\":33042,\"y\":32993,\"floor\":-8}',0)")
                .execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)VALUES(encode($1,'hex')::uuid,0,$2,'source-rakesh',$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11')ON CONFLICT DO NOTHING")
                .bind(world.as_bytes().as_slice()).bind(content_pin.server_artifact_digest().as_slice())
                .bind(content_pin.client_artifact_digest().as_slice()).bind(content_pin.frame_binding_digest().as_slice())
                .execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)ON CONFLICT DO NOTHING")
                .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
            owner.register_activity_runtime(
                ActivityTemplateKey{key:"oteryn:room.rakesh_moonfang_source_portal".into(),revision:"source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105".into()},
                &assignment,destination,door,ActivityRuntimeAssets{movement_cells:cells,rooms:crate::gameplay_transport::SeamRooms::new(Some(instance_room),Some(instance_room))},
            ).await.map_err(|_|"register real current source Instance owner")?;
            let handoffs =
                crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
            let definition = handoffs
                .read_boss_room(
                    &authority,
                    &h.node,
                    admitted.item_fence.ok_or("actual source fence")?,
                    "oteryn:boss.rakesh_moonfang",
                )
                .await?
                .ok_or("readable exactsource portal representation")?;
            assert_eq!(
                definition.policy.maximum_players, 15,
                "accepted technicalbound, not invented solo"
            );
            let allocations = handoffs
                .ready_allocations(&authority, &h.node, world, None)
                .await?;
            assert!(allocations.iter().any(|row| row.scope == scope
                && row.generation.get() == assignment.ownership_generation
                && row.source_revision == assignment.source_revision));
            let registered = owner
                .activity_runtimes
                .get(scope, assignment.ownership_generation)
                .ok_or("actual registered source room")?;
            let registered_runtime = registered.runtime.lock().await;
            assert!(
                crate::gameplay_transport::native_activity::boss_room_native_ready(
                    &registered_runtime,
                    &definition
                )
            );
            drop(registered_runtime);
            assert!(matches!(scope, RuntimeScopeRefV1::Instance { .. }));
            let members: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_party_members WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(
                admitted
                    .item_fence
                    .ok_or("current singleton source")?
                    .character_id
                    .as_bytes()
                    .as_slice(),
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                members, 0,
                "actual WorldParty absence, not a declared solo cap"
            );
        }

        let runtime = deps.runtime.lock().await;
        let origin = runtime.read_actor_position(deps.actor).map_err(debug)?;
        drop(runtime);
        let ordinary = std::collections::BTreeSet::new();
        let deferred = crate::gameplay_transport::source_portal_hold::DeferredPortal::for_step(
            owner.movement_cells,
            origin,
            crate::movement::CardinalStep::East,
            &ordinary,
        );
        let _ = crate::map::floor::CellBlocker::blocks(
            &deferred,
            crate::content::LogicalCell {
                x: 33045,
                y: 32988,
                z: 8,
            },
        );
        let before: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(admitted.item_fence.ok_or("native Character fence")?.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            before, 0,
            "fresh admitted player has no fabricated source quest progress"
        );
        let mut framed_admitted = admitted;
        let frames = if framed_entry {
            Some(framed::step(&owner, &mut framed_admitted).await?)
        } else {
            let (step, _) = owner
                .ordinary_paced_step(
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::gameplay_transport::world_spatial::StepDirection::East,
                    None,
                    None,
                )
                .await;
            assert_eq!(
                step.disposition,
                crate::gameplay_transport::world_spatial::StepDisposition::Moved,
                "the real paced step commits onto the source portal before typed admission"
            );
            None
        };
        if expect_instance {
            let expected =
                crate::foundation::RuntimeScopeRefV1::instance(world, id(88)).map_err(debug)?;
            let destination = if framed_entry {
                framed_admitted
            } else {
                let switch = owner
                    .activity_runtimes
                    .pending_switch(admitted)
                    .map_err(|_| "source scope switch")?;
                assert_eq!(
                    switch
                        .as_ref()
                        .and_then(|switch| switch.destination.item_fence)
                        .map(|fence| fence.runtime_scope),
                    Some(expected),
                    "actual source STEP must enter its genuine registered Instance; geometry/Node/catalogue/25500HP actor independently qualified above"
                );
                switch.ok_or("actual source scope handoff")?.destination
            };
            assert_eq!(
                destination
                    .item_fence
                    .ok_or("real source successor")?
                    .runtime_scope,
                expected
            );
            let registered = owner
                .activity_runtimes
                .get(expected, 1)
                .ok_or("actual destination runtime")?;
            let actual = registered
                .runtime
                .lock()
                .await
                .read_actor_position(
                    destination
                        .runtime_actor
                        .ok_or("actual new scoped player actor")?,
                )
                .map_err(debug)?
                .position();
            assert_eq!((actual.x, actual.y, actual.floor), (33045, 32988, -8));
            let rows:(i64,i64)=sqlx::query_as("SELECT(SELECT count(*)FROM game_activity_handoff_groups),(SELECT count(*)FROM game_boss_room_activities)")
                .fetch_one(&h.pool).await?;
            assert_eq!(rows, (1, 1), "one actual existingowner handoff/room");
            if let Some(frames) = frames {
                framed::export(&owner, destination, frames).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let peer = sequential::join(&mut owner, &h, admitted, destination).await?;
            settlement::death_and_replay(&owner, &h, destination, peer).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let runtime = deps.runtime.lock().await;
        let position = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            (position.x, position.y, position.floor),
            (33039, 32964, 8),
            "the accepted Instance corpse owner hold must not admit a generic warp into an empty arena"
        );
        drop(runtime);
        let after: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(admitted.item_fence.ok_or("native Character fence")?.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            after, 0,
            "refusal never grants the prerequisite or boss progress"
        );
        Ok(())
    })
}

/// A controlled historical Character must have the actual predecessor consumed
/// by the mature activity owner; cast-state initialization does not create it.
async fn assert_current_progression(
    h: &super::fixture::Harness,
    character: &[u8; 16],
) -> TestResult {
    let (level, progression_revision, root_revision): (i64, String, String) = sqlx::query_as(
        "SELECT p.level,p.character_revision::text,r.character_revision::text \
         FROM game_character_progression_state p JOIN game_character_roots r USING(character_id) \
         WHERE r.character_id=encode($1,'hex')::uuid AND p.profile_revision=r.profile_revision \
           AND p.ruleset_revision=r.ruleset_revision AND p.content_revision=r.content_revision",
    )
    .bind(character.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(
        level > 0,
        "actual source Character level, not a fabricated quest gate"
    );
    assert_eq!(
        progression_revision, root_revision,
        "current actual Root predecessor"
    );
    Ok(())
}

/// The existing peer factory creates the historical Character/bootstrap and
/// real fresh admission. Copy only the same trusted fixture's progression
/// predecessor into that distinct Character; never quest/access/reward facts.
async fn initialize_peer_progression(
    h: &super::fixture::Harness,
    peer: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let character = peer
        .item_fence
        .ok_or("actual peer Character fence")?
        .character_id;
    let changed = sqlx::query(
        "INSERT INTO game_character_progression_state \
         SELECT r.character_id,r.character_revision,p.level,p.total_experience,p.profile_revision, \
          p.ruleset_revision,p.content_revision,p.simulation_revision,p.evidence_revision, \
          p.declaration_revision,p.policy_revision,p.reward_revision \
         FROM game_character_roots r CROSS JOIN game_character_progression_state p \
         WHERE r.character_id=encode($1,'hex')::uuid AND p.character_id=encode($2,'hex')::uuid \
          AND r.profile_revision=p.profile_revision AND r.ruleset_revision=p.ruleset_revision \
          AND r.content_revision=p.content_revision",
    )
    .bind(character.as_bytes().as_slice())
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(
        changed.rows_affected(),
        1,
        "one actual historical peer predecessor"
    );
    assert_current_progression(h, character.as_bytes()).await
}
