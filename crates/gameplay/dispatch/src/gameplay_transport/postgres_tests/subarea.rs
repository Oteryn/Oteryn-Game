//! Source BMPs -> compiler -> boot -> serve_admitted -> current-character query.
use super::cyclopedia::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_world_bundle_compiler::{Error, bundle, compile, project::Families};

struct Ground(u32);
impl compile::KeyResolver for Ground {
    fn resolve(&self, _: &str) -> compile::Resolution {
        compile::Resolution::Resolved(bundle::Family::Item, self.0)
    }
    fn terrain(&self, _: &str) -> Result<Option<bundle::Terrain>, Error> {
        Ok(Some(bundle::Terrain {
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(150),
        }))
    }
    fn floor_change(&self, _: &str) -> bool {
        false
    }
}

fn world(
    deps: &Dependencies,
    generation: &crate::content::ActiveGeneration,
) -> TestResult<crate::map::boot::BundleWorld> {
    let index = generation
        .native_gameplay()
        .and_then(|n| n.item_index())
        .ok_or("item index")?;
    let key = "oteryn:item.tibia.i100";
    let ground = index
        .definition_ref("Item", key, "definition-r1")
        .ok_or("ground ref")?;
    let (_, mut pins) = crate::map::boot::tests::bundle();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let families = Families {
        subareas: compile::subareas::Source::load(&root)?,
        ..Families::default()
    };
    let input = compile::Input {
        regions: &[include_bytes!(
            "../../../../../../tools/world-bundle-compiler/tests/fixtures/subarea-z07-x126-y121.b3"
        )
        .to_vec()],
        palette: &[key.into()],
        identity: bundle::Identity {
            project_format_version: pins.project_format_version.clone(),
            world_schema_version: pins.world_schema_version.clone(),
            content_revision: pins.content_revision.clone(),
            ..bundle::Identity::default()
        },
        world: bundle::Extent {
            min_x: 32000,
            min_y: 31000,
            max_x: 33000,
            max_y: 32000,
            floors: vec![-7],
        },
        build_class: bundle::BuildClass::Production,
        draft_areas: vec![],
        families: &families,
    };
    let compiled = compile::compile(&input, &Ground(ground.get() - 1))?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    assert_eq!(
        base.subareas.resolve(32348, 31010, -7),
        None,
        "inaccessible mask pixel was filtered before boot"
    );
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32362,
                y: 31009,
                floor: -7,
            },
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)
}

#[test]
fn subarea_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "subarea PostgreSQL scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "subarea_wire", true, false).await?;
        eprintln!("POSTGRES_STARTED SUBAREA-ADMIT-1 subarea_postgres_real_dispatch");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = world(&deps, generation)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        support::seed(&h, world.base().digest()).await?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activation_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (32362, 31009, 7),
        );
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
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = &cells;
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        assert!(owner.subarea_available());
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offers = crate::gameplay_transport::subarea::offered(
            owner.offered_capabilities(),
            owner.subarea_available(),
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[4, 6, 93]).ok_or("subarea capability")?;
        let mut old = admitted;
        old.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            support::exchange(&owner, &mut old, 1, 109, &[]).await?.0,
            CommandStatus::Rejected
        );
        let selected = admitted.continuity.selected_capabilities;
        admitted.continuity = old.continuity;
        admitted.continuity.selected_capabilities = selected;
        let crate::gameplay_transport::guild_intent::GuildReply::Payload(direct) =
            owner.subarea_command(admitted, 109, &[]).await
        else {
            return Err("direct subarea read".into());
        };
        assert_eq!(direct, vec![1, 23, 0, 0, 0, 109, 0, 0, 0]);
        let (status, bytes) = support::exchange(&owner, &mut admitted, 2, 109, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(bytes, [1, 23, 0, 0, 0, 109, 0, 0, 0]);
        {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            let at = movement.read(deps.actor).map_err(debug)?;
            movement
                .commit_cardinal(
                    at,
                    MovementLocalPosition {
                        x: 32361,
                        y: 31009,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
        }
        assert_eq!(
            support::exchange(&owner, &mut admitted, 3, 109, &[]).await?,
            (CommandStatus::Accepted, vec![0; 9])
        );
        assert_eq!(
            support::exchange(&owner, &mut admitted, 4, 109, &[0])
                .await?
                .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            support::exchange(&owner, &mut admitted, 5, 109, &[]).await?,
            (CommandStatus::Accepted, vec![0; 9])
        );
        Ok(())
    })
}
