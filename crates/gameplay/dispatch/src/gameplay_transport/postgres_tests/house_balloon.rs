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
    Decoration(u32, u32),
    StaticWindow,
    StaticAqueduct,
    TimedRotation,
    TimedNearExpiry,
    TimedWrap,
    BedKit(u32),
    NativeNoop(u32),
    Container,
    ContainerCapacity,
    CarpetTile { other: u32, removed: bool },
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

#[test]
fn w18_house_static_windows_preserve_current_house_fence() -> TestResult {
    run_case(Case::StaticWindow)
}
#[test]
fn w18_house_container_opens_nested_moves_replay_and_fences_actual_owner() -> TestResult {
    run_case(Case::Container)
}

#[test]
fn w18_house_native_chair_rotates_through_existing_item_owner() -> TestResult {
    run_case(Case::Decoration(2358, 2359))
}
#[test]
fn w18_house_native_cupboard_rotation_keeps_instance_and_capacity() -> TestResult {
    run_case(Case::Decoration(34278, 34276))
}
#[test]
fn w18_house_unavailable_native_target_returns_original_furniture_26099() -> TestResult {
    run_case(Case::NativeNoop(26099))
}

#[test]
fn w18_house_unavailable_native_target_returns_original_cupboard() -> TestResult {
    run_case(Case::NativeNoop(34276))
}
#[test]
fn w18_house_unavailable_native_target_returns_original_zao_cabinet() -> TestResult {
    run_case(Case::NativeNoop(37782))
}

#[test]
fn w18_house_conflicting_carpet_refuses_without_receipt_or_source_mutation() -> TestResult {
    run_case(Case::CarpetTile {
        other: 23431,
        removed: false,
    })
}
#[test]
fn w18_house_auto_carpet_top_retires_only_clicked_carpet_and_replays() -> TestResult {
    run_case(Case::CarpetTile {
        other: 5302,
        removed: true,
    })
}

#[test]
fn w18_house_carpet_uses_source_pair_and_replays_without_second_fold() -> TestResult {
    run_case(Case::Decoration(22736, 22737))
}
#[test]
fn w18_house_empty_bathtub_drains_under_native_occupancy_owner() -> TestResult {
    run_case(Case::Decoration(26077, 26076))
}
#[test]
fn w18_house_music_advances_existing_achievement_without_mutating_instrument() -> TestResult {
    run_case(Case::Decoration(2959, 2959))
}
#[test]
fn w18_house_fish_tank_retains_exact_plunging_fish_effect() -> TestResult {
    run_case(Case::Decoration(23691, 23691))
}
#[test]
fn w18_house_birdcage_audits_source_outcome_without_minting() -> TestResult {
    run_case(Case::Decoration(2976, 0))
}

#[test]
fn w18_house_mirror_cooldown_is_durable_and_replay_is_read_only() -> TestResult {
    run_case(Case::Decoration(2603, 2603))
}
#[test]
fn w18_house_magic_tree_transforms_once_under_current_house_custody() -> TestResult {
    run_case(Case::Decoration(26189, 26193))
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
                let started = std::time::Instant::now();
                let h = Harness::create_with_admission_seed(admin, "house_balloon", true, false)
                    .await?;
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

#[test]
fn w18_house_timed_rotation_resets_timer_and_catches_up_all_phases() -> TestResult {
    run_case(Case::TimedRotation)
}

#[test]
fn w18_house_timed_near_expiry_rotation_advances_counterpart_phase() -> TestResult {
    run_case(Case::TimedNearExpiry)
}
#[test]
fn w18_house_timed_wrap_clears_timer_and_unwrap_restarts_full_duration() -> TestResult {
    run_case(Case::TimedWrap)
}

#[test]
fn w18_house_static_aqueduct_rotates_and_decays() -> TestResult {
    run_case(Case::StaticAqueduct)
}
#[test]
fn w18_house_bed_kit_replaces_both_tiles_and_replays() -> TestResult {
    run_case(Case::BedKit(831))
}

#[test]
fn w18_house_container_capacity_v2_retains_legacy_bound_and_preserves_graph() -> TestResult {
    run_case(Case::ContainerCapacity)
}

#[path = "house_balloon/container.rs"]
mod container;
#[path = "house_balloon/decorations.rs"]
mod decorations;
#[path = "house_balloon/rotation.rs"]
mod rotation;
#[path = "house_balloon/scenario.rs"]
mod scenario;
#[path = "house_balloon/timed.rs"]
mod timed;
#[path = "house_balloon/wire.rs"]
pub(super) mod wire;
use scenario::scenario;
use wire::{HouseExchange, exchange};
