//! Connection-owned dialogs; rule callbacks use their existing fenced owners.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::server_modal::input::{
    CAPABILITY_SERVER_MODAL_INPUT_V1, decode_answer_selected,
};
use oteryn_protocol_oteryn::server_modal::{CAPABILITY_SERVER_MODAL_V1, Dialog};
#[path = "server_modal/owner.rs"]
mod owner;
#[path = "server_modal/rules.rs"]
mod rules;
#[path = "server_modal/store.rs"]
mod store;
pub(super) use owner::ModalHub;

/// An uncertain rule commit must not become a refusal or a successful answer.
pub enum ServerModalOutcome {
    Reply(Vec<u8>),
    LoggedOut(Vec<u8>),
    Unavailable,
}

fn actor_is_current(
    runtime: &crate::foundation::ChannelRuntimeV1,
    admitted: AdmittedSession,
) -> bool {
    if super::battle_instinct_status::actor_is_current(runtime, admitted) {
        return true;
    }
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return false;
    };
    runtime.binding().world_id() == admitted.world_id
        && runtime.binding().channel_id() == Some(admitted.channel_id)
        && runtime.owner_fence().is_ok()
        && fence.game_session_id == admitted.game_session_id
        && fence.connection_generation.get() == admitted.continuity.connection_generation
        && crate::durability::consume_transform::HouseActionContext::from_native_owner(
            runtime,
            actor,
            admitted.game_session_id,
            fence,
        )
        .is_ok()
}

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_SERVER_MODAL_V1)
        .collect()
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn server_modal_page(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        kind: u32,
        payload: &[u8],
    ) -> Option<ServerModalOutcome> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SERVER_MODAL_V1)
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        if !actor_is_current(&runtime, admitted) {
            return None;
        }
        let root = self.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )?;
        drop(runtime);
        // Capability 490: dialogs carry their input block and an answer carries its value,
        // which is checked against the pending dialog before the V1 answer reaches its owner.
        let answer;
        let payload = if admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SERVER_MODAL_INPUT_V1)
        {
            self.states.server_modals.select_input(root)?;
            if kind == oteryn_protocol_oteryn::server_modal::COMMAND_TYPE_SERVER_MODAL_ANSWER {
                let input = decode_answer_selected(true, payload).ok()?;
                self.states
                    .server_modals
                    .check_answer_input(root, &input)?;
                answer = input.answer.encode().ok()?;
                answer.as_slice()
            } else {
                payload
            }
        } else {
            payload
        };
        if kind == oteryn_protocol_oteryn::server_modal::COMMAND_TYPE_HIRELING_STORE_SELECTION {
            return store::start(self, admitted, root, payload)
                .await
                .map(ServerModalOutcome::Reply);
        }
        if kind == oteryn_protocol_oteryn::server_modal::COMMAND_TYPE_SERVER_MODAL_ANSWER
            && let Some(reply) = store::answer(self, admitted, root, command_id, payload).await
        {
            return reply.map(ServerModalOutcome::Reply);
        }
        if kind == oteryn_protocol_oteryn::server_modal::COMMAND_TYPE_SERVER_MODAL_ANSWER
            && let Some(reply) = rules::answer(self, admitted, root, command_id, payload).await
        {
            return reply;
        }
        let notice = (!self.server_motd.is_empty()).then(|| Dialog {
            id: 1,
            title: "Server notice".into(),
            message: self.server_motd.to_owned(),
            choices: Vec::new(),
            buttons: vec![(1, "Acknowledge".into()), (2, "Cancel".into())],
            cancel_button: Some(2),
            image: None,
        });
        self.states
            .server_modals
            .execute(root, kind, payload, notice)
            .map(ServerModalOutcome::Reply)
    }
}
macro_rules! authority_methods {
    () => {
        fn server_modal_available(&self) -> bool {
            false
        }
        fn server_modal_execute(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<
            Output = Option<$crate::gameplay_transport::server_modal::ServerModalOutcome>,
        > {
            async { None }
        }
        fn server_modal_offer_bed(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _target: oteryn_protocol_oteryn::world_object::UseTarget,
            _resolved: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> impl std::future::Future<Output = Option<$crate::gameplay_transport::connection::UseOutcome>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn server_modal_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
        }
        async fn server_modal_execute(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
            kind: u32,
            payload: &[u8],
        ) -> Option<server_modal::ServerModalOutcome> {
            self.server_modal_page(admitted, command_id, kind, payload)
                .await
        }
        async fn server_modal_offer_bed(
            &self,
            admitted: connection::AdmittedSession,
            target: oteryn_protocol_oteryn::world_object::UseTarget,
            resolved: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> Option<connection::UseOutcome> {
            self.offer_bed_modal(admitted, target, resolved).await
        }
    };
}
pub(super) use composed_methods;
