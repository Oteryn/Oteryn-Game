//! LOOK-2: command 30 `ITEM_LOOK_INTENT` through the real connection dispatch (`serve_admitted`)
//! of a bundle-World session that selected capability 28 (CP D1240, D1268, D1143).
//!
//! The authority is a fake that answers every look it is asked for and counts the asks, so these
//! tests pin what the connection itself decides before any owner read: the capability gate, the
//! malformed-payload rejection, the 10-per-second GameSession window (ITEMLOOK1-RL-01, carried
//! across a reconnect), the session's own visibility of a Creature target and the handle table of
//! an Item target.

#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, CAPABILITY_ITEM_LOOK_V1, COMMAND_TYPE_ITEM_LOOK_INTENT,
};
use crate::gameplay_transport::item_view::{InventoryItems, ItemKey, ViewItem};
use crate::gameplay_transport::world_spatial::{ActorPosition, ChannelEntity};
use oteryn_protocol_oteryn::item_look::{
    ItemLookIntent, ItemLookRefusal, ItemLookResult, ItemLookSelection, ItemLookTarget,
    encode_item_look_intent_for, encode_item_look_result_for,
};
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
use oteryn_protocol_oteryn::world_spatial_entities::{
    CAPABILITY_WORLD_SPATIAL_ENTITIES, EntityRef as LookedEntity,
};
use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

type TestResult = Result<(), Box<dyn Error>>;

const EXTENDED: ItemLookSelection = ItemLookSelection { extended: true };
const CONTENT: [u8; 32] = [0x5c; 32];
const RAT: [u8; 16] = [9; 16];
const NEVER_SHOWN: [u8; 16] = [7; 16];
const BACKPACK: [u8; 16] = [0xb0; 16];
const COINS: [u8; 16] = [0xb1; 16];
const RAT_TEXT: &str = "You see a rat.";
const SELF_TEXT: &str = "You see yourself.";
const BACKPACK_TEXT: &str = "You see a backpack.";

const fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

const fn at(x: i32, y: i32) -> ActorPosition {
    ActorPosition { x, y, floor: 7 }
}

fn actor() -> ExactActorRef {
    ExactActorRef::transport_fixture(
        WorldId::decode(&uuid_v7(0x33)).expect("world"),
        ChannelId::decode(&uuid_v7(0x44)).expect("channel"),
    )
}

/// A bundle-World session: the selection is negotiated against the bundle offer, the only one
/// that offers capability 28.
fn session(selected: &[u32], continuity: SessionContinuity) -> AdmittedSession {
    AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid_v7(0x22)).expect("session"),
        world_id: WorldId::decode(&uuid_v7(0x33)).expect("world"),
        channel_id: ChannelId::decode(&uuid_v7(0x44)).expect("channel"),
        runtime_actor: Some(actor()),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                selected,
            )
            .expect("select"),
            ..continuity
        },
        item_fence: None,
    }
}

fn view_item(key: [u8; 16], reference: u32, count: u32) -> ViewItem {
    ViewItem {
        key: ItemKey::Instance(key),
        item_definition_ref: NonZeroU32::new(reference).expect("reference"),
        count: NonZeroU32::new(count).expect("count"),
        sub_type: 0,
    }
}

/// The Channel and item owners as the connection sees them: the own player and one rat in
/// view, a main backpack with one stack of coins. Every look it is asked for is answered and
/// counted; what it is never asked for, the connection refused itself.
struct Owners {
    creature_looks: Cell<u32>,
    item_looks: RefCell<Vec<Option<ItemKey>>>,
}

impl Owners {
    fn new() -> Self {
        Self {
            creature_looks: Cell::new(0),
            item_looks: RefCell::new(Vec::new()),
        }
    }
}

impl FreshAdmissionAuthority for Owners {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: CONTENT,
            actor_position: at(100, 100),
        })
    }

    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        let own = actor.placement_identity();
        Some(ChannelEntities {
            content_generation: CONTENT,
            observer: own,
            entities: vec![
                ChannelEntity {
                    direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                    kind: VisibleKind::Player,
                    identity: own,
                    generation: 1,
                    position: at(100, 100),
                    revision: 0,
                },
                ChannelEntity {
                    direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                    kind: VisibleKind::Creature,
                    identity: RAT,
                    generation: 1,
                    position: at(102, 100),
                    revision: 0,
                },
            ],
            lights: EntityLights::new(),
            observed_at_ms: 0,
        })
    }

    async fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems {
            main_backpack: Some(view_item(BACKPACK, 1, 1)),
            entries: vec![view_item(COINS, 2, 3)],
            equipment: Vec::new(),
        })
    }

    async fn look_creature(
        &self,
        actor: ExactActorRef,
        _game_session_id: GameSessionId,
        target: LookedEntity,
    ) -> ItemLookResult {
        self.creature_looks.set(self.creature_looks.get() + 1);
        if target.identity == actor.placement_identity() {
            ItemLookResult::Text(SELF_TEXT.to_owned())
        } else if target.identity == RAT {
            ItemLookResult::Text(RAT_TEXT.to_owned())
        } else {
            panic!("the connection asked about an entity it never showed: {target:?}")
        }
    }

    async fn look_item(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        views: &SessionItemView,
        target: oteryn_protocol_oteryn::item_view::ItemHandle,
    ) -> ItemLookResult {
        let key = views.resolve(target);
        self.item_looks.borrow_mut().push(key);
        match key {
            Some(ItemKey::Instance(BACKPACK)) => ItemLookResult::Text(BACKPACK_TEXT.to_owned()),
            _ => ItemLookResult::Refused(ItemLookRefusal::NotAvailable),
        }
    }
}

fn creature(identity: [u8; 16], generation: u64) -> Vec<u8> {
    encode_item_look_intent_for(
        ItemLookIntent {
            target: ItemLookTarget::Creature(LookedEntity {
                identity,
                generation,
            }),
        },
        EXTENDED,
    )
    .expect("creature intent")
}

fn item(handle: u64) -> Vec<u8> {
    encode_item_look_intent_for(
        ItemLookIntent {
            target: ItemLookTarget::Item(NonZeroU64::new(handle).expect("handle")),
        },
        EXTENDED,
    )
    .expect("item intent")
}

fn command(
    generation: u64,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> Result<Vec<u8>, Box<dyn Error>> {
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )?;
    let mut framed = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    framed.extend_from_slice(&frame);
    Ok(framed)
}

fn looked(
    generation: u64,
    sequence: u64,
    command_id: u64,
    result: &ItemLookResult,
) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_command_result(
        generation,
        sequence,
        command_id,
        CommandStatus::Accepted,
        &encode_item_look_result_for(result, EXTENDED).map_err(|error| format!("{error:?}"))?,
    )?)
}

fn rejected(generation: u64, sequence: u64, command_id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_command_result(
        generation,
        sequence,
        command_id,
        CommandStatus::Rejected,
        &[],
    )?)
}

fn text(text: &str) -> ItemLookResult {
    ItemLookResult::Text(text.to_owned())
}

const fn refused(refusal: ItemLookRefusal) -> ItemLookResult {
    ItemLookResult::Refused(refusal)
}

async fn read_one(client: &mut DuplexStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut length = [0_u8; 4];
    client.read_exact(&mut length).await?;
    let mut frame = vec![0_u8; u32::from_be_bytes(length) as usize];
    client.read_exact(&mut frame).await?;
    Ok(frame)
}

/// Sends `commands` of command type 30 (`(command_id, payload)`), then closes: every frame the
/// connection sent and how it ended.
async fn serve(
    owners: &Owners,
    admitted: AdmittedSession,
    command_type: u32,
    commands: &[(u64, Vec<u8>)],
) -> Result<(Vec<Vec<u8>>, ConnectionEnd), Box<dyn Error>> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for (command_id, payload) in commands {
        client
            .write_all(&command(
                admitted.continuity.connection_generation,
                *command_id,
                command_type,
                payload,
            )?)
            .await?;
    }
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, owners, IDLE_LIVENESS).await;
    drop(server);
    let mut frames = Vec::new();
    while let Ok(frame) = read_one(&mut client).await {
        frames.push(frame);
    }
    Ok((frames, end))
}

/// The command results are the last frames the connection sent, in command order.
fn assert_results(frames: &[Vec<u8>], expected: &[Vec<u8>]) {
    assert!(
        frames.len() > expected.len(),
        "a snapshot and {} results, got {} frames",
        expected.len(),
        frames.len()
    );
    assert_eq!(&frames[frames.len() - expected.len()..], expected);
}

fn run(test: impl Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(test)
}

fn ended(end: ConnectionEnd) -> Result<AdmittedSession, Box<dyn Error>> {
    match end {
        ConnectionEnd::AdmittedThenDisconnected(admitted) => Ok(admitted),
        other => Err(format!("unexpected end {other:?}").into()),
    }
}

const LOOKING: [u32; 3] = [
    CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    CAPABILITY_ITEM_LOOK_V1,
];

#[test]
fn item_look_e2e_command_30_answers_visible_targets_and_refuses_hidden_stale_and_over_rate()
-> TestResult {
    run(async {
        let owners = Owners::new();
        let own = actor().placement_identity();
        let commands = vec![
            // The rat the snapshot showed, at the generation it showed.
            (1, creature(RAT, 1)),
            // Another incarnation of the shown identity: stale, not the shown rat.
            (2, creature(RAT, 2)),
            // An identity the session was never shown.
            (3, creature(NEVER_SHOWN, 1)),
            // The unknown generation (D1240): refused before any lookup.
            (4, creature(RAT, 0)),
            // The own player is in the session's view.
            (5, creature(own, 1)),
            // The main backpack: the first handle of the domain 9 snapshot.
            (6, item(1)),
            // A handle the session's table never issued.
            (7, item(999)),
            (8, creature(RAT, 1)),
            (9, creature(RAT, 1)),
            (10, creature(RAT, 1)),
            // The 11th look within one second: ITEMLOOK1-RL-01, whatever the target.
            (11, creature(RAT, 1)),
            (12, item(1)),
        ];
        let (frames, end) = serve(
            &owners,
            session(&LOOKING, SessionContinuity::FRESH),
            COMMAND_TYPE_ITEM_LOOK_INTENT,
            &commands,
        )
        .await?;
        let not_visible = refused(ItemLookRefusal::NotVisible);
        let expected = vec![
            looked(1, 1, 1, &text(RAT_TEXT))?,
            looked(1, 2, 2, &not_visible)?,
            looked(1, 3, 3, &not_visible)?,
            looked(1, 4, 4, &not_visible)?,
            looked(1, 5, 5, &text(SELF_TEXT))?,
            looked(1, 6, 6, &text(BACKPACK_TEXT))?,
            looked(1, 7, 7, &refused(ItemLookRefusal::NotAvailable))?,
            looked(1, 8, 8, &text(RAT_TEXT))?,
            looked(1, 9, 9, &text(RAT_TEXT))?,
            looked(1, 10, 10, &text(RAT_TEXT))?,
            looked(1, 11, 11, &refused(ItemLookRefusal::RateLimited))?,
            looked(1, 12, 12, &refused(ItemLookRefusal::RateLimited))?,
        ];
        assert_results(&frames, &expected);
        // Only the shown rat (4 times) and the own player were asked about; only the issued
        // backpack handle reached the item owner.
        assert_eq!(owners.creature_looks.get(), 5);
        assert_eq!(
            *owners.item_looks.borrow(),
            vec![Some(ItemKey::Instance(BACKPACK))]
        );

        // The window is the GameSession's: a reconnect within the same second is still over it.
        let ended = ended(end)?;
        let last_sequence = ended.continuity.server_sequence;
        let resumed = SessionContinuity {
            connection_generation: 2,
            ..ended.continuity
        };
        let (frames, end) = serve(
            &owners,
            session(&LOOKING, resumed),
            COMMAND_TYPE_ITEM_LOOK_INTENT,
            &[(13, creature(RAT, 1))],
        )
        .await?;
        assert_results(
            &frames,
            &[looked(
                2,
                last_sequence + 1,
                13,
                &refused(ItemLookRefusal::RateLimited),
            )?],
        );
        assert_eq!(owners.creature_looks.get(), 5);
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        Ok(())
    })
}

#[test]
fn item_look_e2e_command_30_without_capability_28_is_rejected_unregistered() -> TestResult {
    // Capability 28 is offered only to a bundle World: the production offer has no LOOK.
    assert!(
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &LOOKING)
            .expect("bundle")
            .contains(CAPABILITY_ITEM_LOOK_V1)
    );
    for offered in [
        PRODUCTION_OFFERED_CAPABILITIES,
        crate::gameplay_transport::capabilities::ITEM_VIEW_OFFERED_CAPABILITIES,
    ] {
        assert!(
            !SelectedCapabilities::select(
                offered,
                &[CAPABILITY_WORLD_SPATIAL_ENTITIES, CAPABILITY_ITEM_LOOK_V1]
            )
            .expect("selection")
            .contains(CAPABILITY_ITEM_LOOK_V1)
        );
    }
    run(async {
        let owners = Owners::new();
        let legacy_item = encode_item_look_intent_for(
            ItemLookIntent {
                target: ItemLookTarget::Item(NonZeroU64::MIN),
            },
            ItemLookSelection::default(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let (frames, end) = serve(
            &owners,
            session(
                &[
                    CAPABILITY_ITEM_VIEW_MOVE_V1,
                    CAPABILITY_WORLD_SPATIAL_ENTITIES,
                ],
                SessionContinuity::FRESH,
            ),
            COMMAND_TYPE_ITEM_LOOK_INTENT,
            &[(1, creature(RAT, 1)), (2, legacy_item), (3, item(1))],
        )
        .await?;
        assert_results(
            &frames,
            &[rejected(1, 1, 1)?, rejected(1, 2, 2)?, rejected(1, 3, 3)?],
        );
        assert_eq!(owners.creature_looks.get(), 0);
        assert!(owners.item_looks.borrow().is_empty());
        // Unregistered commands spend no LOOK slot.
        let ended = ended(end)?;
        assert_eq!(
            ended.continuity.look,
            crate::gameplay_transport::item_look::LookWindow::EMPTY
        );
        Ok(())
    })
}

#[test]
fn item_look_e2e_malformed_payload_is_rejected() -> TestResult {
    run(async {
        let owners = Owners::new();
        let mut trailing = creature(RAT, 1);
        trailing.push(0);
        let mut short_identity = vec![0x12, 0x11, 0x0a, 0x0f];
        short_identity.extend_from_slice(&[9; 15]);
        let mut twice = vec![0x12, 0x16, 0x0a, 0x10];
        twice.extend_from_slice(&RAT);
        twice.extend_from_slice(&[0x10, 0x01, 0x10, 0x01]);
        let oversized = vec![0x08; 32];
        let malformed = vec![
            (1, Vec::new()),
            (2, vec![0xff]),
            (3, trailing),
            (4, short_identity),
            (5, twice),
            (6, oversized),
            // A zero handle: there is no item without one.
            (7, vec![0x08, 0x00]),
        ];
        let mut commands = malformed.clone();
        // Every command 30 spends a slot of the window, a malformed one too: three more looks
        // fill it and the fourth is over it.
        commands.extend([
            (8, creature(RAT, 1)),
            (9, creature(RAT, 1)),
            (10, creature(RAT, 1)),
            (11, creature(RAT, 1)),
            // A malformed command over the window is refused by the window, before decoding.
            (12, vec![0xff]),
        ]);
        let (frames, _) = serve(
            &owners,
            session(&LOOKING, SessionContinuity::FRESH),
            COMMAND_TYPE_ITEM_LOOK_INTENT,
            &commands,
        )
        .await?;
        let mut expected = Vec::new();
        for (sequence, (command_id, _)) in (1..).zip(&malformed) {
            expected.push(rejected(1, sequence, *command_id)?);
        }
        expected.extend([
            looked(1, 8, 8, &text(RAT_TEXT))?,
            looked(1, 9, 9, &text(RAT_TEXT))?,
            looked(1, 10, 10, &text(RAT_TEXT))?,
            looked(1, 11, 11, &refused(ItemLookRefusal::RateLimited))?,
            looked(1, 12, 12, &refused(ItemLookRefusal::RateLimited))?,
        ]);
        assert_results(&frames, &expected);
        assert_eq!(owners.creature_looks.get(), 3);
        assert!(owners.item_looks.borrow().is_empty());
        Ok(())
    })
}
