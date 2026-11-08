//! Real framed configuration, committed JSONB flags, exact context and stale fences.
use super::{
    TestResult,
    fixture::{Harness, debug, id},
    house_podium_dispatch::{Case, exchange, postgres_case},
};
use crate::durability::{house_podium::HousePodiumRefusal, house_scope_handoff::HouseId};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use oteryn_protocol_oteryn::{CommandStatus, house_podium as old, house_podium_display as wire};
#[test]
fn house_podium_display_context_visibility_preview_and_fences_postgres() -> TestResult {
    postgres_case("podium_display", Case::Display)
}
pub(super) async fn scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let house = HouseId::new(admitted.world_id, "oteryn:content.house.admiral_s_avenue_1")
        .ok_or("house")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let context = owner
        .root
        .house_podium_context(owner.holder, fence, house.clone(), id(71))
        .await?
        .map_err(debug)?;
    assert_eq!(context.podium_kind, 3);
    assert_eq!(context.definition_key, "oteryn:item.tibia.i42367");
    assert_eq!(context.definition_revision, "definition-r1");

    // The framed route rejects handles absent from its current view, before any write.
    let payload = wire::Request::QueryHandle(u64::MAX)
        .encode()
        .ok_or("stale handle")?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        exchange(owner, admitted, command, 733, &payload).await?,
        Some((CommandStatus::Rejected, Vec::new()))
    );
    assert!(admitted.continuity.selected_capabilities.contains(449));
    let actor = admitted.runtime_actor.ok_or("podium actor")?;
    owner
        .runtime
        .lock()
        .await
        .house_actor_state(actor, admitted.game_session_id)
        .map_err(|error| format!("podium actor after rejected handle: {error:?}"))?;
    assert!(owner.house_podium_display_available());
    owner
        .root
        .load_house_podiums(owner.holder, fence, house.clone())
        .await?
        .map_err(debug)?;
    // Exercise the same production resolver with a live House-map Instance, then the
    // actual owner under PostgreSQL; no UUID is accepted from the handle payload.
    use crate::gameplay_transport::item_view::{ItemKey, SessionItemView};
    let mut view = SessionItemView::resume(Default::default()).with_map_view();
    let key = ItemKey::Instance(id(71));
    view.map_view(&[key], |_| Ok(())).map_err(debug)?;
    let handle = view.table().handle(&key).ok_or("live item handle")?;
    let request = crate::gameplay_transport::house_podium_display::resolve(
        wire::Request::QueryHandle(handle.get()),
        Some(&view),
    )
    .ok_or("resolve live podium")?;
    let reply = owner
        .house_podium_display(*admitted, request)
        .await
        .ok_or("live query")?;
    assert!(
        matches!(wire::Reply::decode(&reply), Some(wire::Reply::State(state)) if state.item_definition_key == "oteryn:item.tibia.i42367")
    );
    let resumed = SessionItemView::resume(view.continuity()).with_map_view();
    assert!(
        crate::gameplay_transport::house_podium_display::resolve(
            wire::Request::QueryHandle(handle.get()),
            Some(&resumed)
        )
        .is_none()
    );
    let foreign = ItemKey::Instance(id(72));
    view.map_view(&[foreign], |_| Ok(())).map_err(debug)?;
    let handle = view.table().handle(&foreign).ok_or("foreign handle")?;
    let request = crate::gameplay_transport::house_podium_display::resolve(
        wire::Request::QueryHandle(handle.get()),
        Some(&view),
    )
    .ok_or("resolve foreign")?;
    let reply = owner
        .house_podium_display(*admitted, request)
        .await
        .ok_or("foreign query")?;
    assert_eq!(
        wire::Reply::decode(&reply),
        Some(wire::Reply::Refused(old::SetRefusal::NotHouseItem))
    );
    let value = old::SetRequest {
        item: id(71),
        expected_revision: 0,
        display_kind: 3,
        direction: 1,
        key: "oteryn:creature.rat".into(),
    };
    let visibility = wire::Visibility {
        display: false,
        platform: true,
    };
    let request = wire::Request::Preview {
        value: value.clone(),
        visibility,
    };
    let payload = request.encode().ok_or("preview encoding")?;
    let command = admitted.continuity.next_command_id;
    let (status, reply) = exchange(owner, admitted, command, 733, &payload)
        .await?
        .ok_or("preview result")?;
    assert_eq!(status, CommandStatus::Accepted);
    let Some(wire::Reply::State(preview)) = wire::Reply::decode(&reply) else {
        return Err("preview state".into());
    };
    assert!(!preview.committed);
    assert_eq!(preview.visibility, visibility);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_podium_displays")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let payload = wire::Request::Set {
        value: value.clone(),
        visibility,
    }
    .encode()
    .ok_or("set encoding")?;
    let command = admitted.continuity.next_command_id;
    let (status, reply) = exchange(owner, admitted, command, 733, &payload)
        .await?
        .ok_or("set result")?;
    assert_eq!(status, CommandStatus::Accepted);
    let Some(wire::Reply::State(saved)) = wire::Reply::decode(&reply) else {
        return Err("saved state".into());
    };
    assert!(saved.committed);
    assert_eq!(saved.visibility, visibility);
    assert_eq!(saved.projection.as_ref().map(|p| p.revision), Some(1));
    let stored: serde_json::Value =
        sqlx::query_scalar("SELECT display_variant FROM game_house_podium_displays")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        stored,
        serde_json::json!({"direction":1,"show_display":false,"show_platform":true})
    );
    let command = admitted.continuity.next_command_id;
    let (status, reply) = exchange(owner, admitted, command, 733, &payload)
        .await?
        .ok_or("stale result")?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        wire::Reply::decode(&reply),
        Some(wire::Reply::Refused(old::SetRefusal::Changed))
    );
    let legacy = old::SetRequest {
        expected_revision: 1,
        ..value
    }
    .encode()
    .ok_or("legacy")?;
    let command = admitted.continuity.next_command_id;
    let (_, reply) = exchange(owner, admitted, command, old::COMMAND_SET, &legacy)
        .await?
        .ok_or("legacy set")?;
    assert_eq!(
        old::SetResult::decode(&reply),
        Some(old::SetResult::Accepted { revision: 2 })
    );
    let payload = wire::Request::Query(id(71)).encode().ok_or("query")?;
    let command = admitted.continuity.next_command_id;
    let (_, reply) = exchange(owner, admitted, command, 733, &payload)
        .await?
        .ok_or("query result")?;
    let Some(wire::Reply::State(state)) = wire::Reply::decode(&reply) else {
        return Err("legacy state".into());
    };
    assert_eq!(state.visibility, wire::Visibility::default());
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert_eq!(
        owner
            .root
            .house_podium_context(owner.holder, stale, house.clone(), id(71))
            .await?,
        Err(HousePodiumRefusal::StaleSession)
    );
    use crate::durability::house_scope_handoff::{
        HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
        HouseScopePredecessor,
    };
    use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
    let (generation, revision): (String, String) = sqlx::query_as(
        "SELECT ownership_generation::text,source_revision::text FROM game_runtime_scope_assignments WHERE house_key=$1 AND world_id=encode($2,'hex')::uuid",
    ).bind(house.house_key()).bind(house.world_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,3")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).execute(&h.pool).await?;
    let revoked = owner
        .root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([83; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Revoke {
                house: house.clone(),
                predecessor: HouseScopePredecessor {
                    ownership_generation: generation.parse()?,
                    source_revision: revision.parse()?,
                },
            },
        })
        .await
        .map_err(debug)?;
    assert!(
        matches!(revoked, HouseScopeAssignmentOutcome::Committed(assignment) if !assignment.assigned)
    );
    assert_eq!(
        owner
            .root
            .house_podium_context(owner.holder, fence, house, id(71))
            .await?,
        Err(HousePodiumRefusal::StaleSession)
    );
    Ok(())
}
