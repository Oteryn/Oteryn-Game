//! Literal movement, door USE and source chest claim share the published owners.
use super::{TestResult, fixture::debug};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

async fn use_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("session")?
            .item_fence(),
    );
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

async fn east(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::*;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    chest: &WorldObjectTarget,
    door_key: u64,
) -> TestResult {
    let offered = owner.offered_capabilities();
    let mut supported = std::collections::BTreeSet::new();
    let mut pending = vec![11];
    while let Some(capability) = pending.pop() {
        if supported.insert(capability) {
            pending.extend(
                offered
                    .iter()
                    .find(|row| row.id == capability)
                    .ok_or("capability")?
                    .requires
                    .iter()
                    .copied(),
            );
        }
    }
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            offered,
            &supported.into_iter().collect::<Vec<_>>(),
        )
        .ok_or("selection")?;
    assert!(admitted.continuity.selected_capabilities.contains(11));
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.doors().contains(door_key));
    let target = |revision| {
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&door_key.to_be_bytes());
        WorldObjectTarget {
            placement,
            expected_revision: revision,
        }
    };
    let initial = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;

    assert!(
        map.facts
            .opening_door_enterable(map.overlay.base(), door_key),
        "future door collision"
    );
    {
        let runtime = owner.runtime.lock().await;
        let proof = crate::map::floor::prepare_quest_door_opening(
            &runtime,
            owner.movement_cells,
            map,
            actor,
            admitted.game_session_id,
            initial,
            door_key,
            false,
        );
        assert!(
            proof.is_ok(),
            "opening proof: {:?}; scope_match={} context_match={} map_match={}",
            proof.err(),
            owner.movement_cells.scope().generation_digest
                == runtime.content_pin().server_artifact_digest(),
            initial.context() == runtime.pinned_movement_context(),
            owner.movement_cells.scope().map_revision.as_str()
                == crate::map::overlay::map_revision(map.overlay.base())
        );
    }
    assert_eq!(
        use_target(owner, admitted, 1, &target(0)).await?,
        UseDisposition::Committed
    );
    let inside = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(inside.position().x, initial.position().x - 1);
    assert_eq!(inside.position().y, initial.position().y);
    assert!(inside.revision() > initial.revision());
    assert_eq!(
        use_target(owner, admitted, 1, &target(0)).await?,
        UseDisposition::Committed
    );
    assert_eq!(map.facts.object_revision(door_key), 1);
    assert!(
        !map.facts
            .doors()
            .blocks(crate::map::view::placement_of(door_key).ok_or("door")?.0)
    );
    assert_eq!(
        use_target(owner, admitted, 2, &target(1)).await?,
        UseDisposition::Committed
    );
    assert_eq!(map.facts.object_revision(door_key), 2);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position()
            .x,
        initial.position().x
    );
    assert_eq!(
        use_target(owner, admitted, 3, &target(2)).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, 4, chest).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, 5, &target(3)).await?,
        UseDisposition::NotPossible
    );
    assert_eq!(map.facts.object_revision(door_key), 3);
    east(owner, admitted, 6).await?;
    assert_eq!(map.facts.object_revision(door_key), 4);
    let before = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        use_target(owner, admitted, 7, &target(4)).await?,
        UseDisposition::NotPossible
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        before
    );
    assert_eq!(map.facts.object_revision(door_key), 4);
    let fence = admitted.item_fence.ok_or("fence")?;
    assert!(
        owner
            .root
            .reward_claim_is_claimed(
                owner.character,
                owner.holder,
                fence,
                "oteryn:reward-claim.quest.u7_24.the_annihilator.reward".into()
            )
            .await
            .map_err(debug)?
    );
    assert!(
        !owner
            .root
            .reward_claim_is_claimed(
                owner.character,
                owner.holder,
                fence,
                "oteryn:reward-claim.kv.steelhelmet1".into()
            )
            .await
            .map_err(debug)?
    );
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        owner
            .root
            .reward_claim_is_claimed(
                owner.character,
                owner.holder,
                stale,
                "oteryn:reward-claim.quest.u7_24.the_annihilator.reward".into()
            )
            .await
            .is_err()
    );
    Ok(())
}
