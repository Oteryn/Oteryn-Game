//! Real qualified native Source map/pins with a transport-only cue producer.
//! Brain's separate PG case exercises the actual callback producer.
use super::*;

struct NativeCueAuthority {
    scene: crate::gameplay_transport::wire_position::test_scene::SourceScene,
    cue: Cell<bool>,
    cursor_ready: Cell<bool>,
    step_calls: Cell<usize>,
}
impl FreshAdmissionAuthority for NativeCueAuthority {
    fn offered_capabilities(
        &self,
    ) -> &'static [crate::gameplay_transport::capabilities::OfferedCapability] {
        BUNDLE_WORLD_OFFERED_CAPABILITIES
    }
    async fn native_map_cue_uses_native_floor(&self) -> Option<bool> {
        self.native_world_presentation_available()
            .await
            .then_some(true)
    }
    async fn native_world_presentation_available(&self) -> bool {
        self.scene.runtime.owner_fence().is_ok()
            && self.scene.controller.active().is_some_and(|active| {
                crate::gameplay_transport::native_cue_offer::source_available(
                    active,
                    &self.scene.room,
                    self.scene.runtime.content_pin(),
                )
            })
    }
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }
    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        if actor != self.scene.actor || self.scene.runtime.read_actor_position(actor).is_err() {
            return None;
        }
        Some(WorldSpatialObservation {
            content_generation: self.scene.runtime.content_pin().client_artifact_digest(),
            actor_position: self.scene.wire,
        })
    }
    async fn observe_current_actor_movement_effects(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        after: u64,
    ) -> Vec<(u64, crate::map::overlay::TilePos, u16)> {
        if actor != self.scene.actor || session != self.scene.session {
            return Vec::new();
        }
        self.observe_movement_effects(after)
    }
    fn observe_movement_effects(
        &self,
        after: u64,
    ) -> Vec<(u64, crate::map::overlay::TilePos, u16)> {
        if !self.cue.get() {
            self.cursor_ready.set(true);
        }
        if !self.cue.get() || after != 0 {
            return Vec::new();
        }
        let at = self.scene.wire;
        match (
            u16::try_from(at.x),
            u16::try_from(at.y),
            i8::try_from(at.floor),
        ) {
            (Ok(x), Ok(y), Ok(floor)) => {
                vec![(1, crate::map::overlay::TilePos { x, y, floor }, 11)]
            }
            _ => Vec::new(),
        }
    }
    async fn step(&self, _: ExactActorRef, _: StepDirection) -> StepOutcome {
        self.step_calls.set(self.step_calls.get() + 1);
        StepOutcome::rejected()
    }
}

#[test]
fn srx3_idle_native_map_cue_reaches_wire_without_player_movement() -> TestResult {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| -> TestResult {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(async {
            let scene = crate::gameplay_transport::wire_position::test_scene::source_scene()?;
            assert!(scene.room.pinned_activity_source_room().is_some());
            let authority = NativeCueAuthority {scene,cue:Cell::new(false),cursor_ready:Cell::new(false),step_calls:Cell::new(0)};
            assert!(authority.native_world_presentation_available().await,
                "actual qualified native Bundle and current owner/pins are required before negotiation");
            let active=authority.scene.controller.active().ok_or("active source")?;
            let pin=authority.scene.runtime.content_pin();
            let stale = crate::foundation::ChannelContentPin::from_activation(pin.world_id(),
                pin.activation_sequence(),pin.server_artifact_digest(),pin.client_artifact_digest(),
                pin.frame_binding_digest(),[0;32],(authority.scene.physical.x,authority.scene.physical.y,authority.scene.physical.floor));
            assert!(!crate::gameplay_transport::native_cue_offer::source_available(active,&authority.scene.room,&stale),
                "a native body cannot authorize presentation for a different map pin");
            let (mut server,mut client)=tokio::io::duplex(1<<20);
            let mut admitted=admitted(false)?;
            admitted.world_id=pin.world_id();
            admitted.runtime_actor=Some(authority.scene.actor);
            admitted.game_session_id=authority.scene.session;
            admitted.item_fence=Some(authority.scene.fence);
            admitted.continuity.selected_capabilities=crate::gameplay_transport::connection::fresh_capability_selection(
                &authority,&[6,23,30,oteryn_protocol_oteryn::native_map_cues::CAPABILITY_NATIVE_MAP_CUES_V1]).await
                .ok_or("native selection")?;
            assert!(admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::presentation::CAPABILITY_PRESENTATION_V1),
                "real qualified native map cues must negotiate presentation23 before any timeout test");
            assert!(admitted.continuity.selected_capabilities.contains(385),
                "native signed cues require actual negotiated384 before any timeout test");
            let expected = authority.scene.wire;
            let authority_ref = &authority;
            let observing = async move {
                let setup_deadline = tokio::time::Instant::now() + VISIBILITY_REFRESH * 8;
                let mut native_basis = false;
                let mut cooldown_pair = false;
                loop {
                    let frame = tokio::time::timeout_at(setup_deadline,read_frame(&mut client)).await
                        .map_err(|_|"actual native join snapshot deadline")??;
                    let envelope = crate::foundation::decode_wire_envelope(&frame)?;
                    if envelope.message_type() == MessageType::SnapshotChunk {
                        for domain in oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?.1 {
                            if domain.domain_id == 19 {
                                assert_eq!(domain.snapshot_type, 2, "actual source epoch declares its native basis");
                                assert!(domain.payload.is_empty());
                                native_basis = true;
                            }
                            if domain.domain_id == 20 {
                                assert_eq!(domain.snapshot_type, 1);
                                assert!(oteryn_protocol_oteryn::presentation::decode_actor_cooldowns(domain.payload).map_err(|e|format!("{e:?}"))?.entries.is_empty());
                                cooldown_pair = true;
                            }
                        }
                    }
                    if envelope.message_type() == MessageType::SnapshotCommit { break; }
                }
                assert!(native_basis && cooldown_pair, "negotiated observation installs both initial domains before cues");
                // SnapshotCommit may be read while the server is still sampling
                // its fresh observer cursor. Emit only after that actual epoch
                // boundary, so this is a new cue rather than skipped join history.
                tokio::time::timeout_at(setup_deadline,async {
                    while !authority_ref.cursor_ready.get() {tokio::task::yield_now().await;}
                }).await.map_err(|_|"actual native observer cursor setup deadline")?;
                authority_ref.cue.set(true);
                // Eight actual visibility intervals, without a client command
                // or a manually invoked refresh callback.
                let deadline = tokio::time::Instant::now() + VISIBILITY_REFRESH * 8;
                loop {
                    let frame = tokio::time::timeout_at(deadline,read_frame(&mut client)).await
                        .map_err(|_|"idle native cue was stranded until a player step")??;
                    let envelope = crate::foundation::decode_wire_envelope(&frame)?;
                    if envelope.message_type()!=MessageType::StateDelta {continue;}
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                    if delta.domain_id != oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION {continue;}
                    assert_eq!(delta.delta_type, 2, "native marker epoch uses its negotiated codec");
                    let events = oteryn_protocol_oteryn::native_map_cues::decode_native_map_presentation_batch(delta.payload)
                        .map_err(|error|format!("native presentation decode:{error:?}"))?;
                    assert_eq!(events.len(),1);
                    let oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect{position,..}=&events[0].kind else {
                        return Err("actual native map effect".into());
                    };
                    assert_eq!(*position,expected,"cue and observer must use the SAME actual Source wire frame");
                    break;
                }
                let repeat_deadline = tokio::time::Instant::now() + VISIBILITY_REFRESH * 3;
                loop {
                    let Ok(frame) = tokio::time::timeout_at(repeat_deadline,read_frame(&mut client)).await else {break;};
                    let frame=frame?;
                    let envelope=crate::foundation::decode_wire_envelope(&frame)?;
                    if envelope.message_type()==MessageType::StateDelta {
                        let delta=oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                        assert_ne!(delta.domain_id,oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION,
                            "one retained native cue cannot emit twice on later real server ticks");
                    }
                }
                drop(client);
                Ok::<(),Box<dyn std::error::Error+Send+Sync>>(())
            };
            let mut serving=Some(Box::pin(serve_admitted_actor(&mut server,admitted,&authority,IDLE_LIVENESS)));
            let mut observing=Some(Box::pin(observing));
            let mut observed=None;
            std::future::poll_fn(|cx| {
                if let Some(future)=observing.as_mut()
                    && let std::task::Poll::Ready(result)=std::future::Future::poll(future.as_mut(),cx) {
                    observed=Some(result);
                    observing=None;
                }
                if let Some(future)=serving.as_mut()
                    && std::future::Future::poll(future.as_mut(),cx).is_ready() {
                    serving=None;
                }
                if serving.is_none() && let Some(result)=observed.take() {
                    return std::task::Poll::Ready(result);
                }
                std::task::Poll::Pending
            }).await?;
            assert_eq!(authority.step_calls.get(),0,"no client Move may drive idle delivery");
            Ok(())
        })
    })?.join().map_err(|_|"native idle cue thread panicked")?
}
