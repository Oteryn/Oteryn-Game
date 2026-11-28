//! Negotiated map query reads the scheduler's committed plans and private exploration.

use super::cyclopedia_fixtures::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::ProductionKey;
use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, NodeId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, fresh_capability_selection,
};
use crate::gameplay_transport::world_required_capabilities::{
    BootCapabilityError, BoundRequiredCapabilities, offered_for_scope, with_cyclopedia_offer,
};
use oteryn_gameplay_encounter_rules::raid::{CHECK_INTERVAL_MS, RaidCatalogue, RaidPolicy};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia::{self, raid_warnings as wire};

#[test]
fn cyclopedia_raid_warnings_postgres_dispatch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Cyclopedia raid warning scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_raid_warnings", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let key = ProductionKey::new("raid:libertybay.ferumbras")?;
        let firing = id(98);
        let catalogue = RaidCatalogue::embedded()?;
        let raid = catalogue.get(&key).ok_or("Ferumbras raid")?;
        let variant = raid.select([0; 32], firing)?;
        let wave = variant.definition.waves.first().ok_or("Ferumbras wave")?;
        let spot = variant
            .areas
            .get(&wave.entries.first().ok_or("Ferumbras entry")?.area_anchor)
            .ok_or("Ferumbras location")?
            .from;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = support::world_with_raid_subarea(&deps, generation, spot)?;
        let digest = world.base().digest();
        let cells = world
            .movement_cells(activation_pin.qualified_room().movement_cells())
            .map_err(debug)?;
        support::seed(&h, digest).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_world_reset_epochs SET reset_epoch=1,server_digest=$1,client_digest=$2,frame_digest=$3")
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activation_pin.frame_binding().digest().as_slice())
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=1")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let mut admitted = deps.admitted;
        let pin = ChannelContentPin::from_activation(
            admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activation_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (1, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            admitted.world_id,
            admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(admitted.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.runtime_actor = Some(actor);
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        deps.actor = actor;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;

        let base = cyclopedia::CAPABILITY_CYCLOPEDIA_MAP_V1;
        let warnings = wire::CAPABILITY_CYCLOPEDIA_RAID_WARNINGS_V1;
        let scope = RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id);
        let boot_offers = offered_for_scope(true, Some(generation));
        let declared = [11, 18, 30, warnings];
        let unqualified = BoundRequiredCapabilities::for_channel_boot(
            &declared,
            scope,
            1,
            "policy-1",
            with_cyclopedia_offer(
                boot_offers,
                Some(generation),
                activation_pin.qualified_room().movement_cells(),
            ),
        );
        assert!(matches!(unqualified, Err(BootCapabilityError::Unoffered(id)) if id == warnings));
        let bound = BoundRequiredCapabilities::for_channel_boot(
            &declared,
            scope,
            1,
            "policy-1",
            with_cyclopedia_offer(boot_offers, Some(generation), &cells),
        );
        assert!(
            bound.is_ok(),
            "qualified Channel must accept required capability 626: {bound:?}"
        );
        let bound = bound.map_err(debug)?;
        assert!(bound.effective().contains(&base));

        let selected = fresh_capability_selection(&owner, &[4, 6, base, warnings])
            .await
            .ok_or("selection")?;
        admitted.continuity.selected_capabilities =
            fresh_capability_selection(&owner, &[4, 6, base])
                .await
                .ok_or("legacy selection")?;
        assert_eq!(
            exchange(&owner, &mut admitted).await?.0,
            CommandStatus::Rejected,
            "old clients do not select the new query"
        );
        admitted.continuity.selected_capabilities = selected;
        assert_eq!(
            exchange(&owner, &mut admitted).await?.0,
            CommandStatus::Accepted,
            "negotiated map warning query must be accepted"
        );
        assert!(selected.contains(warnings));

        // Force receipt of a winning scheduler check while retaining the actual
        // production variant's wave timing and placement. No direct firing insert.
        let mut definition = raid.select(digest, firing)?.definition;
        definition.policy = RaidPolicy::Random {
            mean_interval_s: 1,
            min_gap_s: 0,
        };
        let now = i64::try_from(oteryn_error_codes::unix_ms())?;
        let fired_at = now - i64::from(wave.delay_ms) + 10 * 60 * 1_000;
        let mut check = RaidSchedulerCheck {
            world: admitted.world_id,
            channel: admitted.channel_id,
            generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
            reset_epoch: 1,
            now_ms: fired_at - CHECK_INTERVAL_MS,
            bundle_digest: digest,
            candidates: vec![RaidScheduleCandidate {
                key,
                definition,
                eligible: true,
                calendar: None,
                firing_id: firing,
                global_margin_ms: 0,
                lua: None,
                calendar_window: None,
            }],
        };
        assert!(
            h.root
                .check_raid_schedules(&authority, &h.node, check.clone())
                .await?
                .firings
                .is_empty()
        );
        check.now_ms = fired_at;
        assert_eq!(
            h.root
                .check_raid_schedules(&authority, &h.node, check)
                .await?
                .firings
                .first()
                .map(|(id, _)| *id),
            Some(firing)
        );

        assert!(
            read(&owner, &mut admitted).await?.is_empty(),
            "undiscovered area"
        );
        set_discovery(&h, admitted, digest, false).await?;
        assert!(
            read(&owner, &mut admitted).await?.is_empty(),
            "partial area discovery"
        );
        set_discovery(&h, admitted, digest, true).await?;
        assert_eq!(
            read(&owner, &mut admitted).await?,
            [
                wire::Warning {
                    area: 1,
                    subarea: None
                },
                wire::Warning {
                    area: 1,
                    subarea: Some(3)
                },
            ],
            "the committed upcoming wave appears on the explored area and ground-floor subarea"
        );
        sqlx::query(
            "UPDATE game_world_raid_firings SET state=3 WHERE firing_id=encode($1,'hex')::uuid",
        )
        .bind(firing.as_slice())
        .execute(&h.pool)
        .await?;
        assert!(
            read(&owner, &mut admitted).await?.is_empty(),
            "cancelled firing clears the overlay"
        );
        // A failed advisory read must reject this query without closing the connection.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_cyclopedia SET catalogue_digest=$1")
            .bind([0_u8; 32].as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        assert_eq!(
            exchange(&owner, &mut admitted).await?.0,
            CommandStatus::Rejected
        );
        eprintln!("POSTGRES_EXECUTED cyclopedia_raid_warnings_postgres_dispatch");
        Ok(())
    })
}

async fn set_discovery(
    h: &Harness,
    admitted: AdmittedSession,
    digest: [u8; 32],
    complete: bool,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_cyclopedia(character_id,bundle_digest,catalogue_digest,progress,occurrence)
         VALUES(encode($1,'hex')::uuid,$2,$3,$4,encode($5,'hex')::uuid)
         ON CONFLICT(character_id) DO UPDATE SET progress=EXCLUDED.progress"
    ).bind(admitted.item_fence.ok_or("fence")?.character_id.as_bytes().as_slice())
        .bind(digest.as_slice()).bind(crate::domain::cyclopedia::BasicArea::content_digest().as_slice())
        .bind(serde_json::json!({
            "legacy": {"discovered": false, "exploration": null},
            "subareas": {"2": {"discovered": true, "exploration": null},
                         "3": {"discovered": complete, "exploration": null}}
        })).bind(id(99).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let id = admitted.continuity.next_command_id;
    support::exchange(
        owner,
        admitted,
        id,
        cyclopedia::COMMAND_TYPE_CYCLOPEDIA_QUERY,
        &wire::QUERY,
    )
    .await
}

async fn read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<Vec<wire::Warning>> {
    let (status, bytes) = exchange(owner, admitted).await?;
    assert_eq!(status, CommandStatus::Accepted);
    wire::decode(&bytes).map_err(debug)
}
