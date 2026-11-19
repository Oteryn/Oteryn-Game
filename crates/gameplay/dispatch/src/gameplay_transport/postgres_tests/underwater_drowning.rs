use super::fixture::{Harness, debug, id, run};
use super::underwater_drowning_fixtures::{
    on_thread, scenario, scenario_in_world, step, underwater_row,
};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::writer::{Family, PaletteEntry, write};

#[test]
fn fixm10_swimming_and_map_traps_postgres() -> TestResult {
    on_thread(|| {
        scenario(None, false)?;
        scenario_in_world(None, false, false, None)
    })
}

#[test]
fn fixm10_swimming_quest_relocation_postgres() -> TestResult {
    on_thread(|| {
        scenario(Some((0, 5, false)), false)?;
        scenario(Some((7, 4, false)), false)?;
        scenario(Some((6, 4, true)), false)
    })
}

#[test]
fn underwater_drowning_postgres_step_in_drowns_helmets_protect_step_out_removes() -> TestResult {
    on_thread(|| scenario(None, false))
}

#[test]
fn underwater_drowning_postgres_quest_relocation_enters_and_leaves_water() -> TestResult {
    on_thread(|| {
        scenario(Some((0, 2, false)), false)?;
        scenario(Some((3, 4, false)), false)
    })
}

#[test]
fn underwater_drowning_postgres_refused_quest_step_restores_water_state() -> TestResult {
    on_thread(|| {
        scenario(Some((2, 1, true)), false)?;
        scenario(Some((1, 2, true)), false)
    })
}

#[test]
fn underwater_drowning_postgres_portal_landings_enter_and_leave_water() -> TestResult {
    on_thread(|| scenario(None, true))
}

#[test]
fn m10_move_yellow_pillow_postgres_real_dispatch() -> TestResult {
    on_thread(|| {
        let started = std::time::Instant::now();
        let result = run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "m10_move_pillow", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) = activated_with_loot_tables(world, None);
            let active = controller.active().ok_or("active")?;
            // Activated content digests (custody reads pin them), the test row's map binding.
            let channel = Dependencies::new()?.admitted.channel_id;
            let (bytes, mut pins) = underwater_row(false, false)?;
            let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
            bundle.manifest.palette[4].key = "oteryn:item.tibia.i2397".into();
            let coin_palette = u32::try_from(bundle.manifest.palette.len())?;
            bundle.manifest.palette.push(PaletteEntry {
                key: "oteryn:item.tibia.i3031".into(),
                family: Family::Item,
                id: 10,
                terrain: None,
            });
            // A pickupable stack makes spell metadata unknown without blocking
            // the ordinary routed STEP onto the pillow.
            bundle.sectors[0]
                .tiles
                .iter_mut()
                .find(|tile| tile.x == 2)
                .ok_or("tile")?
                .items = vec![
                oteryn_world_bundle::sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: Default::default(),
                },
                oteryn_world_bundle::sector::Item {
                    palette: 4,
                    depth: 0,
                    attrs: Default::default(),
                },
                oteryn_world_bundle::sector::Item {
                    palette: coin_palette,
                    depth: 0,
                    attrs: Default::default(),
                },
            ];
            let bytes =
                write(&bundle.manifest, &bundle.sectors, &Default::default()).map_err(debug)?;
            pins.digest = oteryn_world_bundle::bundle::read(&bytes)
                .map_err(debug)?
                .digest;
            let base = crate::map::load(&bytes, &pins).map_err(debug)?;
            let map = boot(
                &bytes,
                &BootPins {
                    map_revision: crate::map::overlay::map_revision(&base),
                    bundle: pins,
                    start: TilePos {
                        x: 1,
                        y: 0,
                        floor: -7,
                    },
                },
                world,
                channel,
                |key| {
                    let ids = [
                        2397, 3482, 3944, 2147, 44529, 2146, 2148, 3481, 3945, 44530, 3031,
                    ];
                    let index = ids
                        .iter()
                        .position(|id| key == format!("oteryn:item.tibia.i{id}"))?;
                    Some(crate::map::facts::ItemDefinition {
                        reference: std::num::NonZeroU32::new(index as u32 + 1)?,
                        solid: Some(false),
                        blocks_projectile: false,
                        pickupable: key == "oteryn:item.tibia.i3031",
                        door: false,
                    })
                },
            )
            .map_err(debug)?;
            let channel_pin = ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                (1, 0, 7),
            );
            let mut deps = Dependencies::with_content(
                channel_pin,
                crate::content::qualify_native_entry_room(world).map_err(debug)?,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            assert_eq!(deps.admitted.channel_id, channel);
            // The activated room scopes the cells to the activated content digest.
            let cells = map
                .movement_cells(pin.qualified_room().movement_cells())
                .map_err(debug)?;
            assert_eq!(
                cells.spell_tiles().lookup(
                    cells.scope(),
                    crate::content::LogicalCell { x: 2, y: 0, z: 7 },
                ),
                Err(crate::content::SpellTileLookupError::Unknown),
                "the pickupable destination has no fabricated spell metadata"
            );
            deps.combat_room = Some(
                pin.qualified_room()
                    .bundle_view(
                        cells.clone(),
                        map.frame_binding_digest(world),
                        map.map_revision_digest(),
                        crate::content::NativeEntryStart {
                            x: 1,
                            y: 0,
                            floor: 7,
                        },
                    )
                    .ok_or("qualified underwater bundle room")?,
            );
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
            deps.initialize(&owner).await;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'fixm10-fixture')")
            .bind(id(42).as_slice()).bind(1_i16).execute(&h.pool).await?;
            let mover = (deps.admitted, deps.actor, deps.admitted.game_session_id);
            step(&mut owner, mover, StepDirection::East, 2).await?;
            let bundle = owner.bundle_map().ok_or("bundle")?;
            let speech = bundle.facts.movement_speech_since(0);
            assert_eq!(speech.len(), 1);
            assert_eq!(
                (
                    speech[0].1.speaker,
                    speech[0].1.text.clone(),
                    speech[0].1.talk_type
                ),
                (deps.actor, "Faaart!".to_owned(), 36)
            );
            assert_eq!(
                bundle
                    .facts
                    .movement_effects_since(0)
                    .iter()
                    .map(|(_, _, effect)| *effect)
                    .collect::<Vec<_>>(),
                vec![3],
                "committed routed player STEP emits pillow poff"
            );
            {
                use crate::durability::native_map_items_abi::{
                    NativeMapInitializationProof, NativeMapItemPlacement, NativeMapOwnerBinding,
                    map_initializer_seal,
                };
                // Persistence fixture only: the real initializer owns this field
                // and its receipt. This grants no tile or movement qualification.
                struct FieldFixture {
                    binding: NativeMapOwnerBinding,
                    placements: Vec<NativeMapItemPlacement>,
                }
                impl map_initializer_seal::Sealed for FieldFixture {}
                impl NativeMapInitializationProof for FieldFixture {
                    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str> {
                        Ok(self.binding.clone())
                    }
                    fn placements(&self) -> &[NativeMapItemPlacement] {
                        &self.placements
                    }
                }
                step(&mut owner, mover, StepDirection::West, 1).await?;
                let native = active.native_gameplay().ok_or("native field policy")?;
                let policy = native
                    .item_policy("oteryn:item.tibia.i2126", "definition-r1")
                    .ok_or("energy field policy")?;
                assert!(matches!(
                    &policy.record().semantics.classification,
                    crate::content::ReferenceItemField::Known(classification)
                        if matches!(classification.item_type,
                            crate::content::ReferenceItemField::Known(crate::content::ReferenceItemType::MagicField))
                ));
                let (binding, target, before) = {
                    let runtime = deps.runtime.lock().await;
                    let room = deps.combat_room.as_ref().ok_or("bundle room")?;
                    let binding = NativeMapOwnerBinding {
                        world,
                        channel,
                        scope_generation: runtime.binding().scope_generation().get(),
                        content_digest: runtime.content_pin().server_artifact_digest(),
                        map_digest: room.map_revision_digest(),
                        frame_digest: room.frame_binding().digest(),
                    };
                    let target = crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
                        room, &runtime, MovementLocalPosition { x: 2, y: 0, floor: 7 },
                    )?;
                    (
                        binding,
                        target,
                        runtime.read_actor_position(deps.actor).map_err(debug)?,
                    )
                };
                let proof = FieldFixture {
                    binding,
                    placements: vec![NativeMapItemPlacement {
                        placement_key: "fixture:unknown-tile-energy-field".into(),
                        definition: crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2126".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        quantity: 1,
                        ground: target.placement(b"fixture:unknown-tile-field".to_vec()),
                        source_binding: br#"{"fixture":"unknown-tile-energy-field"}"#.to_vec(),
                        source_attributes: serde_json::json!({}),
                        blocks_movement: false,
                        blocks_projectile: false,
                        immovable_block_solid: false,
                        owner_kind: None,
                    }],
                };
                let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                    .execute(&mut *tx)
                    .await?;
                let minted = crate::durability::native_map_items::initialize_in_transaction(
                    &mut tx, &permit, &h.root, &authority, &h.node, &proof,
                )
                .await
                .map_err(debug)?;
                assert_eq!(
                    minted.len(),
                    1,
                    "the rejected step must encounter a real field"
                );
                let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
                let current = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                    &mut tx, &permit, &h.root, &authority, &h.node, scope,
                    proof.binding.scope_generation,
                ).await.map_err(debug)?;
                let read = crate::durability::spell_item_transaction::read_qualification_scope_tile_in_transaction(
                    &mut tx, &current, &target,
                ).await.map_err(debug)?;
                assert_eq!(read.items.len(), 1);
                assert_eq!(read.items[0].item_instance_id, minted[0].item_instance_id);
                assert!(read.items[0].native_map_origin);
                tx.commit().await?;
                drop(permit);
                let before_state = {
                    let runtime = deps.runtime.lock().await;
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .get(&runtime, deps.actor, deps.admitted.game_session_id)
                        .cloned()
                };
                assert!(before_state.is_some());
                let rejected = crate::gameplay_transport::connection::regression_routed_step(
                    &owner,
                    deps.admitted,
                    deps.actor,
                    StepDirection::East,
                )
                .await?;
                assert_eq!(
                    rejected.disposition,
                    StepDisposition::Rejected,
                    "a real field still requires positively qualified tile metadata"
                );
                let runtime = deps.runtime.lock().await;
                assert_eq!(
                    runtime.read_actor_position(deps.actor).map_err(debug)?,
                    before
                );
                assert!(
                    owner.states.spell_states.lock().await.get(
                        &runtime,
                        deps.actor,
                        deps.admitted.game_session_id
                    ) == before_state.as_ref()
                );
            }
            Ok(())
        });
        eprintln!(
            "M10_MOVE_PILLOW_POSTGRES duration={:.3}s result={}",
            started.elapsed().as_secs_f64(),
            if result.is_ok() { "GREEN" } else { "RED" }
        );
        result
    })
}
