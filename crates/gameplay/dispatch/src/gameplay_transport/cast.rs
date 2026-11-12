//! S17 native command composition. Account/password checks remain in Platform;
//! current Character/session facts are read from their existing owners.
#[path = "cast/viewer.rs"]
mod viewer;
use super::*;
use oteryn_gameplay_social::cast::{
    Settings,
    access::CastAccessClient,
    service::{CastService, Operation, Reply},
};
use oteryn_protocol_oteryn::cast::Message;
use std::sync::Arc;

pub struct CastRuntimeConfig {
    pub access: Arc<CastAccessClient>,
    pub node_capacity: usize,
}
#[derive(Clone)]
pub struct CastNode {
    pub service: CastService,
    pub access: Arc<CastAccessClient>,
}
macro_rules! authority_methods {
    () => {
        fn cast_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
        fn cast_end(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> impl std::future::Future<Output = ()> {
            async {}
        }
        fn cast_node(&self) -> Option<$crate::gameplay_transport::cast::CastNode> {
            None
        }
        fn cast_refresh(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> impl std::future::Future<Output = ()> {
            async {}
        }
        fn cast_heard(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _line: &oteryn_protocol_oteryn::chat::ChatLine,
        ) -> impl std::future::Future<Output = ()> {
            async {}
        }
        fn cast_effects(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _events: &[oteryn_protocol_oteryn::presentation::PresentationEvent],
        ) -> impl std::future::Future<Output = ()> {
            async {}
        }
        fn cast_live_chat(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> impl std::future::Future<Output = Vec<(u32, Vec<u8>)>> {
            async { Vec::new() }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn cast_command(
            &self,
            admitted: connection::AdmittedSession,
            kind: u32,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.cast_wire(admitted, kind, payload).await
        }
        async fn cast_end(&self, admitted: connection::AdmittedSession) {
            if let (Some(node), Some(controller)) = (self.states.casts.get(), admitted.controller) {
                let _ = node
                    .service
                    .apply(
                        oteryn_gameplay_social::cast::service::Operation::TransportGone(
                            admitted.game_session_id,
                            controller.transport,
                        ),
                    )
                    .await;
            }
        }
        fn cast_node(&self) -> Option<crate::gameplay_transport::cast::CastNode> {
            if !crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES
                .iter()
                .any(|c| c.id == oteryn_protocol_oteryn::cast::CAPABILITY_CAST_VIEW_V1)
            {
                return None;
            }
            self.states.casts.get().cloned()
        }
        async fn cast_refresh(&self, admitted: connection::AdmittedSession) {
            self.cast_projection(admitted).await;
        }
        async fn cast_heard(
            &self,
            admitted: connection::AdmittedSession,
            line: &oteryn_protocol_oteryn::chat::ChatLine,
        ) {
            self.cast_public_line(admitted, line).await;
        }
        async fn cast_effects(
            &self,
            admitted: connection::AdmittedSession,
            events: &[oteryn_protocol_oteryn::presentation::PresentationEvent],
        ) {
            self.cast_public_effects(admitted, events).await;
        }
        async fn cast_live_chat(
            &self,
            admitted: connection::AdmittedSession,
        ) -> Vec<(u32, Vec<u8>)> {
            self.cast_chat_lines(admitted).await
        }
    };
}
pub(super) use composed_methods;
impl ComposedFreshAdmission<'_, '_, '_> {
    async fn cast_facts(
        &self,
        admitted: connection::AdmittedSession,
    ) -> Option<oteryn_gameplay_social::spectate::TargetFacts> {
        // Canary livestream.cpp:486 keeps a live cast while refreshing its picture.
        // NODEBOOT-1: a busy shared holder is not evidence that the caster is gone.
        // All read/install callers use the same bounded gate before deciding Gone.
        AdmissionRetryBudget::for_command(self.root)
            .retry_unavailable(async || {
                self.cast_facts_once(admitted)
                    .await
                    .ok_or(connection::AdmissionRefusal::Unavailable)
            })
            .await
            .ok()
    }
    async fn cast_facts_once(
        &self,
        admitted: connection::AdmittedSession,
    ) -> Option<oteryn_gameplay_social::spectate::TargetFacts> {
        let actor = admitted.runtime_actor?;
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .ok()?;
        let fence = admitted.item_fence?;
        if item_fence_of(current) != Some(fence)
            || current.current_transport() != Some(admitted.controller?.transport)
            || current.session_state() != GameSessionState::Active
        {
            return None;
        }
        let (_, vitals) = self
            .observe_actor_vitals(actor, admitted.game_session_id)
            .await?;
        let runtime = self.runtime.lock().await;
        let control = match runtime.house_observer(actor).ok()? {
            Some((session, _)) if session == admitted.game_session_id => {
                runtime.house_player_control_facts(actor, session)
            }
            Some(_) => return None,
            None => runtime.player_control_facts(actor, admitted.game_session_id),
        };
        if !control.is_ok_and(|f| f.control_loss.is_none()) {
            return None;
        }
        Some(oteryn_gameplay_social::spectate::TargetFacts {
            authority: current,
            alive: vitals.health > 0,
            observable: true,
            house: None,
        })
    }
    pub(in crate::gameplay_transport) async fn cast_wire(
        &self,
        admitted: connection::AdmittedSession,
        kind: u32,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let request = Message::decode(payload).ok()?;
        // Canary livestream.cpp:249: settings use the current caster. NODEBOOT-1
        // additionally bounds retries of these read/install gates while the privacy
        // worker owns the shared holder. The cast operation itself is never retried.
        let budget = AdmissionRetryBudget::for_command(self.root);
        let facts = self.cast_facts(admitted).await?;
        if kind == 503 {
            let change = match request {
                Message::ConsentSet { revision, allowed } => {
                    Some((i64::try_from(revision).ok()?, allowed))
                }
                Message::ConsentQuery => None,
                _ => return None,
            };
            let fence = budget
                .retry_unavailable(async || {
                    character_gameplay_fence::current(self, admitted.game_session_id, false)
                        .await
                        .ok_or(connection::AdmissionRefusal::Unavailable)
                })
                .await
                .ok()?;
            // The Root publishes the cross-worker denial floor and commits
            // only after every media lease is invalidated or expired.
            // NODEBOOT-1: the privacy worker shares this holder. Retry only
            // its explicit busy refusal under the existing command budget.
            let value = budget
                .retry_unavailable(async || {
                    self.root
                        .character_cast_consent(self.character, self.holder, fence, change)
                        .await
                        .map_err(|error| match error {
                            crate::durability::character_progression::CharacterProgressionError::Unavailable(
                                crate::durability::DurabilityError::RootUnavailable,
                            ) => connection::AdmissionRefusal::Unavailable,
                            _ => connection::AdmissionRefusal::Rejected,
                        })
                })
                .await
                .ok()?;
            use crate::durability::cast_consent::CastConsentOutcome as O;
            let consent = match value {
                O::Read(v) | O::Written(v) | O::Unchanged(v) => v,
                O::Conflict(_) => return None,
            };
            if let Some(node) = self.states.casts.get() {
                let _ = node
                    .service
                    .apply(Operation::Consent(fence.character_id, consent))
                    .await;
            }
            return Message::ConsentState {
                revision: consent.revision.try_into().ok()?,
                allowed: consent.allowed,
            }
            .encode()
            .ok();
        }
        if kind == 502 && matches!(request, Message::ControlQuery) {
            let message = match self.states.casts.get() {
                Some(node) => match node
                    .service
                    .apply(Operation::State(admitted.game_session_id))
                    .await
                {
                    Ok(Reply::State(state)) => Message::CasterState {
                        cast_id: state.cast_id,
                        revision: state.revision,
                        epoch: state.epoch,
                        delay: state.delay_seconds,
                        count: state.viewers,
                    },
                    Err(oteryn_gameplay_social::cast::Error::Unavailable) => Message::End,
                    _ => return None,
                },
                None => Message::End,
            };
            return message.encode().ok();
        }
        let node = self.states.casts.get()?;
        let op = match (kind, request) {
            (
                502,
                Message::Start {
                    revision: 0,
                    access_revision,
                    delay,
                    cap,
                    password,
                },
            ) => Operation::Start(
                facts,
                Settings {
                    delay_seconds: delay,
                    viewer_cap: cap,
                    password_required: password,
                    access_revision,
                    chat_enabled: true,
                },
                self.activity_runtimes.account_for(admitted)?,
            ),
            (
                502,
                Message::Update {
                    revision,
                    access_revision,
                    delay,
                    cap,
                    password,
                },
            ) => Operation::Update(
                facts,
                revision,
                Settings {
                    delay_seconds: delay,
                    viewer_cap: cap,
                    password_required: password,
                    access_revision,
                    chat_enabled: true,
                },
            ),
            (504, Message::Reply(text)) => Operation::Reply(facts, text),
            (502, Message::Stop { revision }) => Operation::Stop(facts, revision),
            (
                505,
                Message::Moderate {
                    revision,
                    subject,
                    action,
                    ..
                },
            ) => {
                let id = oteryn_gameplay_social::cast::ObserverId::from_handle(subject);
                match action {
                    1 => Operation::Mute(facts, revision, id, true),
                    2 => Operation::Mute(facts, revision, id, false),
                    3 => Operation::Kick(facts, revision, id),
                    4 => Operation::Ban(facts, revision, id),
                    _ => return None,
                }
            }
            _ => return None,
        };
        match node.service.apply(op).await.ok()? {
            Reply::State(state) => Message::CasterState {
                cast_id: state.cast_id,
                revision: state.revision,
                epoch: state.epoch,
                delay: state.delay_seconds,
                count: state.viewers,
            }
            .encode()
            .ok(),
            Reply::Applied if kind == 502 => Message::End.encode().ok(),
            Reply::Applied => {
                let Reply::State(state) = node
                    .service
                    .apply(Operation::State(admitted.game_session_id))
                    .await
                    .ok()?
                else {
                    return None;
                };
                Message::CasterState {
                    cast_id: state.cast_id,
                    revision: state.revision,
                    epoch: state.epoch,
                    delay: state.delay_seconds,
                    count: state.viewers,
                }
                .encode()
                .ok()
            }
            _ => None,
        }
    }
    pub(in crate::gameplay_transport) async fn cast_chat_lines(
        &self,
        admitted: connection::AdmittedSession,
    ) -> Vec<(u32, Vec<u8>)> {
        let Some(node) = self.states.casts.get() else {
            return Vec::new();
        };
        if node
            .service
            .apply(Operation::State(admitted.game_session_id))
            .await
            .is_err()
        {
            return Vec::new();
        }

        if self.cast_facts(admitted).await.is_none() {
            let _ = node
                .service
                .apply(Operation::Gone(admitted.game_session_id))
                .await;
            return Vec::new();
        }
        let Ok(Reply::Chat(mut lines)) = node
            .service
            .apply(Operation::CasterChat(admitted.game_session_id))
            .await
        else {
            return Vec::new();
        };
        if let Ok(Reply::Chat(presence)) = node
            .service
            .apply(Operation::CasterPresence(admitted.game_session_id))
            .await
        {
            lines.splice(0..0, presence);
        }
        let mut frames: Vec<_> = lines
            .into_iter()
            .filter_map(|v| {
                Message::CasterLine {
                    handle: v.handle,
                    text: format!("{}: {}", v.name, v.text),
                }
                .encode()
                .ok()
                .map(|payload| (1, payload))
            })
            .collect();
        if let Ok(Reply::Roster(Some((revision, subjects)))) = node
            .service
            .apply(Operation::CasterRoster(admitted.game_session_id))
            .await
            && let Ok(payload) = (Message::CasterRoster { revision, subjects }).encode()
        {
            frames.insert(
                0,
                (
                    oteryn_protocol_oteryn::cast::DELTA_TYPE_CAST_ROSTER,
                    payload,
                ),
            );
        }
        frames
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn cast_projection(
        &self,
        admitted: connection::AdmittedSession,
    ) {
        let Some(node) = self.states.casts.get() else {
            return;
        };
        if node
            .service
            .apply(Operation::State(admitted.game_session_id))
            .await
            .is_err()
        {
            return;
        }

        let Some(facts) = self.cast_facts(admitted).await else {
            let _ = node
                .service
                .apply(Operation::Gone(admitted.game_session_id))
                .await;
            return;
        };
        let Ok(Reply::Capture(permit)) = node.service.apply(Operation::Capture(facts)).await else {
            return;
        };
        let result = match self.cast_map(admitted).await {
            Some((viewport, media)) => self
                .cast_projection_value(admitted, viewport)
                .await
                .map(|(visible, scene, consents, _)| (visible, scene, consents, media)),
            None => None,
        };
        // Current native transport, lease and scope are checked again after every
        // async preference/visibility read; a stale snapshot cannot revive a cast.
        let Some(current) = self.cast_facts(admitted).await else {
            let _ = node
                .service
                .apply(Operation::Gone(admitted.game_session_id))
                .await;
            return;
        };
        if current.authority != facts.authority {
            let _ = node
                .service
                .apply(Operation::Gone(admitted.game_session_id))
                .await;
            return;
        }
        if let Some((visible, scene, consents, media)) = result {
            let _ = node
                .service
                .apply(Operation::PublishView(
                    permit, current, visible, scene, media, consents,
                ))
                .await;
        } else {
            let _ = node.service.apply(Operation::Slate(current)).await;
        }
    }

    async fn cast_projection_value(
        &self,
        admitted: connection::AdmittedSession,
        viewport: viewer::Viewport,
    ) -> Option<(
        Vec<domain::CharacterId>,
        oteryn_gameplay_social::cast::Scene,
        Vec<(
            domain::CharacterId,
            crate::durability::cast_consent::CastConsent,
        )>,
        Vec<world_spatial::EntityRef>,
    )> {
        use world_spatial::EntityKind;
        let actor = admitted.runtime_actor?;
        let channel = self.observe_channel_entities(actor).await?;
        let mut selected = world_spatial::SessionVisibility::default()
            .snapshot(&channel, &mut |_| Ok(()))
            .ok()?;
        if !viewport.matches_runtime(selected.actor_position) {
            return None;
        }
        selected.entities.retain(|e| viewport.visible(e.position));
        let players = {
            let runtime = self.runtime.lock().await;
            // Lock order stays runtime -> spell states, without awaiting under
            // the world guard. Unknown current visibility yields a safe slate.
            let states = self.states.spell_states.try_lock().ok()?;
            let now = self.owner_now().get();
            let needed = |entry: ExactActorRef| {
                selected.entities.iter().any(|e| {
                    e.kind == EntityKind::Player
                        && e.entity.identity == entry.placement_identity()
                        && e.entity.generation == entry.actor_local_generation()
                })
            };
            let hidden = |entry, session, house: bool| -> Option<bool> {
                if session == admitted.game_session_id {
                    return Some(false);
                }
                let player = if house {
                    states
                        .actors
                        .iter()
                        .find(|(held, held_session, _)| *held == entry && *held_session == session)
                        .map(|(_, _, state)| state)?
                } else {
                    states.get(&runtime, entry, session)?
                };
                let spell = player.owned_conditions();
                let owned = if house {
                    runtime
                        .house_actor_conditions_at(entry, session, now)
                        .ok()?
                } else {
                    runtime
                        .actor_conditions_at(entry, Some(session), now)
                        .ok()?
                };
                if !spell.accepts_time(now) || !owned.accepts_time(now) {
                    return None;
                }
                Some(
                    player.equipped_invisible()
                        || spell.invisible_at(now)
                        || owned.invisible_at(now),
                )
            };
            let mut players = Vec::new();
            if let Some((session, _)) = runtime.house_observer(actor).ok()? {
                for entry in runtime.house_visible_players(actor, session).ok()? {
                    if !needed(entry.actor) {
                        continue;
                    }
                    let (session, state) = runtime.house_observer(entry.actor).ok()??;
                    players.push((
                        entry.actor,
                        state.lease.character_id(),
                        entry.position,
                        hidden(entry.actor, session, true)?,
                    ));
                }
            } else {
                for (entry, position, session) in runtime.positioned_melee_census().ok()? {
                    if !needed(entry) {
                        continue;
                    }
                    if let Some(session) = session {
                        let lease = runtime.bound_attacker_lease(entry, session).ok()?;
                        players.push((
                            entry,
                            lease.character_id(),
                            position.position(),
                            hidden(entry, session, false)?,
                        ));
                    }
                }
            }
            players
        };
        let origin = selected.actor_position;
        let mut visible = Vec::new();
        let mut actors = Vec::new();
        let mut shown = Vec::new();
        for entity in &selected.entities {
            if entity.kind != EntityKind::Player {
                if viewer::project_actor(&mut actors, entity, origin, false)? {
                    shown.push(entity.entity);
                }
                continue;
            }
            let (_, character, position, hidden) = players.iter().find(|(actor, _, _, _)| {
                actor.placement_identity() == entity.entity.identity
                    && actor.actor_local_generation() == entity.entity.generation
            })?;
            if !viewer::project_actor(&mut actors, entity, origin, *hidden)? {
                continue;
            }
            shown.push(entity.entity);
            // Recheck positions after the async census. An invisibility change
            // cannot turn a stale hidden position into a released public frame.
            if position.x != entity.position.x
                || position.y != entity.position.y
                || position.floor != entity.position.floor
            {
                return None;
            }
            visible.push(domain::CharacterId::from_bytes(*character.as_bytes()).ok()?);
            if visible.len() > 128 {
                return None;
            }
        }
        let consent = self
            .root
            .read_cast_consents(self.holder, &visible)
            .await
            .ok()?;
        Some((
            visible,
            oteryn_gameplay_social::cast::Scene {
                tiles: Vec::new(),
                actors,
            },
            consent,
            shown,
        ))
    }
}

impl CastRuntimeConfig {
    /// Explicit operator opt-in. Protocol offers remain off until consumer and
    /// shared-host qualification are accepted; configuring a worker grants no capability.
    pub fn from_env(node: [u8; 16]) -> Result<Option<Self>, &'static str> {
        match std::env::var("OTERYN_CAST_ENABLED").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("false") => return Ok(None),
            Ok("true") => {}
            _ => return Err("invalid OTERYN_CAST_ENABLED"),
        }
        use oteryn_gameplay_node_files::secure_file::{FileClass, effective_uid, read_checked};
        let url =
            std::env::var("OTERYN_CAST_PLATFORM_URL").map_err(|_| "missing cast Platform URL")?;
        let capacity = std::env::var("OTERYN_CAST_NODE_CAPACITY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| (1..=4096).contains(n))
            .ok_or("invalid cast node capacity")?;
        let identity =
            std::env::var_os("OTERYN_CAST_IDENTITY_FILE").ok_or("missing cast identity file")?;
        let ca = std::env::var_os("OTERYN_CAST_PLATFORM_CA_FILE")
            .ok_or("missing cast Platform CA file")?;
        let identity = read_checked(
            std::path::Path::new(&identity),
            FileClass::Secret,
            effective_uid(),
            65536,
        )
        .map_err(|_| "invalid cast identity file")?;
        let ca = read_checked(
            std::path::Path::new(&ca),
            FileClass::Trusted,
            effective_uid(),
            65536,
        )
        .map_err(|_| "invalid cast Platform CA file")?;
        let access = CastAccessClient::new(&url, node, &identity, &ca)
            .map_err(|_| "invalid cast mTLS configuration")?;
        Ok(Some(Self {
            access: Arc::new(access),
            node_capacity: capacity,
        }))
    }
}
