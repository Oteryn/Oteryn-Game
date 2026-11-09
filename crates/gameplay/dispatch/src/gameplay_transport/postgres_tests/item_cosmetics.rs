//! W19d: ordinary USE, atomic source effects, exact replay, and fail-closed canonical admission.
#[path = "item_cosmetics/glider.rs"]
mod glider;
#[path = "item_cosmetics/ot_only.rs"]
mod ot_only;
#[path = "item_cosmetics/packet.rs"]
mod packet;
#[path = "item_cosmetics/positive.rs"]
mod positive;
#[path = "item_cosmetics/stock.rs"]
mod stock;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[test]
fn item_cosmetics_unavailable_never_spends_the_source() -> TestResult {
    run_case(false, false)
}

#[test]
fn item_cosmetics_ground_unavailable_preserves_actual_ground_source() -> TestResult {
    run_case(true, false)
}

#[test]
fn item_cosmetics_ground_afflicted_unavailable_preserves_all_carried_pieces() -> TestResult {
    run_case(true, true)
}

fn run_case(ground: bool, afflicted: bool) -> TestResult {
    run_all(ground, afflicted, None)
}

fn run_all(ground: bool, afflicted: bool, case: Option<positive::Case>) -> TestResult {
    run_configuration(ground, afflicted, case, false, None)
}

pub(super) fn run_stock_random_outfit() -> TestResult {
    run_configuration(false, false, Some(positive::Case::RandomOutfit), true, None)
}

pub(super) fn run_stock_named_outfit(name: &'static str) -> TestResult {
    run_configuration(
        false,
        false,
        Some(positive::Case::RandomOutfit),
        true,
        Some(name),
    )
}

pub(super) fn run_stock_glider(ground: bool, missing: bool) -> TestResult {
    run_configuration(
        ground,
        false,
        Some(positive::Case::Glider { missing }),
        true,
        None,
    )
}

fn run_configuration(
    ground: bool,
    afflicted: bool,
    case: Option<positive::Case>,
    stock: bool,
    wanted: Option<&'static str>,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            scenario(ground, afflicted, case, stock, wanted, None)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "cosmetic scenario thread")??;
    Ok(())
}

fn scenario(
    ground: bool,
    afflicted: bool,
    case: Option<positive::Case>,
    stock: bool,
    wanted: Option<&'static str>,
    ot_only: Option<u32>,
) -> TestResult {
    let key = match case {
        Some(positive::Case::Glider { .. }) => "oteryn:item.tibia.i21897",
        Some(
            positive::Case::Parts
            | positive::Case::OmittedGrant
            | positive::Case::ConcurrentGrant
            | positive::Case::ProductionParts,
        ) => "oteryn:item.tibia.i36938",
        Some(positive::Case::Outfit | positive::Case::ProductionOutfit) => {
            "oteryn:item.tibia.i16252"
        }
        Some(positive::Case::ProductionMount) => "oteryn:item.tibia.i23538",
        _ if afflicted => "oteryn:item.tibia.i12551",
        _ => "oteryn:item.tibia.i6579",
    };
    let ot_key = ot_only.map(|id| format!("oteryn:item.tibia.i{id}"));
    let key = ot_key.as_deref().unwrap_or(key);
    run(async |admin| {
        let harness = if case.is_some() || ot_only.is_some() {
            Harness::create_appearance(admin, "item_cosmetics_admitted").await?
        } else {
            Harness::create_with_admission_seed(admin, "item_cosmetics_unavailable", true, false)
                .await?
        };
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let production = matches!(
            case,
            Some(
                positive::Case::ProductionOutfit
                    | positive::Case::ProductionParts
                    | positive::Case::ProductionAddonDoll
                    | positive::Case::ProductionMount
            )
        );
        let cosmetic_packet = if ot_only.is_some() {
            Some(ot_only::packet()?)
        } else if production {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            Some(
                crate::content::native_gameplay::NativeGameplayInput::from_manifest(&path)
                    .map_err(debug)?
                    .cosmetics
                    .bytes,
            )
        } else {
            match case {
                Some(_) => Some(packet::cosmetics_packet()?),
                None => Some(
                    crate::content::native_gameplay::NativeGameplayInput::unavailable_cosmetics()
                        .bytes,
                ),
            }
        };
        let (controller, pin) = if stock {
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?
        } else {
            crate::content::native_gameplay::test_support::cosmetics_fixture::activated_with_optional_cosmetics(
                deps.admitted.world_id, None, cosmetic_packet.as_deref(),
            )
        };
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        if case.is_none() && ot_only.is_none() {
            assert!(
                native.cosmetics().is_none(),
                "explicit unavailable-section fixture"
            );
        } else {
            assert!(native.cosmetics().is_some(), "explicit admitted authoring");
        }
        if !matches!(case, Some(positive::Case::Glider { .. })) {
            assert!(oteryn_gameplay_items::item_cosmetics::rule(key).is_some());
        }
        if ot_only.is_some() {
            ot_only::qualify_complete_pool(native)?;
        }
        let index = native.item_index().ok_or("index")?;
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let bundle_room = super::rust_lottery::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle_room.frame_binding().digest(),
            bundle_room.map_revision_digest(),
            (6, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        use crate::gameplay_transport::spell_premium_coordinator::{
            SpellPremiumCoordinator, purchase_fixture,
        };
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        let coordinator = SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(harness.root.clone()),
        );
        let mut owner = deps.owner(&harness, &authority);
        if case.is_some() {
            owner.premium_coordinator = Some(&coordinator);
            owner.admit_premium(id(40), deps.admitted.game_session_id);
            assert!(
                owner
                    .session_premium_gate(deps.admitted.game_session_id)
                    .await
                    .premium_current(),
                "the positive fixture uses the existing delivered Premium owner"
            );
        }
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        if ground {
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&bundle_room));
        }
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let mut contents = match case {
            Some(positive::Case::Glider { missing }) => {
                let mut pieces = vec![
                    (key, 1),
                    ("oteryn:item.tibia.i21898", 1),
                    ("oteryn:item.tibia.i21906", 1),
                    ("oteryn:item.tibia.i21902", 1),
                    ("oteryn:item.tibia.i21899", 1),
                    ("oteryn:item.tibia.i21905", 1),
                    ("oteryn:item.tibia.i21901", 1),
                ];
                if missing {
                    pieces.retain(|(key, _)| *key != "oteryn:item.tibia.i21898");
                } else {
                    // Retain a second complete recipe to prove already-owned no-spend.
                    pieces.extend(pieces.clone());
                }
                if ground {
                    pieces.splice(0..0, [(key, 1), (bag, 1)]);
                }
                pieces
            }
            Some(
                positive::Case::Parts
                | positive::Case::OmittedGrant
                | positive::Case::ConcurrentGrant
                | positive::Case::ProductionParts,
            ) => vec![(key, 1); 5],
            Some(positive::Case::Outfit | positive::Case::ProductionOutfit) => vec![
                (key, 1),
                ("oteryn:item.tibia.i16253", 1),
                ("oteryn:item.tibia.i16254", 1),
                (key, 1),
            ],
            _ => vec![(key, 1)],
        };
        if afflicted {
            contents.push((bag, 1));
            contents.extend([
                ("oteryn:item.tibia.i12551", 1),
                ("oteryn:item.tibia.i12552", 1),
                ("oteryn:item.tibia.i12553", 1),
                ("oteryn:item.tibia.i12554", 1),
                ("oteryn:item.tibia.i12555", 1),
                ("oteryn:item.tibia.i12556", 1),
            ]);
        }
        super::item_look::seed_main_backpack(&harness, &contents).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let appearance_consumer = stock
            || matches!(case, Some(positive::Case::ProductionAddonDoll));
        let offered = if appearance_consumer {
            crate::gameplay_transport::appearance_change::offer::offered(
                owner.offered_capabilities(),
                owner.appearance_available(),
            )
        } else {
            owner.offered_capabilities().to_vec()
        };
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &offered,
            if appearance_consumer {
                &[3, 4, 6, 11, 15, 18, 25]
            } else {
                &[4, 6, 11, 15, 18]
            },
        )
        .ok_or("offers")?;
        if appearance_consumer {
            assert!(admitted.continuity.selected_capabilities.contains(25));
        }
        let source = id(71);
        if ground {
            let target = {
                let runtime = owner.runtime.lock().await;
                crate::durability::item_loop_move::LoopGround::from_native_owner(
                    &bundle_room,
                    &runtime,
                    MovementLocalPosition {
                        x: 6,
                        y: 0,
                        floor: 7,
                    },
                    &crate::map::overlay::map_revision(&base),
                )
                .map_err(debug)?
            };
            // Historical fixture setup; the real USE runs with ordinary guards and
            // the current Ground owner, actor, content and item-location proof.
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(source.as_slice()).execute(&mut *tx).await?;
            if afflicted || matches!(case, Some(positive::Case::Glider { .. })) {
                // Real carried ingredients below the inner bag. The only visible
                // handle for 12551 is now the actual Ground trigger, regardless of
                // snapshot domain ordering; the shared USE helper cannot pick a bag entry.
                let last = match case {
                    Some(positive::Case::Glider { missing: false }) => 86,
                    _ => 78,
                };
                for (offset, item) in (73_u8..=last).enumerate() {
                    let item = id(item);
                    sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                        .bind(item.as_slice()).execute(&mut *tx).await?;
                    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid)")
                        .bind(item.as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice())
                        .bind(i64::try_from(offset + 1)?).bind(id(79).as_slice()).execute(&mut *tx).await?;
                }
            }
            let source_ref = [
                crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
                id(79).as_slice(),
            ]
            .concat();
            sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,1)")
                .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position).bind(source_ref)
                .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context)
                .execute(&mut *tx).await?;
            tx.commit().await?;
        }
        if let Some(case) = case {
            if let Some(name) = wanted {
                stock::random_outfit(&owner, &harness, admitted, reference, Some(name)).await?;
            } else {
                positive::execute(&owner, &harness, admitted, case, reference).await?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let before: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).fetch_one(&harness.pool).await?;
        let before_all: serde_json::Value = sqlx::query_scalar(
            "SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i",
        )
        .fetch_one(&harness.pool)
        .await?;
        let before_ground: serde_json::Value = sqlx::query_scalar("SELECT coalesce(jsonb_agg(to_jsonb(g) ORDER BY item_instance_id),'[]'::jsonb) FROM game_item_ground_locations g")
            .fetch_one(&harness.pool).await?;
        let before_carried: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_object('main',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY item_instance_id),'[]'::jsonb) FROM game_item_container_entries e),'nested',(SELECT coalesce(jsonb_agg(to_jsonb(n) ORDER BY item_instance_id),'[]'::jsonb) FROM game_item_nested_entries n))")
            .fetch_one(&harness.pool).await?;
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let (_end, result, _actual) =
            super::rust_lottery::exchange(&owner, admitted, 1, &payload, reference(key), None)
                .await?;
        assert_eq!(result, UseDisposition::NotPossible);
        let after: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(
            after, before,
            "unavailable or owner-excluded actions must not burn the source"
        );
        assert_eq!(
            sqlx::query_scalar::<_, serde_json::Value>(
                "SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i"
            )
            .fetch_one(&harness.pool)
            .await?,
            before_all,
            "carried Afflicted ingredients also remain unchanged"
        );
        assert_eq!(sqlx::query_scalar::<_, serde_json::Value>("SELECT coalesce(jsonb_agg(to_jsonb(g) ORDER BY item_instance_id),'[]'::jsonb) FROM game_item_ground_locations g")
            .fetch_one(&harness.pool).await?, before_ground, "Ground source custody and provenance remain unchanged");
        assert_eq!(sqlx::query_scalar::<_, serde_json::Value>("SELECT jsonb_build_object('main',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY item_instance_id),'[]'::jsonb) FROM game_item_container_entries e),'nested',(SELECT coalesce(jsonb_agg(to_jsonb(n) ORDER BY item_instance_id),'[]'::jsonb) FROM game_item_nested_entries n))")
            .fetch_one(&harness.pool).await?, before_carried, "all carried and nested custody stays unchanged");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_consume_transform_records"
            )
            .fetch_one(&harness.pool)
            .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_character_item_cosmetic_progress"
            )
            .fetch_one(&harness.pool)
            .await?,
            0
        );
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE source_kind='item_action'")
            .fetch_one(&harness.pool).await?, 0);
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED canonical_catalogue_unavailable ground={ground} afflicted={afflicted}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn srx4_ot_cosmetic_items_refuse_with_complete_synthetic_catalogue_on_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SRX4_OT_EXCLUSION_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    for ground in [false, true] {
        for item in [6579, 37746] {
            let started = std::time::Instant::now();
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn(move || {
                    scenario(ground, false, None, false, None, Some(item))
                        .map_err(|e| e.to_string())
                })?
                .join()
                .map_err(|_| "OT exclusion thread")??;
            eprintln!(
                "SRX4_OT_EXCLUSION_POSTGRES_EXECUTED item={item} ground={ground} duration={:.3}s",
                started.elapsed().as_secs_f64()
            );
        }
    }
    Ok(())
}

pub(super) fn nx5_production_case(kind: u8) -> TestResult {
    let case = match kind {
        0 => positive::Case::ProductionOutfit,
        1 => positive::Case::ProductionParts,
        2 => positive::Case::ProductionAddonDoll,
        _ => positive::Case::ProductionMount,
    };
    run_all(false, false, Some(case))
}
