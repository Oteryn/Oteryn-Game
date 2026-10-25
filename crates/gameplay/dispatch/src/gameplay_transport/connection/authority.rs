//! The connection authority contract and production composition.

use super::*;

mod composed;

/// The owning fresh-admission authority. Production composes the real owners;
/// nothing on the transport side can mark a connection admitted.
pub trait FreshAdmissionAuthority {
    mechanic_authority_methods!();
    /// The production session owner retains command truth across connection replacement.
    /// Transport-only authorities may omit the resumable command directory.
    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        None
    }
    fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> impl Future<Output = Result<AdmittedSession, AdmissionRefusal>>;

    /// FND-04B §20 same-session reauthenticated recovery of a lost GameSession. Only an
    /// authority-committed switch returns the resumed session; the transport never infers
    /// one from the attempt.
    fn resume(
        &self,
        _attempt: ResumeAttempt<'_>,
    ) -> impl Future<Output = Result<AdmittedSession, AdmissionRefusal>> {
        async { Err(AdmissionRefusal::Unavailable) }
    }

    /// OPS-SAVE-2: the permit one player command runs under, or `None` once the node is
    /// quiescing for its shutdown save: the command then has no effect and gets a rejected
    /// result. Authorities without a shutdown save never refuse.
    fn enter_input(&self) -> impl Future<Output = Option<InputPermit<'_>>> {
        async { Some(InputPermit::ungated()) }
    }

    /// The capabilities this server offers at fresh admission (CAP-NEG-1). Production offers
    /// the registry's offered set; tests inject theirs.
    fn offered_capabilities(&self) -> &'static [OfferedCapability] {
        PRODUCTION_OFFERED_CAPABILITIES
    }

    /// The admitted actor's current own-actor observation for the initial snapshot, or `None`
    /// when this authority serves no gameplay (transport-only fixtures).
    fn observe(
        &self,
        _actor: ExactActorRef,
    ) -> impl Future<Output = Option<WorldSpatialObservation>> {
        async { None }
    }

    /// VIS-3 (MOVE-RL-11 §4): every entity of the admitted actor's Channel with the actor's own
    /// identity among them, read in one Channel-owner work item, for domain 1 under capability 6.
    /// `None` when it cannot be read: a session that selected capability 6 then fails closed.
    /// An authority that knows no other entity (a fixture) shows the own actor alone, from
    /// [`Self::observe`].
    fn observe_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> impl Future<Output = Option<ChannelEntities>> {
        async move {
            let own = self.observe(actor).await?;
            let identity = actor.placement_identity();
            Some(ChannelEntities {
                content_generation: own.content_generation,
                observer: identity,
                entities: vec![ChannelEntity {
                    kind: VisibleKind::Player,
                    identity,
                    generation: 0,
                    position: own.actor_position,
                    revision: 0,
                }],
                lights: EntityLights::new(),
                observed_at_ms: 0,
            })
        }
    }

    /// The Channel's current `WORLD_OBJECT_OVERLAY` for the join/resync snapshot (USE-WIRE-V1,
    /// #162 5868482467), or `None` when this authority serves no gameplay (transport-only
    /// fixtures).
    fn observe_world_object_overlay(
        &self,
    ) -> impl Future<Output = Option<WorldObjectOverlayEntry>> {
        async { None }
    }

    /// One `WORLD_ACTOR_STEP_INTENT` for the admitted actor, applied by the Channel owner.
    fn step(
        &self,
        _actor: ExactActorRef,
        _direction: StepDirection,
    ) -> impl Future<Output = StepOutcome> {
        async { StepOutcome::rejected() }
    }

    /// SPEED-1: one paced `WORLD_ACTOR_STEP_INTENT`, the step and, only when it moved, its step
    /// duration (CONDITIONS-0 §4.2), which the connection's pacing clock waits for before the
    /// next step runs. Without a duration the clock is unchanged: fixtures that serve no
    /// Movement owner keep this default and are unpaced.
    fn paced_step(
        &self,
        actor: ExactActorRef,
        _session: GameSessionId,
        direction: StepDirection,
    ) -> impl Future<Output = (StepOutcome, Option<std::time::Duration>)> {
        async move { (self.step(actor, direction).await, None) }
    }

    /// USE-AUTOWALK-1: resolve an action's current target and find one bounded adjacent route.
    /// `Ok(None)` leaves unsupported/stale targets to their normal handler; `Err` is no path.
    fn item_walk_route(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _target: super::super::AutoWalkTarget,
    ) -> impl Future<Output = Result<Option<super::super::AutoWalkRoute>, ()>> {
        async { Ok(None) }
    }

    /// One ordinary paced step, atomically refusing a queue whose position revision changed.
    fn item_walk_step(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _expected: crate::foundation::MovementPositionSnapshot,
        _direction: StepDirection,
    ) -> impl Future<
        Output = (
            StepOutcome,
            Option<std::time::Duration>,
            Option<crate::foundation::MovementPositionSnapshot>,
        ),
    > {
        async { (StepOutcome::rejected(), None, None) }
    }

    /// Recheck the queue after its last pacing wait, before retrying the item action.
    fn item_walk_valid(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _expected: crate::foundation::MovementPositionSnapshot,
    ) -> impl Future<Output = bool> {
        async { false }
    }

    /// CHASE-1 (RANGED-0 §8): the server-driven chase step of `actor` due now, taken like
    /// [`Self::paced_step`] and answered the same way; `None` when the chase takes no step.
    /// Fixtures without a chase keep this default.
    fn chase_step(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = Option<(StepOutcome, Option<std::time::Duration>)>> {
        async { None }
    }

    /// CHASE-1 (`RANGED0-RL-03`): whether `actor`'s chase has a due search the Channel budget
    /// has not admitted yet. Fixtures without a chase keep this default.
    fn chase_search_waiting(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = bool> {
        async { false }
    }

    /// CHASE-1 (RANGED-0 §8): the actor-owned pacing deadline of `actor` still ahead, which a
    /// replaced or resumed connection starts from. Fixtures without one keep this default.
    fn actor_step_deadline(
        &self,
        _actor: ExactActorRef,
    ) -> impl Future<Output = Option<tokio::time::Instant>> {
        async { None }
    }

    /// CHASE-1: claim `actor`'s pacing clock for one step of the connection owning `holder`,
    /// atomically with the check; `Some` with when to ask again while it is still held.
    fn begin_actor_step(
        &self,
        _actor: ExactActorRef,
        _holder: &std::sync::Arc<()>,
    ) -> impl Future<Output = Option<tokio::time::Instant>> {
        async { None }
    }

    /// CHASE-1: the step `holder` claimed ended; `actor`'s pacing clock is now due at `due`, or
    /// free.
    fn record_actor_step_deadline(
        &self,
        _actor: ExactActorRef,
        _holder: &std::sync::Arc<()>,
        _due: Option<tokio::time::Instant>,
    ) -> impl Future<Output = ()> {
        async {}
    }

    /// CHASE-1: a client step of `actor` arrived; it wins over the chase, whose route is dropped.
    fn yield_chase_to_client(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = ()> {
        async {}
    }

    /// STEP-PACE-WIRE-1 rollout switch: whether a capability 13 session's step results carry the
    /// `step_wait_ms` receipt. Off by default: the process switch
    /// [`step_wait_receipts_enabled`] (`OTERYN_STEP_WAIT_RECEIPTS=1`). Read once per connection.
    fn step_wait_receipts(&self) -> bool {
        step_wait_receipts_enabled()
    }

    /// CLOCK-1 (§11.2): the UTC time fact the World clock is read at, in milliseconds. `None`
    /// when it cannot be read: a session that selected capability 19 then fails closed.
    fn world_clock_utc_ms(&self) -> Option<u64> {
        crate::world_clock::now_utc_ms()
    }

    /// WEATHER-1 (D1025/D1026/D1448): the weather source, or `None`. With `None` capability 26 is
    /// never offered at fresh admission, and a session that selected it earlier is sent one full
    /// `Unavailable`. The default is `None` (fail closed): only an authority that has resolved
    /// the retained World binding for its World exposes a source (D1027 gate in the production
    /// authority).
    fn world_weather(&self) -> Option<&WorldWeatherSource> {
        None
    }

    fn take_tutorial_notice(
        &self,
        _session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice> {
        None
    }

    /// ITEM-TIMED-1: one bounded timed-item payload; None preserves an unknown commit.
    fn timed_item(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _payload: &[u8],
        _target: Option<ItemKey>,
    ) -> impl Future<Output = Option<Vec<u8>>> {
        async {
            oteryn_protocol_oteryn::timed_item::encode_timed_item_result(
                oteryn_protocol_oteryn::timed_item::TimedItemResult::refused(
                    oteryn_protocol_oteryn::timed_item::TimedDisposition::Rejected,
                ),
            )
            .ok()
        }
    }

    /// One `USE_INTENT` for the admitted actor against a world-object placement (USE-WIRE-V1,
    /// #162 5868482467), applied by the Channel owner.
    ///
    /// `command_id` is the FND-02 CommandId of the `USE` and `item_fence` the admitted session's
    /// [`AdmittedSession::item_fence`]; a chest `USE` keys its fenced MINT by both (C2).
    fn use_object(
        &self,
        _actor: ExactActorRef,
        _use: UseCommand,
        _target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> impl Future<Output = UseOutcome> {
        async { UseOutcome::rejected() }
    }

    /// ITEM-USE-1c: one `USE` of the main backpack direct entry `item` (food or a potion) under
    /// capability 15. `command` is as for [`Self::use_object`].
    fn use_item(
        &self,
        _actor: ExactActorRef,
        _use: UseCommand,
        _item: ItemKey,
    ) -> impl Future<Output = UseOutcome> {
        async { UseOutcome::rejected() }
    }

    /// A timed USE keeps an ambiguous commit open for receipt reconciliation.
    fn use_item_completion(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async move { Some(self.use_item(actor, command, item).await) }
    }

    /// BAGS-USE-1: field 5, resolved by the owner against the complete carried tree.
    fn use_item_by_definition(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _definition: std::num::NonZeroU32,
    ) -> impl Future<Output = UseOutcome> {
        async {
            UseOutcome {
                disposition: UseDisposition::NothingToUse,
                ..UseOutcome::rejected()
            }
        }
    }

    /// MINING-1: original USE is retained for replay before either handle is resolved.
    fn use_pick(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: UseIntent,
        _source: Option<ItemKey>,
        _target: Option<ItemKey>,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async { Some(UseOutcome::rejected()) }
    }

    fn offline_training(
        &self,
        _admitted: AdmittedSession,
        _command: UseCommand,
        _target: ItemKey,
    ) -> impl Future<Output = Option<oteryn_protocol_oteryn::loop_training::TrainingResult>> {
        async { Some(oteryn_protocol_oteryn::loop_training::TrainingResult::REFUSED) }
    }
    fn exercise_training(
        &self,
        _admitted: AdmittedSession,
        _command: UseCommand,
        _intent: oteryn_protocol_oteryn::loop_training::ExerciseIntent,
        _weapon: Option<ItemKey>,
        _dummy: Option<ItemKey>,
    ) -> impl Future<Output = Option<oteryn_protocol_oteryn::loop_training::TrainingResult>> {
        async { Some(oteryn_protocol_oteryn::loop_training::TrainingResult::REFUSED) }
    }
    fn training_activity(
        &self,
        _admitted: AdmittedSession,
        _kind: u32,
    ) -> impl Future<Output = bool> {
        async { true }
    }
    /// BED0 §9: choice from a live map-item handle. An authority without the
    /// house reservation/logout owner refuses; decoding never grants bed access.
    fn sleep_in_bed(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: BedSleepIntent,
        target: ItemKey,
    ) -> impl Future<Output = Result<UseOutcome, crate::durability::DurabilityError>> {
        crate::gameplay_transport::bed_sleep::sleep_with_owner(self, actor, command, intent, target)
    }

    /// The owning House runtime resolves the reciprocal bed, reservation,
    /// Premium/reach/block checks and native sex before supplying this context.
    /// Channel authorities have no House owner and retain the closed default.
    fn prepare_bed_sleep(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: BedSleepIntent,
        _target: ItemKey,
    ) -> impl Future<Output = Option<crate::durability::bed_sleep::BedSleepOwner<'_, '_, '_>>> {
        async { None }
    }

    /// CRAFT-1: use-with a map cooking station. Replay resolves before current handles.
    fn use_craft(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: UseIntent,
        _source: Option<ItemKey>,
        _target: Option<ItemKey>,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async { Some(UseOutcome::rejected()) }
    }

    /// Capability 22's exact direct-entry Rune adapter. Unknown commits keep the command open.
    fn use_rune(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: UseIntent,
        _key: Option<ItemKey>,
        _visible: bool,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async { Some(UseOutcome::rejected()) }
    }

    /// ITEM-VIEW-1b: the admitted Character's main backpack and its direct entries
    /// (`read_character_backpack`) for domain 9, read for the snapshot and after each committed
    /// `USE`. `None` when it cannot be read: a session that selected capability 4 then fails
    /// closed.
    fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<InventoryItems>> {
        async { None }
    }

    /// EQUIP-WIRE-1: the admitted Character's occupied equipment slots for domain 9 under
    /// capability 12, in ascending slot order, read for the snapshot and after each commit that
    /// may change domain 9. `None` when they cannot be read: the session then fails closed.
    fn observe_character_equipment(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<Vec<(EquipmentSlot, ViewItem)>>> {
        async { None }
    }

    /// EQUIP-WIRE-1 replay first: the equipment receipt of this command 9's CommandRef, read
    /// after the TRANSFER replay and before its handle is resolved. `None` when it cannot be
    /// read: the connection ends before the CommandId is sequenced.
    fn committed_equipment_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Option<EquipmentMoveReplay>> {
        async { Some(EquipmentMoveReplay::Absent) }
    }

    /// EQUIP-WIRE-1 (ITEM-MOVE-WIRE-1 §4): one equip or unequip through the equipment owner,
    /// keyed by this command 9's CommandRef. The connection has checked that the source is shown
    /// where the move needs it and that the destination slot is shown free; the owner decides the
    /// rest in one transaction.
    fn move_equipment(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _request: EquipmentMove,
    ) -> impl Future<Output = EquipmentMoveResult> {
        async { EquipmentMoveResult::Refused(EquipmentMoveRefusal::Rejected) }
    }

    /// Depot requests are resolved by the live Channel and fenced native item owner.
    fn depot(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: oteryn_protocol_oteryn::depot::DepotIntent,
        _item: Option<ItemKey>,
    ) -> impl Future<Output = Option<oteryn_protocol_oteryn::depot::DepotReply>> {
        async { None }
    }

    /// ITEM-LOOP-1 replay: the item loop receipt of this command 9's CommandRef, read after the
    /// equipment replay and before its handle is resolved. `None` when it cannot be read: the
    /// connection ends before the CommandId is sequenced.
    fn committed_loop_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Option<LoopMoveReplay>> {
        async { Some(LoopMoveReplay::Absent) }
    }

    /// ITEM-LOOP-1 (CP D1438): one move, merge, drop or pickup of the carried tree through the
    /// item loop owner, keyed by this command 9's CommandRef. The connection has resolved the
    /// handles; the owner checks reach and decides the rest in one transaction.
    fn move_loop_item(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _request: LoopMove,
    ) -> impl Future<Output = LoopMoveResult> {
        async { LoopMoveResult::Refused(LoopMoveRefusal::Rejected) }
    }

    /// ITEM-VIEW-1b: the Channel owner's view of the item a `USE` handle resolved to, with the
    /// actor's position, read together. `None` when it cannot be observed (`STALE_STATE`).
    fn observe_item_target(
        &self,
        _actor: ExactActorRef,
        _target: ItemKey,
    ) -> impl Future<Output = Option<ItemTargetObservation>> {
        async { None }
    }

    fn look_item(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _fence: Option<CurrentCharacterItemFence>,
        _views: &SessionItemView,
        _target: oteryn_protocol_oteryn::item_view::ItemHandle,
    ) -> impl Future<Output = ItemLookResult> {
        async { ItemLookResult::Refused(ItemLookRefusal::NotAvailable) }
    }

    fn look_creature(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _target: WireEntityRef,
    ) -> impl Future<Output = ItemLookResult> {
        async { ItemLookResult::Refused(ItemLookRefusal::NotVisible) }
    }

    /// ITEM-MOVE-1 replay first (WIRE-0 §5): the committed TRANSFER of this command 9, looked
    /// up by its CommandRef alone before its handle is resolved, so a command that committed
    /// before a reconnect answers `MOVED` although its handle is now stale. `Ok(None)` proves
    /// that nothing committed for it.
    fn committed_item_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Result<Option<CommittedItemTransfer>, ItemTransferError>> {
        async { Ok(None) }
    }

    /// ITEM-MOVE-1: the corpse-entry TRANSFER of `entry` out of the open `corpse` into the
    /// Character's inventory (D134), keyed by this command 9's CommandRef. The connection has
    /// checked reach and that the entry is shown in the corpse; the TRANSFER decides the rest.
    fn take_corpse_entry(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _corpse: ItemKey,
        _entry: ItemKey,
    ) -> impl Future<Output = Result<ItemTransferOutcome, GroundPickupError>> {
        async {
            Err(GroundPickupError::Transfer(
                ItemTransferError::AuthorityRejected,
            ))
        }
    }

    /// BAGS-WIRE-1: the Channel owner's view of the container a view command or `USE` names,
    /// with reach decided against the actor's position. `None` when it cannot be observed
    /// (`STALE`); a `USE` then takes the corpse path.
    fn observe_container(
        &self,
        _actor: ExactActorRef,
        _fence: Option<CurrentCharacterItemFence>,
        _target: ItemKey,
    ) -> impl Future<Output = Option<ContainerObservation>> {
        async { None }
    }

    /// PARTY-1: basic session-fenced membership commands and own-state query.
    fn party_command(
        &self,
        _admitted: AdmittedSession,
        _command_id: u64,
        _command_type: u32,
        _payload: &[u8],
        _view: &mut crate::gameplay_transport::party_command::SessionPartyView,
    ) -> impl Future<Output = Option<Vec<u8>>> {
        std::future::ready(None)
    }

    fn staff_command(
        &self,
        _admitted: AdmittedSession,
        _receipt: bool,
        _payload: &[u8],
    ) -> impl Future<Output = crate::gameplay_transport::StaffReply> {
        async { crate::gameplay_transport::StaffReply::Rejected }
    }

    /// QUEST-LOG-WIRE-1: the session's quest copy with the quest log content, unless the copy
    /// still has version `since` (`Unchanged`). `Unavailable` while the copy is not loaded: the
    /// domain then shows nothing new and every query is `REJECTED`.
    fn observe_quest_log(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _since: Option<u64>,
    ) -> impl Future<Output = QuestLogObservation> {
        async { QuestLogObservation::Unavailable }
    }

    /// CHAT-WIRE-1: a session with capability 7 starts chatting. Returns the rooms it has open
    /// (CHAT-2B); the authority loads the speaker name and drops undelivered lines.
    fn chat_admit(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = ChatRoomSet> {
        async { ChatRoomSet::default() }
    }

    /// CHAT-WIRE-1: one `CHAT_INTENT` (command 13) of the admitted actor.
    fn chat_intent(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _command_id: u64,
        _intent: ChatIntent,
    ) -> impl Future<Output = ChatIntentResult> {
        async {
            ChatIntentResult {
                disposition: oteryn_protocol_oteryn::chat::ChatDisposition::ChatUnavailable,
                wait_seconds: 0,
            }
        }
    }

    /// CHAT-2B: the open rooms of the session when an intent changed them since its client was
    /// last shown them (a domain 12 delta type 2); `None` when unchanged.
    fn chat_rooms_changed(
        &self,
        _session: GameSessionId,
    ) -> impl Future<Output = Option<ChatRoomSet>> {
        async { None }
    }

    fn chat_guild_admit(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = oteryn_protocol_oteryn::chat::guild_room::GuildRoomState> {
        std::future::ready(oteryn_protocol_oteryn::chat::guild_room::GuildRoomState::Unavailable)
    }

    fn chat_guild_intent(
        &self,
        _admitted: AdmittedSession,
        _text: String,
    ) -> impl Future<Output = ChatIntentResult> {
        std::future::ready(ChatIntentResult {
            disposition: oteryn_protocol_oteryn::chat::ChatDisposition::Rejected,
            wait_seconds: 0,
        })
    }

    fn chat_next_bound(
        &self,
        admitted: AdmittedSession,
        _remaining: &mut usize,
    ) -> impl Future<Output = Option<ChatLine>> {
        self.chat_next(admitted.game_session_id)
    }

    /// CHAT-WIRE-1: the oldest line queued for the session, if any.
    fn chat_next(&self, _session: GameSessionId) -> impl Future<Output = Option<ChatLine>> {
        async { None }
    }

    /// CHAT-WIRE-1: the real line last returned by `chat_next` was written to the client; it
    /// stops counting against `CHAT0-RL-11`.
    fn chat_delivered(&self, _session: GameSessionId) -> impl Future<Output = ()> {
        async {}
    }

    /// NPC-CMD-1a: a session with capability 3 starts (or restarts) without a conversation.
    fn npc_session_open(&self, _session: GameSessionId) -> impl Future<Output = ()> {
        async {}
    }

    /// NPC-CMD-1a: the domain 7 payload when the conversation differs from the NPC `shown` to
    /// the client: one opened from chat, or ended by walking out of range.
    fn npc_follow(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _shown: Option<WireEntityRef>,
    ) -> impl Future<Output = Option<NpcUpdate>> {
        async { None }
    }

    /// NPC-TRADE-2: one `NPC_TRADE_INTENT` (command 8) of the admitted actor, keyed by its
    /// CommandRef: replay first, then one durable NPC trade.
    fn npc_trade(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: oteryn_protocol_oteryn::npc_service::NpcTradeIntent,
    ) -> impl Future<Output = NpcTradeStep> {
        async {
            NpcTradeStep::Refused(
                crate::gameplay_transport::npc_command::NpcTradeRefusal::only(
                    NpcIntentDisposition::Rejected,
                ),
            )
        }
    }

    /// Reconcile only the original pending CommandRef at connection entry. Replacement wire
    /// bytes cannot cancel accepted Bank work. This is not dispatch of a new client command.
    fn resume_npc_bank(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Option<NpcTalkOutcome>> {
        async { None }
    }

    /// NPC-CMD-1a: one `NPC_TALK_INTENT` (command 7) of the admitted actor.
    fn npc_talk(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: NpcTalkIntent,
    ) -> impl Future<Output = NpcTalkOutcome> {
        async {
            NpcTalkOutcome {
                unknown: false,
                inventory_changed: false,
                disposition: NpcIntentDisposition::Rejected,
                conversation: None,
                window: None,
            }
        }
    }

    /// The admitted actor's current `ACTOR_VITALS` revision and value for the initial snapshot,
    /// or `None` while it has no Character cast facts (spell cast §4).
    fn observe_vitals(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<(u64, ActorVitals)>> {
        async { None }
    }

    /// One `WORLD_ACTOR_SPELL_CAST_INTENT` of the admitted actor, applied by the Channel owner
    /// (spell cast §9 step 2). `command_id` is the FND-02 CommandId the cast is bound to.
    fn cast_spell(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _command_id: u64,
        _intent: SpellCastIntent,
    ) -> impl Future<Output = SpellCastOutcome> {
        async { SpellCastOutcome::rejected() }
    }

    /// None keeps an ambiguously committed original spell outstanding. It is
    /// never encoded as a terminal rejection or advanced to the next CommandId.
    fn cast_spell_completion(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: SpellCastIntent,
    ) -> impl Future<Output = Option<SpellCastOutcome>> {
        async move { Some(self.cast_spell(actor, session, command_id, intent).await) }
    }

    /// One `ACCOUNT_ACHIEVEMENTS_QUERY` page of the request's account's earned facts (display
    /// contract §4). The transport passes the account of the admitted controller, never one from
    /// the payload, with the session and CommandId of the request.
    fn account_achievements(
        &self,
        _request: AccountAchievementsRequest,
    ) -> impl Future<Output = AccountAchievementsReply> {
        async { AccountAchievementsReply::Rejected }
    }

    /// Persist a fenced observation of the reset-selected World announcement.
    /// None leaves a possibly committed result outstanding for reconciliation.
    fn world_event(
        &self,
        _command: UseCommand,
    ) -> impl Future<Output = crate::gameplay_transport::world_event::Reply> {
        async { Some(Err(())) }
    }

    /// RANK-1 (HS-WIRE-1): one `HIGHSCORES_QUERY_V1` page of `world`'s persisted snapshot for
    /// the Character the admitted `session` controls. The transport passes the session's World
    /// and GameSession, never values from the payload.
    fn highscores(
        &self,
        _world: WorldId,
        _session: GameSessionId,
        _query: HighscoresQuery,
    ) -> impl Future<Output = HighscoresReply> {
        async { HighscoresReply::Rejected }
    }

    /// The account's achievement watermark for the `ACCOUNT_ACHIEVEMENT_NOTICES` snapshot
    /// (ACHIEVEMENT-0 §5), read for the admitted controller's account; `None` when it cannot be
    /// read.
    fn observe_achievement_notices(
        &self,
        _account_id: [u8; 16],
    ) -> impl Future<Output = Option<AchievementWatermark>> {
        async { None }
    }

    /// The periodic Serene evaluation of the admitted actor (SPELL-D8 §8.2), run every
    /// [`SERENE_EVALUATION`] while it has `ACTOR_VITALS`: the new revision and value when Serene
    /// changed, which the loop publishes as a delta.
    fn tick_vitals(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<(u64, ActorVitals)>> {
        async { None }
    }

    /// ATTACK-1b: one `ATTACK_TARGET_INTENT` (command 11) of the admitted actor, applied by the
    /// Channel owner; `None` stops attacking. `command_id` is the CommandId of the command, whose
    /// `CommandRef` starts the new target's swing lineage.
    fn attack_target(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _command_id: u64,
        _target: Option<WireEntityRef>,
    ) -> impl Future<Output = AttackIntentDisposition> {
        async { AttackIntentDisposition::Rejected }
    }

    /// ATTACK-1b: one `FIGHT_MODES_INTENT` (command 12) of the admitted actor.
    fn fight_modes(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _modes: FightModes,
    ) -> impl Future<Output = AttackIntentDisposition> {
        async { AttackIntentDisposition::Rejected }
    }

    /// ATTACK-1b: the admitted actor's `ACTOR_COMBAT_STATE` (domain 10) for the snapshot, or
    /// `None` while it has none, which ends a session that selected capability 17.
    fn observe_combat_state(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<ActorCombatState>> {
        async { None }
    }

    /// MAP-WIRE-2: domain 17 for the admitted actor's current position, through `view` and the
    /// map handles of `items`: a snapshot when `view` has sent nothing, else a delta or nothing.
    /// `None` when this authority serves no world map: a session that selected capability 18
    /// then fails closed. MAP-CUTOVER-1 composes the bundle World's map facts here.
    fn observe_world_map(
        &self,
        _actor: ExactActorRef,
        _view: &mut SessionMapView,
        _items: &mut SessionItemView,
    ) -> impl Future<Output = Option<Result<Option<MapUpdate>, MapViewError>>> {
        async { None }
    }

    /// ATTACK-1b: run the Channel's due auto-attack swings, every [`COMBAT_REFRESH`] while the
    /// session has domain 10, and return the actor's current combat state.
    fn tick_combat(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<ActorCombatState>> {
        async { None }
    }

    /// After `wait` without restored control, record authoritative unexpected control loss
    /// for the ended admitted connection (`DISCONNECT-PROTECTION-V1` §§1, 4).
    fn lose_control(
        &self,
        _admitted: AdmittedSession,
        _wait: std::time::Duration,
        _reset: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> impl Future<Output = ControlLossResult> {
        async { ControlLossResult::NotApplicable }
    }

    /// Terminally release a resumed session whose recovered connection ended again, so it
    /// never stays ACTIVE on a dead transport, and remove its Channel actor.
    fn release_abandoned(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = GraceExpiryResult> {
        async { GraceExpiryResult::NotApplicable }
    }

    /// LOGOUT-WIRE-1 (§1.6): retire the Channel actor of a session whose current row is
    /// TERMINAL. `Unknown` when the row cannot be read; the lifecycle runs it again.
    fn reconcile_terminal(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = GraceExpiryResult> {
        async { GraceExpiryResult::NotApplicable }
    }

    /// LOGOUT-WIRE-1 (§1.6): one `LOGOUT_INTENT` of the admitted session. Authorities that end
    /// no session refuse it as `BUSY`, with no effect.
    fn logout(&self, _admitted: AdmittedSession) -> impl Future<Output = LogoutReply> {
        async { LogoutReply::Result(LogoutResult::Busy) }
    }

    /// Once the original grace deadline of the recorded loss passes without resumed
    /// control, terminally release the session and remove its Channel actor (FND-04B §6).
    fn expire_control_loss(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = GraceExpiryResult> {
        async { GraceExpiryResult::NotApplicable }
    }
}
