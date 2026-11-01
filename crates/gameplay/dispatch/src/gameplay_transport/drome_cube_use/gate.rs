//! The item-use gate owns one bounded PREPARE until a known Cube outcome.
use super::*;

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport::item_use) struct PendingCubeUse {
    pub(super) request: ItemUseRequest,
}

impl PendingCubeUse {
    pub(super) fn binding(&self) -> Option<[u8; 32]> {
        match self.request.kind {
            ItemUseKind::DromeCube { wire_binding } => Some(wire_binding),
            _ => None,
        }
    }

    fn touched(&self) -> Option<[[u8; 16]; 2]> {
        let output = match self.request.flask {
            ItemUseFlaskPlan::Stack {
                item_instance_id, ..
            }
            | ItemUseFlaskPlan::New {
                item_instance_id, ..
            } => item_instance_id,
            _ => return None,
        };
        Some([self.request.source_item_instance_id, output])
    }
}

impl super::super::ItemUseGate {
    pub(super) fn prepare_cube(
        &mut self,
        request: ItemUseRequest,
        now_ms: u64,
    ) -> Result<(), UseDisposition> {
        if self.cube_pending.is_some() {
            return Err(UseDisposition::Exhausted);
        }
        let pending = PendingCubeUse { request };
        let touched = pending.touched().ok_or(UseDisposition::Rejected)?;
        if pending.binding().is_none() || touched[0] == touched[1] {
            return Err(UseDisposition::Rejected);
        }
        if touched.iter().any(|item| self.unspendable.contains(item)) {
            return Err(UseDisposition::StaleState);
        }
        // No Cube cooldown. Reserve before the await, so cancellation cannot lose the plan.
        self.start_drink(now_ms)?;
        self.unspendable.extend_from_slice(&touched);
        self.cube_pending = Some(Box::new(pending));
        Ok(())
    }

    pub(super) fn mark_cube_unknown(&mut self, command: u64) {
        if self
            .cube_pending
            .as_ref()
            .is_some_and(|pending| pending.request.command_id == command)
            && self.last.is_none_or(|(last, _)| last <= command)
        {
            self.last = Some((command, UseDisposition::Rejected));
        }
    }

    pub(super) fn finish_cube(&mut self, command: u64, result: UseDisposition) -> bool {
        let Some(pending) = self.cube_pending.as_ref() else {
            return false;
        };
        if pending.request.command_id != command {
            return false;
        }
        let Some(touched) = pending.touched() else {
            return false;
        };
        self.unspendable.retain(|item| !touched.contains(item));
        self.cube_pending = None;
        // An old receipt may settle its own reservation, never a later command's result.
        if self.last.is_none_or(|(last, _)| last <= command) {
            self.in_flight_since_ms = None;
            self.last = Some((command, result));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(command: u64) -> ItemUseRequest {
        let definition = |key: &str| TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        };
        ItemUseRequest {
            kind: ItemUseKind::DromeCube {
                wire_binding: [9; 32],
            },
            command_id: command,
            content_revision: "content-r1".into(),
            ruleset_revision: "ruleset-r1".into(),
            sim_revision: "sim-r1".into(),
            transaction_id: [3; 16],
            event_id: [4; 16],
            occurred_at_unix_ms: 1,
            server_build_id: "test".into(),
            source_item_instance_id: [1; 16],
            source_definition: definition(DROME_CUBE_KEY),
            source_quantity_before: 1,
            flask: ItemUseFlaskPlan::New {
                item_instance_id: [2; 16],
                definition: definition("oteryn:item.tibia.i36723"),
                placement_ordinal: 1,
            },
        }
    }

    #[test]
    fn unknown_cube_retains_the_exact_request_and_reserves_source_and_output() {
        let mut gate = super::super::super::ItemUseGate::default();
        let request = request(7);
        assert_eq!(gate.prepare_cube(request.clone(), 10), Ok(()));
        gate.mark_cube_unknown(7);
        assert_eq!(gate.unspendable, [[1; 16], [2; 16]]);
        assert_eq!(
            gate.cube_pending.as_ref().map(|pending| &pending.request),
            Some(&request)
        );
        assert_eq!(
            gate.prepare_cube(request.clone(), 10_000),
            Err(UseDisposition::Exhausted)
        );
        assert!(gate.finish_cube(7, UseDisposition::Committed));
        assert!(gate.unspendable.is_empty());
        assert!(gate.cube_pending.is_none());
        assert_eq!(gate.in_flight_since_ms, None);
        let mut next = request;
        next.command_id = 8;
        assert_eq!(
            gate.prepare_cube(next, 10_001),
            Ok(()),
            "no invented Cube cooldown"
        );
    }

    #[test]
    fn historical_receipt_does_not_clear_other_reservations_or_newer_results() {
        let mut gate = super::super::super::ItemUseGate::default();
        assert_eq!(gate.prepare_cube(request(7), 10), Ok(()));
        gate.mark_cube_unknown(7);
        gate.unspendable.push([8; 16]);
        gate.last = Some((8, UseDisposition::Exhausted));
        gate.in_flight_since_ms = Some(20);
        gate.mark_cube_unknown(7);
        assert_eq!(gate.last, Some((8, UseDisposition::Exhausted)));
        assert!(!gate.finish_cube(6, UseDisposition::Committed));
        assert!(gate.finish_cube(7, UseDisposition::Committed));
        assert_eq!(gate.unspendable, [[8; 16]]);
        assert_eq!(gate.in_flight_since_ms, Some(20));
        assert_eq!(gate.last, Some((8, UseDisposition::Exhausted)));
    }

    #[test]
    fn output_reservation_refuses_before_starting_or_replacing_an_attempt() {
        let mut gate = super::super::super::ItemUseGate::default();
        gate.unspendable.push([2; 16]);
        assert_eq!(
            gate.prepare_cube(request(7), 10),
            Err(UseDisposition::StaleState)
        );
        assert_eq!(gate.in_flight_since_ms, None);
        assert!(gate.cube_pending.is_none());
        assert_eq!(gate.unspendable, [[2; 16]]);
    }
}
