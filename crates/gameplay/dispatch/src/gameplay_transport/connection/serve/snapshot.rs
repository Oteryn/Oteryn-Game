//! Snapshot payload construction, in the original observation and encoding order.

use super::*;

use crate::gameplay_transport::item_view::ItemViewSnapshotDomain as OwnedDomain;

pub(super) struct InitialSnapshot {
    pub(super) overlay_served: bool,
    pub(super) overlay_revision: u64,
    pub(super) vitals: Option<(u64, ActorVitals)>,
    pub(super) notices_selected: bool,
    pub(super) notices: Option<(u64, Vec<u8>)>,
}

pub(super) async fn initial<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    domains: &mut Vec<OwnedDomain>,
) -> Result<InitialSnapshot, ServeEnd> {
    // USE-WIRE-V1 (#162 5868482467): for a session served domain 2 (a fixture World session,
    // without capability 18), the join/resync snapshot also carries the door's current
    // `WORLD_OBJECT_OVERLAY`, the same code path a reconnect resumes through, so a resumed
    // connection gets the door's current overlay exactly as a fresh join does.
    // MAP-WIRE-1 contract §3: a session that selected capability 18 is served by a bundle World,
    // which carries its object states in domain 17 and never serves domain 2.
    let overlay_served = admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_WORLD_OBJECT_OVERLAY);
    let overlay = authority.observe_world_object_overlay().await;
    match (&overlay, admitted.continuity.overlay_revision) {
        // Overlay state for a session with 18 has no domain 17 form: fail closed rather than
        // send domain 2 or drop the state.
        (Some(_), _) if !overlay_served => {
            return Err(ServeEnd::Disconnected);
        }
        // Absence qualifies the initial empty domain, not a reset of an overlay revision already
        // delivered on this session: refuse an unavailable owner. A session with 18 never holds
        // a revision above 0.
        (None, 1..) => return Err(ServeEnd::Disconnected),
        _ => {}
    }
    // Spell cast §3/§4: the own actor's `ACTOR_VITALS`, when it has them, in the same snapshot.
    let vitals = authority
        .observe_vitals(actor, admitted.game_session_id)
        .await;
    let vitals_payload;
    // ACHIEVEMENT-0 §5: with capability 8, the account's watermark at the current revision. A
    // join, resync or reconnect sends this snapshot and never re-sends a delta. A selected
    // domain whose watermark cannot be read fails closed.
    let notices_selected = admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES);
    let notices = match admitted
        .continuity
        .achievement_notice_revision
        .filter(|_| notices_selected)
    {
        None => None,
        Some(notice_revision) => {
            let watermark = match admitted.controller {
                Some(controller) => {
                    authority
                        .observe_achievement_notices(controller.account_id)
                        .await
                }
                None => None,
            };
            let Some(watermark) = watermark else {
                return Err(ServeEnd::Disconnected);
            };
            Some((
                notice_revision,
                encode_achievement_notices_snapshot(watermark),
            ))
        }
    };
    let overlay_payload;
    let overlay_revision = overlay.as_ref().map_or(0, |entry| entry.revision);
    if overlay_served {
        // A session served domain 2 (without capability 18): an empty overlay is still the
        // registered domain's complete snapshot. Source-qualified maps have no legacy entry-room
        // door; publish no invented placement, while allowing the client to distinguish empty
        // from missing.
        let Ok(bytes) = encode_world_object_overlay_snapshot(overlay.as_slice()) else {
            return Err(ServeEnd::Disconnected);
        };
        overlay_payload = bytes;
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: overlay_revision,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: overlay_payload,
        });
    }
    if let Some((vitals_revision, value)) = &vitals {
        let Ok(bytes) = encode_actor_vitals(value) else {
            return Err(ServeEnd::Disconnected);
        };
        vitals_payload = bytes;
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_ACTOR_VITALS,
            revision: *vitals_revision,
            snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
            payload: vitals_payload,
        });
    }
    Ok(InitialSnapshot {
        overlay_served,
        overlay_revision,
        vitals,
        notices_selected,
        notices,
    })
}

pub(super) struct ItemSnapshot {
    pub(super) party_view: crate::gameplay_transport::party_command::SessionPartyView,
    pub(super) item_view: Option<SessionItemView>,
    pub(super) visibility: Option<SessionVisibility>,
    pub(super) appearances: SessionAppearance,
    pub(super) selected_capabilities: SelectedCapabilities,
}

pub(super) async fn items<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    domains: &mut Vec<OwnedDomain>,
    baseline: WorldSpatialObservation,
    revision: u64,
) -> Result<ItemSnapshot, ServeEnd> {
    let spatial: (u32, Vec<u8>);
    // ITEM-VIEW-1b: with capability 4, domains 9 and 11 above every revision the session has
    // seen, with fresh handles. The revisions advance before the write. An unreadable backpack
    // fails closed; the carried open corpse is shown again only while it is in reach.
    let party_view = crate::gameplay_transport::party_command::SessionPartyView::default();
    let mut item_view = None;
    let item_snapshot;
    if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_CHARACTER_INVENTORY)
    {
        let mut view = SessionItemView::resume(admitted.continuity.item_view);
        if admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_CONTAINER_VIEWS)
        {
            view = view.with_container_tree();
        }
        if admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW)
        {
            view = view.with_map_view();
        }
        if admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ITEM_EQUIP_DROP_V1)
        {
            view = view.with_equip_drop();
        }
        let Some(inventory) =
            item_move::observe_inventory(authority, actor, admitted.game_session_id, &view).await
        else {
            return Err(ServeEnd::Disconnected);
        };
        let reopen = match view.carried_open_corpse() {
            Some(key) => authority.observe_item_target(actor, key).await,
            None => None,
        };
        let snapshot = view.snapshot(inventory, reopen);
        admitted.continuity.item_view = view.continuity();
        let Ok(snapshot) = snapshot else {
            return Err(ServeEnd::Disconnected);
        };
        item_snapshot = snapshot;
        for domain in item_snapshot {
            domains.push(OwnedDomain {
                domain_id: domain.domain_id,
                revision: domain.revision,
                snapshot_type: domain.snapshot_type,
                payload: domain.payload,
            });
        }
        item_view = Some(view);
    }
    // VIS-3 (MOVE-RL-11 §4): with capability 6, domain 1 is the VIS-1 interest set of the
    // session's Channel; without it, the v1 own-actor type. A resume resends it the same way.
    let selected_capabilities = admitted.continuity.selected_capabilities;
    let mut appearances = SessionAppearance::new(selected_capabilities);
    let mut visibility = None;
    if selected_capabilities.contains(CAPABILITY_WORLD_SPATIAL_ENTITIES) {
        let Some(channel) = authority.observe_visible_entities(actor).await else {
            return Err(ServeEnd::Disconnected);
        };
        let mut view = SessionVisibility::with_objects(item_view.is_some())
            .lit(selected_capabilities.contains(CAPABILITY_WORLD_LIGHT_V1));
        let Some(encoded) = appearances
            .initial(
                authority,
                actor,
                admitted,
                &mut view,
                &channel,
                item_view.as_mut(),
            )
            .await
        else {
            return Err(ServeEnd::Disconnected);
        };
        if let Some(item_view) = &item_view {
            admitted.continuity.item_view = item_view.continuity();
        }
        spatial = encoded;
        visibility = Some(view);
    } else {
        spatial = (
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            encode_world_spatial(&baseline),
        );
    }
    domains.insert(
        0,
        OwnedDomain {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision,
            snapshot_type: spatial.0,
            payload: spatial.1,
        },
    );
    Ok(ItemSnapshot {
        party_view,
        item_view,
        visibility,
        appearances,
        selected_capabilities,
    })
}

pub(super) struct SessionSnapshot {
    pub(super) container_views: Option<ContainerViewState>,
    pub(super) quest_log: Option<QuestLogState>,
    pub(super) chat_active: bool,
    pub(super) npc_active: bool,
    pub(super) combat_sent: Option<(u64, ActorCombatState)>,
}

pub(super) async fn sessions<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    domains: &mut Vec<OwnedDomain>,
    item_view: &mut Option<SessionItemView>,
) -> Result<SessionSnapshot, ServeEnd> {
    // BAGS-WIRE-1: with capability 14 (which requires 4), domain 14 with no open view above
    // every revision the session has seen; views close on every reconnect and transfer.
    let mut container_views = None;
    let views_snapshot;
    if let Some(view) = item_view.as_mut().filter(|_| {
        admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_CONTAINER_VIEWS)
    }) {
        let snapshot = view.views_snapshot();
        admitted.continuity.item_view = view.continuity();
        let Ok(snapshot) = snapshot else {
            return Err(ServeEnd::Disconnected);
        };
        views_snapshot = snapshot;
        domains.push(OwnedDomain {
            domain_id: views_snapshot.domain_id,
            revision: views_snapshot.revision,
            snapshot_type: views_snapshot.snapshot_type,
            payload: views_snapshot.payload,
        });
        container_views = Some(ContainerViewState::default());
    }
    // QUEST-LOG-WIRE-1: with capability 16, domain 16 with no view and the tracked quests, above
    // every revision the session has seen; the requested view closes on every reconnect and
    // transfer.
    let mut quest_log = None;
    let quest_log_snapshot: QuestLogDomain;
    if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_QUEST_LOG)
    {
        let observation = authority
            .observe_quest_log(actor, admitted.game_session_id, None)
            .await;
        let Ok((state, domain)) =
            QuestLogState::snapshot(&mut admitted.continuity.quest_log, observation)
        else {
            return Err(ServeEnd::Disconnected);
        };
        quest_log_snapshot = domain;
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_QUEST_LOG,
            revision: quest_log_snapshot.to,
            snapshot_type: SNAPSHOT_TYPE_QUEST_LOG_V1,
            payload: quest_log_snapshot.payload,
        });
        quest_log = Some(state);
    }
    // CHAT-WIRE-1: with capability 7, domain 12 with the open rooms, above every revision the
    // session has seen. Lines are never replayed.
    let chat_snapshot: Vec<u8>;
    let chat_active = admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_CHAT);
    if chat_active {
        let rooms = authority.chat_admit(actor, admitted.game_session_id).await;
        let Some(revision) = admitted.continuity.chat_revision.checked_add(1) else {
            return Err(ServeEnd::Disconnected);
        };
        admitted.continuity.chat_revision = revision;
        let Some((snapshot_type, payload)) =
            crate::gameplay_transport::guild_chat::wire::snapshot(authority, *admitted, rooms)
                .await
        else {
            return Err(ServeEnd::Disconnected);
        };
        chat_snapshot = payload;
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_CHAT,
            revision,
            snapshot_type,
            payload: chat_snapshot,
        });
    }
    // NPC-CMD-1a: with capability 3, domain 7 (no conversation: a fresh snapshot ends it) and
    // domain 8 (a closed window), each above its last revision.
    let npc_snapshots: Option<(Vec<u8>, Vec<u8>)> = if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_NPC_CONVERSATION)
    {
        authority.npc_session_open(admitted.game_session_id).await;
        let (Some(conversation_revision), Some(window_revision)) = (
            admitted.continuity.npc.conversation_revision.checked_add(1),
            admitted.continuity.npc.window_revision.checked_add(1),
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        let (Ok(conversation), Ok(window)) = (
            encode_npc_conversation(&NpcConversation::default()),
            encode_npc_trade_window(&NpcTradeWindow::default()),
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        admitted.continuity.npc = NpcContinuity {
            conversation_revision,
            window_revision,
        };
        Some((conversation, window))
    } else {
        None
    };
    let npc_active = npc_snapshots.is_some();
    if let Some((conversation, window)) = npc_snapshots {
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_NPC_CONVERSATION,
            revision: admitted.continuity.npc.conversation_revision,
            snapshot_type: SNAPSHOT_TYPE_NPC_CONVERSATION_V1,
            payload: conversation,
        });
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_NPC_TRADE_WINDOW,
            revision: admitted.continuity.npc.window_revision,
            snapshot_type: SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1,
            payload: window,
        });
    }
    // ATTACK-1b: with capability 17, domain 10 above every revision the session has seen. The
    // target was cleared by the reconnect or transfer; the in-fight deadline survives it.
    let combat_snapshot = if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_ACTOR_COMBAT_STATE)
    {
        let Some(state) = authority
            .observe_combat_state(actor, admitted.game_session_id)
            .await
        else {
            return Err(ServeEnd::Disconnected);
        };
        let Some((revision, payload)) = admitted.continuity.combat.snapshot(&state) else {
            return Err(ServeEnd::Disconnected);
        };
        Some((revision, payload, state))
    } else {
        None
    };
    let combat_sent = combat_snapshot
        .as_ref()
        .map(|(revision, _, state)| (*revision, *state));
    if let Some((revision, payload, _)) = combat_snapshot {
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_ACTOR_COMBAT_STATE,
            revision,
            snapshot_type: SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
            payload,
        });
    }
    Ok(SessionSnapshot {
        container_views,
        quest_log,
        chat_active,
        npc_active,
        combat_sent,
    })
}

pub(super) struct WorldSnapshot {
    pub(super) world_map: Option<SessionMapView>,
    pub(super) world_clock: WorldClockSource,
    pub(super) world_clock_pacer: WorldClockDeltaPacer,
    pub(super) weather_sent: Option<WorldWeather>,
}

pub(super) async fn world<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    domains: &mut Vec<OwnedDomain>,
    item_view: &mut Option<SessionItemView>,
    selected_capabilities: SelectedCapabilities,
    baseline: WorldSpatialObservation,
) -> Result<WorldSnapshot, ServeEnd> {
    // MAP-WIRE-2: with capability 18 (which requires 4), domain 17 above every revision the
    // session has seen, with fresh map handles. It must fit the single snapshot chunk.
    let mut world_map = None;
    let map_snapshot;
    if let Some(items) = item_view.as_mut().filter(|_| {
        admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW)
    }) {
        let mut view = SessionMapView::starting_after(admitted.continuity.world_map_revision);
        let observed = authority.observe_world_map(actor, &mut view, items).await;
        admitted.continuity.item_view = items.continuity();
        let Some(Ok(Some(MapUpdate::Snapshot(snapshot)))) = observed else {
            return Err(ServeEnd::Disconnected);
        };
        admitted.continuity.world_map_revision = view.revision();
        map_snapshot = snapshot;
        domains.push(OwnedDomain {
            domain_id: map_snapshot.domain_id,
            revision: map_snapshot.revision,
            snapshot_type: map_snapshot.snapshot_type,
            payload: map_snapshot.payload,
        });
        world_map = Some(view);
    }
    // CLOCK-1 (§11.3): with capability 19 (which requires 6), domain 18 at the session's current
    // revision; a resume or transfer sends only this snapshot and never replays a delta.
    let world_clock = WorldClockSource::new(DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS);
    let mut world_clock_pacer = WorldClockDeltaPacer::default();
    let clock_snapshot;
    if selected_capabilities.contains(CAPABILITY_WORLD_LIGHT_V1) {
        let Some((now, reading)) = authority
            .world_clock_utc_ms()
            .and_then(|now| Some((now, world_clock.reading(now)?)))
        else {
            return Err(ServeEnd::Disconnected);
        };
        let Ok(payload) = encode_world_clock(reading) else {
            return Err(ServeEnd::Disconnected);
        };
        world_clock_pacer.reset(reading, now);
        clock_snapshot = payload;
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_WORLD_CLOCK,
            revision: admitted.continuity.world_clock_revision,
            snapshot_type: SNAPSHOT_TYPE_WORLD_CLOCK_V1,
            payload: clock_snapshot,
        });
    }
    // WEATHER-1 (D1026): with capability 26 (which requires 6), domain 22 at the session's current
    // revision; a resume or transfer sends only this full snapshot, never a delta replay, and a
    // lost binding is one full `Unavailable`, never a fictitious preset.
    let mut weather_sent = None;
    let weather_snapshot;
    if selected_capabilities.contains(CAPABILITY_WORLD_WEATHER_V1) {
        let observed = observe_weather(authority, baseline.actor_position);
        let Ok(payload) = encode_world_weather(observed) else {
            return Err(ServeEnd::Disconnected);
        };
        weather_sent = Some(observed);
        weather_snapshot = payload;
        domains.push(OwnedDomain {
            domain_id: STATE_DOMAIN_WORLD_WEATHER,
            revision: admitted.continuity.world_weather_revision,
            snapshot_type: SNAPSHOT_TYPE_WORLD_WEATHER_V1,
            payload: weather_snapshot,
        });
    }
    Ok(WorldSnapshot {
        world_map,
        world_clock,
        world_clock_pacer,
        weather_sent,
    })
}
