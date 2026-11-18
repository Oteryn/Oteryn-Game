//! Drive the retained Doomforge teleport through the session's normal paced STEP.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{channel_content_pin::ChannelContentPin, qualify_native_entry_room};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::boot::{boot, tests as fixture};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::{bundle, sector, writer};

const START: TilePos = TilePos {
    x: 34040,
    y: 32330,
    floor: -14,
};
const PORTAL: TilePos = TilePos { x: 34041, ..START };
const FORGE: TilePos = TilePos {
    x: 34062,
    y: 32334,
    ..START
};

#[test]
fn nrwicked_doomforge_step_cannot_use_a_retargeted_destination() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(
        admin,
        "nrwicked_doomforge_retargeted",
        TilePos {
            y: FORGE.y + 1,
            ..FORGE
        },
    )
}

#[test]
fn nrwicked_doomforge_step_cannot_use_the_raw_destination() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "nrwicked_doomforge", FORGE)
}

fn run(admin: String, fixture_name: &'static str, destination: TilePos) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            scenario(admin, fixture_name, destination).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Doomforge scenario thread panicked")?
        .map_err(Into::into)
}

fn scenario(admin: String, fixture_name: &'static str, destination: TilePos) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let h = Harness::create_with_admission_seed(admin, fixture_name, true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let initial = Dependencies::new()?;
            let world = initial.admitted.world_id;
            let channel = initial.admitted.channel_id;
            let room = qualify_native_entry_room(world).map_err(debug)?;
            let (bytes, pins) = portal_fixture(destination)?;
            let map = boot(&bytes, &pins, world, channel, |key| {
                (key == "oteryn:item.tibia.i37000")
                    .then(|| fixture::floor_items("oteryn:item.tibia.i37001"))
                    .flatten()
            })
            .map_err(debug)?;
            assert_eq!(map.floors().teleport(PORTAL), Some(destination));
            let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
            let pin = ChannelContentPin::from_activation(
                world,
                1,
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                (i32::from(START.x), i32::from(START.y), 14),
            );
            let deps = Dependencies::with_content(
                pin,
                room,
                crate::spell::SpellBook::new(vec![]).map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.movement_cells = (&cells).into();
            let session = deps.admitted.game_session_id;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        deps.actor,
                        session,
                        crate::gameplay_transport::actor_spell::tests::FACTS,
                        (0, 0),
                        owner.owner_now(),
                    )
                    .ok_or("spell state")?;
            }
            let (outcome, _) = owner
                .paced_step(deps.actor, session, StepDirection::East)
                .await;
            assert_eq!(outcome.disposition, StepDisposition::Blocked);
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: i32::from(START.x),
                    y: i32::from(START.y),
                    floor: 14
                },
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
}

fn portal_fixture(destination: TilePos) -> TestResult<(Vec<u8>, crate::map::boot::BootPins)> {
    let (bytes, mut pins) = fixture::teleport_bundle();
    let mut manifest = bundle::read(&bytes)?.manifest;
    manifest.world = bundle::Extent {
        min_x: START.x,
        min_y: START.y,
        max_x: destination.x + 1,
        max_y: destination.y + 1,
        floors: vec![START.floor],
    };
    let mut portal = manifest.palette.pop().ok_or("portal palette")?;
    portal.key = "oteryn:item.tibia.i37000".into();
    manifest.palette.truncate(1);
    manifest.palette.push(portal);
    let tile = |at: TilePos| sector::Tile {
        x: at.x,
        y: at.y,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: sector::Attrs::default(),
        }],
    };
    let mut entrance = tile(PORTAL);
    entrance.items.push(sector::Item {
        palette: 1,
        depth: 0,
        attrs: sector::Attrs {
            teleport: Some((
                destination.x,
                destination.y,
                destination.floor.to_ne_bytes()[0],
            )),
            ..Default::default()
        },
    });
    let sectors = [
        bundle::Sector {
            floor: START.floor,
            sx: START.x / 32,
            sy: START.y / 32,
            tiles: vec![tile(START), entrance],
        },
        bundle::Sector {
            floor: FORGE.floor,
            sx: destination.x / 32,
            sy: destination.y / 32,
            tiles: vec![tile(destination)],
        },
    ];
    let bytes = writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let mut pins = fixture::floor_pins(pins);
    pins.start = START;
    Ok((bytes, pins))
}
