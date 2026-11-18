//! Crystal 201189f8 moviment_trap.lua: a real creature push delivers merchant StepIn.
use super::autowalk::at;
use super::creature_push::{prepare, reference, scenario, spawn};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::creature_push::{CreaturePushDisposition, CreaturePushIntent};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use tokio::io::DuplexStream;

#[test]
fn rottin_merchant_step_in_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_bushwhacking", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking PostgreSQL thread panicked")?
        .map_err(Into::into)
}

pub(super) async fn exercise(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    prepare(client, owner).await?;
    let actor = admitted.runtime_actor.ok_or("pusher")?;
    let merchant = spawn(owner, "oteryn:creature.travelling_merchant", at(3, 2)).await?;
    let map = owner.bundle_map().ok_or("bundle")?;
    let tile = TilePos {
        x: 4,
        y: 2,
        floor: -7,
    };
    assert!(map.facts.create_movement_item(tile, 12187, 0, || true));
    let before = map.facts.movement_items(tile);
    assert_eq!(before.len(), 1);
    let intent = CreaturePushIntent {
        target: reference(owner, merchant).await?,
        destination: ActorPosition {
            x: 4,
            y: 2,
            floor: 7,
        },
    };
    assert_eq!(
        owner
            .push_creature_now(actor, admitted.game_session_id, intent)
            .await,
        CreaturePushDisposition::Moved,
    );
    assert!(!owner.runtime.lock().await.contains_live_creature(merchant));
    let after = map.facts.movement_items(tile);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].source, before[0].source);
    assert_eq!(after[0].facts.appearance_id, 12189);
    Ok(())
}
