//! The item-use gate owns one bounded Cube/concoction PREPARE until a known outcome.
use super::*;

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport::item_use) struct PendingCubeUse {
    pub(super) request: ItemUseRequest,
    concoction_binding: Option<[u8; 32]>,
}

impl PendingCubeUse {
    pub(super) fn binding(&self) -> Option<[u8; 32]> {
        match (self.request.kind, self.concoction_binding) {
            (ItemUseKind::DromeCube { wire_binding }, None) => Some(wire_binding),
            (ItemUseKind::Concoction, Some(binding)) => Some(binding),
            _ => None,
        }
    }

    fn touched(&self) -> Option<Vec<[u8; 16]>> {
        let source = self.request.source_item_instance_id;
        match (&self.request.flask, self.request.kind) {
            (ItemUseFlaskPlan::None, ItemUseKind::Concoction) => Some(vec![source]),
            (
                ItemUseFlaskPlan::Stack {
                    item_instance_id, ..
                }
                | ItemUseFlaskPlan::New {
                    item_instance_id, ..
                },
                ItemUseKind::DromeCube { .. },
            ) if *item_instance_id != source => Some(vec![source, *item_instance_id]),
            _ => None,
        }
    }
}

impl super::super::ItemUseGate {
    pub(super) fn prepare_cube(
        &mut self,
        request: ItemUseRequest,
        now_ms: u64,
    ) -> Result<(), UseDisposition> {
        self.prepare_drome(request, None, now_ms)
    }

    pub(super) fn prepare_concoction(
        &mut self,
        request: ItemUseRequest,
        binding: [u8; 32],
        now_ms: u64,
    ) -> Result<(), UseDisposition> {
        self.prepare_drome(request, Some(binding), now_ms)
    }

    fn prepare_drome(
        &mut self,
        request: ItemUseRequest,
        concoction_binding: Option<[u8; 32]>,
        now_ms: u64,
    ) -> Result<(), UseDisposition> {
        if self.cube_pending.is_some() {
            return Err(UseDisposition::Exhausted);
        }
        let pending = PendingCubeUse {
            request,
            concoction_binding,
        };
        let touched = pending.touched().ok_or(UseDisposition::Rejected)?;
        if pending.binding().is_none() {
            return Err(UseDisposition::Rejected);
        }
        if touched.iter().any(|item| self.unspendable.contains(item)) {
            return Err(UseDisposition::StaleState);
        }
        // No Cube cooldown; concoction cooldowns are durable. Reserve before the await,
        // so cancellation cannot lose the exact plan or its source-only burn binding.
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

    #[test]
    fn unknown_concoction_retains_one_source_and_cannot_replace_a_cube() {
        let mut gate = super::super::super::ItemUseGate::default();
        let mut aid = request(7);
        aid.kind = ItemUseKind::Concoction;
        aid.flask = ItemUseFlaskPlan::None;
        aid.source_definition.production_key = "oteryn:item.tibia.i36723".into();
        assert_eq!(
            gate.prepare_cube(aid.clone(), 10),
            Err(UseDisposition::Rejected)
        );
        assert_eq!(gate.prepare_concoction(aid.clone(), [4; 32], 10), Ok(()));
        assert_eq!(gate.unspendable, [[1; 16]]);
        assert_eq!(
            gate.cube_pending.as_ref().and_then(|p| p.binding()),
            Some([4; 32])
        );
        gate.mark_cube_unknown(7);
        assert_eq!(
            gate.prepare_cube(request(8), 10_000),
            Err(UseDisposition::Exhausted)
        );
        gate.unspendable.push([8; 16]);
        assert!(gate.finish_cube(7, UseDisposition::Committed));
        assert_eq!(gate.unspendable, [[8; 16]]);
        assert!(!gate.finish_cube(7, UseDisposition::Committed));
        assert_eq!(gate.prepare_cube(request(8), 10_001), Ok(()));
    }

    #[test]
    fn concoction_rejects_an_output_or_reserved_source_before_start() {
        let mut gate = super::super::super::ItemUseGate::default();
        let mut aid = request(7);
        aid.kind = ItemUseKind::Concoction;
        assert_eq!(
            gate.prepare_concoction(aid.clone(), [4; 32], 10),
            Err(UseDisposition::Rejected)
        );
        aid.flask = ItemUseFlaskPlan::None;
        gate.unspendable.push([1; 16]);
        assert_eq!(
            gate.prepare_concoction(aid, [4; 32], 10),
            Err(UseDisposition::StaleState)
        );
        assert!(gate.cube_pending.is_none());
        assert_eq!(gate.in_flight_since_ms, None);
    }
}
