use super::*;
use crate::gameplay_transport::{actor_spell, world_spatial};

pub(super) const CHARACTER: [u8; 16] = uuid_v7(0x11);
pub(super) const SESSION: [u8; 16] = uuid_v7(0x22);
pub(super) const WORLD: [u8; 16] = uuid_v7(0x33);
pub(super) const CHANNEL: [u8; 16] = uuid_v7(0x44);

pub(super) const fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

pub(super) struct Authority {
    pub(super) calls: Cell<usize>,
    pub(super) outcome: Result<(), AdmissionRefusal>,
    pub(super) seen: RefCell<Vec<(CharacterId, Vec<u8>)>>,
}

impl Authority {
    pub(super) fn new(outcome: Result<(), AdmissionRefusal>) -> Self {
        Self {
            calls: Cell::new(0),
            outcome,
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.calls.set(self.calls.get() + 1);
        self.seen
            .borrow_mut()
            .push((attempt.character_id, attempt.admission_material.to_vec()));
        self.outcome.map(|()| AdmittedSession {
            game_session_id: attempt.game_session_id,
            world_id: WorldId::decode(&WORLD).expect("world"),
            channel_id: ChannelId::decode(&CHANNEL).expect("channel"),
            runtime_actor: None,
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        })
    }
}

pub(super) struct Identifiers;
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&SESSION).ok()
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[0x5a; 16]).ok()
    }
}

pub(super) fn varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

pub(super) fn scalar(output: &mut Vec<u8>, field: u64, value: u64) {
    varint(output, field << 3);
    varint(output, value);
}

pub(super) fn bytes(output: &mut Vec<u8>, field: u64, value: &[u8]) {
    varint(output, (field << 3) | 2);
    varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

pub(super) fn envelope(message_type: u64, generation: u64, payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    scalar(&mut output, 1, message_type);
    if generation != 0 {
        scalar(&mut output, 2, generation);
    }
    bytes(&mut output, 4, payload);
    output
}

pub(super) fn bootstrap(major: u64, profile: u64, material: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, major);
    scalar(&mut payload, 2, profile);
    scalar(&mut payload, 3, 1);
    bytes(&mut payload, 5, material);
    bytes(&mut payload, 6, &CHARACTER);
    bytes(&mut payload, 7, b"seam-test");
    envelope(1, 0, &payload)
}

pub(super) fn resume() -> Vec<u8> {
    let mut payload = Vec::new();
    bytes(&mut payload, 1, &SESSION);
    bytes(&mut payload, 2, b"recovery");
    scalar(&mut payload, 4, 1);
    scalar(&mut payload, 5, 1);
    scalar(&mut payload, 6, 1);
    bytes(&mut payload, 8, b"seam-test");
    envelope(3, 0, &payload)
}

pub(super) fn framed(body: &[u8]) -> Vec<u8> {
    let mut output = (body.len() as u32).to_be_bytes().to_vec();
    output.extend_from_slice(body);
    output
}

/// Drive one connection with the given client frames; return the end state
/// and every frame the server wrote.
pub(super) async fn drive(
    authority: &Authority,
    client_frames: &[Vec<u8>],
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
    for frame in client_frames {
        client.write_all(&framed(frame)).await?;
    }
    client.shutdown().await?;
    let end = serve_connection(&mut server, authority, &Identifiers).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
        cursor += 4 + length;
    }
    Ok((end, frames))
}

pub(super) fn run<F: Future<Output = Result<(), Box<dyn Error>>>>(
    test: F,
) -> Result<(), Box<dyn Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(test)
}

pub(super) fn error_frame(error: FoundationProtocolError, generation: u64) -> Vec<u8> {
    encode_protocol_error(error, generation).expect("error frame")
}

pub(super) fn at(x: i32) -> WorldSpatialObservation {
    WorldSpatialObservation {
        content_generation: [0x5c; 32],
        actor_position: world_spatial::ActorPosition { x, y: 0, floor: 0 },
    }
}

pub(super) fn command(
    generation: u64,
    id: u64,
    command_type: u64,
    direction: StepDirection,
) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, id);
    scalar(&mut payload, 2, command_type);
    bytes(
        &mut payload,
        4,
        &world_spatial::encode_step_intent(direction),
    );
    envelope(7, generation, &payload)
}

/// A fixture authority for USE_INTENT dispatch (USE-WIRE-V1, #162 5868482467): every call is
/// recorded and answered with one canned `UseOutcome`, independent of `target`.
pub(super) struct UseAuthority {
    pub(super) uses: RefCell<Vec<oteryn_protocol_oteryn::world_object::WorldObjectTarget>>,
    pub(super) commands: RefCell<Vec<UseCommand>>,
    pub(super) overlay: Option<WorldObjectOverlayEntry>,
    /// The account watermark the domain-13 snapshot reads.
    pub(super) watermark: Option<AchievementWatermark>,
    pub(super) outcome: UseOutcome,
}

impl FreshAdmissionAuthority for UseAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_world_object_overlay(&self) -> Option<WorldObjectOverlayEntry> {
        self.overlay.clone()
    }

    async fn observe_achievement_notices(
        &self,
        _account_id: [u8; 16],
    ) -> Option<AchievementWatermark> {
        self.watermark
    }

    async fn use_object(
        &self,
        _actor: ExactActorRef,
        command: UseCommand,
        target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        self.commands.borrow_mut().push(command);
        self.uses.borrow_mut().push(target);
        self.outcome.clone()
    }
}

pub(super) fn use_command(
    generation: u64,
    id: u64,
    command_type: u64,
    placement: &[u8],
    expected_revision: u64,
) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, id);
    scalar(&mut payload, 2, command_type);
    let intent = oteryn_protocol_oteryn::world_object::encode_use_intent(
        &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement: placement.to_vec(),
            expected_revision,
        },
    )
    .expect("encode use intent");
    bytes(&mut payload, 4, &intent);
    envelope(7, generation, &payload)
}

/// Serve one positioned admitted session over the given client frames.
pub(super) async fn drive_admitted<A: FreshAdmissionAuthority>(
    authority: &A,
    client_frames: &[Vec<u8>],
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let world_id = WorldId::decode(&WORLD)?;
    let channel_id = ChannelId::decode(&CHANNEL)?;
    let admitted = AdmittedSession {
        game_session_id: GameSessionId::decode(&SESSION)?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity::FRESH,
        item_fence: None,
    };
    drive_session(authority, admitted, client_frames).await
}

/// Serve `admitted` over the given client frames.
pub(super) async fn drive_session<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    client_frames: &[Vec<u8>],
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
    for frame in client_frames {
        client.write_all(&framed(frame)).await?;
    }
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
        cursor += 4 + length;
    }
    Ok((end, frames))
}

// V1 encodes the empty entry list as an empty payload, while retaining
// the registered domain, snapshot type and initial revision in the envelope.
pub(super) fn empty_overlay_snapshot() -> DomainSnapshot<'static> {
    DomainSnapshot {
        domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        revision: 0,
        snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
        payload: &[],
    }
}

pub(super) fn baseline() -> Vec<Vec<u8>> {
    baseline_at(ADMITTED_GENERATION)
}

pub(super) fn baseline_at(generation: u64) -> Vec<Vec<u8>> {
    encode_single_chunk_snapshot(
        generation,
        1,
        0,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &encode_world_spatial(&at(0)),
            },
            empty_overlay_snapshot(),
        ],
    )
    .expect("snapshot")
    .into()
}

pub(super) const FAST: LivenessPolicy = LivenessPolicy {
    interval: std::time::Duration::from_millis(20),
    missed_limit: 3,
};

pub(super) fn ack(generation: u64, probe_id: u64) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, probe_id);
    envelope(6, generation, &payload)
}

pub(super) fn positioned() -> Result<AdmittedSession, Box<dyn Error>> {
    let world_id = WorldId::decode(&WORLD)?;
    let channel_id = ChannelId::decode(&CHANNEL)?;
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&SESSION)?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity::FRESH,
        item_fence: None,
    })
}

/// The first successor generation of a same-session resume (FND-04B §16).
pub(super) const RESUMED_GENERATION: u64 = ADMITTED_GENERATION + 1;

/// `positioned` resumed on the next connection generation before any result of its first
/// CommandId was delivered: the continuity an accepted original Bank command is retained in.
pub(super) fn resumed_positioned() -> Result<AdmittedSession, Box<dyn Error>> {
    let mut resumed = positioned()?;
    resumed.continuity.connection_generation = RESUMED_GENERATION;
    Ok(resumed)
}

pub(super) fn split_frames(output: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
        cursor += 4 + length;
    }
    Ok(frames)
}

/// Spell cast §9 step 2 over the real Channel owner: a positioned actor of a real
/// `ChannelRuntimeV1` whose Character cast facts are supplied, the V1 spell book and the
/// composed cast work item, served by the same `serve_admitted` loop as production.
pub(super) struct SpellAuthority {
    pub(super) runtime: crate::foundation::ChannelRuntimeV1,
    pub(super) states: RefCell<actor_spell::ChannelSpellStates>,
    pub(super) book: crate::spell::SpellBook,
    pub(super) casts: Cell<usize>,
    /// The owner publishes no ACTOR_VITALS observation: the join carries no snapshot.
    pub(super) hide_vitals: bool,
}

impl FreshAdmissionAuthority for SpellAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        if self.hide_vitals {
            return None;
        }
        actor_spell::observe_vitals(&self.runtime, &self.states.borrow(), actor, game_session_id)
    }

    async fn cast_spell(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: SpellCastIntent,
    ) -> SpellCastOutcome {
        self.casts.set(self.casts.get() + 1);
        actor_spell::cast_in_channel(
            &self.runtime,
            &mut self.states.borrow_mut(),
            &self.book,
            actor,
            game_session_id,
            command_id,
            &intent,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
    }
}

pub(super) fn cast_command(id: u64, intent: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, id);
    scalar(
        &mut payload,
        2,
        u64::from(COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT),
    );
    bytes(&mut payload, 4, intent);
    envelope(7, ADMITTED_GENERATION, &payload)
}

pub(super) const SERENE_VITALS: ActorVitals = ActorVitals {
    health: 185,
    max_health: 185,
    mana: 90,
    max_mana: 90,
    soul: 100,
    harmony: 2,
    serene: true,
};
