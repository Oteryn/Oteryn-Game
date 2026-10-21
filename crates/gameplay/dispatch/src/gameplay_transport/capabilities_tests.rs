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

const REGISTRY: &str =
    include_str!("../../../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");

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
    // without one, production falls back to that set without capability 4.
    assert_eq!(pairs(ITEM_VIEW_OFFERED_CAPABILITIES), offered);
    let fallback: Vec<(u32, Vec<u32>)> = offered
        .iter()
        .filter(|(id, _)| *id != CAPABILITY_ITEM_VIEW_MOVE_V1)
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

#[test]
fn selection_is_the_offered_supported_intersection_and_ignores_unknown_ids() {
    let selected = SelectedCapabilities::select(INJECTED, &[1, 3, 8, 500, 4_000_000]);
    assert_eq!(selected.map(|s| s.as_slice().to_vec()), Some(vec![1, 8]));
    let none = SelectedCapabilities::select(INJECTED, &[2, 3, 500]);
    assert_eq!(none, Some(SelectedCapabilities::NONE));
    assert_eq!(
        SelectedCapabilities::select(INJECTED, &[]),
        Some(SelectedCapabilities::NONE)
    );
}

#[test]
fn selection_keeps_a_capability_only_with_all_its_requires() {
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(INJECTED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    // 7 requires 8; 10 requires 1; 6 requires 99, which is never offered.
    assert_eq!(select(&[7]), Vec::<u32>::new());
    assert_eq!(select(&[7, 8]), [7, 8]);
    assert_eq!(select(&[10]), Vec::<u32>::new());
    assert_eq!(select(&[1, 10]), [1, 10]);
    assert_eq!(select(&[6, 99]), Vec::<u32>::new());
    assert_eq!(select(&[1, 6, 7, 8, 10, 99]), [1, 7, 8, 10]);
    // The closure is transitive: a requirement that drops out takes its dependants with it.
    const CHAIN: &[OfferedCapability] = &[
        OfferedCapability {
            id: 1,
            requires: &[6],
        },
        OfferedCapability {
            id: 6,
            requires: &[7],
        },
        OfferedCapability {
            id: 7,
            requires: &[8],
        },
        OfferedCapability {
            id: 8,
            requires: &[],
        },
    ];
    let chain = |supported: &[u32]| {
        SelectedCapabilities::select(CHAIN, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    assert_eq!(chain(&[1, 6, 7]), Vec::<u32>::new());
    assert_eq!(chain(&[1, 6, 7, 8]), [1, 6, 7, 8]);
    assert_eq!(chain(&[1, 7, 8]), [7, 8]);
}

#[test]
fn the_production_set_selects_only_capabilities_6_7_9_13_17_19_and_21_whatever_the_client_supports()
-> Result<(), Box<dyn Error>> {
    let mut everything: Vec<u32> = Vec::new();
    for capability in registry_capabilities()? {
        everything.push(u32::try_from(capability["id"].as_u64().ok_or("id")?)?);
    }
    everything.extend([2, 3, 4, 5, 9, 11, 12, 13, 14]);
    everything.sort_unstable();
    everything.dedup();
    assert_eq!(
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &everything)
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[6, 7, 9, 13, 17, 19, 21, 26][..])
    );
    // MAP-ITEM-REF-1: the not-yet-offered Item view set adds 4, which requires 6.
    assert_eq!(
        SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &everything)
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[4, 6, 7, 9, 13, 17, 19, 21, 26][..])
    );
    assert_eq!(
        SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &[4, 13])
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[13][..])
    );
    Ok(())
}

#[test]
fn resume_keeps_exactly_the_original_selection() {
    let original = SelectedCapabilities::select(INJECTED, &[1, 8]).expect("bounded");
    assert!(original.resumable_with(&[1, 8]));
    // A wider supported set is fine but never widens the session's selection.
    assert!(original.resumable_with(&[1, 7, 8, 10]));
    assert_eq!(original.as_slice(), [1, 8]);
    // A supported set that lacks a selected capability cannot keep it.
    assert!(!original.resumable_with(&[1]));
    assert!(!original.resumable_with(&[]));
    assert!(SelectedCapabilities::NONE.resumable_with(&[]));
}

#[test]
fn commands_and_domains_follow_the_selection() {
    let none = SelectedCapabilities::NONE;
    // Core commands and domains never depend on a capability.
    for core in [1, 2, 3, 10] {
        assert!(none.command_selected(core), "command {core}");
    }
    for core in [1, 2, 3] {
        assert!(none.domain_selected(core), "domain {core}");
    }
    // Capability 1: commands 4 and 5, domains 4 and 5. Capability 7: command 13, domain 12.
    // Capability 8: domain 13. Capability 10: domain 15. Capability 16: command 22, domain 16.
    // Capability 17: commands 11 and 12, domain 10. Capability 19: domain 18. Capability 11: command 15.
    // Capability 21: command 23. Capability 23: domains 19 and 20.
    for (capability, commands, domains) in [
        (1, &[4, 5][..], &[4, 5][..]),
        (7, &[13][..], &[12][..]),
        (8, &[][..], &[13][..]),
        (10, &[][..], &[15][..]),
        (16, &[22][..], &[16][..]),
        (17, &[11, 12][..], &[10][..]),
        (19, &[][..], &[18][..]),
        (11, &[15][..], &[][..]),
        (21, &[23][..], &[][..]),
        (23, &[][..], &[19, 20][..]),
    ] {
        let selected = selection(&[capability]);
        for &command in commands {
            assert!(!none.command_selected(command), "command {command}");
            assert!(selected.command_selected(command), "command {command}");
        }
        for &domain in domains {
            assert!(!none.domain_selected(domain), "domain {domain}");
            assert!(selected.domain_selected(domain), "domain {domain}");
        }
    }
    assert!(!selection(&[8]).command_selected(13));
    assert!(!selection(&[7]).domain_selected(13));
    assert!(!selection(&[14]).command_selected(22));
    assert!(!selection(&[14]).domain_selected(16));
    assert!(!selection(&[16]).command_selected(11));
    assert!(!selection(&[16]).command_selected(12));
    assert!(!selection(&[16]).domain_selected(10));
    // MAP-WIRE-1 §3: one object domain per World. Capability 18 supersedes core domain 2.
    assert!(!selection(&[4, 6, 18]).domain_selected(2));
    assert!(selection(&[4, 6, 18]).domain_selected(17));
    assert!(selection(&[4, 6]).domain_selected(2));
}

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

fn command(id: u64, command_type: u32, payload: &[u8]) -> Vec<u8> {
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
        let (admitted, frames) = admit(&authority, &bootstrap(&[1, 7, 8, 10, 77])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(accepted_selection(&frames)?, [1, 7, 8, 10]);
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            [1, 7, 8, 10]
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
        let (admitted, frames) = admit(&authority, &bootstrap(&[1, 6, 7, 8, 10, 13, 17])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6, 7, 13, 17]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            [6, 7, 13, 17]
        );
        assert_eq!(admitted.continuity.achievement_notice_revision, None);
        // ATTACK-1b: 6 and 17 alone select both; 17 without its required 6 selects nothing.
        let (_, frames) = admit(&authority, &bootstrap(&[6, 17])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6, 17]);
        let (_, frames) = admit(&authority, &bootstrap(&[17])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        // CLOCK-1: 19 is selected only with its required 6.
        let (_, frames) = admit(&authority, &bootstrap(&[6, 19])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6, 19]);
        let (_, frames) = admit(&authority, &bootstrap(&[19])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        // CHAT-WIRE-1: 7 requires nothing and is selected alone.
        let (_, frames) = admit(&authority, &bootstrap(&[7])?).await?;
        assert_eq!(accepted_selection(&frames)?, [7]);
        // A client without them selects nothing.
        let (admitted, frames) = admit(&authority, &bootstrap(&[1, 8, 10])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(
            admitted.continuity.selected_capabilities,
            SelectedCapabilities::NONE
        );
        Ok(())
    })
}

#[test]
fn the_bundle_world_offered_set_is_3_the_item_view_set_12_14_and_18_requiring_4_and_6()
-> Result<(), Box<dyn Error>> {
    // MAP-CUTOVER-1b: the bundle set is the Item view set (which offers 4) with 3 and 18;
    // ITEM-LOOP-1 (CP D1438) adds 12 (requires 4) and 14 (requires 4 and 12). 18's crate and
    // registry `requires` stay [4, 6] and the registry keeps 3, 12, 14 and 18 `offered: false`.
    use oteryn_protocol_oteryn::container_tree::{
        CAPABILITY_CONTAINER_TREE_V1, CAPABILITY_CONTAINER_TREE_V1_REQUIRES,
    };
    use oteryn_protocol_oteryn::item_view::{
        CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES,
    };
    let bundle_only = [
        oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1,
        CAPABILITY_ITEM_EQUIP_DROP_V1,
        CAPABILITY_CONTAINER_TREE_V1,
        CAPABILITY_WORLD_MAP_VIEW_V1,
    ];
    let shared: Vec<OfferedCapability> = BUNDLE_WORLD_OFFERED_CAPABILITIES
        .iter()
        .filter(|offer| !bundle_only.contains(&offer.id))
        .copied()
        .collect();
    assert_eq!(shared, ITEM_VIEW_OFFERED_CAPABILITIES);
    let requires = |id: u32| {
        BUNDLE_WORLD_OFFERED_CAPABILITIES
            .iter()
            .find(|offer| offer.id == id)
            .map(|offer| offer.requires)
    };
    assert_eq!(requires(bundle_only[0]), Some(&[][..]));
    assert_eq!(
        requires(CAPABILITY_ITEM_EQUIP_DROP_V1),
        Some(CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES)
    );
    assert_eq!(
        requires(CAPABILITY_CONTAINER_TREE_V1),
        Some(CAPABILITY_CONTAINER_TREE_V1_REQUIRES)
    );
    assert_eq!(requires(CAPABILITY_WORLD_MAP_VIEW_V1), Some(&[4, 6][..]));
    // WEATHER-1: the bundle set offers 26 (shared with the Item view set) with its own `requires`.
    assert_eq!(
        requires(oteryn_protocol_oteryn::world_weather::CAPABILITY_WORLD_WEATHER_V1),
        Some(WORLD_WEATHER.requires)
    );
    assert_eq!(BUNDLE_WORLD_OFFERED_CAPABILITIES.len(), shared.len() + 4);
    assert!(
        BUNDLE_WORLD_OFFERED_CAPABILITIES
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
    // Every requirement of an offered capability is offered.
    assert!(BUNDLE_WORLD_OFFERED_CAPABILITIES.iter().all(|offer| {
        offer
            .requires
            .iter()
            .all(|required| requires(*required).is_some())
    }));
    assert!(BUNDLE_WORLD_OFFERED_CAPABILITIES.len() <= SELECTED_CAPACITY);
    let registry = registry_capabilities()?;
    for id in bundle_only {
        let entry = registry
            .iter()
            .find(|capability| capability["id"].as_u64() == Some(u64::from(id)))
            .ok_or("bundle-only capability")?;
        assert_eq!(entry["offered"].as_bool(), Some(false));
        let gate = entry["offer_gate"].as_str().ok_or("offer_gate")?;
        assert!(gate.contains("BUNDLE_WORLD_OFFERED_CAPABILITIES"), "{id}");
    }
    let entry = registry
        .iter()
        .find(|capability| capability["id"].as_u64() == Some(18))
        .ok_or("capability 18")?;
    assert_eq!(ids(&entry["requires"])?, [4, 6]);
    Ok(())
}

#[test]
fn a_bundle_world_admits_only_a_client_that_selects_18() -> Result<(), Box<dyn Error>> {
    run(async {
        let mut authority = NegotiatingAuthority::new(Some(BUNDLE_WORLD_OFFERED_CAPABILITIES));
        // ADMIT0: a bundle World's floor is 18, so its boot declares 18 (closure 4, 6, 18).
        authority.required = Some(
            super::super::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[CAPABILITY_WORLD_MAP_VIEW_V1],
                crate::foundation::RuntimeScopeRefV1::channel(
                    WorldId::decode(&WORLD)?,
                    ChannelId::decode(&CHANNEL)?,
                ),
                1,
                "capability-transport-fixture",
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
            )
            .map_err(|error| format!("{error:?}"))?,
        );
        let required =
            encode_protocol_error(FoundationProtocolError::AdmissionCapabilityRequired, 0)?;
        // 4, 6 and 18 select 18 with the rest of the production set the client supports.
        let (admitted, frames) = admit(&authority, &bootstrap(&[4, 6, 13, 17, 18])?).await?;
        assert_eq!(accepted_selection(&frames)?, [4, 6, 13, 17, 18]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_WORLD_MAP_VIEW_V1)
        );
        // A client lacking any member of the closure is refused by the owner, after its
        // authentication, with exactly one ADMISSION_CAPABILITY_REQUIRED (1116).
        for supported in [&[4, 6, 13, 17][..], &[6, 13, 17, 18], &[4, 13, 17, 18], &[]] {
            let (refused, frames) = admit(&authority, &bootstrap(supported)?).await?;
            assert!(matches!(
                refused,
                Err(ConnectionEnd::AdmissionRefused(
                    AdmissionRefusal::Classified(
                        FoundationProtocolError::AdmissionCapabilityRequired
                    )
                ))
            ));
            assert_eq!(frames, std::slice::from_ref(&required));
        }
        Ok(())
    })
}

#[test]
fn a_bundle_world_recovery_lacking_18_is_refused_with_1118_and_one_frame()
-> Result<(), Box<dyn Error>> {
    run(async {
        let mut authority = NegotiatingAuthority::new(Some(BUNDLE_WORLD_OFFERED_CAPABILITIES));
        authority.required = Some(
            super::super::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[CAPABILITY_WORLD_MAP_VIEW_V1],
                crate::foundation::RuntimeScopeRefV1::channel(
                    WorldId::decode(&WORLD)?,
                    ChannelId::decode(&CHANNEL)?,
                ),
                1,
                "capability-transport-fixture",
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
            )
            .map_err(|error| format!("{error:?}"))?,
        );
        let (admitted, _) = admit(&authority, &bootstrap(&[4, 6, 13, 17, 18])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        authority.lost.set(Some(admitted));
        // ADMIT0 §5 / D1187: a recovery lacking a required member is answered with exactly
        // one RECOVERY_CAPABILITY_REQUIRED (1118, OPERATION_TERMINAL), not silently dropped.
        let required =
            encode_protocol_error(FoundationProtocolError::RecoveryCapabilityRequired, 0)?;
        for supported in [&[4, 6, 13, 17][..], &[6, 13, 17, 18], &[]] {
            let (refused, frames) = admit(&authority, &resume(supported)).await?;
            assert_eq!(
                refused.err(),
                Some(ConnectionEnd::AdmissionRefused(
                    AdmissionRefusal::RecoveryCapabilityRequired
                ))
            );
            assert_eq!(frames, std::slice::from_ref(&required));
        }
        assert_eq!(
            FoundationProtocolError::RecoveryCapabilityRequired.disposition(),
            crate::foundation::ProtocolDisposition::OperationTerminal
        );
        // The same session, supporting the closure, resumes with its selection.
        let (resumed, frames) = admit(&authority, &resume(&[4, 6, 13, 17, 18])).await?;
        let resumed = resumed.map_err(|end| format!("{end:?}"))?;
        assert!(
            resumed
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_WORLD_MAP_VIEW_V1)
        );
        assert_eq!(frames.len(), 1);
        Ok(())
    })
}

#[test]
fn a_fixture_world_offers_exactly_the_production_set_and_never_18() -> Result<(), Box<dyn Error>> {
    run(async {
        // MAP-CUTOVER-1b: a World without a bundle keeps the production set and domain 2.
        let authority = NegotiatingAuthority::new(None);
        assert_eq!(
            authority.offered_capabilities(),
            PRODUCTION_OFFERED_CAPABILITIES
        );
        assert!(authority.required.is_none());
        let (admitted, frames) = admit(&authority, &bootstrap(&[4, 6, 13, 17, 18])?).await?;
        let selected = accepted_selection(&frames)?;
        assert!(!selected.contains(&CAPABILITY_WORLD_MAP_VIEW_V1));
        admitted.map_err(|end| format!("{end:?}"))?;
        Ok(())
    })
}

/// Tibian noon (minute 720) of the default epoch: day light 250, color 215.
const NOON_UTC_MS: u64 = 720 * 2_500;

#[test]
fn admission_selecting_19_sends_the_domain_18_snapshot_at_the_carried_revision()
-> Result<(), Box<dyn Error>> {
    run(async {
        // CLOCK-1 (§11.3): the snapshot reads the World clock at the authority's time fact.
        use oteryn_protocol_oteryn::world_clock::{WorldClock, encode_world_clock};
        let noon = encode_world_clock(WorldClock {
            tibian_minute: 720,
            light_level: 250,
            light_color: 215,
        })
        .map_err(|error| format!("{error:?}"))?;
        let carries_noon = |frames: &[Vec<u8>]| {
            frames
                .concat()
                .windows(noon.len())
                .any(|w| w == noon.as_slice())
        };
        let authority = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(&authority, &bootstrap(&[6, 19])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6, 19]);
        let mut admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(admitted.continuity.world_clock_revision, 0);
        admitted.continuity.world_clock_revision = 7;
        let (end, frames) = serve(&authority, admitted, &[]).await?;
        assert!(carries_noon(&frames));
        // A resume or transfer never winds the revision back.
        let ConnectionEnd::AdmittedThenDisconnected(served) = end else {
            return Err(format!("{end:?}").into());
        };
        assert_eq!(served.continuity.world_clock_revision, 7);
        let (admitted, _) = admit(&authority, &bootstrap(&[6])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let (_, frames) = serve(&authority, admitted, &[]).await?;
        assert!(!carries_noon(&frames));
        Ok(())
    })
}

#[test]
fn item_view_admission_selecting_4_sends_domain_9_and_without_4_none() -> Result<(), Box<dyn Error>>
{
    run(async {
        // MAP-ITEM-REF-1: the Item view offered set selects 4 with its required 6, and the
        // admitted session is sent domain 9; a session admitted without 4 is sent none. The
        // fixture serves no combat state, so the client here does not support 17.
        let authority = NegotiatingAuthority::new(Some(ITEM_VIEW_OFFERED_CAPABILITIES));
        let inventory = authority
            .observe_character_inventory(
                ExactActorRef::transport_fixture(
                    WorldId::decode(&WORLD)?,
                    ChannelId::decode(&CHANNEL)?,
                ),
                GameSessionId::decode(&SESSION)?,
            )
            .await
            .ok_or("inventory")?;
        let [nine, _] = SessionItemView::resume(ItemViewContinuity::default())
            .snapshot(inventory, None)
            .map_err(|error| format!("{error:?}"))?;
        let carries_nine = |frames: &[Vec<u8>]| {
            frames
                .concat()
                .windows(nine.payload.len())
                .any(|w| w == nine.payload)
        };
        let (admitted, frames) = admit(&authority, &bootstrap(&[4, 6, 13])?).await?;
        assert_eq!(accepted_selection(&frames)?, [4, 6, 13]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let (_, frames) = serve(&authority, admitted, &[]).await?;
        assert!(carries_nine(&frames));
        let (admitted, frames) = admit(&authority, &bootstrap(&[6, 13])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6, 13]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let (_, frames) = serve(&authority, admitted, &[]).await?;
        assert!(!frames.is_empty());
        assert!(!carries_nine(&frames));
        Ok(())
    })
}

#[test]
fn resume_keeps_the_original_selection_and_a_downgrade_falls_back_to_fresh_admission()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let (admitted, _) = admit(&authority, &bootstrap(&[1, 8])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        authority.lost.set(Some(admitted));

        // A resume that supports more keeps exactly the original selection.
        let (resumed, frames) = admit(&authority, &resume(&[1, 7, 8, 10])).await?;
        let resumed = resumed.map_err(|end| format!("{end:?}"))?;
        assert_eq!(resumed.continuity.selected_capabilities.as_slice(), [1, 8]);
        let expected = encode_server_resume_accepted(&ServerResumeAcceptedValue {
            game_session_id: resumed.game_session_id,
            connection_generation: resumed.continuity.connection_generation,
            current_server_sequence: resumed.continuity.server_sequence,
            next_command_id: resumed.continuity.next_command_id,
            schema_revision: 1,
            selected_capabilities: &[1, 8],
        })?;
        assert_eq!(frames, [expected]);

        // A resume that lacks selected capability 8 is not acknowledged: the client falls back
        // to fresh admission, which negotiates again from what it now supports.
        let (refused, frames) = admit(&authority, &resume(&[1])).await?;
        assert!(frames.is_empty());
        assert!(matches!(
            refused,
            Err(ConnectionEnd::AdmittedThenDisconnected(_))
        ));
        let (fresh, frames) = admit(&authority, &bootstrap(&[1])?).await?;
        assert!(fresh.is_ok());
        assert_eq!(accepted_selection(&frames)?, [1]);
        Ok(())
    })
}

#[test]
fn an_unselected_capability_command_is_refused_and_a_core_one_dispatched()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North);
        let frames = [
            command(1, 13, b"chat"),
            command(2, 4, b"charm"),
            command(3, 1, &step),
        ];
        let (_, written) = serve(&authority, session(SessionContinuity::FRESH)?, &frames).await?;
        let rejected = |sequence, id| {
            crate::foundation::encode_command_result(
                1,
                sequence,
                id,
                crate::foundation::CommandStatus::Rejected,
                &[],
            )
        };
        assert!(written.contains(&rejected(1, 1)?));
        assert!(written.contains(&rejected(2, 2)?));
        assert_eq!(authority.steps.get(), 1);
        Ok(())
    })
}

#[test]
fn an_unselected_domain_is_never_sent() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let spatial = encode_world_spatial(&WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        });
        let notices = encode_achievement_notices_snapshot(AchievementWatermark {
            fact_count: 1,
            total_points: 3,
        });
        let mut domains = vec![DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 1,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &spatial,
        }];
        // The registered overlay domain is always sent, empty at revision 0.
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 0,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &[],
        });
        let core: Vec<Vec<u8>> = encode_single_chunk_snapshot(1, 1, 0, &domains)?.into();
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            revision: 0,
            snapshot_type: SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
            payload: &notices,
        });
        let with_notices: Vec<Vec<u8>> = encode_single_chunk_snapshot(1, 1, 0, &domains)?.into();

        // A notice revision without capability 8 selected sends nothing of domain 13.
        let unselected = SessionContinuity {
            achievement_notice_revision: Some(0),
            ..SessionContinuity::FRESH
        };
        let (_, frames) = serve(&authority, session(unselected)?, &[]).await?;
        assert_eq!(frames, core);
        let selected = SessionContinuity {
            achievement_notice_revision: Some(0),
            selected_capabilities: selection(&[8]),
            ..SessionContinuity::FRESH
        };
        let (_, frames) = serve(&authority, session(selected)?, &[]).await?;
        assert_eq!(frames, with_notices);
        Ok(())
    })
}

// ITEM-USE-WIRE-1: capability 15 ITEM_USE_V1 is registered and gated, not offered, and
// requires 4; fields 4 and 5 of USE_INTENT are REJECTED without it.

/// An injected offered set: 4 alone and 15 requiring 4.
const ITEM_USE_OFFERED: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_ITEM_USE_V1,
        requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1],
    },
];

#[test]
fn item_use_wire_capability_15_is_registered_not_offered_and_requires_4()
-> Result<(), Box<dyn Error>> {
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_ITEM_USE_V1)
        .ok_or("capability 15")?;
    assert_eq!(entry["offered"], false);
    assert_eq!(ids(&entry["requires"])?, [CAPABILITY_ITEM_VIEW_MOVE_V1]);
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_ITEM_USE_V1)
    );
    // CAP-NEG-1: 15 without 4 is never selected.
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(ITEM_USE_OFFERED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    assert_eq!(select(&[15]), Vec::<u32>::new());
    assert_eq!(select(&[4, 15]), [4, 15]);
    // It owns no command type or domain: it selects none of capability 4's.
    let selected = selection(&[15]);
    assert!(!selected.command_selected(COMMAND_TYPE_ITEM_MOVE_INTENT));
    assert!(!selected.domain_selected(STATE_DOMAIN_CHARACTER_INVENTORY));
    assert!(!selected.domain_selected(STATE_DOMAIN_OPEN_CONTAINER));
    Ok(())
}

#[test]
fn item_use_wire_a_resume_that_drops_15_falls_back_to_fresh_admission() {
    let original = SelectedCapabilities::select(ITEM_USE_OFFERED, &[4, 15]).expect("bounded");
    assert_eq!(original.as_slice(), [4, 15]);
    assert!(original.resumable_with(&[4, 15]));
    // A resume whose supported set lacks 15 (or 4) would change the set: it is refused as
    // resume-unavailable and the client falls back to fresh admission.
    assert!(!original.resumable_with(&[4]));
    assert!(!original.resumable_with(&[15]));
    // A session admitted without 15 never gains it on resume.
    let without = SelectedCapabilities::select(ITEM_USE_OFFERED, &[4]).expect("bounded");
    assert!(without.resumable_with(&[4, 15]));
    assert_eq!(without.as_slice(), [4]);
}

/// One use result as written: command ID, status and payload.
type UseResult = (u64, crate::foundation::CommandStatus, Vec<u8>);

#[test]
fn item_use_wire_fields_4_and_5_are_rejected_without_capability_15() -> Result<(), Box<dyn Error>> {
    use super::super::item_view::UseItemTarget;
    use crate::foundation::CommandStatus;
    use oteryn_protocol_oteryn::decode_command_result;
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
        encode_use_result,
    };
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    use std::num::NonZeroU32;
    run(async {
        let authority = NegotiatingAuthority::new(Some(ITEM_USE_OFFERED));
        let creature = Some(EntityRef {
            identity: [0x5c; 16],
            generation: 1,
        });
        let with = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(ItemHandle::MIN),
            use_with: creature,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let by_definition = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(NonZeroU32::MIN),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let by_definition_with = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(NonZeroU32::MIN),
            use_with: creature,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let frames = [
            command(1, COMMAND_TYPE_USE_INTENT, &with),
            command(2, COMMAND_TYPE_USE_INTENT, &by_definition),
            command(3, COMMAND_TYPE_USE_INTENT, &by_definition_with),
        ];
        // The use results written, by command ID.
        let results = |written: &[Vec<u8>]| -> Result<Vec<UseResult>, Box<dyn Error>> {
            let mut results = Vec::new();
            for frame in written {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload())?;
                    results.push((result.command_id, result.status, result.payload.to_vec()));
                }
            }
            Ok(results)
        };
        let rejected = (
            CommandStatus::Rejected,
            encode_use_result(UseDisposition::Rejected).unwrap_or_default(),
        );
        for selected in [&[][..], &[CAPABILITY_ITEM_VIEW_MOVE_V1]] {
            let continuity = SessionContinuity {
                selected_capabilities: selection(selected),
                ..SessionContinuity::FRESH
            };
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            let expected: Vec<_> = (1..=3)
                .map(|id| (id, rejected.0, rejected.1.clone()))
                .collect();
            assert_eq!(results(&written)?, expected, "{selected:?}");
        }
        // Under capability 15, before ITEM-USE-1: a use on a creature fails closed, a use by
        // definition finds nothing to use, a corpse handle still opens and any other item handle
        // still finds nothing to use.
        let backpack = ItemHandle::MIN;
        let use_handle = |id| {
            command(
                id,
                COMMAND_TYPE_USE_INTENT,
                &oteryn_protocol_oteryn::world_object::encode_use_item_intent(backpack),
            )
        };
        let with_backpack = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(backpack),
            use_with: creature,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let item_use = selection(&[CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_USE_V1]);
        let here = ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        };
        for (target, disposition) in [
            (UseItemTarget::NotACorpse, UseDisposition::NothingToUse),
            (
                UseItemTarget::Corpse {
                    position: ActorPosition {
                        x: 1,
                        y: 0,
                        floor: 0,
                    },
                    contents: Vec::new(),
                },
                UseDisposition::Committed,
            ),
        ] {
            *authority.item_target.borrow_mut() = Some(ItemTargetObservation {
                actor: here,
                target,
            });
            let continuity = SessionContinuity {
                selected_capabilities: item_use,
                ..SessionContinuity::FRESH
            };
            let frames = [
                command(1, COMMAND_TYPE_USE_INTENT, &with_backpack),
                command(2, COMMAND_TYPE_USE_INTENT, &by_definition),
                command(3, COMMAND_TYPE_USE_INTENT, &by_definition_with),
                use_handle(4),
            ];
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            assert_eq!(
                results(&written)?,
                [
                    (1, rejected.0, rejected.1.clone()),
                    (
                        2,
                        CommandStatus::Accepted,
                        encode_use_result(UseDisposition::NothingToUse).unwrap_or_default()
                    ),
                    (3, rejected.0, rejected.1.clone()),
                    (
                        4,
                        CommandStatus::Accepted,
                        encode_use_result(disposition).unwrap_or_default()
                    ),
                ],
                "{disposition:?}"
            );
        }
        Ok(())
    })
}

// RUNE-WIRE-1 (RUNE-USE-0 §4): capability 22 RUNE_USE_V1 is registered, not offered before
// RUNE-CAST-1, and requires 15; the USE position arm is REJECTED with or without it.

/// An injected offered set: 4, 15 requiring 4 and 22 requiring 15.
const RUNE_USE_OFFERED: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_ITEM_USE_V1,
        requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1],
    },
    OfferedCapability {
        id: oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1,
        requires: &[CAPABILITY_ITEM_USE_V1],
    },
];

#[test]
fn rune_wire_capability_21_is_registered_not_offered_and_requires_15() -> Result<(), Box<dyn Error>>
{
    use oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1;
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_RUNE_USE_V1)
        .ok_or("capability 22")?;
    assert_eq!(entry["offered"], false);
    assert_eq!(ids(&entry["requires"])?, [CAPABILITY_ITEM_USE_V1]);
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_RUNE_USE_V1)
    );
    // CAP-NEG-1: 22 without 15 (or 15 without 4) is never selected.
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(RUNE_USE_OFFERED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    assert_eq!(select(&[22]), Vec::<u32>::new());
    assert_eq!(select(&[4, 22]), [4]);
    assert_eq!(select(&[15, 22]), Vec::<u32>::new());
    assert_eq!(select(&[4, 15, 22]), [4, 15, 22]);
    // It owns no command type or domain.
    let selected = selection(&[22]);
    assert!(!selected.command_selected(COMMAND_TYPE_ITEM_MOVE_INTENT));
    assert!(!selected.domain_selected(STATE_DOMAIN_CHARACTER_INVENTORY));
    Ok(())
}

#[test]
fn rune_wire_the_position_arm_is_rejected_with_or_without_capability_21()
-> Result<(), Box<dyn Error>> {
    use crate::foundation::CommandStatus;
    use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
    use oteryn_protocol_oteryn::decode_command_result;
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    use oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1;
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
        encode_use_result,
    };
    run(async {
        let authority = NegotiatingAuthority::new(Some(RUNE_USE_OFFERED));
        let at_position = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(ItemHandle::MIN),
            use_with: None,
            use_with_map_item: None,
            use_with_position: Some(SpellTargetPosition {
                x: 100,
                y: -100,
                floor: 7,
            }),
        })
        .map_err(|error| format!("{error:?}"))?;
        let frames = [command(1, COMMAND_TYPE_USE_INTENT, &at_position)];
        let rejected = encode_use_result(UseDisposition::Rejected).unwrap_or_default();
        // Without 22 the arm is undecodable; under 22 it fails closed until RUNE-CAST-1.
        for selected in [
            &[][..],
            &[CAPABILITY_ITEM_VIEW_MOVE_V1],
            &[CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_USE_V1],
            &[
                CAPABILITY_ITEM_VIEW_MOVE_V1,
                CAPABILITY_ITEM_USE_V1,
                CAPABILITY_RUNE_USE_V1,
            ],
        ] {
            let continuity = SessionContinuity {
                selected_capabilities: selection(selected),
                ..SessionContinuity::FRESH
            };
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            let mut results = Vec::new();
            for frame in &written {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload())?;
                    results.push((result.command_id, result.status, result.payload.to_vec()));
                }
            }
            assert_eq!(
                results,
                [(1, CommandStatus::Rejected, rejected.clone())],
                "{selected:?}"
            );
        }
        Ok(())
    })
}

/// ITEM-USE-1c: a main backpack entry is used only under capability 15; without it the handle
/// keeps the corpse path and the use is never reached.
#[test]
fn a_backpack_entry_is_used_only_under_capability_15() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::decode_command_result;
    use oteryn_protocol_oteryn::world_object::encode_use_item_intent;
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, encode_use_result,
    };
    run(async {
        let authority = NegotiatingAuthority::new(Some(ITEM_USE_OFFERED));
        authority.backpack_entry.set(true);
        let entry = std::num::NonZeroU64::new(2).ok_or("handle")?;
        let frames = [command(
            1,
            COMMAND_TYPE_USE_INTENT,
            &encode_use_item_intent(entry),
        )];
        for (selected, used, disposition) in [
            (
                &[CAPABILITY_ITEM_VIEW_MOVE_V1][..],
                0,
                UseDisposition::StaleState,
            ),
            (
                &[CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_USE_V1][..],
                1,
                UseDisposition::Full,
            ),
        ] {
            let continuity = SessionContinuity {
                selected_capabilities: selection(selected),
                ..SessionContinuity::FRESH
            };
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            let mut payloads = Vec::new();
            for frame in &written {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    payloads.push(decode_command_result(envelope.payload())?.payload.to_vec());
                }
            }
            assert_eq!(
                payloads,
                [encode_use_result(disposition).unwrap_or_default()],
                "{selected:?}"
            );
            assert_eq!(authority.item_uses.borrow().len(), used, "{selected:?}");
        }
        assert_eq!(
            *authority.item_uses.borrow(),
            [ItemKey::Instance([0xa6; 16])]
        );
        Ok(())
    })
}

// MAP-WIRE-2: capability 18 WORLD_MAP_VIEW_V1 is registered and gated, not offered (MAP-CUTOVER-1
// offers it), and requires 4 and 6; domain 17 is never sent without it.

/// An injected offered set: 4, 6 and 18 requiring both.
const WORLD_MAP_OFFERED: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_MAP_VIEW_V1,
        requires: &[
            CAPABILITY_ITEM_VIEW_MOVE_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
        ],
    },
];

#[test]
fn map_wire_capability_18_is_registered_not_offered_and_requires_4_and_6()
-> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::world_map::{
        CAPABILITY_WORLD_MAP_VIEW_REQUIRES, STATE_DOMAIN_WORLD_MAP_VIEW,
    };
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_WORLD_MAP_VIEW_V1)
        .ok_or("capability 18")?;
    assert_eq!(entry["offered"], false);
    assert_eq!(ids(&entry["requires"])?, CAPABILITY_WORLD_MAP_VIEW_REQUIRES);
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_WORLD_MAP_VIEW_V1)
    );
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(WORLD_MAP_OFFERED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    // CAP-NEG-1: 18 without 4 or without 6 is never selected.
    assert_eq!(select(&[6, 18]), [6]);
    assert_eq!(select(&[4, 18]), [4]);
    assert_eq!(select(&[18]), Vec::<u32>::new());
    assert_eq!(select(&[4, 6, 18]), [4, 6, 18]);
    // Domain 17 is sent only with 18 selected.
    assert!(!selection(&[4, 6]).domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW));
    assert!(selection(&[4, 6, 18]).domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW));
    // A resume that drops 18 falls back to fresh admission.
    let original = SelectedCapabilities::select(WORLD_MAP_OFFERED, &[4, 6, 18]).expect("bounded");
    assert!(!original.resumable_with(&[4, 6]));
    Ok(())
}

#[test]
fn npc_wire_capability_3_is_registered_not_offered_and_gates_commands_7_and_8()
-> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::npc_service::{
        CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT,
        STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW,
    };
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_NPC_SERVICE_V1)
        .ok_or("capability 3")?;
    assert_eq!(entry["name"], "NPC_SERVICE_V1");
    assert_eq!(entry["offered"], false);
    assert!(entry.get("requires").is_none());
    assert_eq!(
        ids(&entry["command_types"])?,
        [COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT]
    );
    assert_eq!(
        ids(&entry["state_domains"])?,
        [STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW]
    );
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_NPC_SERVICE_V1)
    );
    // Without 3 selected no domain 7 or 8 is sent and no command 7 or 8 is dispatched.
    let none = SelectedCapabilities::NONE;
    assert!(!none.command_selected(COMMAND_TYPE_NPC_TALK_INTENT));
    assert!(!none.command_selected(COMMAND_TYPE_NPC_TRADE_INTENT));
    assert!(!none.domain_selected(STATE_DOMAIN_NPC_CONVERSATION));
    assert!(!none.domain_selected(STATE_DOMAIN_NPC_TRADE_WINDOW));
    run(async {
        // A client asking for capability 3 is not granted it.
        let authority = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(&authority, &bootstrap(&[1, 3, 6])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert!(!admitted.continuity.selected_capabilities.contains(3));
        // Commands 7 and 8 are refused as unsupported for everyone.
        let frames = [command(1, 7, b"hi"), command(2, 8, b"buy")];
        let (_, written) = serve(&authority, session(SessionContinuity::FRESH)?, &frames).await?;
        let rejected = |sequence, id| {
            crate::foundation::encode_command_result(
                1,
                sequence,
                id,
                crate::foundation::CommandStatus::Rejected,
                &[],
            )
        };
        assert!(written.contains(&rejected(1, 1)?));
        assert!(written.contains(&rejected(2, 2)?));
        Ok(())
    })
}

// CHAT-WIRE-1: capability 7 over two connections of one Channel — a say reaches the hearer in
// range as a domain 12 line delta and never the session out of range.

mod chat_wire {
    use super::super::super::actor_spell::tests::runtime_with_capacity;
    use super::super::super::chat_intent::{ChatRuntime, chat_in_channel};
    use super::*;
    use crate::foundation::CommandStatus;
    use oteryn_protocol_oteryn::chat::{
        ChatDisposition, ChatIntent, ChatLine, ChatRoomSet, ChatSpeechMode,
        decode_chat_intent_result, decode_chat_line, encode_chat_intent,
    };
    use oteryn_protocol_oteryn::{decode_command_result, decode_state_delta};
    use oteryn_simulation_determinism::SemanticTimeMicros;
    use std::task::Poll;
    use std::time::Duration;

    const NOW: SemanticTimeMicros = SemanticTimeMicros::from_micros(1_000_000);

    struct ChatAuthority {
        runtime: RefCell<crate::foundation::ChannelRuntimeV1>,
        chat: ChatRuntime,
        names: Vec<(GameSessionId, &'static str)>,
    }

    impl FreshAdmissionAuthority for ChatAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Unavailable)
        }

        async fn resume(
            &self,
            _attempt: ResumeAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(WorldSpatialObservation {
                content_generation: [0x5c; 32],
                actor_position: ActorPosition {
                    x: 0,
                    y: 0,
                    floor: 0,
                },
            })
        }

        async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
            StepOutcome {
                disposition: StepDisposition::Blocked,
                moved_to: None,
            }
        }

        async fn chat_admit(&self, _actor: ExactActorRef, session: GameSessionId) -> ChatRoomSet {
            if let Some((_, name)) = self.names.iter().find(|(known, _)| *known == session) {
                self.chat.admit(session, name, NOW);
            }
            ChatRoomSet::default()
        }

        async fn chat_intent(
            &self,
            actor: ExactActorRef,
            session: GameSessionId,
            intent: ChatIntent,
        ) -> oteryn_protocol_oteryn::chat::ChatIntentResult {
            if let Some(done) = self.chat.pre_gate(session, &intent, NOW) {
                return done;
            }
            chat_in_channel(
                &self.chat,
                &self.runtime.borrow(),
                &Default::default(),
                actor,
                session,
                &intent,
                NOW,
            )
        }

        async fn chat_next(&self, session: GameSessionId) -> Option<ChatLine> {
            self.chat.next(session)
        }

        async fn chat_delivered(&self, session: GameSessionId) {
            self.chat.delivered(session);
        }

        async fn chat_rooms_changed(&self, session: GameSessionId) -> Option<ChatRoomSet> {
            self.chat.take_rooms_change(session)
        }
    }

    fn admitted(
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<AdmittedSession, Box<dyn Error>> {
        let mut admitted = super::session(SessionContinuity {
            selected_capabilities: super::selection(&[7]),
            ..SessionContinuity::FRESH
        })?;
        admitted.game_session_id = session;
        admitted.runtime_actor = Some(actor);
        Ok(admitted)
    }

    /// Serves `admitted` over `frames`, then keeps the client open for `linger`.
    async fn serve_lingering(
        authority: &ChatAuthority,
        admitted: AdmittedSession,
        frames: &[Vec<u8>],
        linger: Duration,
    ) -> Result<(AdmittedSession, Vec<Vec<u8>>), Box<dyn Error>> {
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
        for frame in frames {
            let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
            framed.extend_from_slice(frame);
            client.write_all(&framed).await?;
        }
        let closing = async {
            tokio::time::sleep(linger).await;
            client.shutdown().await?;
            let mut output = Vec::new();
            client.read_to_end(&mut output).await?;
            Ok::<_, std::io::Error>(output)
        };
        let serving = async move {
            let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
            drop(server);
            end
        };
        let (mut serving, mut closing) = (std::pin::pin!(serving), std::pin::pin!(closing));
        let (mut end, mut output) = (None, None);
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if output.is_none()
                && let Poll::Ready(value) = closing.as_mut().poll(context)
            {
                output = Some(value);
            }
            if end.is_some() && output.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        let ConnectionEnd::AdmittedThenDisconnected(admitted) = end.ok_or("end")? else {
            return Err("the connection did not end as a disconnect".into());
        };
        Ok((admitted, split(&output.ok_or("output")??)?))
    }

    fn lines(frames: &[Vec<u8>]) -> Result<Vec<ChatLine>, Box<dyn Error>> {
        let mut lines = Vec::new();
        for frame in frames {
            let envelope = decode_wire_envelope(frame)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(envelope.payload())?;
                if delta.domain_id == 12 && delta.delta_type == 1 {
                    assert_eq!(delta.new_revision, delta.base_revision + 1);
                    lines.push(decode_chat_line(delta.payload).map_err(|e| format!("{e:?}"))?);
                }
            }
        }
        Ok(lines)
    }

    #[test]
    fn a_say_reaches_the_session_in_range_and_not_the_one_out_of_range()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let (mut runtime, speaker, speaker_session) = runtime_with_capacity(0x71, 3);
            let mut others = Vec::new();
            for tag in [0x72, 0x73] {
                let session = GameSessionId::decode(&[
                    0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
                ])?;
                let reservation = runtime
                    .reserve_fresh_session(session)
                    .map_err(|e| format!("{e:?}"))?;
                let actor = runtime
                    .commit_fresh_session(reservation)
                    .map_err(|e| format!("{e:?}"))?;
                others.push((actor, session));
            }
            for actor in [speaker, others[0].0, others[1].0] {
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(|e| format!("{e:?}"))?;
            }
            for _ in 0..40 {
                let at = runtime
                    .read_actor_position(others[1].0)
                    .map_err(|e| format!("{e:?}"))?;
                let mut next = at.position();
                next.x += 1;
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(at, next)
                    .map_err(|e| format!("{e:?}"))?;
            }
            let (near, far) = (others[0], others[1]);
            let authority = ChatAuthority {
                runtime: RefCell::new(runtime),
                chat: ChatRuntime::default(),
                names: vec![(speaker_session, "Aela"), (near.1, "Bryn"), (far.1, "Cade")],
            };
            let say = encode_chat_intent(&ChatIntent::Say {
                mode: ChatSpeechMode::Say,
                text: "hello there".to_owned(),
            })
            .map_err(|e| format!("{e:?}"))?;
            let linger = Duration::from_millis(700);
            let hearing = serve_lingering(&authority, admitted(near.0, near.1)?, &[], linger);
            let missing = serve_lingering(&authority, admitted(far.0, far.1)?, &[], linger);
            let speaking = async {
                tokio::time::sleep(Duration::from_millis(150)).await;
                serve_lingering(
                    &authority,
                    admitted(speaker, speaker_session)?,
                    &[command(1, 13, &say)],
                    Duration::ZERO,
                )
                .await
            };
            let (hearing, missing, speaking) = join3(hearing, missing, speaking).await;
            let (near_end, near_frames) = hearing?;
            let (far_end, far_frames) = missing?;
            let (_, speaker_frames) = speaking?;

            let mut results = Vec::new();
            for frame in &speaker_frames {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload())?;
                    assert_eq!(result.status, CommandStatus::Accepted);
                    results.push(
                        decode_chat_intent_result(result.payload).map_err(|e| format!("{e:?}"))?,
                    );
                }
            }
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].disposition, ChatDisposition::Ok);

            let heard = lines(&near_frames)?;
            assert_eq!(heard.len(), 1, "{heard:?}");
            assert!(matches!(
                &heard[0],
                ChatLine::Local { speaker_name, text, .. }
                    if speaker_name == "Aela" && text == "hello there"
            ));
            // The snapshot (revision 1) and the line (revision 2); the far session has the
            // snapshot only.
            assert_eq!(near_end.continuity.chat_revision, 2);
            assert!(lines(&far_frames)?.is_empty());
            assert_eq!(far_end.continuity.chat_revision, 1);
            Ok(())
        })
    }

    #[test]
    fn an_opened_room_follows_its_result_as_one_rooms_delta() -> Result<(), Box<dyn Error>> {
        use crate::chat::relay::{RelayKey, RelayKeyRing};
        use oteryn_protocol_oteryn::chat::{ChatRoom, DELTA_TYPE_CHAT_ROOMS_V1, decode_chat_rooms};
        use std::str::FromStr as _;
        run(async {
            let (runtime, actor, session) = runtime_with_capacity(0x71, 3);
            let authority = ChatAuthority {
                runtime: RefCell::new(runtime),
                chat: ChatRuntime::default(),
                names: vec![(session, "Aela")],
            };
            // A configured relay that never connects: opening a room needs no listener.
            let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [1; 32])])
                .map_err(|e| format!("{e:?}"))?;
            let options =
                sqlx::postgres::PgConnectOptions::from_str("postgresql://nobody@127.0.0.1:1/none")?;
            let world = [0x01, 0x90, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
            let relay = super::super::super::chat_relay::ChatRelay::new(ring, world, options)
                .map_err(|e| format!("{e:?}"))?;
            authority.chat.install_relay(std::sync::Arc::new(relay));
            let open = encode_chat_intent(&ChatIntent::OpenRoom(ChatRoom::Help))
                .map_err(|e| format!("{e:?}"))?;
            let (end, frames) = serve_lingering(
                &authority,
                admitted(actor, session)?,
                &[command(1, 13, &open), command(2, 13, &open)],
                Duration::from_millis(300),
            )
            .await?;
            let mut rooms = Vec::new();
            let mut results = Vec::new();
            for frame in &frames {
                let envelope = decode_wire_envelope(frame)?;
                match envelope.message_type() {
                    MessageType::StateDelta => {
                        let delta = decode_state_delta(envelope.payload())?;
                        if delta.domain_id == 12 && delta.delta_type == DELTA_TYPE_CHAT_ROOMS_V1 {
                            rooms.push(
                                decode_chat_rooms(delta.payload).map_err(|e| format!("{e:?}"))?,
                            );
                        }
                    }
                    MessageType::CommandResult => {
                        let result = decode_command_result(envelope.payload())?;
                        results.push(
                            decode_chat_intent_result(result.payload)
                                .map_err(|e| format!("{e:?}"))?
                                .disposition,
                        );
                    }
                    _ => {}
                }
            }
            assert_eq!(results, vec![ChatDisposition::Ok, ChatDisposition::Ok]);
            // The second open changes nothing, so it sends no delta.
            assert_eq!(rooms.len(), 1, "{rooms:?}");
            assert!(rooms[0].contains(ChatRoom::Help));
            assert_eq!(end.continuity.chat_revision, 2);
            Ok(())
        })
    }

    async fn join3<A: Future, B: Future, C: Future>(
        a: A,
        b: B,
        c: C,
    ) -> (A::Output, B::Output, C::Output) {
        let (mut a, mut b, mut c) = (std::pin::pin!(a), std::pin::pin!(b), std::pin::pin!(c));
        let (mut x, mut y, mut z) = (None, None, None);
        std::future::poll_fn(|context| {
            if x.is_none()
                && let Poll::Ready(value) = a.as_mut().poll(context)
            {
                x = Some(value);
            }
            if y.is_none()
                && let Poll::Ready(value) = b.as_mut().poll(context)
            {
                y = Some(value);
            }
            if z.is_none()
                && let Poll::Ready(value) = c.as_mut().poll(context)
            {
                z = Some(value);
            }
            if x.is_some() && y.is_some() && z.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        (x.expect("first"), y.expect("second"), z.expect("third"))
    }
}

// LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6): command 23 on the connection, behind an
// authority that answers each logout with a scripted reply.

mod logout_wire {
    use super::super::super::connection::LogoutReply;
    use super::*;
    use crate::foundation::CommandStatus;
    use oteryn_protocol_oteryn::logout::{
        LogoutResult, decode_logout_result, encode_logout_intent,
    };
    use oteryn_protocol_oteryn::{decode_command_result, decode_protocol_error};
    use std::time::Duration;

    struct LogoutAuthority {
        reply: Cell<LogoutReply>,
        /// How long one logout takes; frames read meanwhile are held.
        delay: Duration,
        logouts: Cell<u32>,
        steps: Cell<u32>,
        /// SPEED-1: the pace each step records; zero leaves steps unpaced.
        pace: Duration,
    }

    impl LogoutAuthority {
        fn new(reply: LogoutReply) -> Self {
            Self {
                reply: Cell::new(reply),
                delay: Duration::ZERO,
                logouts: Cell::new(0),
                steps: Cell::new(0),
                pace: Duration::ZERO,
            }
        }
    }

    impl FreshAdmissionAuthority for LogoutAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Unavailable)
        }

        async fn resume(
            &self,
            _attempt: ResumeAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(WorldSpatialObservation {
                content_generation: [0x5c; 32],
                actor_position: ActorPosition {
                    x: 0,
                    y: 0,
                    floor: 0,
                },
            })
        }

        async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
            self.steps.set(self.steps.get() + 1);
            StepOutcome {
                disposition: StepDisposition::Blocked,
                moved_to: None,
            }
        }

        async fn paced_step(
            &self,
            actor: ExactActorRef,
            _session: GameSessionId,
            direction: StepDirection,
        ) -> (StepOutcome, Option<Duration>) {
            let pace = (!self.pace.is_zero()).then_some(self.pace);
            (self.step(actor, direction).await, pace)
        }

        async fn logout(&self, _admitted: AdmittedSession) -> LogoutReply {
            self.logouts.set(self.logouts.get() + 1);
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            self.reply.get()
        }
    }

    fn admitted(selected: &[u32]) -> Result<AdmittedSession, Box<dyn Error>> {
        super::session(SessionContinuity {
            selected_capabilities: super::selection(selected),
            ..SessionContinuity::FRESH
        })
    }

    /// Serves `admitted`; the client writes `frames`, stays open for `linger`, then closes.
    async fn serve_open(
        authority: &LogoutAuthority,
        admitted: AdmittedSession,
        frames: &[Vec<u8>],
        linger: Duration,
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
        for frame in frames {
            let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
            framed.extend_from_slice(frame);
            client.write_all(&framed).await?;
        }
        let closing = async {
            tokio::time::sleep(linger).await;
            client.shutdown().await?;
            let mut output = Vec::new();
            client.read_to_end(&mut output).await?;
            Ok::<_, std::io::Error>(output)
        };
        let serving = async move {
            let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
            drop(server);
            end
        };
        let (mut serving, mut closing) = (std::pin::pin!(serving), std::pin::pin!(closing));
        let (mut end, mut output) = (None, None);
        std::future::poll_fn(|context| {
            if end.is_none()
                && let std::task::Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if output.is_none()
                && let std::task::Poll::Ready(value) = closing.as_mut().poll(context)
            {
                output = Some(value);
            }
            if end.is_some() && output.is_some() {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        Ok((end.ok_or("end")?, split(&output.ok_or("output")??)?))
    }

    /// One written command result: (command id, status, payload).
    type Written = (u64, CommandStatus, Vec<u8>);

    /// The command results written.
    fn results(frames: &[Vec<u8>]) -> Result<Vec<Written>, Box<dyn Error>> {
        let mut results = Vec::new();
        for frame in frames {
            let envelope = decode_wire_envelope(frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload())?;
                results.push((result.command_id, result.status, result.payload.to_vec()));
            }
        }
        Ok(results)
    }

    fn logout_result(
        results: &[Written],
        id: u64,
    ) -> Result<(CommandStatus, LogoutResult), Box<dyn Error>> {
        let (_, status, payload) = results
            .iter()
            .find(|(command, _, _)| *command == id)
            .ok_or("no logout result")?;
        Ok((
            *status,
            decode_logout_result(payload).map_err(|e| format!("{e:?}"))?,
        ))
    }

    fn step() -> Vec<u8> {
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North)
    }

    #[test]
    fn without_capability_21_a_logout_is_refused_before_the_authority() -> Result<(), Box<dyn Error>>
    {
        run(async {
            let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
            let frames = [command(1, 23, &encode_logout_intent())];
            let (end, written) =
                serve_open(&authority, admitted(&[])?, &frames, Duration::ZERO).await?;
            assert!(
                matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                "{end:?}"
            );
            assert_eq!(
                results(&written)?,
                [(1, CommandStatus::Rejected, Vec::new())]
            );
            assert_eq!(authority.logouts.get(), 0);
            Ok(())
        })
    }

    #[test]
    fn a_non_empty_intent_is_rejected_with_an_empty_payload() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
            let frames = [command(1, 23, &[0x08, 0x01])];
            let (_, written) =
                serve_open(&authority, admitted(&[21])?, &frames, Duration::ZERO).await?;
            assert_eq!(
                results(&written)?,
                [(1, CommandStatus::Rejected, Vec::new())]
            );
            assert_eq!(authority.logouts.get(), 0);
            Ok(())
        })
    }

    #[test]
    fn accepted_is_written_then_the_connection_closes_without_reading_further()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
            let frames = [
                command(1, 23, &encode_logout_intent()),
                command(2, 1, &step()),
            ];
            let (end, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(50),
            )
            .await?;
            assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
            let results = results(&written)?;
            assert_eq!(results.len(), 1);
            assert_eq!(
                logout_result(&results, 1)?,
                (CommandStatus::Accepted, LogoutResult::Accepted)
            );
            assert_eq!(authority.steps.get(), 0);
            Ok(())
        })
    }

    #[test]
    fn in_fight_is_rejected_with_its_retry_and_the_session_keeps_playing()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::InFight {
                retry_after_ms: 1_500,
            }));
            let frames = [
                command(1, 23, &encode_logout_intent()),
                command(2, 1, &step()),
            ];
            let (end, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(50),
            )
            .await?;
            assert!(
                matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                "{end:?}"
            );
            let results = results(&written)?;
            assert_eq!(
                logout_result(&results, 1)?,
                (
                    CommandStatus::Rejected,
                    LogoutResult::InFight {
                        retry_after_ms: 1_500
                    }
                )
            );
            assert!(results.iter().any(|(id, _, _)| *id == 2));
            assert_eq!(authority.steps.get(), 1);
            Ok(())
        })
    }

    #[test]
    fn busy_is_rejected_and_a_step_held_behind_the_logout_runs_after_it()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
            authority.delay = Duration::from_millis(30);
            let frames = [
                command(1, 23, &encode_logout_intent()),
                command(2, 1, &step()),
            ];
            let (_, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(100),
            )
            .await?;
            let results = results(&written)?;
            assert_eq!(
                results.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
                [1, 2]
            );
            assert_eq!(
                logout_result(&results, 1)?,
                (CommandStatus::Rejected, LogoutResult::Busy)
            );
            assert_eq!(authority.steps.get(), 1);
            Ok(())
        })
    }

    #[test]
    fn an_unknown_outcome_writes_no_result_and_ends_as_a_disconnect() -> Result<(), Box<dyn Error>>
    {
        run(async {
            let authority = LogoutAuthority::new(LogoutReply::Unknown);
            let frames = [
                command(1, 23, &encode_logout_intent()),
                command(2, 1, &step()),
            ];
            let (end, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(50),
            )
            .await?;
            assert!(
                matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                "{end:?}"
            );
            assert!(results(&written)?.is_empty());
            assert_eq!(authority.steps.get(), 0);
            Ok(())
        })
    }

    #[test]
    fn a_second_logout_before_the_first_result_is_a_protocol_error() -> Result<(), Box<dyn Error>> {
        run(async {
            let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
            authority.delay = Duration::from_millis(30);
            let frames = [
                command(1, 23, &encode_logout_intent()),
                command(2, 23, &encode_logout_intent()),
            ];
            let (_, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(100),
            )
            .await?;
            assert_eq!(authority.logouts.get(), 1);
            let results = results(&written)?;
            assert_eq!(results.len(), 1);
            assert_eq!(
                logout_result(&results, 1)?,
                (CommandStatus::Rejected, LogoutResult::Busy)
            );
            let last = decode_wire_envelope(written.last().ok_or("no frame")?)?;
            assert_eq!(last.message_type(), MessageType::ProtocolError);
            assert_eq!(
                decode_protocol_error(last.payload())?.error_code,
                FoundationProtocolError::TooManyOutstandingCommands.code()
            );
            Ok(())
        })
    }

    #[test]
    fn a_second_logout_before_an_accepted_result_is_a_protocol_error_after_the_logout()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
            authority.delay = Duration::from_millis(30);
            let frames = [
                command(1, 23, &encode_logout_intent()),
                command(2, 23, &encode_logout_intent()),
            ];
            let (end, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(100),
            )
            .await?;
            assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
            assert_eq!(authority.logouts.get(), 1);
            let results = results(&written)?;
            assert_eq!(results.len(), 1);
            assert_eq!(
                logout_result(&results, 1)?,
                (CommandStatus::Accepted, LogoutResult::Accepted)
            );
            let last = decode_wire_envelope(written.last().ok_or("no frame")?)?;
            assert_eq!(last.message_type(), MessageType::ProtocolError);
            assert_eq!(
                decode_protocol_error(last.payload())?.error_code,
                FoundationProtocolError::TooManyOutstandingCommands.code()
            );
            Ok(())
        })
    }

    #[test]
    fn a_second_logout_held_behind_a_paced_step_is_a_protocol_error() -> Result<(), Box<dyn Error>>
    {
        run(async {
            // Both logouts are read and held while step 2 waits for its pace, so the first one
            // starts with the second already held (#2127 Codex P2 4279788741).
            let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
            authority.pace = Duration::from_millis(30);
            let frames = [
                command(1, 1, &step()),
                command(2, 1, &step()),
                command(3, 23, &encode_logout_intent()),
                command(4, 23, &encode_logout_intent()),
            ];
            let (_, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(100),
            )
            .await?;
            assert_eq!(authority.steps.get(), 2);
            assert_eq!(authority.logouts.get(), 1);
            let results = results(&written)?;
            assert_eq!(
                results.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
                [1, 2, 3]
            );
            assert_eq!(
                logout_result(&results, 3)?,
                (CommandStatus::Rejected, LogoutResult::Busy)
            );
            let last = decode_wire_envelope(written.last().ok_or("no frame")?)?;
            assert_eq!(last.message_type(), MessageType::ProtocolError);
            assert_eq!(
                decode_protocol_error(last.payload())?.error_code,
                FoundationProtocolError::TooManyOutstandingCommands.code()
            );
            Ok(())
        })
    }

    #[test]
    fn a_second_logout_after_the_first_result_is_an_ordinary_command() -> Result<(), Box<dyn Error>>
    {
        run(async {
            let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
            let frames = [
                command(1, 23, &encode_logout_intent()),
                command(2, 23, &encode_logout_intent()),
            ];
            let (_, written) = serve_open(
                &authority,
                admitted(&[21])?,
                &frames,
                Duration::from_millis(50),
            )
            .await?;
            assert_eq!(authority.logouts.get(), 2);
            assert_eq!(results(&written)?.len(), 2);
            Ok(())
        })
    }
}

/// WEATHER-1 (D1023..D1026, D1448): capability 26 through the real admission and serve path.
/// The source is the shipped Oteryn-designed `content/weather` data, exactly what production
/// loads: production offers 26 once that data loads and fails closed when it does not.
mod weather_wire {
    use super::*;
    use crate::world_weather::{WEATHER_SLOT_MS, shipped};

    use oteryn_protocol_oteryn::world_weather::{
        CAPABILITY_WORLD_WEATHER_V1, DELTA_TYPE_WORLD_WEATHER_V1, SNAPSHOT_TYPE_WORLD_WEATHER_V1,
        STATE_DOMAIN_WORLD_WEATHER, WorldWeather, decode_world_weather,
    };
    use oteryn_protocol_oteryn::{decode_snapshot_chunk, decode_state_delta};
    use std::task::Poll;
    use std::time::Duration;

    const THAIS: ActorPosition = ActorPosition {
        x: 32369,
        y: 32241,
        floor: 7,
    };
    const SVARGROND: ActorPosition = ActorPosition {
        x: 32257,
        y: 31159,
        floor: 7,
    };

    /// The shipped source's observation for a position at the start of an absolute slot.
    fn expected(position: ActorPosition, slot: u64) -> Result<WorldWeather, Box<dyn Error>> {
        Ok(shipped()
            .ok_or("the shipped weather data loads")?
            .observe(position, slot * WEATHER_SLOT_MS))
    }

    fn with_position(position: ActorPosition, slot: u64) -> NegotiatingAuthority {
        let authority = NegotiatingAuthority::new(None);
        authority.position.set(position);
        authority.clock_ms.set(slot * WEATHER_SLOT_MS + 17);
        authority
    }

    /// Domain 22 in the frames: the snapshot entries `(revision, value)` and the deltas
    /// `(base, new, value)`; no other domain may carry weather.
    #[allow(clippy::type_complexity)]
    fn weather_of(
        frames: &[Vec<u8>],
    ) -> Result<(Vec<(u64, WorldWeather)>, Vec<(u64, u64, WorldWeather)>), Box<dyn Error>> {
        let (mut snapshots, mut deltas) = (Vec::new(), Vec::new());
        for frame in frames {
            let envelope = decode_wire_envelope(frame)?;
            match envelope.message_type() {
                MessageType::SnapshotChunk => {
                    let (_, domains) = decode_snapshot_chunk(envelope.payload())?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_WORLD_WEATHER {
                            assert_eq!(domain.snapshot_type, SNAPSHOT_TYPE_WORLD_WEATHER_V1);
                            snapshots.push((
                                domain.revision,
                                decode_world_weather(domain.payload)
                                    .map_err(|e| format!("{e:?}"))?,
                            ));
                        }
                    }
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload())?;
                    if delta.domain_id == STATE_DOMAIN_WORLD_WEATHER {
                        assert_eq!(delta.delta_type, DELTA_TYPE_WORLD_WEATHER_V1);
                        deltas.push((
                            delta.base_revision,
                            delta.new_revision,
                            decode_world_weather(delta.payload).map_err(|e| format!("{e:?}"))?,
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok((snapshots, deltas))
    }

    #[test]
    fn production_offers_capability_26_with_the_shipped_data_and_fails_closed_without_it()
    -> Result<(), Box<dyn Error>> {
        // D1027 signed off: the static sets list 26 (registry `offered: true`); a connection
        // keeps it only while its authority exposes a source.
        for set in [
            PRODUCTION_OFFERED_CAPABILITIES,
            ITEM_VIEW_OFFERED_CAPABILITIES,
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
        ] {
            assert!(
                set.iter()
                    .any(|capability| capability.id == CAPABILITY_WORLD_WEATHER_V1)
            );
            assert_eq!(with_weather_offer(set, true), set);
            assert!(
                with_weather_offer(set, false)
                    .iter()
                    .all(|capability| capability.id != CAPABILITY_WORLD_WEATHER_V1)
            );
            assert_eq!(with_weather_offer(set, true).len(), set.len());
        }
        assert!(shipped().is_some());
        run(async {
            // The production default authority exposes the shipped data, so 26 is offered
            // (requiring 6); a client that does not ask for it never gets it.
            let live = NegotiatingAuthority::new(None);
            let (admitted, frames) = admit(&live, &bootstrap(&[6, 26])?).await?;
            assert!(admitted.is_ok());
            assert_eq!(accepted_selection(&frames)?, [6, 26]);
            let (admitted, frames) = admit(&live, &bootstrap(&[6])?).await?;
            assert_eq!(accepted_selection(&frames)?, [6]);
            let (_, frames) = serve(&live, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
            assert!(weather_of(&frames)?.0.is_empty());
            // Missing or invalid data (no source) fails closed: not offered, so 6 alone.
            let closed = NegotiatingAuthority::new(None);
            closed.weather_bound.set(false);
            let (admitted, frames) = admit(&closed, &bootstrap(&[6, 26])?).await?;
            assert!(admitted.is_ok());
            assert_eq!(accepted_selection(&frames)?, [6]);
            let (_, frames) = serve(&closed, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
            assert!(weather_of(&frames)?.0.is_empty());
            Ok(())
        })
    }

    #[test]
    fn admission_sends_the_domain_22_snapshot_of_the_actors_area_and_never_rerolls()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = with_position(THAIS, 2);
            let (admitted, _) = admit(&authority, &bootstrap(&[6, 26])?).await?;
            let mut admitted = admitted.map_err(|e| format!("{e:?}"))?;
            assert_eq!(admitted.continuity.world_weather_revision, 0);
            admitted.continuity.world_weather_revision = 5;
            let (end, frames) = serve(&authority, admitted, &[]).await?;
            let (snapshots, deltas) = weather_of(&frames)?;
            assert!(deltas.is_empty());
            let first = expected(THAIS, 2)?;
            assert_eq!(snapshots, [(5, first)]);
            assert!(matches!(first, WorldWeather::Active { .. }));
            // A resume or transfer never winds the revision back.
            let ConnectionEnd::AdmittedThenDisconnected(served) = end else {
                return Err(format!("{end:?}").into());
            };
            assert_eq!(served.continuity.world_weather_revision, 5);
            // Reconnect (later in the same slot) shows the same value: no reroll.
            let again = with_position(THAIS, 2);
            again.clock_ms.set(2 * WEATHER_SLOT_MS + 899_000);
            let (admitted, _) = admit(&again, &bootstrap(&[6, 26])?).await?;
            let (_, frames) = serve(&again, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
            assert_eq!(weather_of(&frames)?.0, [(0, first)]);
            // Each area draws independently: a far cold city differs in some slot.
            let mut differs = None;
            for slot in 0..200 {
                if expected(THAIS, slot)? != expected(SVARGROND, slot)? {
                    differs = Some(slot);
                    break;
                }
            }
            let slot = differs.ok_or("two areas never differ")?;
            let cold = with_position(SVARGROND, slot);
            let (admitted, _) = admit(&cold, &bootstrap(&[6, 26])?).await?;
            let (_, frames) = serve(&cold, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
            assert_eq!(weather_of(&frames)?.0, [(0, expected(SVARGROND, slot)?)]);
            Ok(())
        })
    }

    #[test]
    fn a_new_period_a_lost_source_a_returned_source_and_an_area_change_each_send_one_full_replacement()
    -> Result<(), Box<dyn Error>> {
        let first_slot = 2_u64;
        let first = expected(THAIS, first_slot)?;
        // A later slot inside a different period, then a slot where the two areas differ.
        let mut next_slot = first_slot;
        while expected(THAIS, next_slot)? == first {
            next_slot += 1;
        }
        let mut return_slot = next_slot + 20;
        while expected(THAIS, return_slot)? == expected(SVARGROND, return_slot)? {
            return_slot += 1;
        }
        run(async {
            let authority = with_position(THAIS, first_slot);
            let (admitted, _) = admit(&authority, &bootstrap(&[6, 26])?).await?;
            let mut admitted = admitted.map_err(|e| format!("{e:?}"))?;
            admitted.continuity.world_weather_revision = 5;
            let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
            let driving = async {
                let beat = Duration::from_millis(400);
                // Within the same period nothing is sent.
                tokio::time::sleep(beat).await;
                authority.clock_ms.set(next_slot * WEATHER_SLOT_MS + 1);
                tokio::time::sleep(beat).await;
                authority.weather_bound.set(false);
                tokio::time::sleep(beat).await;
                // Still unavailable in a later slot: nothing more (one Unavailable only).
                authority.clock_ms.set(return_slot * WEATHER_SLOT_MS);
                tokio::time::sleep(beat).await;
                authority.weather_bound.set(true);
                tokio::time::sleep(beat).await;
                // The player walks into another area: one more full replacement.
                authority.position.set(SVARGROND);
                tokio::time::sleep(beat).await;
                client.shutdown().await?;
                let mut output = Vec::new();
                client.read_to_end(&mut output).await?;
                Ok::<_, std::io::Error>(output)
            };
            let serving = async {
                let end = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
                drop(server);
                end
            };
            let (mut serving, mut driving) = (std::pin::pin!(serving), std::pin::pin!(driving));
            let (mut end, mut output) = (None, None);
            std::future::poll_fn(|context| {
                if end.is_none()
                    && let Poll::Ready(value) = serving.as_mut().poll(context)
                {
                    end = Some(value);
                }
                if output.is_none()
                    && let Poll::Ready(value) = driving.as_mut().poll(context)
                {
                    output = Some(value);
                }
                if end.is_some() && output.is_some() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            let ConnectionEnd::AdmittedThenDisconnected(served) = end.ok_or("end")? else {
                return Err("the connection did not end as a disconnect".into());
            };
            let frames = split(&output.ok_or("output")??)?;
            let (snapshots, deltas) = weather_of(&frames)?;
            assert_eq!(snapshots, [(5, first)]);
            assert_eq!(
                deltas,
                [
                    (5, 6, expected(THAIS, next_slot)?),
                    (6, 7, WorldWeather::Unavailable),
                    (7, 8, expected(THAIS, return_slot)?),
                    (8, 9, expected(SVARGROND, return_slot)?),
                ]
            );
            assert_eq!(served.continuity.world_weather_revision, 9);
            // Weather rides its own domain: exactly the four replacements above, no more.
            let mut weather_deltas = 0;
            for frame in &frames {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::StateDelta
                    && decode_state_delta(envelope.payload())?.domain_id
                        == STATE_DOMAIN_WORLD_WEATHER
                {
                    weather_deltas += 1;
                }
            }
            assert_eq!(weather_deltas, 4);
            Ok(())
        })
    }
}
