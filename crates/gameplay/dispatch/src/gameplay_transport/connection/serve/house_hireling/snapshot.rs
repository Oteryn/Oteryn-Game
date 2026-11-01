//! House-scoped snapshots through the existing item, NPC and appearance codecs.
use super::*;
use crate::gameplay_transport::item_view::ItemViewSnapshotDomain as OwnedDomain;

impl State {
    pub(in crate::gameplay_transport::connection::serve) async fn prepare<
        A: FreshAdmissionAuthority,
    >(
        authority: &A,
        actor: ExactActorRef,
        admitted: &mut AdmittedSession,
    ) -> Result<(Self, Vec<OwnedDomain>), ServeEnd> {
        let selected = admitted.continuity.selected_capabilities;
        let enabled = selected.contains(oteryn_protocol_oteryn::hireling::CAPABILITY_HIRELING_V1);
        let mut items = if selected
            .contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1)
            && (enabled
                || selected.contains(
                    oteryn_protocol_oteryn::house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
                )) {
            let mut view = SessionItemView::resume(admitted.continuity.item_view);
            if selected.contains(CAPABILITY_ITEM_EQUIP_DROP_V1) {
                view = view.with_equip_drop();
            }
            if selected.contains(CAPABILITY_CONTAINER_TREE_V1) {
                view = view.with_container_tree();
            }
            Some(view)
        } else {
            None
        };
        let mut payloads = Vec::new();
        let mut containers = None;
        if let Some(view) = items.as_mut() {
            let inventory =
                item_move::observe_inventory(authority, actor, admitted.game_session_id, view)
                    .await
                    .ok_or(ServeEnd::Disconnected)?;
            payloads.extend(
                view.snapshot(inventory, None)
                    .map_err(|_| ServeEnd::Disconnected)?,
            );
            if selected.contains(CAPABILITY_CONTAINER_TREE_V1) {
                payloads.push(view.views_snapshot().map_err(|_| ServeEnd::Disconnected)?);
                containers = Some(ContainerViewState::default());
            }
            admitted.continuity.item_view = view.continuity();
        }
        let mut appearances = SessionAppearance::new(selected);
        let mut visibility = None;
        let mut spatial_revision = admitted.continuity.spatial_revision;
        if selected.contains(CAPABILITY_WORLD_SPATIAL_ENTITIES) {
            let entities = authority
                .observe_visible_entities(actor)
                .await
                .ok_or(ServeEnd::Disconnected)?;
            let mut view = SessionVisibility::with_objects(items.is_some())
                .lit(selected.contains(CAPABILITY_WORLD_LIGHT_V1));
            let (snapshot_type, payload) = appearances
                .initial(
                    authority,
                    actor,
                    admitted,
                    &mut view,
                    &entities,
                    items.as_mut(),
                )
                .await
                .ok_or(ServeEnd::Disconnected)?;
            spatial_revision = spatial_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            payloads.push(OwnedDomain {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: spatial_revision,
                snapshot_type,
                payload,
            });
            admitted.continuity.spatial_revision = spatial_revision;
            if let Some(items) = &items {
                admitted.continuity.item_view = items.continuity();
            }
            visibility = Some(view);
        } else if selected.domain_selected(STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY) {
            let observation = authority
                .observe(actor)
                .await
                .ok_or(ServeEnd::Disconnected)?;
            spatial_revision = spatial_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            payloads.push(OwnedDomain {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: spatial_revision,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: encode_world_spatial(&observation),
            });
            admitted.continuity.spatial_revision = spatial_revision;
        }
        let npc_active = enabled && selected.domain_selected(STATE_DOMAIN_NPC_CONVERSATION);
        if npc_active {
            // NPC-CMD-1a: reconnect and transfer close the old conversation/window.
            authority.npc_session_open(admitted.game_session_id).await;
            let conversation_revision = admitted
                .continuity
                .npc
                .conversation_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let window_revision = admitted
                .continuity
                .npc
                .window_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            payloads.push(OwnedDomain {
                domain_id: STATE_DOMAIN_NPC_CONVERSATION,
                revision: conversation_revision,
                snapshot_type: SNAPSHOT_TYPE_NPC_CONVERSATION_V1,
                payload: encode_npc_conversation(&NpcConversation::default())
                    .map_err(|_| ServeEnd::Disconnected)?,
            });
            payloads.push(OwnedDomain {
                domain_id: STATE_DOMAIN_NPC_TRADE_WINDOW,
                revision: window_revision,
                snapshot_type: SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1,
                payload: encode_npc_trade_window(&NpcTradeWindow::default())
                    .map_err(|_| ServeEnd::Disconnected)?,
            });
            admitted.continuity.npc = NpcContinuity {
                conversation_revision,
                window_revision,
            };
        }
        if enabled
            && selected.domain_selected(STATE_DOMAIN_ACTOR_VITALS)
            && let Some((revision, vitals)) = authority
                .observe_vitals(actor, admitted.game_session_id)
                .await
        {
            payloads.push(OwnedDomain {
                domain_id: STATE_DOMAIN_ACTOR_VITALS,
                revision,
                snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: encode_actor_vitals(&vitals).map_err(|_| ServeEnd::Disconnected)?,
            });
            admitted.continuity.vitals_revision = revision;
        }
        Ok((
            Self {
                actor,
                enabled,
                items,
                containers,
                visibility,
                appearances,
                spatial_revision,
                npc_shown: None,
                npc_active,
                stash_rows: None,
                stash_revision: 0,
                pacer: StepPacer::resumed(authority.actor_step_deadline(actor).await),
                step_holder: std::sync::Arc::new(()),
                step_wait_receipts: authority.step_wait_receipts(),
            },
            payloads,
        ))
    }

    pub(in crate::gameplay_transport::connection::serve) fn appearance_domain<'a>(
        &'a self,
        domains: &mut Vec<DomainSnapshot<'a>>,
    ) {
        self.appearances.domain(domains);
    }
}
