//! Native source portals and two admitted players crossing pressure-controlled walls.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id},
};
use crate::foundation::{CharacterId, CharacterLease, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Passage, Position, Trigger};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};

pub(super) fn source(name: &str) -> TestResult<(u16, Position, Option<Position>)> {
    let uid: u16 = name.rsplit('-').next().ok_or("source uid")?.parse()?;
    let (x, y, z, destination) = match uid {
        29002 => (32810, 31595, 5, None),
        29003 => (32794, 31595, 5, None),
        38003 => (32675, 31646, 10, Some((32725, 31589, 12))),
        38004 => (32669, 31653, 10, Some((32679, 31673, 10))),
        38005 => (32794, 31576, 5, Some((32812, 31577, 5))),
        38006 => (32812, 31576, 5, Some((32794, 31577, 5))),
        _ => return Err("unknown Draconia route anchor".into()),
    };
    Ok((
        uid,
        Position { x, y, floor: -z },
        destination.map(|(x, y, z)| Position { x, y, floor: -z }),
    ))
}
pub(super) fn gate(uid: u16) -> Option<Position> {
    match uid {
        29002 => Some(Position {
            x: 32796,
            y: 31594,
            floor: -5,
        }),
        29003 => Some(Position {
            x: 32796,
            y: 31576,
            floor: -5,
        }),
        _ => None,
    }
}
pub(super) fn fixture(mut callback: Callback, name: &str) -> TestResult<Callback> {
    let (uid, position, destination) = source(name)?;
    callback.key = format!("oteryn:quest-callback/{name}");
    callback.quest = "oteryn:quest.draconia_quest".into();
    callback.trigger = Trigger::StepIn;
    callback.selector.position = Some(position);
    callback.selector.unique = (uid != 38003).then_some(uid);
    callback.selector.action = None;
    callback.selector.appearance = Some(if destination.is_some() { 1949 } else { 431 });
    callback.lever = None;
    callback.reversible = false;
    callback.passages = gate(uid)
        .into_iter()
        .map(|position| Passage {
            position,
            appearance: 1270,
            replacement: None,

            create_on_close: None,
        })
        .collect();
    callback.teleport = destination;
    callback.storage.clear();
    callback.offerings.clear();
    callback.return_on_failure = false;
    Ok(callback)
}
pub(super) fn native_anchor(callback: &Callback, trigger: Position, appearance: u16) -> bool {
    callback
        .key
        .strip_prefix("oteryn:quest-callback/")
        .filter(|name| name.starts_with("draconia-route-"))
        .and_then(|name| source(name).ok())
        .is_some_and(|(uid, position, destination)| {
            position == trigger
                && callback.selector.unique == Some(uid)
                && appearance == if destination.is_some() { 1949 } else { 431 }
        })
}
pub(super) fn native_teleport(callback: &Callback) -> Option<(u16, u16, u8)> {
    let name = callback.key.strip_prefix("oteryn:quest-callback/")?;
    if !name.starts_with("draconia-route-") {
        return None;
    }
    let (uid, position, destination) = source(name).ok()?;
    if uid != 38003 || callback.selector.position != Some(position) {
        return None;
    }
    let destination = destination?;
    Some((
        destination.x,
        destination.y,
        // Bundles store the signed native floor byte; B3 stores positive legacy z.
        destination.floor.to_ne_bytes()[0],
    ))
}

#[test]
fn draconia_route_native_29002() -> TestResult {
    scenario("draconia-route-29002")
}
#[test]
fn draconia_route_native_29003() -> TestResult {
    scenario("draconia-route-29003")
}
#[test]
fn draconia_route_native_38003() -> TestResult {
    scenario("draconia-route-38003")
}
#[test]
fn draconia_route_native_38004() -> TestResult {
    scenario("draconia-route-38004")
}
#[test]
fn draconia_route_native_38005() -> TestResult {
    scenario("draconia-route-38005")
}
#[test]
fn draconia_route_native_38006() -> TestResult {
    scenario("draconia-route-38006")
}
fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            let (_, position, _) = source(name).map_err(|e| e.to_string())?;
            super::quest_world_callbacks::scenario(name, position, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Draconia route scenario panicked")??;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    direction: StepDirection,
) -> TestResult<Vec<u8>> {
    super::quest_world_callbacks::wire(owner, admitted, command, 1, encode_step_intent(direction))
        .await
}
pub(super) async fn exercise(
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let (uid, _, destination) = source(name)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    if let Some(destination) = destination {
        let reply = step(owner, admitted, 1, StepDirection::East).await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
        assert_eq!(
            at.position(),
            super::quest_world_callbacks::local(destination)
        );
        assert_eq!(step(owner, admitted, 1, StepDirection::East).await?, reply);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?,
            at
        );
    } else {
        let gate = gate(uid).ok_or("gate")?;
        let guest_session = GameSessionId::decode(&id(110)).map_err(debug)?;
        let observer = {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(guest_session)
                .map_err(debug)?;
            let observer = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    observer,
                    super::quest_world_callbacks::local(Position {
                        x: gate.x + 1,
                        ..gate
                    }),
                )
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    observer,
                    guest_session,
                    CharacterLease::new(CharacterId::decode(&id(101)).map_err(debug)?, 2)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
            observer
        };
        deps.initialize_player(owner, observer, guest_session).await;
        let mut guest = *admitted;
        guest.game_session_id = guest_session;
        guest.runtime_actor = Some(observer);
        owner.admit_progression_session(&mut guest).await;
        guest.item_fence = Some(
            owner
                .current_quest_fence(guest.game_session_id)
                .await
                .map_err(|()| "guest fence")?
                .ok_or("guest fence")?
                .item_fence(),
        );
        owner.admit_quest_session(&guest).await;
        let blocked = step(owner, &mut guest, 1, StepDirection::West).await?;
        assert_eq!(
            decode_step_result(&blocked).map_err(debug)?,
            StepDisposition::Blocked,
            "closed source wall refuses the second player"
        );
        let press = step(owner, admitted, 1, StepDirection::East).await?;
        assert_eq!(
            decode_step_result(&press).map_err(debug)?,
            StepDisposition::Moved
        );
        let map = owner.bundle_map().ok_or("map")?;
        let tile = crate::map::overlay::TilePos {
            x: gate.x,
            y: gate.y,
            floor: gate.floor,
        };
        assert!(
            !crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                .map_err(debug)?
                .ok_or("gate tile")?
                .entries
                .iter()
                .any(|entry| entry.facts.appearance_id == 1270),
            "source pressure plate opens its paired wall"
        );
        let cross = step(owner, &mut guest, 2, StepDirection::West).await?;
        assert_eq!(
            decode_step_result(&cross).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(observer)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(gate)
        );
        assert_eq!(
            step(owner, &mut guest, 2, StepDirection::West).await?,
            cross
        );
        assert_eq!(
            decode_step_result(&step(owner, &mut guest, 3, StepDirection::West).await?)
                .map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(
            decode_step_result(&step(owner, admitted, 2, StepDirection::West).await?)
                .map_err(debug)?,
            StepDisposition::Moved
        );
        assert!(
            crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                .map_err(debug)?
                .ok_or("gate tile")?
                .entries
                .iter()
                .any(|entry| entry.facts.appearance_id == 1270),
            "departure restores the wall after the guest crosses"
        );
        println!(
            "DRACONIA_PARTY_WIRE two_admitted_characters=true blocked_then_crossed=true departure_restored=true"
        );
    }
    let after = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), before.tracks());
    let after_receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(after_receipts, receipts);
    Ok(())
}
