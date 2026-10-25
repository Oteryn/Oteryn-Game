#![allow(clippy::expect_used)]

use super::super::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, ConnectionIdentifiers, FirstEntryOutcome,
    FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS, ResumeAttempt,
    SessionContinuity, StepOutcome, admit_frame, serve_admitted,
};
use super::super::item_view::{
    InventoryItems, ItemKey, ItemTargetObservation, ItemViewContinuity, SessionItemView, ViewItem,
};
use super::super::world_spatial::{
    ActorPosition, CAPABILITY_WORLD_SPATIAL_ENTITIES, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection, StepDisposition, WorldSpatialObservation,
    encode_world_spatial,
};
use super::*;
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;

use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, CharacterId, ExactActorRef, FoundationProtocolError,
    GameSessionId, MessageType, ServerAcceptedValue, ServerResumeAcceptedValue, WorldId,
    decode_wire_envelope, encode_protocol_error, encode_server_accepted,
    encode_server_resume_accepted,
};
use crate::foundation::{DomainSnapshot, encode_single_chunk_snapshot};
use oteryn_protocol_oteryn::achievement_notices::{
    AchievementWatermark, SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
    STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES, encode_achievement_notices_snapshot,
};
use oteryn_protocol_oteryn::item_view::{
    COMMAND_TYPE_ITEM_MOVE_INTENT, STATE_DOMAIN_CHARACTER_INVENTORY, STATE_DOMAIN_OPEN_CONTAINER,
};
use oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1;
use oteryn_protocol_oteryn::world_object::{
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
};
use oteryn_protocol_oteryn::{
    ClientBootstrapValue, decode_server_accepted, encode_client_bootstrap,
};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::future::Future;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

const REGISTRY: &str = oteryn_protocol_oteryn::PROTOCOL_OTERYN_V1_REGISTRY_JSON;

const CHARACTER: [u8; 16] = uuid_v7(0x11);
const SESSION: [u8; 16] = uuid_v7(0x22);
const WORLD: [u8; 16] = uuid_v7(0x33);
const CHANNEL: [u8; 16] = uuid_v7(0x44);
const ACCOUNT: [u8; 16] = uuid_v7(0x55);
const TRANSPORT: [u8; 16] = [0x5a; 16];

const fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

/// An injected offered set: 1 alone, 7 requiring 8, 8, 10 requiring 1, 6 requiring an unoffered 99.
const INJECTED: &[OfferedCapability] = &[
    OfferedCapability {
        id: 1,
        requires: &[],
    },
    OfferedCapability {
        id: 6,
        requires: &[99],
    },
    OfferedCapability {
        id: 7,
        requires: &[8],
    },
    OfferedCapability {
        id: 8,
        requires: &[],
    },
    OfferedCapability {
        id: 10,
        requires: &[1],
    },
];

fn registry_capabilities() -> Result<Vec<Value>, Box<dyn Error>> {
    let registry: Value = serde_json::from_str(REGISTRY)?;
    Ok(registry["capabilities"]
        .as_array()
        .ok_or("capabilities")?
        .clone())
}

fn ids(value: &Value) -> Result<Vec<u32>, Box<dyn Error>> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_u64()
                    .and_then(|id| u32::try_from(id).ok())
                    .ok_or_else(|| "id".into())
            })
            .collect(),
        _ => Err("ids".into()),
    }
}

fn selection(ids: &[u32]) -> SelectedCapabilities {
    let offered: Vec<OfferedCapability> = ids
        .iter()
        .map(|&id| OfferedCapability { id, requires: &[] })
        .collect();
    SelectedCapabilities::select(&offered, ids).expect("selection")
}

#[test]
fn the_production_offered_set_is_the_registry_offered_set_and_registered()
-> Result<(), Box<dyn Error>> {
    let mut registered = Vec::new();
    let mut offered = Vec::new();
    for capability in registry_capabilities()? {
        let id = u32::try_from(capability["id"].as_u64().ok_or("id")?)?;
        registered.push(id);
        if capability["offered"].as_bool().ok_or("offered")? {
            offered.push((id, ids(&capability["requires"])?));
        }
    }
    let pairs = |set: &[OfferedCapability]| -> Vec<(u32, Vec<u32>)> {
        set.iter()
            .map(|capability| (capability.id, capability.requires.to_vec()))
            .collect()
    };
    // The registry offered set is the set a generation pinning a non-empty Item key set offers;
    // without one, production also omits capabilities requiring Item views (TRADE-WIRE-1).
    assert_eq!(pairs(ITEM_VIEW_OFFERED_CAPABILITIES), offered);
    let fallback: Vec<(u32, Vec<u32>)> = offered
        .iter()
        .filter(|(id, requires)| {
            *id != CAPABILITY_ITEM_VIEW_MOVE_V1 && !requires.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1)
        })
        .cloned()
        .collect();
    assert_eq!(pairs(PRODUCTION_OFFERED_CAPABILITIES), fallback);
    for set in [
        PRODUCTION_OFFERED_CAPABILITIES,
        ITEM_VIEW_OFFERED_CAPABILITIES,
    ] {
        assert!(set.windows(2).all(|pair| pair[0].id < pair[1].id));
        assert!(set.len() <= SELECTED_CAPACITY);
        for capability in set {
            assert!(
                capability
                    .requires
                    .iter()
                    .all(|required| registered.contains(required))
            );
        }
    }
    // `REGISTERED_CAPABILITY_IDS_V1` is what the acceptance encoder checks a selected set
    // against: the whole offered set, selected at once, is accepted, and an unregistered ID is
    // not.
    let accepted = |selected: &[u32]| {
        encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&SESSION).expect("session"),
            world_id: WorldId::decode(&WORLD).expect("world"),
            channel_id: ChannelId::decode(&CHANNEL).expect("channel"),
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: selected,
        })
    };
    let all: Vec<u32> = PRODUCTION_OFFERED_CAPABILITIES
        .iter()
        .map(|capability| capability.id)
        .collect();
    assert!(accepted(&all).is_ok());
    assert!(accepted(&registered).is_ok());
    assert!(accepted(&[2]).is_err());
    Ok(())
}

#[path = "capability_quest_log_tests.rs"]
mod quest_log_offer;

#[path = "capability_offers_golden_tests.rs"]
mod offers_golden;

#[path = "capability_fragment_tests.rs"]
mod fragments;
use fragments::runtime_fragments;

/// A superseding capability is offered only by a bundle World, never by the production or the
/// fixture item-view set, so a fixture World session keeps the core domain it supersedes. A
/// superseded domain is a core domain: no capability gates it.
#[test]
fn a_superseding_capability_is_bundle_only_and_supersedes_only_core_domains() {
    let offers = |set: &[OfferedCapability], id: u32| set.iter().any(|offer| offer.id == id);
    assert!(!SUPERSEDED.is_empty());
    for (capability, domains) in SUPERSEDED {
        assert!(offers(BUNDLE_WORLD_OFFERED_CAPABILITIES, *capability));
        assert!(!offers(PRODUCTION_OFFERED_CAPABILITIES, *capability));
        assert!(!offers(ITEM_VIEW_OFFERED_CAPABILITIES, *capability));
        for domain in *domains {
            assert!(
                GATED.iter().all(|(_, _, gated)| !gated.contains(domain)),
                "domain {domain}"
            );
        }
    }
}

// The production connection path: `admit_frame` and `serve_admitted` with an injected offered
// set, or the production one.

struct Identifiers;
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&SESSION).ok()
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&TRANSPORT).ok()
    }
}

/// Admits and resumes; `resume` hands back `lost` without checking the selection, so the
/// connection's own refusal is observable. `offered` is `None` for the production set.
struct NegotiatingAuthority {
    offered: Option<&'static [OfferedCapability]>,
    /// ADMIT0: the boot-bound requirements this authority checks after its (fixture)
    /// authentication, as the composed owner does; `None` requires nothing.
    required: Option<
        std::sync::Arc<super::super::world_required_capabilities::BoundRequiredCapabilities>,
    >,
    lost: Cell<Option<AdmittedSession>>,
    steps: Cell<u32>,
    /// ITEM-USE-WIRE-1: what an item USE observes; the backpack it serves under capability 4.
    item_target: RefCell<Option<ItemTargetObservation>>,
    /// ITEM-USE-1c: whether the backpack holds one direct entry, and the items used.
    backpack_entry: Cell<bool>,
    item_uses: RefCell<Vec<ItemKey>>,
    /// WEATHER-1: whether the shipped weather source is readable now, the authority's UTC time
    /// fact, and the admitted actor's position.
    weather_bound: Cell<bool>,
    clock_ms: Cell<u64>,
    position: Cell<ActorPosition>,
    /// PARITY-EVENT-CALLBACKS-1: the Channel's due server-save countdown minute.
    server_save: Cell<Option<super::super::server_save_countdown::DueServerSaveCountdown>>,
}

impl NegotiatingAuthority {
    fn new(offered: Option<&'static [OfferedCapability]>) -> Self {
        Self {
            offered,
            required: None,
            lost: Cell::new(None),
            steps: Cell::new(0),
            item_target: RefCell::new(None),
            backpack_entry: Cell::new(false),
            item_uses: RefCell::new(Vec::new()),
            weather_bound: Cell::new(true),
            clock_ms: Cell::new(NOON_UTC_MS),
            position: Cell::new(ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            }),
            server_save: Cell::new(None),
        }
    }
}

fn session(continuity: SessionContinuity) -> Result<AdmittedSession, Box<dyn Error>> {
    let world_id = WorldId::decode(&WORLD)?;
    let channel_id = ChannelId::decode(&CHANNEL)?;
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&SESSION)?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(super::super::connection::ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&TRANSPORT)?,
            account_id: ACCOUNT,
        }),
        continuity,
        item_fence: None,
    })
}

impl FreshAdmissionAuthority for NegotiatingAuthority {
    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        if let Some(required) = &self.required {
            required
                .authenticate_support(attempt.supported_capabilities)
                .map_err(|error| match error {
                    super::super::world_required_capabilities::SessionCapabilityError::MissingSupport(_) => {
                        AdmissionRefusal::Classified(FoundationProtocolError::AdmissionCapabilityRequired)
                    }
                    _ => AdmissionRefusal::Unavailable,
                })?;
        }
        session(SessionContinuity::FRESH).map_err(|_| AdmissionRefusal::Unavailable)
    }

    async fn resume(
        &self,
        attempt: ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        let lost = self.lost.get().ok_or(AdmissionRefusal::Rejected)?;
        // ADMIT0 §4 / D1084: as the V2 recovery owner, after its (fixture) authentication.
        if let Some(required) = &self.required {
            required
                .authenticate_resume_support(
                    attempt.supported_capabilities,
                    lost.continuity.selected_capabilities,
                )
                .map_err(|error| match error {
                    super::super::world_required_capabilities::SessionCapabilityError::MissingSupport(_) => {
                        AdmissionRefusal::RecoveryCapabilityRequired
                    }
                    _ => AdmissionRefusal::Unavailable,
                })?;
        }
        Ok(lost)
    }

    fn offered_capabilities(&self) -> &'static [OfferedCapability] {
        self.offered.unwrap_or(PRODUCTION_OFFERED_CAPABILITIES)
    }

    fn world_clock_utc_ms(&self) -> Option<u64> {
        Some(self.clock_ms.get())
    }

    fn world_weather(&self) -> Option<&crate::world_weather::WorldWeatherSource> {
        crate::world_weather::shipped().filter(|_| self.weather_bound.get())
    }

    fn server_save_countdown(
        &self,
    ) -> Option<super::super::server_save_countdown::DueServerSaveCountdown> {
        self.server_save.get()
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: self.position.get(),
        })
    }

    async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
        self.steps.set(self.steps.get() + 1);
        StepOutcome {
            disposition: StepDisposition::Blocked,
            moved_to: None,
        }
    }

    async fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems {
            main_backpack: Some(ViewItem {
                key: ItemKey::Instance([0xa5; 16]),
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }),
            entries: self
                .backpack_entry
                .get()
                .then_some(ViewItem {
                    key: ItemKey::Instance([0xa6; 16]),
                    item_definition_ref: std::num::NonZeroU32::MIN,
                    count: std::num::NonZeroU32::MIN,
                    sub_type: 0,
                })
                .into_iter()
                .collect(),
            equipment: Vec::new(),
        })
    }

    async fn use_item(
        &self,
        _actor: ExactActorRef,
        _use: super::super::connection::UseCommand,
        item: ItemKey,
    ) -> super::super::connection::UseOutcome {
        self.item_uses.borrow_mut().push(item);
        super::super::connection::UseOutcome {
            disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Full,
            ..super::super::connection::UseOutcome::rejected()
        }
    }

    async fn observe_item_target(
        &self,
        _actor: ExactActorRef,
        _target: ItemKey,
    ) -> Option<ItemTargetObservation> {
        self.item_target.borrow().clone()
    }

    async fn observe_achievement_notices(
        &self,
        _account_id: [u8; 16],
    ) -> Option<AchievementWatermark> {
        Some(AchievementWatermark {
            fact_count: 1,
            total_points: 3,
        })
    }
}

fn varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn scalar(output: &mut Vec<u8>, field: u64, value: u64) {
    varint(output, field << 3);
    varint(output, value);
}

fn bytes(output: &mut Vec<u8>, field: u64, value: &[u8]) {
    varint(output, (field << 3) | 2);
    varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn envelope(message_type: u64, generation: u64, payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    scalar(&mut output, 1, message_type);
    if generation != 0 {
        scalar(&mut output, 2, generation);
    }
    bytes(&mut output, 4, payload);
    output
}

fn bootstrap(supported: &[u32]) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_client_bootstrap(&ClientBootstrapValue {
        schema_revision: 1,
        character_id: CharacterId::decode(&CHARACTER)?,
        admission_material: b"grant",
        client_build_id: "cap-neg-test",
        supported_capabilities: supported,
    })?)
}

/// `ClientResume` (FND-02 §9): session 1, material 2, last applied 3, major 4, profile 5,
/// schema 6, capabilities 7, build 8.
fn resume(supported: &[u32]) -> Vec<u8> {
    let mut payload = Vec::new();
    bytes(&mut payload, 1, &SESSION);
    bytes(&mut payload, 2, b"recovery");
    scalar(&mut payload, 4, 1);
    scalar(&mut payload, 5, 1);
    scalar(&mut payload, 6, 1);
    for &capability in supported {
        scalar(&mut payload, 7, u64::from(capability));
    }
    bytes(&mut payload, 8, b"cap-neg-test");
    envelope(3, 0, &payload)
}

pub(in crate::gameplay_transport) fn command(
    id: u64,
    command_type: u32,
    payload: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    scalar(&mut body, 1, id);
    scalar(&mut body, 2, u64::from(command_type));
    bytes(&mut body, 4, payload);
    envelope(7, 1, &body)
}

fn split(output: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
        cursor += 4 + length;
    }
    Ok(frames)
}

fn run<F: Future<Output = Result<(), Box<dyn Error>>>>(test: F) -> Result<(), Box<dyn Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(Box::pin(test))
}

/// One entry frame through `admit_frame`: the end (or session) and the frames written.
async fn admit(
    authority: &NegotiatingAuthority,
    frame: &[u8],
) -> Result<(Result<AdmittedSession, ConnectionEnd>, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 16);
    let admitted = admit_frame(&mut server, frame, authority, &Identifiers).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok((admitted, split(&output)?))
}

/// Serve `admitted` over `frames`; the frames written.
async fn serve(
    authority: &NegotiatingAuthority,
    admitted: AdmittedSession,
    frames: &[Vec<u8>],
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
    for frame in frames {
        let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(frame);
        client.write_all(&framed).await?;
    }
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok((end, split(&output)?))
}

fn accepted_selection(frames: &[Vec<u8>]) -> Result<Vec<u32>, Box<dyn Error>> {
    let [frame] = frames else {
        return Err(format!("expected one frame, got {}", frames.len()).into());
    };
    let envelope = decode_wire_envelope(frame)?;
    assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
    Ok(decode_server_accepted(envelope.payload())?.selected_capabilities)
}

#[test]
fn fresh_admission_echoes_and_keeps_the_selection() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let (admitted, frames) = admit(
            &authority,
            &bootstrap(&[
                1,  // BESTIARY_CHARMS_V1
                7,  // CHAT_V1
                8,  // ACHIEVEMENT_NOTICES_V1
                10, // ANALYSER_V1
                77, // unknown capability
            ])?,
        )
        .await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let expected = [
            1,  // BESTIARY_CHARMS_V1
            7,  // CHAT_V1
            8,  // ACHIEVEMENT_NOTICES_V1
            10, // ANALYSER_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let expected = [
            1,  // BESTIARY_CHARMS_V1
            7,  // CHAT_V1
            8,  // ACHIEVEMENT_NOTICES_V1
            10, // ANALYSER_V1
        ];
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            expected
        );
        // Capability 8 selected: domain 13 starts at revision 0.
        assert_eq!(admitted.continuity.achievement_notice_revision, Some(0));

        // Requires closure on the wire: 7 without 8 is not selected.
        let (admitted, frames) = admit(&authority, &bootstrap(&[7])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(admitted.continuity.achievement_notice_revision, None);
        Ok(())
    })
}

#[test]
fn production_admission_selects_capabilities_6_7_13_and_17_and_nothing_else()
-> Result<(), Box<dyn Error>> {
    run(async {
        // SPEED-1, VIS-3, CHAT-WIRE-1 and ATTACK-1b (§1.9): the production offered set selects
        // capabilities 6, 7, 13 and 17 for a client that supports them, and only them.
        let authority = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(
            &authority,
            &bootstrap(&[
                1,  // BESTIARY_CHARMS_V1
                6,  // WORLD_SPATIAL_ENTITIES
                7,  // CHAT_V1
                8,  // ACHIEVEMENT_NOTICES_V1
                10, // ANALYSER_V1
                13, // PACED_MOVEMENT_V1
                17, // ATTACK_V1
            ])?,
        )
        .await?;
        let expected = [
            6,  // WORLD_SPATIAL_ENTITIES
            7,  // CHAT_V1
            13, // PACED_MOVEMENT_V1
            17, // ATTACK_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let expected = [
            6,  // WORLD_SPATIAL_ENTITIES
            7,  // CHAT_V1
            13, // PACED_MOVEMENT_V1
            17, // ATTACK_V1
        ];
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            expected
        );
        assert_eq!(admitted.continuity.achievement_notice_revision, None);
        // ATTACK-1b: 6 and 17 alone select both; 17 without its required 6 selects nothing.
        let (_, frames) = admit(
            &authority,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                17, // ATTACK_V1
            ])?,
        )
        .await?;
        let expected = [
            6,  // WORLD_SPATIAL_ENTITIES
            17, // ATTACK_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let (_, frames) = admit(&authority, &bootstrap(&[17])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        // CLOCK-1: 19 is selected only with its required 6.
        let (_, frames) = admit(
            &authority,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                19, // WORLD_LIGHT_V1
            ])?,
        )
        .await?;
        let expected = [
            6,  // WORLD_SPATIAL_ENTITIES
            19, // WORLD_LIGHT_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let (_, frames) = admit(&authority, &bootstrap(&[19])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        // CHAT-WIRE-1: 7 requires nothing and is selected alone.
        let (_, frames) = admit(&authority, &bootstrap(&[7])?).await?;
        assert_eq!(accepted_selection(&frames)?, [7]);
        // A client without them selects nothing.
        let (admitted, frames) = admit(
            &authority,
            &bootstrap(&[
                1,  // BESTIARY_CHARMS_V1
                8,  // ACHIEVEMENT_NOTICES_V1
                10, // ANALYSER_V1
            ])?,
        )
        .await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(
            admitted.continuity.selected_capabilities,
            SelectedCapabilities::NONE
        );
        Ok(())
    })
}

include!("capability_connection_1_tests.rs");

include!("capability_connection_2_tests.rs");

// Independent full-set snapshot captured from current main, shared with protocol goldens.
fn golden_offer_ids(set: &str) -> Result<Vec<u32>, Box<dyn Error>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../protocol-oteryn/src/tests/capabilities_golden");
    let mut entries: Vec<Value> = Vec::new();
    for item in std::fs::read_dir(directory)? {
        let item = item?;
        if item.path().extension().is_some_and(|ext| ext == "json") {
            entries.push(serde_json::from_str(&std::fs::read_to_string(
                item.path(),
            )?)?);
        }
    }
    entries.sort_by_key(|entry| entry["id"].as_u64());
    entries
        .iter()
        .filter(|entry| entry["offers"][set] == true)
        .map(|entry| Ok(u32::try_from(entry["id"].as_u64().ok_or("id")?)?))
        .collect()
}
