//! ACTMARK: native action bindings through ordinary STEP transport.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};

fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}

#[test]
fn actmark_binding_rejects_unknown_action_at_load() -> TestResult {
    let mut value: serde_json::Value =
        serde_json::from_str(world_callbacks::action_markers::EMBEDDED)?;
    value["bindings"][0]["action"] = 65535.into();
    for callback in value["bindings"][0]["callbacks"]
        .as_array_mut()
        .ok_or("callbacks")?
    {
        callback["selector"]["action"] = 65535.into();
    }
    let error = world_callbacks::action_markers::load(&value.to_string())
        .err()
        .ok_or("unknown action admitted")?;
    assert!(error.contains("65535"), "{error}");
    Ok(())
}

#[test]
fn actmark_four_stairs_have_compiled_action_attributes() -> TestResult {
    for (_, from, _) in routes() {
        let p = position(from);
        let bytes = std::fs::read(format!(
            "{}/../../../content/world/placements/region-z{:02}-x{:03}-y{:03}.b3",
            env!("CARGO_MANIFEST_DIR"),
            -p.floor,
            p.x / 256,
            p.y / 256
        ))?;
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            &bytes,
            oteryn_world_bundle::bundle::TILE_LIMITS,
            &mut oteryn_world_bundle::sector::Budget {
                tiles: 65536,
                entries: 1_000_000,
            },
        )?;
        let tile = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|tile| tile.x == p.x && tile.y == p.y)
            .ok_or("stair tile")?;
        assert_eq!(tile.items[0].attrs.action, Some(25018), "stair {from:?}");
    }
    Ok(())
}

type Route = (&'static str, (u16, u16, i8), (u16, u16, i8));
fn routes() -> [Route; 4] {
    [
        (
            "crystal-stairs-29",
            (31973, 32905, -11),
            (31971, 32904, -10),
        ),
        ("crystal-stairs-31", (31973, 32905, -10), (31975, 32911, -9)),
        ("crystal-stairs-33", (32009, 32928, -9), (32009, 32929, -10)),
        ("crystal-stairs-35", (32009, 32928, -10), (32009, 32929, -9)),
    ]
}

#[test]
fn actmark_all_four_stairs_run_donor_stepin_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    std::thread::Builder::new()
        .name("actmark-stairs".into())
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for (name, from, to) in routes() {
                super::quest_world_callbacks::stateless_scenario(
                    name,
                    position(from),
                    position(to),
                )
                .map_err(|error| error.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "actmark thread panicked")?
        .map_err(Into::into)
}

#[test]
fn actmark_tile_without_action_does_nothing_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    negative("actmark-no-marker")
}
#[test]
fn actmark_unmatched_position_keeps_donor_feedback_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    negative("actmark-unmatched-position")
}
fn negative(name: &'static str) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, position((31973, 32905, -11)), false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "actmark thread panicked")?
        .map_err(Into::into)
}

pub(super) fn callback() -> TestResult<Callback> {
    world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key.ends_with("crystal-stairs-29"))
        .ok_or_else(|| "action binding".into())
}

pub(super) fn world(
    callback: &Callback,
    trigger: Position,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    let mut physical = callback.clone();
    if name == "actmark-no-marker" {
        physical.selector.action = None;
    }
    super::quest_world_callbacks::world(
        &physical,
        if name == "actmark-unmatched-position" {
            Position {
                x: trigger.x + 1,
                ..trigger
            }
        } else {
            trigger
        },
    )
}

pub(super) async fn exercise(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    name: &str,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::*;
    let entered = crate::foundation::MovementLocalPosition {
        x: 31973,
        y: 32905,
        floor: 11,
    };
    let steps = if name == "actmark-unmatched-position" {
        2
    } else {
        1
    };
    for command in 1..=steps {
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(super::fixture::debug)?,
            StepDisposition::Moved
        );
    }
    let runtime = owner.runtime.lock().await;
    let actual = runtime
        .read_actor_position(actor)
        .map_err(super::fixture::debug)?
        .position();
    assert_eq!(
        actual,
        crate::foundation::MovementLocalPosition {
            x: entered.x + i32::try_from(steps - 1)?,
            ..entered
        }
    );
    let effects = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_effects_since(0);
    assert_eq!(effects.len(), if steps == 2 { 2 } else { 0 });
    assert!(effects.iter().all(|(_, _, effect)| *effect == 11));
    Ok(())
}
