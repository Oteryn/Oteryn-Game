//! Six native bridge cells and top-creature evacuation through the real USE owner.
use super::fixture::{debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
const LEVER: TilePos = TilePos {
    x: 32413,
    y: 32230,
    floor: -10,
};
const LANDING: MovementLocalPosition = MovementLocalPosition {
    x: 32409,
    y: 32231,
    floor: 10,
};

#[test]
fn questpc8_life_ring_bridge_moves_occupants_with_all_six_cells() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-life-ring-bridge", || {
        quest_world_callbacks::scenario(
            "authored-life-ring-bridge",
            Position {
                x: 32414,
                y: 32231,
                floor: -10,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_life_ring_bridge_moves_occupants_with_all_six_cells duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    // The native landing is ground 351, whose source id also qualifies zone facts.
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i351".into();
    for (index, (appearance, walkable)) in [
        (4598, Some(false)),
        (2772, None),
        (5770, Some(true)),
        (4788, None),
    ]
    .into_iter()
    .enumerate()
    {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(index + 1)?,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: if walkable.is_some() {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable,
                ground_speed: walkable.map(|value| if value { 150 } else { 0 }),
            }),
        });
    }
    let item = |palette| sector::Item {
        palette,
        depth: 0,
        attrs: Default::default(),
    };
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for y in 32230..=32233 {
        for x in 32408..=32415 {
            let bridge = (32410..=32412).contains(&x) && (32231..=32232).contains(&y);
            let mut items = vec![item(if bridge { 1 } else { 0 })];
            if bridge && x == 32410 {
                items.push(item(4));
            }
            if (x, y) == (LEVER.x, LEVER.y) {
                items.push(item(2));
            }
            grouped
                .entry((y / 32, x / 32))
                .or_default()
                .push(sector::Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items,
                });
        }
    }
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor: LEVER.floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    data.manifest.world = bundle::Extent {
        min_x: 32408,
        min_y: 32230,
        max_x: 32416,
        max_y: 32234,
        floors: vec![-10],
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 32413,
                y: 32231,
                floor: -10,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    try_step(
        owner,
        admitted,
        1,
        StepDirection::West,
        StepDisposition::Blocked,
    )
    .await?;
    let target = object_target(owner, LEVER, 1)?;
    assert_eq!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Committed
    );
    step(owner, admitted, 3, StepDirection::West).await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let rat = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let health = runtime
            .companion_policy("oteryn:creature.rat")
            .map_err(debug)?
            .maximum_health;
        let actor = runtime
            .admit_encounter_actor(
                "oteryn:creature.rat",
                health,
                MovementLocalPosition {
                    x: 32411,
                    y: 32232,
                    floor: 10,
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(actor, "oteryn:creature.rat")
            .map_err(debug)?;
        actor
    };
    let target = object_target(owner, LEVER, 1)?;
    assert_eq!(
        use_target(owner, admitted, 4, &target).await?,
        UseDisposition::Committed
    );
    for y in 32231..=32232 {
        for x in 32410..=32412 {
            assert_eq!(
                owner
                    .movement_cells
                    .index()
                    .lookup(owner.movement_cells().scope(), LogicalCell { x, y, z: 10 }),
                Ok(CollisionClass::Blocked)
            );
        }
    }
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position(),
            LANDING,
            "closing the bridge must evacuate the activating player"
        );
        assert_eq!(
            runtime.read_actor_position(rat).map_err(debug)?.position(),
            LANDING,
            "the other top creature must share the source landing"
        );
    }
    assert_eq!(
        use_target(owner, admitted, 4, &target).await?,
        UseDisposition::Committed
    );
    try_step(
        owner,
        admitted,
        5,
        StepDirection::East,
        StepDisposition::Blocked,
    )
    .await?;
    Ok(())
}
async fn try_step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    direction: StepDirection,
    expected: StepDisposition,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(decode_step_result(&result).map_err(debug)?, expected);
    Ok(())
}
