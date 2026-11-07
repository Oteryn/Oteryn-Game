//! Ordinary command 9 retains the existing native handles, replay owner and result codec.
use super::*;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::refusal_detail::{RefusalDetail, append_refusal_detail};

pub(super) async fn route<A: FreshAdmissionAuthority>(
    state: &mut State,
    authority: &A,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let selected = admitted.continuity.selected_capabilities;
    routing::item_move::route(
        authority,
        state.actor,
        admitted,
        command,
        &mut state.items,
        &mut state.containers,
        selected,
    )
    .await
}

pub(super) fn reply(
    selected: SelectedCapabilities,
    outcome: ItemMoveOutcome,
    refusal: Option<RefusalDetail>,
) -> Result<(CommandStatus, Vec<u8>), ServeEnd> {
    let mut payload = encode_item_move_result_for(
        outcome,
        ItemMoveSelection {
            equip_drop: selected.contains(CAPABILITY_ITEM_EQUIP_DROP_V1),
            world_interaction: selected.contains(CAPABILITY_WORLD_INTERACTION_V1),
            ..ItemMoveSelection::default()
        },
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    append_refusal_detail(
        &mut payload,
        selected.as_slice(),
        refusal.or(RefusalDetail::of_item_move(outcome)),
    );
    Ok((CommandStatus::Accepted, payload))
}

impl State {
    pub(super) async fn moved<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
        outcome: ItemMoveOutcome,
        deltas: &[ItemViewDelta],
    ) -> Result<(), ServeEnd> {
        for delta in deltas {
            self.item_delta(stream, admitted, delta).await?;
        }
        if outcome == ItemMoveOutcome::Moved {
            // The existing inventory refresh also publishes current container and House map deltas.
            self.inventory(stream, authority, admitted).await?;
        }
        Ok(())
    }
}
