//! BED-1: bed sleep through the production `ComposedFreshAdmission::production_bed_sleep`, reached by
//! the real USE dispatch (`bed_sleep::dispatch`), against PostgreSQL. Crystal `bed.cpp:178`
//! (`canUse`: premium, inside a house) and `:262` (`sleep`: sleeper recorded, player logged out).
//! The House with its bed pair is a test-support placement: no production bundle World carries one yet.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::content::{LogicalCell, QualifiedHouseTiles};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::ChannelContentPin;
use crate::gameplay_transport::SeamRooms;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
use oteryn_protocol_oteryn::bed::BedSleepIntent;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";

#[test]
fn bed_sleep_production_dispatch_records_sleeper_for_premium_and_refuses_otherwise() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                let started = std::time::Instant::now();
                let h =
                    Harness::create_with_admission_seed(admin, "bed_sleep_production", true, false)
                        .await?;
                let result = scenario(&h).await;
                h.cleanup().await?;
                result?;
                println!(
                    "PG_MARKER bed-sleep-production refusal sleeper receipt elapsed={:?}",
                    started.elapsed()
                );
                Ok(())
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "bed sleep production thread panicked")??;
    Ok(())
}

async fn rows(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_house_bed_sleepers),(SELECT count(*) FROM game_house_bed_sleep_receipts)",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn session_state(h: &Harness, session: &[u8]) -> TestResult<i16> {
    Ok(sqlx::query_scalar(
        "SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session)
    .fetch_one(&h.pool)
    .await?)
}

async fn scenario(h: &Harness) -> TestResult {
    super::reset();
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
    // The House east of the entry cell: a door cell, then a bed head and its partner.
    let start = room.entry_start();
    let cell = |dx: i32| LogicalCell {
        x: start.x + dx,
        y: start.y,
        z: i32::from(start.floor),
    };
    let scope = room.movement_cells().scope().clone();
    let room = room.with_house_tiles_for_test(QualifiedHouseTiles::single_bed_house_for_test(
        scope,
        HOUSE,
        [cell(1), cell(2), cell(3)],
        cell(0),
        cell(1),
        cell(2),
        cell(3),
    ));
    // The pin carries the room's own digests, so the House presence proof binds to this room.
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content(
        pin,
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &evidence.evidence;
    owner.rooms = SeamRooms::new(Some(&deps.room), None);
    // The transfer carries the House capability (id 49) and its requirements through the handoff.
    const OFFERS: &[crate::gameplay_transport::capabilities::OfferedCapability] = &[
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 4,
            requires: &[6],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 6,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 49,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 86,
            requires: &[],
        },
    ];
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[],
        crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id, deps.admitted.channel_id),
        1,
        "policy-1",
        OFFERS,
    )
    .map_err(debug)?;
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    owner
        .runtime
        .lock()
        .await
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
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    // The Character's stored sex selects the sleeping look; the admission seed leaves it unset.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_roots SET sex=1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice())
        .bind(HOUSE)
        .execute(&h.pool)
        .await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("house")?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 49, 86])
            .ok_or("house offer")?;
    let mut body = vec![8, 1, 18, HOUSE.len() as u8];
    body.extend(HOUSE.as_bytes());
    let (inside, _) = owner
        .transfer_house(admitted, 1, &body)
        .await
        .ok_or("house transfer outcome")?;
    let admitted = inside.ok_or("house transfer admission")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let fence =
        crate::gameplay_transport::character_gameplay_fence::current(&owner, session, false)
            .await
            .ok_or("fence")?;
    assert!(owner.track_restored(
        session,
        actor,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence,
    ));

    let (digest, key) = {
        let runtime = owner.runtime.lock().await;
        let head = cell(2);
        let key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: u16::try_from(head.x)?,
                y: u16::try_from(head.y)?,
                floor: i8::try_from(head.z)?,
            },
            0,
        )
        .ok_or("placement key")?;
        (runtime.content_pin().server_artifact_digest(), key)
    };
    let target = ItemKey::MapBase {
        bundle_digest: digest,
        placement_key: key,
        reset_epoch: 0,
    };
    let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let handle = view
        .map_view(&[target], |table| {
            table
                .handle(&target)
                .ok_or(crate::gameplay_transport::item_view::ItemViewError::LimitExceeded)
        })
        .map_err(debug)?;
    let command = UseCommand {
        game_session_id: session,
        command_id: admitted.continuity.next_command_id,
        item_fence: admitted.item_fence,
    };
    let intent = BedSleepIntent {
        target: handle,
        skill: 6,
    };

    // Crystal bed.cpp:178: no Premium (no registration, closed gate) refuses and writes nothing.
    let refused = crate::gameplay_transport::bed_sleep::dispatch(
        &owner,
        deps.actor,
        command,
        Ok(intent),
        Some(&view),
    )
    .await
    .map_err(debug)?;
    assert_eq!(refused.disposition, UseDisposition::Rejected);
    assert_eq!(rows(h).await?, (0, 0));
    let before = session_state(h, session.as_bytes().as_slice()).await?;
    assert_ne!(before, 3);

    // Premium account: the same USE records the sleeper and logs the session out (bed.cpp:262).
    let (consumer, clock, activation, _) =
        crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let coordinator =
        crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
    owner.premium_coordinator = Some(&coordinator);
    owner.admit_premium(id(40), session);
    let slept = crate::gameplay_transport::bed_sleep::dispatch(
        &owner,
        deps.actor,
        command,
        Ok(intent),
        Some(&view),
    )
    .await
    .map_err(debug)?;
    assert_eq!(slept.disposition, UseDisposition::OfflineTraining);
    assert_eq!(rows(h).await?, (1, 1));
    assert_eq!(session_state(h, session.as_bytes().as_slice()).await?, 3);
    Ok(())
}
