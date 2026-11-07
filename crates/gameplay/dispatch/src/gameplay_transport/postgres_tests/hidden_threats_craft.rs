//! Source-pinned Hidden Threats craft operations through actual framed item USE and PG.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[derive(Clone, Copy, Debug)]
enum Case {
    Filing,
    HeatingJoining,
    Finishing,
}
#[test]
fn hidden_threats_craft_filing_retains_file_and_replaces_fragments_in_full_bag() -> TestResult {
    postgres(Case::Filing)
}
#[test]
fn hidden_threats_craft_heating_and_all_four_join_pairs_match_source() -> TestResult {
    postgres(Case::HeatingJoining)
}
#[test]
fn hidden_threats_craft_anvil_and_water_finish_the_forged_key_once() -> TestResult {
    postgres(Case::Finishing)
}
fn postgres(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("hidden-threats-craft", move || {
        scenario(case)
    });
    println!(
        "POSTGRES_CASE_{} hidden_threats_craft case={case:?} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "hidden_threats_craft", true, false).await?;
        let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(version, 180006);
        println!("POSTGRES_CASE_STARTED hidden_threats_craft case={case:?} version={version}");
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let map = stations(native)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        assert_eq!(
            cells.scope().coordinate_frame.as_str(),
            "oteryn:frame/world-bundle-v3"
        );
        assert_eq!(
            cells.scope().map_revision.as_str(),
            crate::map::overlay::map_revision(map.base()).as_str()
        );
        assert_eq!(
            cells.scope().generation_digest,
            pin.identity().server_artifact_digest()
        );
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            map.map_revision_digest(),
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
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
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
        super::item_look::seed_main_backpack(&harness, &[]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let file_before = prepare_steps(case, &mut owner, &mut admitted, &harness).await?;
        forge_steps(case, &mut owner, &mut admitted, &harness, file_before).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

const STATIONS: [(u16, &str); 6] = [
    (7821, "oteryn:item.tibia.i7821"),
    (7822, "oteryn:item.tibia.i7822"),
    (7823, "oteryn:item.tibia.i7823"),
    (7824, "oteryn:item.tibia.i7824"),
    (3458, "oteryn:item.tibia.i3458"),
    (10113, "oteryn:item.tibia.i10113"),
];
fn stations(
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let index = native.item_index().ok_or("native index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let bag = "oteryn:item.tibia.i2854";
    let state_refs = STATIONS
        .iter()
        .skip(1)
        .map(|(_, key)| Ok((*key, reference(key).ok_or("station reference")?.get() - 1)))
        .collect::<TestResult<Vec<_>>>()?;
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_states(
        (bag, reference(bag).ok_or("bag ref")?.get() - 1),
        (
            STATIONS[0].1,
            reference(STATIONS[0].1).ok_or("lava ref")?.get() - 1,
        ),
        &state_refs,
    );
    let data = bundle::read(&bytes)?;
    let mut tiles = Vec::new();
    for y in 0..3 {
        for x in 6..14 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if y == 1 && (7..13).contains(&x) {
                let key = STATIONS[usize::from(x - 7)].1;
                let palette = data
                    .manifest
                    .palette
                    .iter()
                    .position(|entry| entry.key == key)
                    .ok_or("station palette")?;
                items.push(sector::Item {
                    palette: u32::try_from(palette)?,
                    depth: 0,
                    attrs: Default::default(),
                });
            }
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
        }
    }
    let sectors = vec![bundle::Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles,
    }];
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
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
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            let policy = native.item_policy(key, "definition-r1")?.record();
            let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical
            else {
                return None;
            };
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                door: false,
            })
        },
    )
    .map_err(debug)
}

async fn mint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    ordinal: u64,
) -> TestResult {
    super::quest_world_callbacks::input_item(
        owner,
        admitted.item_fence.ok_or("mint fence")?,
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 6,
                y: 0,
                floor: -7,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        ordinal,
        true,
    )
    .await
}
async fn count(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

async fn craft_expected(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    used: u16,
    target: u16,
    map: bool,
    expected: UseDisposition,
) -> TestResult<(u64, Vec<u8>)> {
    let command = admitted.continuity.next_command_id;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let used = index
        .definition_ref(
            "Item",
            &format!("oteryn:item.tibia.i{used}"),
            "definition-r1",
        )
        .ok_or("used ref")?;
    let target_definition = if map {
        None
    } else {
        Some(
            index
                .definition_ref(
                    "Item",
                    &format!("oteryn:item.tibia.i{target}"),
                    "definition-r1",
                )
                .ok_or("target ref")?,
        )
    };
    let target = if map {
        let ordinal = STATIONS
            .iter()
            .position(|(id, _)| *id == target)
            .ok_or("source station")?;
        let x = 7 + u16::try_from(ordinal)?;
        super::quest_source_objects::fixture_relocate(
            owner,
            admitted,
            admitted.runtime_actor.ok_or("actor")?,
            oteryn_gameplay_quests::quest::world_callbacks::Position { x, y: 0, floor: -7 },
        )
        .await?;
        let world = owner.bundle_map().ok_or("world")?;
        let mut placement = world.overlay.base().digest().to_vec();
        placement.extend_from_slice(
            &oteryn_world_bundle::bundle::placement_key(-7, x, 1, 1)
                .ok_or("station key")?
                .to_be_bytes(),
        );
        MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })
    } else {
        MapUseTarget::Item(std::num::NonZeroU64::new(1).ok_or("placeholder")?)
    };
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("placeholder")?),
        use_with: None,
        use_with_map_item: Some(target),
        use_with_position: None,
    })
    .map_err(debug)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let (next, result, actual) = super::item_craft_carried_fluid::exchange_carried(
        owner,
        *admitted,
        command,
        &payload,
        Some(used),
        target_definition,
    )
    .await?;
    *admitted = next;
    assert_eq!(result, expected, "source craft command{command}");
    Ok((command, actual))
}

async fn craft(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    used: u16,
    target: u16,
    map: bool,
) -> TestResult<(u64, Vec<u8>)> {
    craft_expected(
        owner,
        admitted,
        used,
        target,
        map,
        UseDisposition::Committed,
    )
    .await
}

async fn prepare_steps(
    case: Case,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult<serde_json::Value> {
    for (n, item) in [(700, 27270), (701, 27261), (702, 27262)] {
        mint(owner, admitted, item, n).await?;
    }
    let file_before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE definition_production_key='oteryn:item.tibia.i27270' AND lifecycle=1").fetch_one(&h.pool).await?;
    if matches!(case, Case::Filing) {
        // The production transfer owner merges coins; use non-stackable native trays
        // so this is genuinely a full backpack rather than a fixture-only custody shape.
        for n in 703..720 {
            mint(owner, admitted, 3464, n).await?;
        }
        let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(occupied, 20, "fixture must actually fill its backpack");
        craft_expected(
            owner,
            admitted,
            27270,
            3464,
            false,
            UseDisposition::NothingToUse,
        )
        .await?;
        sqlx::query("CREATE SEQUENCE fixture_forge_failure_seen")
            .execute(&h.pool)
            .await?;
        sqlx::query(
            "GRANT USAGE,SELECT ON SEQUENCE fixture_forge_failure_seen TO oteryn_game_runtime",
        )
        .execute(&h.pool)
        .await?;
        sqlx::query("CREATE FUNCTION fixture_forge_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('fixture_forge_failure_seen'); RAISE EXCEPTION 'injected deferred forge failure'; END $$").execute(&h.pool).await?;
        sqlx::query("CREATE CONSTRAINT TRIGGER fixture_forge_fail AFTER INSERT ON game_item_craft_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fixture_forge_fail()").execute(&h.pool).await?;
        let instances: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
            .fetch_one(&h.pool)
            .await?;
        let (failed_command, _) = craft_expected(
            owner,
            admitted,
            27270,
            27261,
            false,
            UseDisposition::Rejected,
        )
        .await?;
        let seen: bool = sqlx::query_scalar("SELECT is_called FROM fixture_forge_failure_seen")
            .fetch_one(&h.pool)
            .await?;
        assert!(seen, "injected failure must execute after all craft writes");
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(instances, after, "failed COMMIT rolls back output MINT");
        assert_eq!(
            count(h, 27261).await?,
            1,
            "failed COMMIT rolls back input BURN"
        );
        assert_eq!(count(h, 27264).await?, 0);
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_craft_records WHERE command_id=$1::bigint::numeric",
        )
        .bind(i64::try_from(failed_command)?)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            receipts, 0,
            "uncommitted craft cannot acquire replay authority"
        );
        sqlx::query("DROP TRIGGER fixture_forge_fail ON game_item_craft_records")
            .execute(&h.pool)
            .await?;
    }
    Ok(file_before)
}

async fn forge_steps(
    case: Case,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    file_before: serde_json::Value,
) -> TestResult {
    let (first_command, first) = craft(owner, admitted, 27270, 27261, false).await?;
    let (next, replay, _) = super::item_craft_carried_fluid::exchange_carried(
        owner,
        *admitted,
        first_command,
        &first,
        None,
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(
        count(h, 27264).await?,
        1,
        "replay cannot polish an additional fragment"
    );
    craft(owner, admitted, 27270, 27262, false).await?;
    assert_eq!(count(h, 27261).await?, 0);
    assert_eq!(count(h, 27262).await?, 0);
    assert_eq!(count(h, 27264).await?, 1);
    assert_eq!(count(h, 27263).await?, 1);
    if !matches!(case, Case::Filing) {
        for (n, item) in (720..726).zip([27263, 27264, 27263, 27264, 27263, 27264]) {
            mint(owner, admitted, item, n).await?;
        }
        craft_expected(
            owner,
            admitted,
            27263,
            10113,
            true,
            UseDisposition::NothingToUse,
        )
        .await?;
        for station in [7821, 7822, 7823, 7824] {
            craft(owner, admitted, 27263, station, true).await?;
            craft(owner, admitted, 27264, station, true).await?;
        }
        assert_eq!(count(h, 27265).await?, 4);
        assert_eq!(count(h, 27266).await?, 4);
        for (used, target) in [
            (27265, 27265),
            (27266, 27266),
            (27265, 27266),
            (27266, 27265),
        ] {
            craft(owner, admitted, used, target, false).await?;
        }
        assert_eq!(count(h, 27265).await?, 0);
        assert_eq!(count(h, 27266).await?, 0);
        assert_eq!(count(h, 27267).await?, 4);
        if matches!(case, Case::Finishing) {
            let mut final_payload = None;
            for _ in 0..4 {
                craft(owner, admitted, 27267, 3458, true).await?;
                final_payload = Some(craft(owner, admitted, 27268, 10113, true).await?);
            }
            assert_eq!(count(h, 27267).await?, 0);
            assert_eq!(count(h, 27268).await?, 0);
            assert_eq!(count(h, 27269).await?, 4);
            let (command, payload) = final_payload.ok_or("finished key")?;
            let (next, replay, _) = super::item_craft_carried_fluid::exchange_carried(
                owner, *admitted, command, &payload, None, None,
            )
            .await?;
            *admitted = next;
            assert_eq!(replay, UseDisposition::Committed);
            assert_eq!(count(h, 27269).await?, 4);
        }
    }
    let file_after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE definition_production_key='oteryn:item.tibia.i27270' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(
        file_before, file_after,
        "retained file must preserve exact identity and item state"
    );
    Ok(())
}
