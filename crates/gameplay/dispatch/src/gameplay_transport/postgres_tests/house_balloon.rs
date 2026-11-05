//! W19: ordinary framed USE in actual House custody, including a real guest ACL.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer::{RecoverySource, prepare_owned_session};
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
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const POSITION: [i32; 3] = [32310, 32804, 7];
const BEFORE: &str = "oteryn:item.tibia.i37414";
const AFTER: &str = "oteryn:item.tibia.i37471";
const INNER: &str = "oteryn:item.tibia.i2854";
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
];

// Like the existing House-wrap fixture, boot only the bounded command set
// under test. The map case must declare the mandatory offered map floor;
// inventory-only cases keep the map offer closed and prove USE without domain17.
const INVENTORY_OFFERS: &[OfferedCapability] = &[
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
        id: 14,
        requires: &[4, 12],
    },
    OfferedCapability {
        id: 15,
        requires: &[4],
    },
    OfferedCapability {
        id: 49,
        requires: &[],
    },
    OfferedCapability {
        id: 86,
        requires: &[],
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Placed,
    Carried,
    Nested,
    Foreign,
}

#[test]
fn w19_house_balloon_guest_uses_real_map_handle_and_replays() -> TestResult {
    run_case(Case::Placed)
}
#[test]
fn w19_house_balloon_carried_inside_uses_existing_inventory_capability() -> TestResult {
    run_case(Case::Carried)
}
#[test]
fn w19_house_balloon_nested_uses_actual_container_command_and_replays() -> TestResult {
    run_case(Case::Nested)
}
#[test]
fn w19_house_balloon_outside_house_and_changed_character_custody_refuse() -> TestResult {
    run_case(Case::Foreign)
}

fn run_case(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, "house_balloon", true, false)
                    .await?;
                let result = scenario(&h, case).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "House balloon thread")??;
    Ok(())
}

#[path = "house_balloon/scenario.rs"]
mod scenario;
#[path = "house_balloon/wire.rs"]
mod wire;
use scenario::scenario;
use wire::{HouseExchange, exchange};
