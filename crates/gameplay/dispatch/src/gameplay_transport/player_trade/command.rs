use super::super::item_ref_admission::NativeItemWeights;
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn trade_command(
        &self,
        admitted: AdmittedSession,
        id: u64,
        intent: Intent,
        item: Option<ItemKey>,
    ) -> Option<ResultCode> {
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(id).ok()?);
        if !self.states.command_ingress.pending(command) {
            return Some(ResultCode::Rejected);
        }
        let arrival_ms = self.owner_now().get() / 1000;
        {
            let mut book = self.states.player_trades.0.lock().ok()?;
            let live = book.live.get_mut(&admitted.game_session_id)?;
            if live.admitted.item_fence != admitted.item_fence
                || live.admitted.continuity.connection_generation
                    != admitted.continuity.connection_generation
            {
                return Some(ResultCode::Stale);
            }
            if !live.rate.admit(arrival_ms) {
                return Some(ResultCode::Rejected);
            }
            // Refuse conflicts against the retained frozen request before waiting for its
            // lane. In particular a cancel never reconciles and then claims cancellation.
            if book
                .record(admitted.game_session_id)
                .is_some_and(|record| record.request.is_some())
            {
                return Some(ResultCode::PartnerBusy);
            }
        }
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        let (permit, runtime) = loop {
            let permit = self.spell_lane_permit().await?;
            let runtime = self.runtime.lock().await;
            let staged = self
                .states
                .player_trades
                .0
                .lock()
                .ok()?
                .record(admitted.game_session_id)
                .is_some_and(|record| record.staged.is_some());
            if !staged {
                break (permit, runtime);
            }
            drop(runtime);
            drop(permit);
            // Simultaneous accepts wait for the first Foundation terminalization;
            // they never replace its immutable accept or spuriously return BUSY.
            self.states
                .player_trades
                .wait_staged(admitted.game_session_id, deadline)
                .await?;
        };
        let now = self.owner_now().get() / 1000;
        let record = self
            .states
            .player_trades
            .0
            .lock()
            .ok()?
            .record(admitted.game_session_id);
        if let Some(mut current) = record.clone() {
            current.lifecycle.observe(now).ok()?;
            if matches!(current.lifecycle.phase(), Phase::Closed(_)) {
                self.states
                    .player_trades
                    .0
                    .lock()
                    .ok()?
                    .close(&current, ResultCode::Cancelled)?;
                return Some(ResultCode::Stale);
            }
        }
        match intent {
            Intent::Cancel => {
                if let Some(record) = record {
                    if record.request.is_some() {
                        return Some(ResultCode::PartnerBusy);
                    }
                    self.states
                        .player_trades
                        .0
                        .lock()
                        .ok()?
                        .close(&record, ResultCode::Cancelled)?;
                }
                Some(ResultCode::Cancelled)
            }
            Intent::Offer { partner, .. } => {
                let Some(ItemKey::Instance(item)) = item else {
                    return Some(ResultCode::NotTradeable);
                };
                let code = self
                    .trade_offer(
                        &permit, &runtime, admitted, command, partner, item, record, now,
                    )
                    .await
                    .unwrap_or_else(|r| r);
                if code == ResultCode::TooFar {
                    let mut book = self.states.player_trades.0.lock().ok()?;
                    let name = book
                        .live
                        .values()
                        .find(|live| entity(live.actor) == partner)?
                        .name
                        .clone();
                    let live = book.live.get_mut(&admitted.game_session_id)?;
                    live.revision = live.revision.checked_add(1)?;
                    live.state.message = format!("{name} tells you to move closer.");
                }
                Some(code)
            }
            Intent::Accept { trade_id } => {
                let Some(record) = record else {
                    return Some(ResultCode::Stale);
                };
                if record.id != Some(trade_id) {
                    return Some(ResultCode::Stale);
                }
                self.trade_accept(&permit, &runtime, admitted, command, record, now)
                    .await
            }
            Intent::Look {
                trade_id,
                counter_offer,
            } => {
                let Some(record) = record else {
                    return Some(ResultCode::Stale);
                };
                if record.id != Some(trade_id) || record.request.is_some() {
                    return Some(ResultCode::Stale);
                }
                let lives = self.trade_lives(&record)?;
                if let Err(reason) = self.trade_pair(&permit, &runtime, &lives, command).await {
                    let reason = ResultCode::from(reason);
                    self.states
                        .player_trades
                        .0
                        .lock()
                        .ok()?
                        .close(&record, reason)?;
                    return Some(reason);
                }
                let fences = self.trade_fences(&lives).await.ok()?;
                if self.trade_unchanged(&record, fences).await.is_err() {
                    self.states
                        .player_trades
                        .0
                        .lock()
                        .ok()?
                        .close(&record, ResultCode::Stale)?;
                    return Some(ResultCode::Stale);
                }
                let side = usize::from(record.sessions[1] == admitted.game_session_id);
                // A held trade item is at its owner's authoritative runtime position.
                let look_distance = if counter_offer {
                    let positions = lives.each_ref().map(|live| {
                        runtime.read_actor_position(live.actor).map(|p| p.position())
                    });
                    let [Ok(a), Ok(b)] = positions else {
                        return Some(ResultCode::Stale);
                    };
                    i32::try_from(a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)))
                        .unwrap_or(i32::MAX)
                } else {
                    0
                };
                let leaf = record.offers[if counter_offer { 1 - side } else { side }].as_ref()?;
                let (definition, count) = leaf.definition_and_quantity().ok()?;
                let native = self.active_generation?.native_gameplay()?;
                let policy =
                    native.item_policy(&definition.production_key, &definition.revision_ref)?;
                let Some(text) = std::num::NonZeroU32::new(count).and_then(|count| {
                    oteryn_gameplay_items::look::item_text(
                        &policy.record().semantics,
                        count,
                        look_distance,
                    )
                }) else {
                    return Some(ResultCode::Rejected);
                };
                let mut book = self.states.player_trades.0.lock().ok()?;
                let live = book.live.get_mut(&admitted.game_session_id)?;
                live.revision = live.revision.checked_add(1)?;
                live.state.look = text;
                Some(ResultCode::Offered)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn trade_offer(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        admitted: AdmittedSession,
        command: CommandRef,
        partner: EntityRef,
        item: [u8; 16],
        prior: Option<Record>,
        now: u64,
    ) -> Result<ResultCode, ResultCode> {
        let lives = {
            let book = self
                .states
                .player_trades
                .0
                .lock()
                .map_err(|_| ResultCode::Rejected)?;
            let own = book
                .live
                .get(&admitted.game_session_id)
                .ok_or(ResultCode::Rejected)?
                .clone();
            let other = book
                .live
                .values()
                .find(|l| entity(l.actor) == partner)
                .ok_or_else(|| {
                    if runtime
                        .visible_entities()
                        .players
                        .iter()
                        .any(|p| entity(p.actor) == partner)
                    {
                        ResultCode::NotSupported
                    } else {
                        ResultCode::Rejected
                    }
                })?
                .clone();
            if own.actor == other.actor {
                return Err(ResultCode::Rejected);
            }
            if book.index.contains_key(&other.admitted.game_session_id)
                && prior
                    .as_ref()
                    .is_none_or(|r| !r.sessions.contains(&other.admitted.game_session_id))
            {
                return Err(ResultCode::PartnerBusy);
            }
            if !has_capacity(book.records.len(), prior.is_some()) {
                return Err(ResultCode::PartnerBusy);
            }
            [own, other]
        };
        self.trade_pair(permit, runtime, &lives, command)
            .await
            .map_err(ResultCode::from)?;
        let fences = self.trade_fences(&lives).await.map_err(ResultCode::from)?;
        // A new offer after READY retires the old pair and both accepts. Its id can
        // never authorize a later exchange (PLAYER-TRADE0 changes-cancel rule).
        if let Some(old) = &prior {
            if old.request.is_some() {
                return Err(ResultCode::PartnerBusy);
            }
            if old.lifecycle.phase() != Phase::Offered
                || old.sessions[0] == admitted.game_session_id
            {
                self.states
                    .player_trades
                    .0
                    .lock()
                    .map_err(|_| ResultCode::Rejected)?
                    .close(old, ResultCode::Stale)
                    .ok_or(ResultCode::Rejected)?;
                return Err(ResultCode::Stale);
            }
            if old.sessions
                != [
                    lives[1].admitted.game_session_id,
                    lives[0].admitted.game_session_id,
                ]
            {
                return Err(ResultCode::PartnerBusy);
            }
        }
        let digest = runtime.content_pin().server_artifact_digest();
        let leaf = self
            .root
            .observe_player_trade_offer(
                self.character,
                self.holder,
                fences[0],
                command,
                item,
                digest,
            )
            .await
            .map_err(|_| ResultCode::Rejected)?
            .map_err(result)?;
        let projection = self.trade_projection(runtime, &leaf)?;
        let mut backpacks = Vec::new();
        for fence in fences {
            let backpack = self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
                .map_err(|_| ResultCode::Rejected)?
                .ok_or(ResultCode::NoBackpack)?;
            backpacks.push(Some(backpack));
        }
        let record = if let Some(mut old) = prior {
            self.trade_unchanged(&old, [fences[1], fences[0]])
                .await
                .map_err(ResultCode::from)?;
            old.lifecycle
                .counter_offer(command, command, now)
                .map_err(result)?;
            old.offers[1] = Some(leaf);
            old.items[1] = Some(item);
            old.projections[1] = Some(projection);
            old.commands[1] = command;
            old.pair = Some(command);
            old.id = Some(identity(command, b"pair"));
            old
        } else {
            Record {
                sessions: [
                    lives[0].admitted.game_session_id,
                    lives[1].admitted.game_session_id,
                ],
                binding: runtime.binding(),
                digest,
                lifecycle: Trade::offered(command, now),
                offers: [Some(leaf), None],
                items: [Some(item), None],
                backpacks: backpacks.try_into().map_err(|_| ResultCode::Rejected)?,
                projections: [Some(projection), None],
                commands: [
                    command,
                    CommandRef::new(
                        lives[1].admitted.game_session_id,
                        CommandId::new(1).map_err(|_| ResultCode::Rejected)?,
                    ),
                ],
                pair: None,
                id: None,
                accepts: [None, None],
                staged: None,
                request: None,
            }
        };
        let mut book = self
            .states
            .player_trades
            .0
            .lock()
            .map_err(|_| ResultCode::Rejected)?;
        book.publish(&record).ok_or(ResultCode::Rejected)?;
        Ok(ResultCode::Offered)
    }

    async fn trade_accept(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        admitted: AdmittedSession,
        command: CommandRef,
        mut record: Record,
        now: u64,
    ) -> Option<ResultCode> {
        if record.request.is_some() {
            return Some(ResultCode::PartnerBusy);
        }
        let lives = self.trade_lives(&record)?;
        let preflight = async {
            if record.binding != runtime.binding()
                || record.digest != runtime.content_pin().server_artifact_digest()
            {
                return Err(ResultCode::Stale);
            }
            self.trade_pair(permit, runtime, &lives, command)
                .await
                .map_err(ResultCode::from)?;
            let fences = self.trade_fences(&lives).await.map_err(ResultCode::from)?;
            self.trade_unchanged(&record, fences)
                .await
                .map_err(ResultCode::from)?;
            let pair = record.pair.ok_or(ResultCode::Stale)?;
            let native = self
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or(ResultCode::Rejected)?;
            let leaves = self
                .root
                .preflight_player_trade(
                    self.character,
                    self.holder,
                    fences,
                    record.commands,
                    [
                        record.items[0].ok_or(ResultCode::Stale)?,
                        record.items[1].ok_or(ResultCode::Stale)?,
                    ],
                    record.digest,
                    &NativeItemWeights(native),
                )
                .await
                .map_err(|_| ResultCode::Rejected)?
                .map_err(result)?;
            if record.offers != leaves.clone().map(Some) {
                return Err(ResultCode::Stale);
            }
            Ok((fences, leaves, pair))
        }
        .await;
        let (fences, leaves, pair) = match preflight {
            Ok(v) => v,
            Err(reason) => {
                self.states
                    .player_trades
                    .0
                    .lock()
                    .ok()?
                    .close(&record, reason)?;
                return Some(reason);
            }
        };
        let side = usize::from(record.sessions[1] == admitted.game_session_id);
        if record.accepts[side].is_some() {
            return Some(ResultCode::Accepted);
        }
        if record.staged.is_some() {
            return Some(ResultCode::PartnerBusy);
        }
        let participant = TradeParticipant {
            accepted: fences[side],
            command,
            leaf: leaves[side].clone(),
        };
        let accepted = record.lifecycle.accept(side, &pair, command, now).ok()?;
        match accepted {
            Accept::First => {
                // Installed only in CommandIngress::terminalize's mutation closure.
                record.staged = Some((side, participant));
                self.states.player_trades.0.lock().ok()?.publish(&record)?;
                Some(ResultCode::Accepted)
            }
            Accept::Repeated => Some(ResultCode::Accepted),
            Accept::Transfer => {
                let first = record.accepts[1 - side].clone()?;
                let occurred_at_unix_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_millis()
                    .try_into()
                    .ok()?;
                let progression = self
                    .root
                    .read_character_progression(self.character, fences[side].character_id)
                    .await
                    .ok()??;
                let request = PlayerTradeRequest {
                    pair,
                    occurrence_id: identity(command, b"occurrence"),
                    transaction_id: identity(command, b"transaction"),
                    event_id: identity(command, b"event"),
                    occurred_at_unix_ms,
                    participants: [first, participant],
                    content_digest: record.digest,
                    weight_digest: record.digest,
                    node_id: *self.holder.fact().node_id().as_bytes(),
                    node_registration_revision: self.holder.fact().registration_revision(),
                    content_revision: self
                        .active_generation?
                        .identity()
                        .content_revision()
                        .to_owned(),
                    ruleset_revision: progression.context.ruleset,
                    sim_revision: progression.context.simulation,
                };
                record.accepts[side] = Some(request.participants[1].clone());
                record.request = Some(request.clone());
                self.states.player_trades.0.lock().ok()?.publish(&record)?;
                // Freeze in the owner before the first storage await. Cancellation leaves
                // both actor indexes and the identical request available to the lane resolver.
                let native = self.active_generation?.native_gameplay()?;
                let verdict = self
                    .root
                    .exchange_player_trade(
                        permit,
                        self.character,
                        self.holder,
                        [fences[1 - side], fences[side]],
                        &NativeItemWeights(native),
                        request,
                    )
                    .await;
                self.trade_verdict(&record, verdict.ok()?)
            }
        }
    }
    fn trade_verdict(
        &self,
        record: &Record,
        verdict: Result<TradeReceipt, Refusal>,
    ) -> Option<ResultCode> {
        let result = match verdict {
            Ok(TradeReceipt::Committed | TradeReceipt::AlreadyCommitted) => ResultCode::Completed,
            Ok(TradeReceipt::Absent) => return None,
            Err(e) => result(e),
        };
        self.states
            .player_trades
            .0
            .lock()
            .ok()?
            .close(record, result)?;
        Some(result)
    }

    /// Every physical lane acquisition reconciles a parked trade before new work.
    pub(in crate::gameplay_transport) async fn resolve_player_trades(
        &self,
        permit: &SpellLanePermit,
    ) -> bool {
        let record = {
            let Ok(book) = self.states.player_trades.0.lock() else {
                return false;
            };
            book.records.values().find(|r| r.request.is_some()).cloned()
        };
        let Some(record) = record else {
            return true;
        };
        let Some(request) = record.request.clone() else {
            return false;
        };
        let _runtime = self.runtime.lock().await;
        let verdict = match self
            .root
            .reconcile_player_trade(self.character, request.clone())
            .await
        {
            Ok(Ok(TradeReceipt::Absent)) => {
                let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
                    return false;
                };
                let mut fences = request.participants.each_ref().map(|p| p.accepted);
                for fence in &mut fences {
                    if let Ok((current, _)) =
                        super::super::FreshAdmissionStore::from_root(self.root.clone())
                            .current_session_at(fence.game_session_id)
                            .await
                        && let Some(current) = super::super::item_fence_of(current)
                    {
                        *fence = current;
                    }
                }
                self.root
                    .exchange_player_trade(
                        permit,
                        self.character,
                        self.holder,
                        fences,
                        &NativeItemWeights(native),
                        request,
                    )
                    .await
            }
            outcome => outcome,
        };
        let Some(code) = verdict.ok().and_then(|v| self.trade_verdict(&record, v)) else {
            return false;
        };
        let command = record.request.as_ref().map(|r| r.participants[1].command);
        command
            .and_then(|command| self.states.command_ingress.complete_trade(command, code))
            .is_some()
    }
}
