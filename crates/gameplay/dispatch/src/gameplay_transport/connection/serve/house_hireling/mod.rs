//! The existing item/NPC owners while the real House scope owns the player.
//! House movement retains native House custody and the existing STEP pacing.
use super::*;
mod reply;
mod snapshot;
pub(super) mod step;
pub(super) use reply::result;

pub(super) struct State {
    actor: ExactActorRef,
    enabled: bool,
    items: Option<SessionItemView>,
    containers: Option<ContainerViewState>,
    visibility: Option<SessionVisibility>,
    appearances: SessionAppearance,
    spatial_revision: u64,
    npc_shown: Option<WireEntityRef>,
    npc_active: bool,
    stash_rows: Option<Vec<[u8; 16]>>,
    stash_revision: u64,
    pacer: StepPacer,
    step_holder: std::sync::Arc<()>,
    step_wait_receipts: bool,
}

impl State {
    pub(super) async fn route<A: FreshAdmissionAuthority>(
        &mut self,
        authority: &A,
        admitted: &mut AdmittedSession,
        command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    ) -> Option<Dispatch> {
        use oteryn_protocol_oteryn::{hireling, stash, store, world_object};
        if !admitted
            .continuity
            .selected_capabilities
            .command_selected(command.command_type)
        {
            return Some(Dispatch::Unregistered);
        }
        if matches!(
            command.command_type,
            store::COMMAND_TYPE_STORE_CATALOGUE_PAGE
                | store::COMMAND_TYPE_STORE_HISTORY_PAGE
                | store::COMMAND_TYPE_STORE_PURCHASE
                | store::COMMAND_TYPE_STORE_INBOX_CLAIM
        ) {
            self.stash_rows = None;
            return Some(routing::store::route(authority, admitted, command).await);
        }
        if !self.enabled {
            return Some(Dispatch::Unregistered);
        }
        if command.command_type != stash::COMMAND_TYPE_STASH_INTENT {
            self.stash_rows = None;
        }
        match command.command_type {
            world_object::COMMAND_TYPE_USE_INTENT => {
                // The House services use carried items/Inbox definitions. Map,
                // creature and rune targets retain their separate scope owners.
                let plain = decode_use_intent_target(
                    admitted.continuity.selected_capabilities.as_slice(),
                    command.payload,
                )
                .is_ok_and(|intent| {
                    matches!(
                        intent.target,
                        UseTarget::Item(_) | UseTarget::ItemByDefinition(_)
                    ) && intent.use_with.is_none()
                        && intent.use_with_map_item.is_none()
                        && intent.use_with_position.is_none()
                });
                if !plain {
                    return Some(Dispatch::Use(UseOutcome::rejected()));
                }
                routing::use_item::route(
                    authority,
                    self.actor,
                    admitted,
                    command,
                    &mut self.items,
                    &mut self.containers,
                    &self.visibility,
                )
                .await
            }
            hireling::COMMAND_TYPE_HIRELING => {
                routing::hireling::route(authority, admitted, command).await
            }
            hireling::management::COMMAND_TYPE_HIRELING_MANAGEMENT => {
                routing::hireling_management::route(authority, admitted, command).await
            }
            oteryn_protocol_oteryn::npc_service::COMMAND_TYPE_NPC_TALK_INTENT => {
                let mut buffered = None;
                let mut relocation = None;
                routing::npc::route(
                    authority,
                    self.actor,
                    admitted,
                    command,
                    &mut buffered,
                    &mut relocation,
                )
                .await
            }
            // Existing native commands: NPC-TRADE-2, BAGS-WIRE-1 and NPC-TRADE-SELLALL-1.
            8 => {
                routing::npc_trade::route(authority, self.actor, admitted, command, &mut self.items)
                    .await
            }
            21 => {
                routing::container_view::route(
                    authority,
                    self.actor,
                    admitted,
                    command,
                    &mut self.containers,
                    &mut self.items,
                )
                .await
            }
            97 => {
                routing::npc_shop::route(authority, self.actor, admitted, command, &mut self.items)
                    .await
            }
            stash::COMMAND_TYPE_STASH_INTENT => {
                routing::stash::route(
                    authority,
                    self.actor,
                    admitted,
                    command,
                    &self.items,
                    &mut self.stash_rows,
                )
                .await
            }
            _ => Some(Dispatch::Unregistered),
        }
    }

    pub(super) async fn effects<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
        dispatch: Dispatch,
    ) -> Result<(), ServeEnd> {
        match dispatch {
            Dispatch::NoLogout | Dispatch::Logout(LogoutResult::InFight { .. }) => {
                let text = if matches!(dispatch, Dispatch::NoLogout) {
                    oteryn_protocol_oteryn::zones_nologout::NO_LOGOUT_MESSAGE
                } else {
                    oteryn_protocol_oteryn::zones_nologout::IN_FIGHT_MESSAGE
                };
                let mut sequence = admitted.continuity.server_sequence;
                zones_nologout_transport::emit(stream, admitted, &mut sequence, text)
                    .await
                    .ok_or(ServeEnd::Disconnected)?;
            }
            Dispatch::Step(
                StepOutcome {
                    moved_to: Some(observation),
                    ..
                },
                _,
            ) if self.visibility.is_none() => {
                let sequence = admitted
                    .continuity
                    .server_sequence
                    .checked_add(1)
                    .ok_or(ServeEnd::Disconnected)?;
                let revision = self
                    .spatial_revision
                    .checked_add(1)
                    .ok_or(ServeEnd::Disconnected)?;
                let payload = encode_world_spatial(&observation);
                let frame = encode_state_delta(
                    admitted.continuity.connection_generation,
                    sequence,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    self.spatial_revision,
                    revision,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &payload,
                )
                .map_err(|_| ServeEnd::Disconnected)?;
                self.spatial_revision = revision;
                admitted.continuity.spatial_revision = revision;
                admitted.continuity.server_sequence = sequence;
                write_frame(stream, &frame)
                    .await
                    .map_err(|_| ServeEnd::Disconnected)?;
            }
            Dispatch::Npc(_, update, changed) => {
                if changed {
                    self.inventory(stream, authority, admitted).await?;
                }
                self.npc(stream, admitted, &update).await?;
            }
            Dispatch::NpcTrade(_, update, deltas) | Dispatch::NpcShop(_, _, update, deltas) => {
                for delta in deltas {
                    self.item_delta(stream, admitted, &delta).await?;
                }
                self.npc(stream, admitted, &update).await?;
            }
            Dispatch::Use(outcome) if outcome.disposition == UseDisposition::Committed => {
                self.inventory(stream, authority, admitted).await?;
                self.vitals(stream, authority, admitted, false).await?;
            }
            Dispatch::UseItem(_, Some(delta)) | Dispatch::ContainerView(_, Some(delta)) => {
                self.item_delta(stream, admitted, &delta).await?;
            }
            Dispatch::Stash(reply) => {
                let revision = self
                    .stash_revision
                    .checked_add(1)
                    .ok_or(ServeEnd::Disconnected)?;
                let sequence = admitted
                    .continuity
                    .server_sequence
                    .checked_add(1)
                    .ok_or(ServeEnd::Disconnected)?;
                let payload = oteryn_protocol_oteryn::stash::encode_stash_reply(&reply)
                    .map_err(|_| ServeEnd::Disconnected)?;
                let frame = encode_state_delta(
                    admitted.continuity.connection_generation,
                    sequence,
                    oteryn_protocol_oteryn::stash::DOMAIN_STASH,
                    self.stash_revision,
                    revision,
                    1,
                    &payload,
                )
                .map_err(|_| ServeEnd::Disconnected)?;
                self.stash_revision = revision;
                admitted.continuity.server_sequence = sequence;
                write_frame(stream, &frame)
                    .await
                    .map_err(|_| ServeEnd::Disconnected)?;
                if reply.outcome == oteryn_protocol_oteryn::stash::StashOutcome::Committed {
                    self.inventory(stream, authority, admitted).await?;
                }
            }
            _ => {}
        }
        self.spatial(stream, authority, admitted).await
    }

    async fn item_delta<S: AsyncWrite + Unpin>(
        &self,
        stream: &mut S,
        admitted: &mut AdmittedSession,
        delta: &ItemViewDelta,
    ) -> Result<(), ServeEnd> {
        let (sequence, frame) = item_view_delta(
            admitted.continuity.connection_generation,
            admitted.continuity.server_sequence,
            delta,
        )
        .ok_or(ServeEnd::Disconnected)?;
        admitted.continuity.server_sequence = sequence;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)
    }

    pub(super) async fn inventory<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
    ) -> Result<(), ServeEnd> {
        if let Some(items) = self.items.as_mut() {
            let mut sequence = admitted.continuity.server_sequence;
            item_use_view_refresh::refresh(
                stream,
                authority,
                self.actor,
                admitted,
                items,
                self.containers.as_mut(),
                &mut sequence,
            )
            .await
            .ok_or(ServeEnd::Disconnected)?;
        }
        Ok(())
    }

    async fn npc<S: AsyncWrite + Unpin>(
        &mut self,
        stream: &mut S,
        admitted: &mut AdmittedSession,
        update: &NpcUpdate,
    ) -> Result<(), ServeEnd> {
        let mut sequence = admitted.continuity.server_sequence;
        let frames = npc_update_frames(
            admitted.continuity.connection_generation,
            &mut sequence,
            &mut admitted.continuity.npc,
            update,
            &mut self.npc_shown,
        )
        .ok_or(ServeEnd::Disconnected)?;
        admitted.continuity.server_sequence = sequence;
        for frame in frames {
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
        }
        Ok(())
    }

    async fn spatial<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
    ) -> Result<(), ServeEnd> {
        if let Some(view) = self.visibility.as_mut() {
            let entities = authority
                .observe_visible_entities(self.actor)
                .await
                .ok_or(ServeEnd::Disconnected)?;
            let sequence = admitted.continuity.server_sequence;
            let (_, revision) = self
                .appearances
                .refresh(
                    stream,
                    authority,
                    self.actor,
                    admitted,
                    view,
                    &entities,
                    self.items.as_mut(),
                    sequence,
                    self.spatial_revision,
                )
                .await
                .ok_or(ServeEnd::Disconnected)?;
            self.spatial_revision = revision;
            admitted.continuity.spatial_revision = revision;
        }
        Ok(())
    }

    async fn vitals<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
        tick: bool,
    ) -> Result<(), ServeEnd> {
        // The existing owner advances/checkpoints gourmet conditions even when
        // the client did not select the optional vitals projection.
        let changed = if tick {
            authority
                .tick_vitals(self.actor, admitted.game_session_id)
                .await
        } else {
            None
        };
        if !admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_ACTOR_VITALS)
            || admitted.continuity.vitals_revision == 0
        {
            return Ok(());
        }
        let current = authority
            .observe_vitals(self.actor, admitted.game_session_id)
            .await;
        if let Some((to, value)) = current
            .or(changed)
            .filter(|(revision, _)| *revision > admitted.continuity.vitals_revision)
        {
            let (sequence, frame) = vitals_delta(
                admitted.continuity.connection_generation,
                admitted.continuity.server_sequence,
                admitted.continuity.vitals_revision,
                to,
                &value,
            )
            .ok_or(ServeEnd::Disconnected)?;
            admitted.continuity.server_sequence = sequence;
            admitted.continuity.vitals_revision = to;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
        }
        Ok(())
    }

    pub(super) async fn tick<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
    ) -> Result<(), ServeEnd> {
        if self.enabled {
            self.vitals(stream, authority, admitted, true).await?;
        }
        if self.npc_active
            && let Some(update) = authority
                .npc_follow(self.actor, admitted.game_session_id, self.npc_shown)
                .await
        {
            self.npc(stream, admitted, &update).await?;
        }
        self.spatial(stream, authority, admitted).await
    }
}
