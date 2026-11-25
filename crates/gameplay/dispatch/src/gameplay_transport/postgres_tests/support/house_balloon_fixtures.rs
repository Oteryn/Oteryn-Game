//! W19: ordinary framed USE in actual House custody, including a real guest ACL.
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};

use super::fixture::{Harness, debug, id, run};
use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MessageType, MovementLocalPosition,
    decode_wire_envelope,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};

use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use serde_json::Value;

pub(super) const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const POSITION: [i32; 3] = [32310, 32804, 7];
const BEFORE: &str = "oteryn:item.tibia.i37414";
const AFTER: &str = "oteryn:item.tibia.i37471";

const MAP_OFFERS: &[OfferedCapability] = &[
    OfferedCapability {
        id: 4,
        requires: &[6],
    },
    OfferedCapability {
        id: 6,
        requires: &[],
    },
    OfferedCapability {
        id: 11,
        requires: &[15, 18],
    },
    OfferedCapability {
        id: 12,
        requires: &[4],
    },
    OfferedCapability {
        id: 14,
        requires: &[4, 12],
    },
    OfferedCapability {
        id: 15,
        requires: &[4],
    },
    OfferedCapability {
        id: 18,
        requires: &[4, 6],
    },
    OfferedCapability {
        id: 49,
        requires: &[],
    },
    OfferedCapability {
        id: 86,
        requires: &[],
    },
    OfferedCapability {
        id: 445,
        requires: &[49],
    },
    OfferedCapability {
        id: 446,
        requires: &[14, 49],
    },
    OfferedCapability {
        id: 510,
        requires: &[14],
    },
];

// Like the existing House-wrap fixture, boot only the bounded command set
// under test. The map case must declare the mandatory offered map floor;
// inventory-only cases keep the map offer closed and prove USE without domain17.
const INVENTORY_OFFERS: &[OfferedCapability] = &[
    OfferedCapability {
        id: 3,
        requires: &[],
    },
    OfferedCapability {
        id: 4,
        requires: &[6],
    },
    OfferedCapability {
        id: 6,
        requires: &[],
    },
    OfferedCapability {
        id: 12,
        requires: &[4],
    },
    OfferedCapability {
        id: 13,
        requires: &[],
    },
    OfferedCapability {
        id: 14,
        requires: &[4, 12],
    },
    OfferedCapability {
        id: 15,
        requires: &[4],
    },
    OfferedCapability {
        id: 25,
        requires: &[6],
    },
    OfferedCapability {
        id: 49,
        requires: &[],
    },
    OfferedCapability {
        id: 51,
        requires: &[],
    },
    OfferedCapability {
        id: 86,
        requires: &[],
    },
    OfferedCapability {
        id: 115,
        requires: &[4],
    },
    OfferedCapability {
        id: 165,
        requires: &[3, 51],
    },
    OfferedCapability {
        id: 500,
        requires: &[],
    },
    OfferedCapability {
        id: 502,
        requires: &[6, 13, 25],
    },
    OfferedCapability {
        id: 508,
        requires: &[4, 6],
    },
    OfferedCapability {
        id: 509,
        requires: &[6],
    },
    OfferedCapability {
        id: 511,
        requires: &[4, 115],
    },
    OfferedCapability {
        id: 596,
        requires: &[511],
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Case {
    Placed,
    Carried,
    Lantern {
        placed: bool,
        definition: bool,
        source: u32,
    },
    ResidentTick,
    ResidentExpiryTick,
    ResidentLetter,
    ResidentMail,
    ResidentEquip,
    ResidentTimedEquip(super::house_resident_reads_fixtures::ClockCase),
    ResidentAppearanceRevoked,
    ResidentMount,
    ResidentQuest,
    Nested,
    Foreign,
    Decoration(u32, u32),
    StaticWindow,
    StaticAqueduct,
    TimedRotation,
    TimedNearExpiry,
    TimedWrap,
    BedKit(u32),
    NativeNoop(u32),
    Container,
    ContainerMove,
    ContainerCapacity,
    CarpetTile { other: u32, removed: bool },
}

pub(super) fn run_case(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async |admin| {
                let started = std::time::Instant::now();
                let h = if matches!(
                    case,
                    Case::ResidentMount | Case::ResidentTick | Case::ResidentAppearanceRevoked
                ) {
                    Harness::create_appearance(admin, "house_balloon").await?
                } else {
                    Harness::create_with_admission_seed(admin, "house_balloon", true, false).await?
                };
                let result = scenario(&h, case).await;
                h.cleanup().await?;
                eprintln!("W18_PG_CASE {case:?} elapsed={:?}", started.elapsed());
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "House balloon thread")??;
    Ok(())
}

#[path = "../house_balloon/container.rs"]
mod container;
#[path = "../house_balloon/decorations.rs"]
mod decorations;
#[path = "../house_balloon/rotation.rs"]
mod rotation;
#[path = "../house_balloon/scenario.rs"]
mod scenario;
#[path = "../house_balloon/timed.rs"]
mod timed;

use scenario::scenario;
use wire::{HouseExchange, exchange};

pub(super) const INNER: &str = "oteryn:item.tibia.i2854";

#[path = "house_balloon/wire.rs"]
pub(super) mod wire;
