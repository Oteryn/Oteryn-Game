//! A paced original discarded before dispatch has a known local refusal, not an unknown effect.
use super::*;
use oteryn_protocol_oteryn::refusal_detail::{RefusalDetail, append_refusal_detail};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDisposition, decode_step_intent, encode_step_outcome,
};

impl SessionCommandStream {
    /// Called only after the movement owner returned Wait, before the original frame is buffered.
    pub(in crate::gameplay_transport) fn buffer_step(
        &mut self,
        command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    ) -> Option<()> {
        if command.command_type != COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
            || decode_step_intent(command.payload).is_err()
        {
            return None;
        }
        let Some(state) = &self.state else {
            return Some(());
        };
        let mut state = state.lock().ok()?;
        let pending = state.pending_delivery?;
        let id = CommandId::new(command.command_id).ok()?;
        if !state.active
            || state.ingress.outstanding() != 1
            || pending.command != command.command_id
            || pending.proven_native.is_some()
            || !matches!(
                state.ingress.classify_duplicate(id, &semantic(id)?),
                DuplicateDisposition::PendingOriginal
            )
        {
            return None;
        }
        state.pending_delivery.as_mut()?.buffered_step = true;
        Some(())
    }
}

impl SessionIngress {
    /// Closing the stream discards its buffer. A retry that entered execution already cleared
    /// this marker; its unknown effect remains pending under the ordinary recovery guards.
    pub(super) fn complete_discarded_step(&mut self, admitted: &AdmittedSession) -> Option<()> {
        let Some(pending) = self
            .pending_delivery
            .filter(|pending| pending.buffered_step)
        else {
            return Some(());
        };
        let id = CommandId::new(pending.command).ok()?;
        let next = pending.command.checked_add(1)?;
        let sequence = admitted.continuity.server_sequence.checked_add(1)?;
        if self.active
            || self.activity_recovery
            || pending.generation != admitted.continuity.connection_generation
            || pending.selected != admitted.continuity.selected_capabilities
            || pending.proven_native.is_some()
            || self.ingress.outstanding() != 1
            || self.ingress.next_command_id().map(CommandId::get) != Some(next)
            || ![pending.command, next].contains(&admitted.continuity.next_command_id)
            || sequence < pending.sequence
            || self.result_sequence_floor > admitted.continuity.server_sequence
            || !matches!(
                self.ingress.classify_duplicate(id, &semantic(id)?),
                DuplicateDisposition::PendingOriginal
            )
        {
            return None;
        }
        let mut payload =
            encode_step_outcome(pending.selected.as_slice(), StepDisposition::Rejected, 0);
        append_refusal_detail(
            &mut payload,
            pending.selected.as_slice(),
            RefusalDetail::of_step(StepDisposition::Rejected),
        );
        // Periodic egress may have advanced while this step waited. Allocate its one local
        // terminal result after the current loss cursor, then reconcile that exact successor.
        self.pending_delivery.as_mut()?.sequence = sequence;
        self.complete_reconciled(id, true, &payload)
    }
}
