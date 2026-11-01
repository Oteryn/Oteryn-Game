//! Native completion uses the same one pending delivery and retained outcome.
use super::*;

impl SessionIngress {
    pub(super) fn complete_native_if_disconnected(&mut self) -> Option<()> {
        if self.active {
            return Some(());
        }
        let Some(pending) = self.pending_delivery else {
            return Some(());
        };
        let Some(result) = pending.proven_native else {
            return Some(());
        };
        let (rejected, payload) = result.payload(pending.selected)?;
        self.complete_reconciled(CommandId::new(pending.command).ok()?, rejected, &payload)
    }

    pub(super) fn complete_reconciled(
        &mut self,
        command: CommandId,
        rejected: bool,
        payload: &[u8],
    ) -> Option<()> {
        let pending = self.pending_delivery?;
        if pending.command != command.get() {
            return None;
        }
        let frame = oteryn_protocol_oteryn::encode_command_result(
            pending.generation,
            pending.sequence,
            pending.command,
            if rejected {
                oteryn_protocol_oteryn::CommandStatus::Rejected
            } else {
                oteryn_protocol_oteryn::CommandStatus::Accepted
            },
            payload,
        )
        .ok()?;
        let digest = base64::engine::general_purpose::STANDARD.encode(Sha256::digest(&frame));
        let outcome =
            TerminalSemanticOutcome::new("command-result", &digest, pending.sequence).ok()?;
        self.ingress.terminalize(command, outcome, || {}).ok()?;
        self.retained = Some(frame);
        self.result_sequence_floor = pending.sequence;
        self.pending_delivery = None;
        self.reconciled_sequence = Some(pending.sequence);
        Some(())
    }

    pub(super) fn recovered_continuity(
        &self,
        caller_next: u64,
        caller_sequence: u64,
    ) -> Option<(u64, u64)> {
        let Some(sequence) = self.reconciled_sequence else {
            return Some((caller_next, caller_sequence));
        };
        if self.active || self.ingress.outstanding() != 0 {
            return None;
        }
        let next = self.ingress.next_command_id()?.get();
        if caller_next != next {
            // An unknown command published no ACK or counter advance. Only
            // this retained reconciled result can supply its one successor.
            if caller_next.checked_add(1)? != next {
                return None;
            }
            let envelope =
                oteryn_protocol_oteryn::decode_wire_envelope(self.retained.as_deref()?).ok()?;
            let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload()).ok()?;
            if result.command_id != caller_next || envelope.server_sequence() != sequence {
                return None;
            }
        }
        if caller_sequence < sequence {
            if caller_sequence.checked_add(1)? != sequence {
                return None;
            }
            Some((next, sequence))
        } else {
            Some((next, caller_sequence))
        }
    }
}
