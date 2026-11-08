use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, prepare_current_session, reset};
use super::TestResult;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use oteryn_protocol_oteryn as wire;

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    BrowseField,
    DailyBoost,
    DepotSearch,
    Hireling,
    HotkeyEquip,
    ItemForge,
    Store,
}

pub(super) fn scenario(case: Case) -> TestResult {
    let started = std::time::Instant::now();
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!(
            "CLIENT-E2E-1 {case:?}: elapsed={:?}; SKIPPED (no PostgreSQL admin route); no database evidence",
            started.elapsed()
        );
        return Ok(());
    }
    let result = std::thread::Builder::new()
        .name(format!("client-e2e-{case:?}"))
        .stack_size(256 << 20)
        .spawn(move || body(case).map_err(|error| error.to_string()))?
        .join()
        .unwrap_or_else(|_| Err("CLIENT-E2E-1 scenario panicked".to_owned()));
    eprintln!(
        "CLIENT-E2E-1 {case:?}: elapsed={:?}, result={result:?}",
        started.elapsed()
    );
    result.map_err(Into::into)
}

fn body(case: Case) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "client_e2e", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        deps.room = activation.qualified_room().clone();
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        let index = native.item_index().ok_or("item index")?;
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        // Reuse E2E-WORLD-1's actual bundle fixture, including the locker at x=6.
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (
                "oteryn:item.tibia.i2854",
                reference("oteryn:item.tibia.i2854").ok_or("bag")?.get() - 1,
            ),
            (
                "oteryn:item.tibia.i3031",
                reference("oteryn:item.tibia.i3031").ok_or("gold")?.get() - 1,
            ),
        );
        // E2E-WORLD-1 authors the depot town on the fixture's locker before boot.
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        bundle.sectors.first_mut().ok_or("bundle sector")?
            .tiles.iter_mut().find(|tile| tile.x == 6).ok_or("locker tile")?
            .items.get_mut(1).ok_or("locker item")?.attrs.depot = Some(0);
        let bytes = oteryn_world_bundle::writer::write(
            &bundle.manifest, &bundle.sectors, &bundle.spawns,
        ).map_err(debug)?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?.digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 7,
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
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activation.frame_binding().digest(),
            world.map_revision_digest(),
            (7, 0, 7),
        );
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activation.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        // Initialize epoch zero from the same issued generation, before seeding value.
        // The cyclopedia seed uses placeholder digests and cannot precede this activation.
        sqlx::query("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
            .bind(id(42).as_slice()).bind(world.base().digest().as_slice())
            .bind("client-e2e-bundle")
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activation.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        seed_values(&h).await?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        if matches!(case, Case::Hireling) {
            deps.actor =
                super::hireling::prepare(h.pool.clone(), &mut runtime, deps.admitted).await?;
        } else {
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 7,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
        }
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        if !matches!(case, Case::Hireling) {
            deps.initialize(&owner).await;
        }
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        if matches!(case, Case::Store) {
            owner.store = Some(super::super::store::platform(admitted.world_id.as_bytes()));
        }
        let mut offers = owner.offered_capabilities().to_vec();
        offers =
            crate::gameplay_transport::daily_boost::offered(&offers, owner.daily_boost_available());
        let capability = match case {
            Case::BrowseField => 230,
            Case::DailyBoost => 151,
            Case::DepotSearch => 153,
            Case::Hireling => 165,
            Case::HotkeyEquip => 179,
            Case::ItemForge => 152,
            Case::Store => 154,
        };
        // Use the registry's real prerequisite closure; never manufacture an offer.
        let mut supported = vec![capability];
        let mut at = 0;
        while at < supported.len() {
            if let Some(offer) = offers.iter().find(|offer| offer.id == supported[at]) {
                for required in offer.requires {
                    if !supported.contains(required) {
                        supported.push(*required);
                    }
                }
            }
            at += 1;
        }
        supported.sort_unstable();
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &supported).ok_or("selection")?;
        // Store deliberately stops here until production composition offers it.
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(capability),
            "{case:?} is not offered by composed server"
        );
        match case {
            Case::BrowseField => super::browse_field::exercise(&h, &owner, &mut admitted).await?,
            Case::DailyBoost => super::daily_boost::exercise(&h, &owner, &mut admitted).await?,
            Case::DepotSearch => super::depot_search::exercise(&h, &owner, &mut admitted).await?,
            Case::Hireling => super::hireling::exercise(&h, &owner, &mut admitted).await?,
            Case::HotkeyEquip => super::hotkey_equip::exercise(&h, &owner, &mut admitted).await?,
            Case::ItemForge => super::item_forge::exercise(&h, &owner, &mut admitted).await?,
            Case::Store => super::store::exercise(&h, &owner, &mut admitted).await?,
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

// Value preconditions from the cyclopedia fixture, without its unrelated reset epoch.
async fn seed_values(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&mut *tx)
        .await?;
    let uuid = |n| crate::durability::item_fee_burn::test_support::uuid(n);
    let seed = format!(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES({}, {}, {},1,sha256('seed'),{},{},{},{},1,1000,0,1,sha256('seed'),0,{},{},{}); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES({},{},{},{},1,1000,0,1000,{}); INSERT INTO game_account_bank_balances VALUES({},{},1000,{});",
        uuid(80),
        uuid(81),
        uuid(82),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(43),
        uuid(83),
        uuid(84),
        uuid(70),
        uuid(85),
        uuid(81),
        uuid(40),
        uuid(42),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(85)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    let seed = format!(
        "{}{} INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES({},{},{},{}); INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES({},{},{},{},1,{});",
        crate::durability::item_fee_burn::test_support::item(
            70,
            "oteryn:item.tibia.i2854",
            "definition-r1",
            1
        ),
        crate::durability::item_fee_burn::test_support::item(
            72,
            "oteryn:item.tibia.i3031",
            "definition-r1",
            5
        ),
        uuid(41),
        uuid(70),
        uuid(42),
        uuid(74),
        uuid(41),
        uuid(70),
        uuid(72),
        uuid(42),
        uuid(75)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Same envelope codec used by the session's gated_command path. The existing fixture
/// drives serve_admitted over a duplex gameplay connection and decodes COMMAND_RESULT.
pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, bytes) =
        super::super::cyclopedia::support::exchange(owner, admitted, command, kind, payload)
            .await?;
    assert!(
        matches!(
            status,
            wire::CommandStatus::Accepted | wire::CommandStatus::DuplicateReplay
        ),
        "{kind}/{command}: {status:?}"
    );
    Ok(bytes)
}

pub(super) async fn count(h: &Harness, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)
}

/// Retain actual server events as well as replies (rather than inventing a client delta).
pub(super) async fn events(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    commands: &[(u64, u32, Vec<u8>)],
) -> TestResult<Vec<Vec<u8>>> {
    let mut input = Vec::new();
    for (id, kind, payload) in commands {
        let frame = wire::encode_client_command(
            admitted.continuity.connection_generation,
            &wire::ClientCommandValue {
                command_id: *id,
                command_type: *kind,
                payload,
            },
        )
        .map_err(debug)?;
        input.extend(u32::try_from(frame.len())?.to_be_bytes());
        input.extend(frame);
    }
    let (end, output) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::super::weather_wire::fixture_access::serve_input(owner, admitted, &input),
    )
    .await??;
    assert!(end.contains("AdmittedThenDisconnected"), "{end}");
    let mut at = 0;
    let mut frames = Vec::new();
    while at < output.len() {
        let size =
            u32::from_be_bytes(output.get(at..at + 4).ok_or("frame length")?.try_into()?) as usize;
        at += 4;
        frames.push(output.get(at..at + size).ok_or("truncated frame")?.to_vec());
        at += size;
    }
    Ok(frames)
}
