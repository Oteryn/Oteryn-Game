//! Connection-owned dialogs; rule callbacks use their existing fenced owners.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::server_modal::{CAPABILITY_SERVER_MODAL_V1, Dialog};
#[path = "server_modal/owner.rs"]
mod owner;
#[path = "server_modal/store.rs"]
mod store;
pub(super) use owner::ModalHub;

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
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SERVER_MODAL_V1)
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
            return None;
        }
        let root = self.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )?;
        drop(runtime);
        if kind == oteryn_protocol_oteryn::server_modal::COMMAND_TYPE_HIRELING_STORE_SELECTION {
            return store::start(self, admitted, root, payload).await;
        }
        if kind == oteryn_protocol_oteryn::server_modal::COMMAND_TYPE_SERVER_MODAL_ANSWER
            && let Some(reply) = store::answer(self, admitted, root, command_id, payload).await
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
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
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
        ) -> Option<Vec<u8>> {
            self.server_modal_page(admitted, command_id, kind, payload)
                .await
        }
    };
}
pub(super) use composed_methods;
