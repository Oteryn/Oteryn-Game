//! PAR-P12 Cyclopedia character tabs on the admitted session owner (capability 199).
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::cyclopedia_character as wire;

macro_rules! authority_methods {
    () => {
        fn cyclopedia_character_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_type: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Result<Vec<u8>, ()>> {
            async { Err(()) }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn cyclopedia_character_command(
            &self,
            admitted: connection::AdmittedSession,
            command_type: u32,
            payload: &[u8],
        ) -> Result<Vec<u8>, ()> {
            self.cyclopedia_character_wire(admitted, command_type, payload)
                .await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn cyclopedia_character_wire(
        &self,
        admitted: AdmittedSession,
        command_type: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>, ()> {
        let Some(fence) = admitted.item_fence else {
            return Err(());
        };
        if admitted.controller.is_none()
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1)
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return Err(());
        }
        let now = super::character_info::unix_now_ms();
        match command_type {
            wire::COMMAND_TYPE_CYCLOPEDIA_BADGES => {
                wire::decode_badges_request(payload).map_err(|_| ())?;
                let badges = self
                    .root
                    .refresh_account_badges(self.character, self.holder, fence, now)
                    .await
                    .map_err(|_| ())?;
                wire::encode_badges(&badges).map_err(|_| ())
            }
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB => {
                let request = wire::decode_tab_request(payload).map_err(|_| ())?;
                let target = match request.target {
                    Some(bytes) => crate::domain::CharacterId::from_bytes(bytes).map_err(|_| ())?,
                    None => fence.character_id,
                };
                if matches!(
                    request.tab,
                    wire::CharacterTab::Offence
                        | wire::CharacterTab::Defence
                        | wire::CharacterTab::Misc
                ) && target == fence.character_id
                {
                    self.refresh_cyclopedia_combat(
                        admitted.runtime_actor.ok_or(())?,
                        admitted.game_session_id,
                        Some(fence),
                    )
                    .await?;
                }
                self.root
                    .read_cyclopedia_character_tab(
                        self.character,
                        self.holder,
                        fence,
                        target,
                        request.tab,
                        request.page,
                        now,
                    )
                    .await
                    .map_err(|_| ())?
                    .encode()
                    .map_err(|_| ())
            }
            _ => Err(()),
        }
    }
}
