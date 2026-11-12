//! Session session commands.
use super::*;

impl<S: SessionStream> Session<S> {
    pub(super) fn check_generation(
        &self,
        envelope: &oteryn_protocol_oteryn::WireEnvelopeView<'_>,
    ) -> Result<(), SessionError> {
        if envelope.connection_generation() != self.connection_generation {
            return Err(SessionError::ConnectionGenerationMismatch {
                expected: self.connection_generation,
                actual: envelope.connection_generation(),
            });
        }
        Ok(())
    }

    pub(super) async fn answer_probe(&mut self, payload: &[u8]) -> Result<(), SessionError> {
        let probe_id = decode_liveness_probe(payload)?;
        if probe_id <= self.last_probe_id {
            return Err(SessionError::ProbeIdNotAdvancing {
                last: self.last_probe_id,
                received: probe_id,
            });
        }
        self.last_probe_id = probe_id;
        let ack = encode_liveness_ack(
            self.connection_generation,
            probe_id,
            self.last_server_sequence,
        )?;
        bounded(
            self.deadline,
            "LivenessAck write",
            write_frame(&mut self.stream, &ack),
        )
        .await
    }

    /// Sends the FND-02 `ClientCommand` type 1 `WORLD_ACTOR_STEP_INTENT` for `direction` and
    /// decodes its `CommandResult` and, when it `Moved`, the one `WORLD_SPATIAL` delta.
    pub async fn step(&mut self, direction: StepDirection) -> Result<StepOutcome, SessionError> {
        self.ensure_usable()?;
        if direction.is_diagonal()
            && !self.is_selected(world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1)
        {
            return Err(SessionError::CapabilityNotSelected {
                capability: world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1,
            });
        }
        let payload = world_spatial::encode_step_intent(direction);
        let outcome = self.exchange_step(&payload).await;
        self.poison_on_error(outcome)
    }

    /// `step`, retried with doubling backoff while the server answers `TooEarly` (capability 13),
    /// at most [`STEP_RETRY_ATTEMPTS`] sends; returns the last outcome.
    pub async fn step_retrying(
        &mut self,
        direction: StepDirection,
    ) -> Result<StepOutcome, SessionError> {
        let mut backoff = STEP_RETRY_INITIAL_BACKOFF;
        let mut attempt = 1;
        loop {
            let outcome = self.step(direction).await?;
            if outcome.disposition != StepDisposition::TooEarly || attempt >= STEP_RETRY_ATTEMPTS {
                return Ok(outcome);
            }
            tokio::time::sleep(backoff).await;
            backoff = backoff.saturating_mul(2);
            attempt += 1;
        }
    }

    /// Sends the FND-02 `ClientCommand` type 2 `USE_INTENT` (USE-WIRE-V1) for the object at
    /// `placement`, naming the overlay revision this client believes current
    /// (`expected_revision`), and decodes its `CommandResult` and, when `Committed`, the one
    /// `WORLD_OBJECT_OVERLAY` delta.
    pub async fn use_object(
        &mut self,
        placement: &[u8],
        expected_revision: u64,
    ) -> Result<UseOutcome, SessionError> {
        self.ensure_usable()?;
        let payload = world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.to_vec(),
            expected_revision,
        })?;
        let outcome = self.exchange_use(&payload).await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 2 `USE_INTENT` with the field 2 item target
    /// (capability 4), which opens a corpse. Refused before anything is sent when capability 4 is
    /// not selected. It returns at its result; the domain 11 and 9 deltas it causes go through
    /// the domain store and the event queue.
    pub async fn use_item(&mut self, handle: ItemHandle) -> Result<UseOutcome, SessionError> {
        self.ensure_usable()?;
        self.ensure_item_view()?;
        let payload = world_object::encode_use_item_intent(handle);
        let outcome = self.exchange_use(&payload).await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 9 `ITEM_MOVE_INTENT` (capability 4) and decodes its
    /// `CommandResult`. Only the capability-4 destination (the main backpack) is encodable: the
    /// codec refuses the others before anything is sent. Every outcome but `Moved` (`Stale`
    /// included) is a normal result that changes no local state.
    pub async fn move_item(
        &mut self,
        intent: &ItemMoveIntent,
    ) -> Result<ItemMoveOutcomeResult, SessionError> {
        self.ensure_usable()?;
        self.ensure_item_view()?;
        let payload = item_view::encode_item_move_intent(intent).map_err(SessionError::ItemView)?;
        let outcome = self.exchange_move(&payload).await;
        self.poison_on_error(outcome)
    }

    pub(super) fn ensure_item_view(&self) -> Result<(), SessionError> {
        if self.is_selected(CAPABILITY_ITEM_VIEW_MOVE_V1) {
            Ok(())
        } else {
            Err(SessionError::CapabilityNotSelected {
                capability: CAPABILITY_ITEM_VIEW_MOVE_V1,
            })
        }
    }

    pub(super) async fn exchange_move(
        &mut self,
        payload: &[u8],
    ) -> Result<ItemMoveOutcomeResult, SessionError> {
        let result = self
            .send_and_read_result(COMMAND_TYPE_ITEM_MOVE_INTENT, payload)
            .await?;
        let outcome = item_view::decode_item_move_result_for(
            self.item_result_payload(&result.payload)?.0,
            item_view::ItemMoveSelection {
                equip_drop: self.is_selected(12),
                container_tree: self.is_selected(14),
                world_interaction: self.is_selected(11),
            },
        )
        .map_err(SessionError::ItemView)?;
        check_status_pairing(
            result.command_id,
            result.status,
            outcome == ItemMoveOutcome::Rejected,
        )?;
        Ok(ItemMoveOutcomeResult {
            command_id: result.command_id,
            status: result.status,
            outcome,
            result_server_sequence: result.server_sequence,
        })
    }

    /// Sends the FND-02 `ClientCommand` type 3 `WORLD_ACTOR_SPELL_CAST_INTENT` for the 1-based
    /// `spell` index, and decodes its `CommandResult` and, when it `Cast`, the one `ACTOR_VITALS`
    /// delta. A `Rejected` result (the closed server gate) is a normal outcome, not an error.
    pub async fn cast_spell(
        &mut self,
        spell: NonZeroU32,
        target: SpellTarget,
        aim_at_target: bool,
    ) -> Result<CastOutcome, SessionError> {
        self.ensure_usable()?;
        let payload = actor_spell::encode_spell_cast_intent(&SpellCastIntent {
            spell,
            target,
            aim_at_target,
        });
        let outcome = self.exchange_cast(&payload).await;
        self.poison_on_error(outcome)
    }

    /// Capability 480 `SPELL_AIM_MODE_V1`: the cast intent with field 5 `aim_mode`. FACING and
    /// TARGET go with the `NONE` target, CURSOR with the tile under the cursor; `aim_at_target`
    /// stays false. Refused before anything is sent when 480 is not selected.
    pub async fn cast_spell_aimed(
        &mut self,
        spell: NonZeroU32,
        aim: AimedTarget,
    ) -> Result<CastOutcome, SessionError> {
        self.ensure_usable()?;
        if !self.is_selected(actor_spell::CAPABILITY_SPELL_AIM_MODE_V1) {
            return Err(SessionError::CapabilityNotSelected {
                capability: actor_spell::CAPABILITY_SPELL_AIM_MODE_V1,
            });
        }
        let (aim_mode, target) = match aim {
            AimedTarget::Facing => (SpellAimMode::Facing, SpellTarget::None),
            AimedTarget::Cursor(position) => {
                (SpellAimMode::Cursor, SpellTarget::Position(position))
            }
            AimedTarget::Target => (SpellAimMode::Target, SpellTarget::None),
        };
        let payload = actor_spell::encode_aimed_spell_cast_intent(&AimedSpellCastIntent {
            intent: SpellCastIntent {
                spell,
                target,
                aim_at_target: false,
            },
            aim_mode: Some(aim_mode),
        })?;
        let outcome = self.exchange_cast(&payload).await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 11 `ATTACK_TARGET_INTENT` (capability 17): attack
    /// `target`, or stop attacking with `None`. Refused before anything is sent when capability
    /// 17 is not selected. Every disposition is a normal outcome; the `ACTOR_COMBAT_STATE` delta
    /// it causes goes through the domain store and the event queue.
    pub async fn attack_target(
        &mut self,
        target: Option<&EntityRef>,
    ) -> Result<AttackOutcome, SessionError> {
        self.ensure_usable()?;
        self.ensure_attack()?;
        let payload = attack::encode_attack_target_intent(target).map_err(SessionError::Attack)?;
        let outcome = self
            .exchange_attack(attack::COMMAND_TYPE_ATTACK_TARGET_INTENT, &payload)
            .await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 12 `FIGHT_MODES_INTENT` (capability 17), like
    /// [`Self::attack_target`].
    pub async fn set_fight_modes(
        &mut self,
        modes: &FightModes,
    ) -> Result<AttackOutcome, SessionError> {
        self.ensure_usable()?;
        self.ensure_attack()?;
        let payload = attack::encode_fight_modes_intent(modes);
        let outcome = self
            .exchange_attack(attack::COMMAND_TYPE_FIGHT_MODES_INTENT, &payload)
            .await;
        self.poison_on_error(outcome)
    }

    pub(super) fn ensure_attack(&self) -> Result<(), SessionError> {
        if self.is_selected(CAPABILITY_ATTACK_V1) {
            Ok(())
        } else {
            Err(SessionError::CapabilityNotSelected {
                capability: CAPABILITY_ATTACK_V1,
            })
        }
    }

    pub(super) async fn exchange_attack(
        &mut self,
        command_type: u32,
        payload: &[u8],
    ) -> Result<AttackOutcome, SessionError> {
        let result = self.send_and_read_result(command_type, payload).await?;
        // The server's rate limit answers `Rejected` with an empty payload: a normal refusal.
        let disposition = if result.status == CommandStatus::Rejected && result.payload.is_empty() {
            AttackIntentDisposition::Rejected
        } else {
            attack::decode_attack_intent_result(&result.payload).map_err(SessionError::Attack)?
        };
        check_status_pairing(
            result.command_id,
            result.status,
            disposition == AttackIntentDisposition::Rejected,
        )?;
        Ok(AttackOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
        })
    }

    /// Sends the FND-02 `ClientCommand` type 23 `LOGOUT_INTENT` (capability 21) and decodes its
    /// `CommandResult`. Refused before anything is sent when capability 21 is not selected.
    /// `IN_FIGHT` and `BUSY` are normal refusals that leave the session playing. After
    /// `ACCEPTED` the server has ended the session and closes the transport, so this session
    /// sends nothing further.
    pub async fn logout(&mut self) -> Result<LogoutOutcome, SessionError> {
        self.ensure_usable()?;
        if !self.is_selected(logout::CAPABILITY_LOGOUT_V1) {
            return Err(SessionError::CapabilityNotSelected {
                capability: logout::CAPABILITY_LOGOUT_V1,
            });
        }
        let outcome = self.exchange_logout().await;
        if matches!(
            outcome,
            Ok(LogoutOutcome {
                result: Some(LogoutResult::Accepted),
                ..
            })
        ) {
            self.unusable = true;
        }
        self.poison_on_error(outcome)
    }

    pub(super) async fn exchange_logout(&mut self) -> Result<LogoutOutcome, SessionError> {
        let result = self
            .send_and_read_result(
                logout::COMMAND_TYPE_LOGOUT_INTENT,
                &logout::encode_logout_intent(),
            )
            .await?;
        let decoded = if result.status == CommandStatus::Rejected && result.payload.is_empty() {
            None
        } else {
            Some(logout::decode_logout_result(&result.payload).map_err(SessionError::Logout)?)
        };
        check_status_pairing(
            result.command_id,
            result.status,
            decoded.is_none_or(LogoutResult::is_rejected),
        )?;
        Ok(LogoutOutcome {
            command_id: result.command_id,
            status: result.status,
            result: decoded,
            result_server_sequence: result.server_sequence,
        })
    }

    /// Sends the FND-02 `ClientCommand` type 13 `CHAT_INTENT` (capability 7) and decodes its
    /// `CommandResult`. Refused before anything is sent when capability 7 is not selected or the
    /// intent breaks a codec bound (empty or oversized text or name). A `Muted` or `Exhausted`
    /// result is a normal outcome that changes no local state; the command returns at its result
    /// and every resulting delta goes through the domain store and the event queue.
    pub async fn chat(&mut self, intent: &ChatIntent) -> Result<ChatOutcome, SessionError> {
        self.ensure_usable()?;
        if !self.is_selected(CAPABILITY_CHAT_V1) {
            return Err(SessionError::CapabilityNotSelected {
                capability: CAPABILITY_CHAT_V1,
            });
        }
        guild_chat::check_intent(intent, &self.selected_capabilities)?;
        let payload = chat::encode_chat_intent(intent)?;
        let outcome = self.exchange_chat(&payload, intent).await;
        self.poison_on_error(outcome)
    }

    pub(super) async fn exchange_chat(
        &mut self,
        payload: &[u8],
        intent: &ChatIntent,
    ) -> Result<ChatOutcome, SessionError> {
        let result = self
            .send_and_read_result(COMMAND_TYPE_CHAT_INTENT, payload)
            .await?;
        let decoded = chat::decode_chat_intent_result(&result.payload)?;
        guild_chat::check_result_for_intent(
            decoded.disposition,
            &self.selected_capabilities,
            intent,
        )?;
        check_status_pairing(
            result.command_id,
            result.status,
            decoded.disposition == ChatDisposition::Rejected,
        )?;
        Ok(ChatOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition: decoded.disposition,
            wait_seconds: decoded.wait_seconds,
            result_server_sequence: result.server_sequence,
        })
    }
}
