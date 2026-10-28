//! Production House-runtime qualification for bed sleep.

use super::*;
use crate::content::LogicalCell;
use crate::durability::bed_sleep::{BedSleepOwner, BedSleepRequest};
use crate::durability::house_scope_handoff::HouseId;
use crate::gameplay_transport::connection::{UseCommand, UseOutcome};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::bed::BedSleepIntent;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn production_bed_sleep(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: BedSleepIntent,
        target: ItemKey,
    ) -> Result<UseOutcome, crate::durability::DurabilityError> {
        if !self
            .session_premium_gate(command.game_session_id)
            .await
            .premium_current()
        {
            return Ok(UseOutcome::rejected());
        }
        {
            let states = self.states.spell_states.lock().await;
            if pvp_live::logout_blocked(&states, actor, command.game_session_id, self.owner_now()) {
                return Ok(UseOutcome::rejected());
            }
        }
        let ItemKey::MapBase {
            placement_key,
            bundle_digest,
            reset_epoch,
        } = target
        else {
            return Ok(UseOutcome::rejected());
        };
        if bundle_digest
            != self
                .runtime
                .lock()
                .await
                .content_pin()
                .server_artifact_digest()
            || reset_epoch != self.reset_epoch
        {
            return Ok(UseOutcome::rejected());
        }
        let Some(current) =
            character_gameplay_fence::current(self, command.game_session_id, false).await
        else {
            return Ok(UseOutcome::rejected());
        };
        if command.item_fence != Some(current.item_fence()) {
            return Ok(UseOutcome::rejected());
        }
        let (session, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(command.game_session_id)
                .await?;
        let Some(transport) = session.current_transport() else {
            return Ok(UseOutcome::rejected());
        };
        let sex = self
            .root
            .read_character_sex(self.character, current.character_id)
            .await?;
        let Some(final_position) = self
            .final_last_position(command.game_session_id, actor)
            .await
            .filter(|capture| capture.record.rest.is_some())
        else {
            return Ok(UseOutcome::rejected());
        };
        let Some(room) = self.rooms.gate_source_world() else {
            return Ok(UseOutcome::rejected());
        };
        let runtime = self.runtime.lock().await;
        let Ok(position) = runtime.read_actor_position(actor) else {
            return Ok(UseOutcome::rejected());
        };
        let Ok(presence) = crate::spell::house_execution::current_house_presence(
            room,
            &runtime,
            command.game_session_id,
            actor,
            position,
        ) else {
            return Ok(UseOutcome::rejected());
        };
        let Some((target_position, _)) = crate::map::view::placement_of(placement_key) else {
            return Ok(UseOutcome::rejected());
        };
        let target_cell = LogicalCell {
            x: i32::from(target_position.x),
            y: i32::from(target_position.y),
            z: i32::from(target_position.floor),
        };
        let Some(head) = presence.bed_head_at(target_cell) else {
            return Ok(UseOutcome::rejected());
        };
        let actor_position = position.position();
        if actor_position.floor != i16::from(target_position.floor)
            || actor_position.x.abs_diff(i32::from(target_position.x)) > 1
            || actor_position.y.abs_diff(i32::from(target_position.y)) > 1
        {
            return Ok(UseOutcome::rejected());
        }
        let Some(house) = HouseId::new(self.world_id, presence.house_key()) else {
            return Ok(UseOutcome::rejected());
        };
        let owner = BedSleepOwner {
            root: self.root,
            recovery: self.character,
            node: self.holder,
            presence: &presence,
            request: BedSleepRequest {
                fence: current.item_fence(),
                transport,
                command_id: command.command_id,
                house,
                head,
                sex,
                skill: intent.skill,
                final_position: Some(final_position),
            },
        };
        super::bed_sleep::finish_owner(command, intent, owner).await
    }
}
