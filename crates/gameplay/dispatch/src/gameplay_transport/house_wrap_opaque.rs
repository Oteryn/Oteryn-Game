//! Passive native format negotiation; command402 keeps its existing owner.
use super::capabilities::OfferedCapability;
use oteryn_protocol_oteryn::house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_OPAQUE_V1;
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_HOUSE_FURNITURE_WRAP_OPAQUE_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn house_wrap_opaque_available(&self) -> bool {
            false
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn house_wrap_opaque_available(&self) -> bool {
            self.house_wrap_available()
        }
    };
}
pub(super) use composed_methods;

use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand},
    item_view::ItemKey,
};
use crate::durability::{consume_transform::HouseActionContext, item_mint::TypedDefinitionRef};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::house_wrap as wire;
#[path = "house_wrap_cue.rs"]
mod cues;

fn observed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    request: wire::OpaqueWrapRequest,
) -> Option<TypedDefinitionRef> {
    let (key, revision) = owner
        .active_generation?
        .native_gameplay()?
        .item_index()?
        .definition_at(request.observed_definition)?;
    Some(TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    })
}
async fn context(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> Option<(HouseActionContext, bool)> {
    let actor = admitted.runtime_actor?;
    let runtime = owner.runtime.lock().await;
    super::item_ref_admission::pinned_index(&runtime, owner.active_generation)?;
    let context = HouseActionContext::from_native_owner(
        &runtime,
        actor,
        admitted.game_session_id,
        admitted.item_fence?,
    )
    .ok()?;
    // The normal Runtime→spell-state order is retained without a lock across
    // another await. Busy state narrows fresh admission; replay remains valid.
    let ready = owner.states.spell_states.try_lock().is_ok_and(|s| {
        !s.is_dead(actor) && !s.has_pending_spell_commit(actor, admitted.game_session_id)
    });
    let ready = ready
        && runtime
            .house_player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|f| f.control_loss.is_none());
    Some((context, ready))
}
pub(super) async fn execute_opaque(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    request: wire::OpaqueWrapRequest,
    source: Option<ItemKey>,
    carried: bool,
) -> Option<wire::Reply> {
    let refuse = || Some(wire::Reply::Refused(wire::Refusal::Stale));
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_FURNITURE_WRAP_OPAQUE_V1)
    {
        return refuse();
    }
    let Some(expected) = observed(owner, request) else {
        return refuse();
    };
    let _lane = owner.spell_lane_permit().await?;
    let Some((context, ready)) = context(owner, admitted).await else {
        return refuse();
    };
    owner
        .root
        .expire_house_furniture(owner.holder, admitted.item_fence?, context.house().clone())
        .await
        .ok()?;
    let actor = admitted.runtime_actor?;
    let source = match source {
        Some(ItemKey::Instance(item)) => Some(item),
        _ => None,
    };
    let ready = ready
        && owner
            .states
            .player_trades
            .physical(admitted.game_session_id, source);
    let preview = if ready {
        match source {
            Some(item) => owner
                .root
                .read_house_furniture_wrap_source(owner.holder, context.clone(), item)
                .await
                .ok()?,
            None => None,
        }
    } else {
        None
    };
    let preview = preview.filter(|p| p.definition == expected && p.placed != carried);
    let prepared = preview.as_ref().map(|p| (p.state_revision, p.position));
    let use_command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command,
        item_fence: admitted.item_fence,
    };
    let cue = if let Some(preview) = &preview {
        Some(cues::Prepared::reserve(owner, actor, use_command, preview.position).await?)
    } else {
        None
    };
    let native = owner.active_generation?.native_gameplay()?;
    let intent = if carried {
        request.encode_carried().to_vec()
    } else {
        request.encode().to_vec()
    };
    let result = owner
        .root
        .wrap_house_furniture_opaque(
            owner.holder,
            context,
            CommandRef::new(admitted.game_session_id, CommandId::new(command).ok()?),
            intent,
            source,
            expected,
            native,
            prepared,
        )
        .await
        .ok()?;
    if result.fresh
        && matches!(result.reply, wire::Reply::Committed(_))
        && let Some(cue) = cue
    {
        cue.publish(owner, actor, use_command).await;
    }
    Some(result.reply)
}
pub(super) async fn probe(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    request: wire::OpaqueWrapRequest,
    source: Option<ItemKey>,
) -> bool {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_FURNITURE_WRAP_OPAQUE_V1)
    {
        return false;
    }
    let (Some(expected), Some(ItemKey::Instance(source))) = (observed(owner, request), source)
    else {
        return false;
    };
    let Some((context, true)) = context(owner, admitted).await else {
        return false;
    };
    if !owner
        .states
        .player_trades
        .physical(admitted.game_session_id, Some(source))
    {
        return false;
    }
    let Some(native) = owner.active_generation.and_then(|g| g.native_gameplay()) else {
        return false;
    };
    owner
        .root
        .house_furniture_wrap_affordance(owner.holder, context, source, expected, native)
        .await
        .unwrap_or(false)
}
