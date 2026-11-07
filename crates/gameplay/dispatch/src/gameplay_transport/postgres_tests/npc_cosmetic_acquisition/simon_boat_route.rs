//! Test-only source travel: real Venore captain dialogue, audited fare and arrival.
use super::*;
pub(super) async fn reach_simon(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: &mut u64,
) -> TestResult {
    walk::walk(
        owner,
        admitted,
        crate::foundation::MovementLocalPosition {
            x: 32956,
            y: 32021,
            floor: 6,
        },
    )
    .await?;
    let captain = {
        let runtime = owner.runtime.lock().await;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| runtime.npc_key(n.actor) == Some("oteryn:npc.captain_fearless"))
            .ok_or("actual Venore Captain Fearless source placement")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        }
    };
    // Controlled historical physical fare isolates travel; it is not a coin-producer proof.
    super::super::super::item_forge::support::seed_items(
        h,
        &[
            (96, "oteryn:item.tibia.i3035", 1, 0),
            (97, "oteryn:item.tibia.i3031", 70, 0),
        ],
    )
    .await?;
    let before = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_newhaven_travel_receipts WHERE character_id=encode($1,'hex')::uuid"
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    talk(owner, admitted, captain, *command, &["hi", "thais", "yes"]).await?;
    *command = command.checked_add(3).ok_or("travel command overflow")?;
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actual source actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (position.x, position.y, position.floor),
        (32310, 32210, 6),
        "only actual committed source travel moves the player across the sea"
    );
    let after = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_newhaven_travel_receipts WHERE character_id=encode($1,'hex')::uuid"
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after, before + 1, "one actual paid travel receipt");
    ladder_route::traverse(owner, admitted, command).await
}
