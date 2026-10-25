use super::*;
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Local reservation observation only; the eventual writer still checks its own fences.
    /// Even an empty book answer follows the current carrier check.
    pub(in crate::gameplay_transport) async fn trade_item_available(
        &self,
        actor: ExactActorRef,
        command: super::super::connection::UseCommand,
        item: Option<ItemKey>,
    ) -> Option<bool> {
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().ok()?;
        let facts = runtime
            .player_control_facts(actor, command.game_session_id)
            .ok()?;
        if facts.control_loss.is_some() {
            return Some(false);
        }
        let mut book = self.states.player_trades.0.lock().ok()?;
        if book
            .live
            .get(&command.game_session_id)
            .is_some_and(|live| live.admitted.item_fence != command.item_fence)
        {
            return Some(false);
        }
        let Some(record) = book.record(command.game_session_id) else {
            return Some(true);
        };
        let reserved = match item {
            Some(ItemKey::Instance(item)) => record.items.contains(&Some(item)),
            None => true,
            _ => false,
        };
        if reserved {
            if record.request.is_none() {
                book.close(&record, ResultCode::Stale)?;
            }
            return Some(false);
        }
        Some(true)
    }
    pub(in crate::gameplay_transport) async fn trade_inventory_deltas(
        &self,
        admitted: AdmittedSession,
        view: &mut super::super::item_view::SessionItemView,
        containers: Option<&mut super::super::container_view::ContainerViewState>,
    ) -> Option<Vec<super::super::item_view::ItemViewDelta>> {
        let _permit = self.spell_lane_permit().await?;
        let actor = admitted.runtime_actor?;
        let inventory =
            super::super::item_move::observe_inventory(self, actor, admitted.game_session_id, view)
                .await?;
        let mut deltas: Vec<_> = view
            .inventory_committed(inventory)
            .ok()?
            .into_iter()
            .collect();
        if let Some(containers) = containers {
            deltas.extend(
                super::super::container_view::refresh::refresh(
                    self,
                    actor,
                    admitted.item_fence,
                    containers,
                    view,
                )
                .await
                .ok()?,
            );
        }
        Some(deltas)
    }
    pub(in crate::gameplay_transport) async fn trade_observation(
        &self,
        session: GameSessionId,
    ) -> Option<(u64, State, bool)> {
        let permit = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        let record = self.states.player_trades.0.lock().ok()?.record(session);
        if let Some(mut record) = record {
            let now = self.owner_now().get() / 1000;
            record.lifecycle.observe(now).ok()?;
            let reason = if matches!(record.lifecycle.phase(), Phase::Closed(_)) {
                Some(ResultCode::Cancelled)
            } else if record.request.is_some() {
                None
            } else {
                let check = async {
                    if record.binding != runtime.binding()
                        || record.digest != runtime.content_pin().server_artifact_digest()
                    {
                        return Err(ResultCode::Stale);
                    }
                    let lives = self.trade_lives(&record).ok_or(ResultCode::Stale)?;
                    let side = usize::from(session == record.sessions[1]);
                    self.trade_pair(&permit, &runtime, &lives, record.commands[side])
                        .await?;
                    let fences = self.trade_fences(&lives).await?;
                    self.trade_unchanged(&record, fences).await?;
                    for (side, item) in record.items.iter().enumerate() {
                        if let Some(item) = item {
                            let leaf = self
                                .root
                                .observe_player_trade_offer(
                                    self.character,
                                    self.holder,
                                    fences[side],
                                    record.commands[side],
                                    *item,
                                    record.digest,
                                )
                                .await
                                .map_err(|_| ResultCode::Rejected)?
                                .map_err(result)?;
                            if record.offers[side].as_ref() != Some(&leaf) {
                                return Err(ResultCode::Stale);
                            }
                            self.trade_projection(&runtime, &leaf)?;
                        }
                    }
                    Ok(())
                }
                .await;
                check.err()
            };
            if let Some(reason) = reason {
                self.states
                    .player_trades
                    .0
                    .lock()
                    .ok()?
                    .close(&record, reason)?;
            }
        }
        let book = self.states.player_trades.0.lock().ok()?;
        let live = book.live.get(&session)?;
        Some((live.revision, live.state.clone(), live.inventory_changed))
    }
}
macro_rules! authority_methods {
    () => {
        fn player_trade_open(&self, _admitted: $crate::gameplay_transport::connection::AdmittedSession) -> impl std::future::Future<Output=Option<()>> { async { None } }
        fn player_trade_command(&self, _admitted: $crate::gameplay_transport::connection::AdmittedSession, _id: u64, _intent: oteryn_protocol_oteryn::player_trade::Intent, _item: Option<$crate::gameplay_transport::item_view::ItemKey>) -> impl std::future::Future<Output=Option<oteryn_protocol_oteryn::player_trade::ResultCode>> { async { Some(oteryn_protocol_oteryn::player_trade::ResultCode::Rejected) } }
        fn player_trade_observation(&self, _session: $crate::foundation::GameSessionId) -> impl std::future::Future<Output=Option<(u64, oteryn_protocol_oteryn::player_trade::State, bool)>> { async { None } }
        fn player_trade_inventory(&self, _admitted: $crate::gameplay_transport::connection::AdmittedSession, _view: &mut $crate::gameplay_transport::item_view::SessionItemView, _containers: Option<&mut $crate::gameplay_transport::container_view::ContainerViewState>) -> impl std::future::Future<Output=Option<Vec<$crate::gameplay_transport::item_view::ItemViewDelta>>> { async { None } }
        fn player_trade_item_available(&self, _actor: $crate::foundation::ExactActorRef, _command: $crate::gameplay_transport::connection::UseCommand, _item: Option<$crate::gameplay_transport::item_view::ItemKey>) -> impl std::future::Future<Output=Option<bool>> { async { Some(true) } }
        fn player_trade_terminalized(&self, _session: $crate::foundation::GameSessionId, _command: u64) {}
        fn player_trade_delivered(&self, _session: $crate::foundation::GameSessionId, _revision: u64) {}
        fn player_trade_disconnected(&self, _session: $crate::foundation::GameSessionId) {}
    };
}
pub(in crate::gameplay_transport) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn player_trade_open(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> Option<()> {
            self.trade_open(admitted).await
        }
        async fn player_trade_command(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            id: u64,
            intent: oteryn_protocol_oteryn::player_trade::Intent,
            item: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> Option<oteryn_protocol_oteryn::player_trade::ResultCode> {
            self.trade_command(admitted, id, intent, item).await
        }
        async fn player_trade_observation(
            &self,
            session: $crate::foundation::GameSessionId,
        ) -> Option<(u64, oteryn_protocol_oteryn::player_trade::State, bool)> {
            self.trade_observation(session).await
        }
        async fn player_trade_inventory(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            view: &mut $crate::gameplay_transport::item_view::SessionItemView,
            containers: Option<&mut $crate::gameplay_transport::container_view::ContainerViewState>,
        ) -> Option<Vec<$crate::gameplay_transport::item_view::ItemViewDelta>> {
            self.trade_inventory_deltas(admitted, view, containers)
                .await
        }
        async fn player_trade_item_available(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            item: Option<$crate::gameplay_transport::item_view::ItemKey>,
        ) -> Option<bool> {
            self.trade_item_available(actor, command, item).await
        }
        fn player_trade_terminalized(
            &self,
            session: $crate::foundation::GameSessionId,
            command: u64,
        ) {
            self.states.player_trades.terminalized(session, command);
        }
        fn player_trade_delivered(
            &self,
            session: $crate::foundation::GameSessionId,
            revision: u64,
        ) {
            self.trade_delivered(session, revision);
        }
        fn player_trade_disconnected(&self, session: $crate::foundation::GameSessionId) {
            self.states.player_trades.cancel(session);
        }
    };
}
pub(in crate::gameplay_transport) use composed_methods;
