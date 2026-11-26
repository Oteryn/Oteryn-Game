use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::item_decay_retire::{CORPSE_DECAY_AFTER_MS, CorpseDecayFence};
use crate::gameplay_transport::capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, dispatch_corpse_use_for_test};
use crate::gameplay_transport::item_view::{OpenDecision, SessionItemView};
use crate::gameplay_transport::world_spatial::SessionVisibility;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::world_object::{COMMAND_TYPE_USE_INTENT, encode_use_item_intent};
use oteryn_protocol_oteryn::world_spatial_entities::{EntityDetail, EntityKind};
use std::future::Future;

pub(super) async fn qualify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("admitted actor")?;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[4, 6])
            .ok_or("item and spatial capabilities")?;
    let channel = owner
        .observe_visible_entities(actor)
        .await
        .ok_or("visible entities")?;
    let mut items = SessionItemView::resume(admitted.continuity.item_view);
    let snapshot = SessionVisibility::with_objects(true)
        .snapshot(&channel, &mut |entities| {
            items.attach_spatial_handles(entities).map_err(|_| {
                oteryn_protocol_oteryn::world_spatial::WorldSpatialError::LimitExceeded
            })
        })
        .map_err(debug)?;
    let (identity, handle) = snapshot.entities.iter().find_map(|entity| {
        match entity.detail {
            EntityDetail::Object { item_handle: Some(handle), .. }
            | EntityDetail::ObjectWithAppearance { item_handle: Some(handle), .. }
                if entity.kind == EntityKind::Corpse => Some((entity.entity.identity, handle)),
            _ => None,
        }
    }).ok_or("server-issued Rat corpse handle")?;
    let corpse = owner.runtime.lock().await.bound_corpse(identity).ok_or("bound Rat corpse")?;
    let mut view = Some(items);
    let payload = encode_use_item_intent(handle);
    let mut command = ClientCommandView {
        command_id: 1,
        command_type: COMMAND_TYPE_USE_INTENT,
        payload: &payload,
    };

    h.root.await_ready().await?;
    let held = h.root.try_issue_semantic_pass()?;
    let refused = h.root.read_corpse_contents(owner.character, corpse.item.item_instance_id).await;
    assert!(matches!(refused, Err(crate::durability::item_transfer::ItemTransferError::Unavailable(
        crate::durability::DurabilityError::RootUnavailable
    ))));
    eprintln!("POSTGRES_CASE_EXECUTED rat_corpse_use busy_fenced_read");
    let opened = {
        let mut use_item = std::pin::pin!(dispatch_corpse_use_for_test(
            owner, admitted, &mut view, command,
        ));
        // Poll once with the real holder occupied. The base completes as StaleState.
        let first = std::future::poll_fn(|cx| std::task::Poll::Ready(use_item.as_mut().poll(cx))).await;
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        match first {
            std::task::Poll::Ready(result) => result,
            std::task::Poll::Pending => use_item.await,
        }
        .ok_or("first USE dispatch")?
    };
    assert_eq!(opened.as_ref().map(|(decision, _)| *decision), Ok(OpenDecision::Open));
    let delta = opened.map_err(debug)?.1.ok_or("opened corpse delta")?;
    assert_eq!(delta.domain_id, oteryn_protocol_oteryn::item_view::STATE_DOMAIN_OPEN_CONTAINER);

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at = floor(extract(epoch FROM clock_timestamp())*1000)::bigint - $2 WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(corpse.item.item_instance_id.as_slice())
        .bind(CORPSE_DECAY_AFTER_MS + 1)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let generation = owner.runtime.lock().await.owner_fence().map_err(debug)?.generation();
    let permit = owner.spell_lane_permit().await.ok_or("corpse retirement lane")?;
    h.root.await_ready().await?;
    h.root.retire_decayed_corpse(
        &permit,
        owner.character,
        &h.node,
        CorpseDecayFence {
            runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id),
            scope_ownership_generation: generation,
        },
        corpse.item.item_instance_id,
    ).await.map_err(debug)?;

    h.root.await_ready().await?;
    command.command_id += 1;
    let removed = dispatch_corpse_use_for_test(owner, admitted, &mut view, command)
        .await
        .ok_or("USE of removed corpse dispatch")?
        .map_err(debug)?;
    assert_eq!(removed.0, OpenDecision::StaleState);
    assert!(removed.1.is_none());
    Ok(())
}
