//! USE reads a carried authored document in the connection's existing modal window.
//! Crystal 201189f data-global/startup/tables/chest.lua:2124-2129 grants book 2821.
use super::{AdmittedSession, ComposedFreshAdmission, actor_is_current};
use crate::content::ReferenceItemField::Known;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::UseOutcome;
use oteryn_protocol_oteryn::{server_modal::Dialog, world_object::UseDisposition};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn offer_carried_text_modal(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        instance: [u8; 16],
    ) -> Option<UseOutcome> {
        use oteryn_protocol_oteryn::server_modal::CAPABILITY_SERVER_MODAL_V1;
        if !admitted.continuity.selected_capabilities.contains(CAPABILITY_SERVER_MODAL_V1) {
            return None;
        }
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
        let root = self.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )?;
        let _permit = self.spell_lane_permit().await?;
        let generation = self.active_generation?;
        {
            let runtime = self.runtime.lock().await;
            if !actor_is_current(&runtime, admitted)
                || super::super::item_ref_admission::pinned_index(&runtime, Some(generation)).is_none()
                || !self.states.player_trades.physical_service(admitted.game_session_id)
            {
                return None;
            }
        }
        let (definition, body) = self.root.read_carried_authored_text(
            self.character, self.holder, admitted.item_fence?, command, instance,
        ).await.ok()??;
        let policy = generation.native_gameplay()?
            .item_policy(&definition.production_key, &definition.revision_ref)?;
        let record = policy.record();
        if record.production_definition.family != definition.family
            || !matches!(&record.semantics.readable_writeable, Known(facts)
                if facts.readable == Known(true) && facts.writeable == Known(false))
        {
            return None;
        }
        let Known(presentation) = &record.semantics.presentation else { return None; };
        let Known(title) = &presentation.name else { return None; };
        let runtime = self.runtime.lock().await;
        if !actor_is_current(&runtime, admitted)
            || super::super::item_ref_admission::pinned_index(&runtime, Some(generation)).is_none()
            || self.states.server_messages.connection_root(
                admitted.game_session_id,
                admitted.continuity.connection_generation,
            ) != Some(root)
        {
            return None;
        }
        let queued = self.states.server_modals.queue_text(root, Dialog {
            id: 1,
            title: title.clone(),
            message: body,
            choices: Vec::new(),
            buttons: vec![(1, "Close".into())],
            cancel_button: Some(1),
            image: None,
        }).is_some();
        Some(UseOutcome {
            disposition: if queued { UseDisposition::Committed } else { UseDisposition::Occupied },
            ..UseOutcome::rejected()
        })
    }
}
