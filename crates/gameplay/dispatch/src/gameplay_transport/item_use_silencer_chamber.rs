//! Carried chamber USE, using the current actor and the published plateau area.
use super::*;
use crate::durability::item_use::silencer_chamber::PlateauWorld;
use oteryn_gameplay_encounter_rules::raid::source_silencer_plateau::SOURCE_SPECTATOR_AREA;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_silencer_chamber(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        fence: CurrentCharacterItemFence,
        inventory: &ItemUseInventory,
        entry: &BackpackEntry,
    ) -> Result<UseDisposition, UseDisposition> {
        let [from, to] = self
            .silencer_plateau_positions(SOURCE_SPECTATOR_AREA)
            .await
            .ok_or(UseDisposition::NotPossible)?;
        let bundle_digest = self
            .raid_bundle_digest()
            .ok_or(UseDisposition::NotPossible)?;
        let session = command.game_session_id;
        let source = entry.item.item_instance_id;
        let request = ItemUseRequest {
            kind: ItemUseKind::SilencerChamber,
            command_id: command.command_id,
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: entry_chest::SIM_REVISION.to_owned(),
            transaction_id: crate::operator_files::uuid_v7()
                .map_err(|_| UseDisposition::Rejected)?,
            event_id: crate::operator_files::uuid_v7().map_err(|_| UseDisposition::Rejected)?,
            occurred_at_unix_ms: unix_ms().map_err(|_| UseDisposition::Rejected)?,
            server_build_id: SERVER_BUILD_ID.to_owned(),
            source_item_instance_id: source,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask: ItemUseFlaskPlan::None,
        };
        {
            let runtime = self.runtime.lock().await;
            let inside = |position: crate::foundation::MovementLocalPosition| {
                position.floor == from.floor
                    && (from.x..=to.x).contains(&position.x)
                    && (from.y..=to.y).contains(&position.y)
            };
            let position = runtime
                .read_actor_position(actor)
                .map_err(|_| UseDisposition::StaleState)?;
            if !inside(position.position())
                || runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .any(|creature| inside(creature.position))
            {
                return Err(UseDisposition::NotPossible);
            }
            let mut states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, session) {
                return Err(UseDisposition::Rejected);
            }
            let gate = states
                .item_uses
                .gate(&runtime, actor, session)
                .ok_or(UseDisposition::Rejected)?;
            if gate.unspendable.contains(&source) {
                return Err(UseDisposition::StaleState);
            }
            gate.start(request.kind, self.owner_ms())?;
        }

        let commit = self.root.commit_silencer_chamber_use(
            self.character,
            self.holder,
            fence,
            request,
            PlateauWorld {
                reset_epoch: self.reset_epoch,
                bundle_digest,
            },
        );
        let bound = Duration::from_millis(ITEMUSE0_RL_04_AMBIGUOUS_MS);
        let (disposition, ambiguous) = match tokio::time::timeout(bound, commit).await {
            Ok(Ok(_)) => (UseDisposition::Committed, false),
            Ok(Err(ItemUseError::Cooldown)) => (UseDisposition::Exhausted, false),
            Ok(Err(ItemUseError::StaleState)) => (UseDisposition::StaleState, false),
            Ok(Err(ItemUseError::Unavailable(_))) | Err(_) => (UseDisposition::Rejected, true),
            Ok(Err(_)) => (UseDisposition::Rejected, false),
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let reserved = [source];
        if let Some(gate) = states.item_uses.gate(&runtime, actor, session) {
            gate.settle(
                command.command_id,
                disposition,
                if ambiguous { &reserved } else { &[] },
            );
        }
        Ok(disposition)
    }
}
