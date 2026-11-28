#[path = "cyclopedia_character/gallery.rs"]
mod gallery;
#[path = "cyclopedia_character/holdings.rs"]
mod holdings;
// PAR-P12 Cyclopedia character tabs on the admitted session owner (capability 199).
use super::{AdmissionRetryBudget, ComposedFreshAdmission, connection::{AdmissionRefusal, AdmittedSession}};
use crate::durability::{DurabilityError, character_authority::CharacterAuthorityError};
use oteryn_protocol_oteryn::cyclopedia_character as wire;

macro_rules! authority_methods {
    () => {
        fn cyclopedia_character_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_id: u64,
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
            command_id: u64,
            command_type: u32,
            payload: &[u8],
        ) -> Result<Vec<u8>, ()> {
            self.cyclopedia_character_wire(admitted, command_id, command_type, payload)
                .await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn cyclopedia_movement_speed(
        &self,
        admitted: AdmittedSession,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Option<wire::MovementSpeed> {
        let actor = admitted.runtime_actor?;
        let session = admitted.game_session_id;
        let equipment = self.observe_movement_equipment(actor, session).await?;
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, session)?;
        let delta = equipment.observed_delta(&runtime, &states, actor, fence)?;
        let current = super::actor_spell::actor_movement::spell_state_speed(
            &runtime, state, actor, session, self.owner_now().get(), Some(delta), true,
        )?;
        Some(wire::MovementSpeed {
            current,
            base: u16::try_from(state.owned_base_speed()).ok()?,
        })
    }

    pub(super) async fn cyclopedia_character_wire(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
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
                if payload.first() == Some(&wire::badge_visibility::PREFIX) {
                    let request =
                        wire::badge_visibility::Request::decode(payload).map_err(|_| ())?;
                    self.root
                        .refresh_account_badges(self.character, self.holder, fence, now)
                        .await
                        .map_err(|_| ())?;
                    return self
                        .root
                        .account_badge_visibility(
                            self.character,
                            self.holder,
                            fence,
                            command_id,
                            request,
                        )
                        .await
                        .map_err(|_| ())?
                        .encode()
                        .map_err(|_| ());
                }
                wire::decode_badges_request(payload).map_err(|_| ())?;
                let badges = self
                    .root
                    .refresh_account_badges(self.character, self.holder, fence, now)
                    .await
                    .map_err(|_| ())?;
                wire::encode_badges(&badges).map_err(|_| ())
            }
            wire::blessings_dialog::COMMAND_TYPE_BLESSINGS_HOLDINGS => {
                if !payload.is_empty()
                    || !admitted.continuity.selected_capabilities
                        .contains(wire::blessings_dialog::CAPABILITY_BLESSINGS_DIALOG_V1)
                    || !admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::death_protection::CAPABILITY_DEATH_AND_BLESSINGS_V1)
                {
                    return Err(());
                }
                let holdings = self.root
                    .read_blessings_holdings(self.character, self.holder, fence)
                    .await
                    .map_err(|_| ())?;
                wire::blessings_dialog::encode_holdings(&holdings).ok_or(())
            }
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB => {
                if payload.first() == Some(&wire::blessings_dialog::SELECTOR) {
                    if payload != [wire::blessings_dialog::SELECTOR]
                        || !admitted
                            .continuity
                            .selected_capabilities
                            .contains(wire::blessings_dialog::CAPABILITY_BLESSINGS_DIALOG_V1)
                    {
                        return Err(());
                    }
                    let current = super::character_gameplay_fence::current(
                        self,
                        admitted.game_session_id,
                        false,
                    )
                    .await
                    .ok_or(())?;
                    if current.connection_generation.get()
                        != admitted.continuity.connection_generation
                        || current.character_id != fence.character_id
                    {
                        return Err(());
                    }
                    let premium = self
                        .session_premium_gate(admitted.game_session_id)
                        .await
                        .premium_current();
                    return self
                        .root
                        .read_blessings_dialog(self.character, self.holder, current, premium)
                        .await
                        .map_err(|_| ())?
                        .encode()
                        .ok_or(());
                }
                if payload.first() == Some(&wire::protection::SELECTOR) {
                    if payload != [wire::protection::SELECTOR] {
                        return Err(());
                    }
                    let current = super::character_gameplay_fence::current(
                        self,
                        admitted.game_session_id,
                        false,
                    )
                    .await
                    .ok_or(())?;
                    if current.connection_generation.get()
                        != admitted.continuity.connection_generation
                        || current.character_id != fence.character_id
                    {
                        return Err(());
                    }
                    let premium = self
                        .session_premium_gate(admitted.game_session_id)
                        .await
                        .premium_current();
                    return self
                        .root
                        .read_death_forecast(self.character, self.holder, current, premium)
                        .await
                        .map_err(|_| ())?
                        .encode()
                        .ok_or(());
                }
                if payload.first() == Some(&wire::items::filtered::SELECTOR) {
                    return self.cyclopedia_holdings(admitted, payload).await;
                }
                if payload.first() == Some(&wire::items::SELECTOR) {
                    let offset = wire::items::decode_request(payload).map_err(|_| ())?;
                    return self
                        .root
                        .read_cyclopedia_item_summary(self.character, self.holder, fence, offset)
                        .await
                        .map_err(|_| ())?
                        .encode()
                        .map_err(|_| ());
                }
                if matches!(payload.first(), Some(&wire::gallery::SELECTOR | &wire::gallery::owned::SELECTOR)) {
                    return self.cyclopedia_gallery(admitted, payload).await;
                }
                if payload.first() == Some(&wire::offence::SELECTOR) {
                    let query = wire::offence::Query::decode(payload).map_err(|_| ())?;
                    let target = match query.target {
                        Some(bytes) => {
                            crate::domain::CharacterId::from_bytes(bytes).map_err(|_| ())?
                        }
                        None => fence.character_id,
                    };
                    if target == fence.character_id {
                        self.refresh_cyclopedia_combat(
                            admitted.runtime_actor.ok_or(())?,
                            admitted.game_session_id,
                            Some(fence),
                        )
                        .await?;
                    }
                    return self
                        .root
                        .read_cyclopedia_offence(self.character, self.holder, fence, target, query)
                        .await
                        .map_err(|_| ())?
                        .encode()
                        .map_err(|_| ());
                }
                if payload.first() == Some(&wire::profile::SELECTOR) {
                    let target = match wire::profile::decode_request(payload).map_err(|_| ())? {
                        Some(bytes) => {
                            crate::domain::CharacterId::from_bytes(bytes).map_err(|_| ())?
                        }
                        None => fence.character_id,
                    };
                    if target == fence.character_id {
                        self.root
                            .refresh_account_badges(self.character, self.holder, fence, now)
                            .await
                            .map_err(|_| ())?;
                    }
                    return self
                        .root
                        .read_cyclopedia_profile(self.character, self.holder, fence, target)
                        .await
                        .map_err(|_| ())?
                        .encode()
                        .map_err(|_| ());
                }
                if payload.first() == Some(&wire::achievements::SELECTOR) {
                    let request = wire::achievements::decode_request(payload).map_err(|_| ())?;
                    let target = match request.target {
                        Some(bytes) => {
                            crate::domain::CharacterId::from_bytes(bytes).map_err(|_| ())?
                        }
                        None => fence.character_id,
                    };
                    let (name, facts) = self
                        .root
                        .read_cyclopedia_achievement_facts(
                            self.character,
                            self.holder,
                            fence,
                            target,
                        )
                        .await
                        .map_err(|_| ())?;
                    let page = self
                        .achievements
                        .account_achievements_page(&facts, u32::from(request.page))
                        .map_err(|_| ())?;
                    return wire::achievements::AchievementPage {
                        character: *target.as_bytes(),
                        name,
                        page,
                    }
                    .encode()
                    .map_err(|_| ());
                }
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
                let (view, mut extension) = AdmissionRetryBudget::for_command(self.root)
                    .retry_unavailable(async || {
                        self.root
                            .read_cyclopedia_character_tab_extended(
                                self.character,
                                self.holder,
                                fence,
                                target,
                                request.tab,
                                request.page,
                                now,
                            )
                            .await
                            .map_err(|error| match error {
                                CharacterAuthorityError::Unavailable(DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded) => AdmissionRefusal::Unavailable,
                                _ => AdmissionRefusal::Rejected,
                            })
                    })
                    .await
                    .map_err(|_| ())?;
                // Each extension field reaches only a session that selected its capability.
                if !admitted
                    .continuity
                    .selected_capabilities
                    .contains(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_VOCATION_V1)
                {
                    extension.vocation_id = None;
                }
                if !admitted
                    .continuity
                    .selected_capabilities
                    .contains(wire::CAPABILITY_CYCLOPEDIA_DEATH_DETAILS_V1)
                {
                    extension.deaths = None;
                }
                let mut bytes = view.encode_extended(&extension).map_err(|_| ())?;
                if request.tab == wire::CharacterTab::GeneralStats
                    && admitted.continuity.selected_capabilities
                        .contains(wire::CAPABILITY_CYCLOPEDIA_MOVEMENT_SPEED_V1)
                {
                    let speed = if target == fence.character_id {
                        self.cyclopedia_movement_speed(admitted, fence).await
                    } else {
                        None
                    };
                    wire::MovementSpeed::encode_suffix(speed, &mut bytes);
                }
                Ok(bytes)
            }
            _ => Err(()),
        }
    }
}
