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
        id: 7,
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
        id: 288,
        requires: &[6],
    },
    OfferedCapability {
        id: 388,
        requires: &[11],
    },
    OfferedCapability {
        id: 350,
        requires: &[6],
    },
    OfferedCapability {
        id: 386,
        requires: &[7],
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
    GiftSantaDoll,
    GiftPanda,
    GiftPandaFar,
    GiftDragon,
    GiftDragonDeath,
    GiftDragonDeathCrash,
    GiftDragonDeathHigh,
    GiftBankBreak,
    GiftBankKeep,
    Placed,
    Cue,
    GoldenFull,
    GoldenLocked,
    Voice,
    NativeMove,
    Cupcake,
    Rotate,
    AbsoluteRotate,
    AbsoluteLegacyPlace,
    AbsoluteHistory,
    ArtistEasel,
    StaticBase,
    DracoyleWest,
    DracoyleEast,
    Carried,
    CarriedCandle,
    Nested,
    Foreign,
}

impl Case {
    fn absolute_source(self) -> bool {
        matches!(
            self,
            Self::AbsoluteRotate | Self::AbsoluteLegacyPlace | Self::AbsoluteHistory
        )
    }
    fn gift(self) -> bool {
        matches!(
            self,
            Self::GiftSantaDoll
                | Self::GiftPanda
                | Self::GiftPandaFar
                | Self::GiftDragon
                | Self::GiftDragonDeath
                | Self::GiftDragonDeathCrash
                | Self::GiftDragonDeathHigh
                | Self::GiftBankBreak
                | Self::GiftBankKeep
        )
    }
    fn placed(self) -> bool {
        matches!(
            self,
            Self::Placed
                | Self::Rotate
                | Self::AbsoluteRotate
                | Self::AbsoluteLegacyPlace
                | Self::AbsoluteHistory
                | Self::DracoyleWest
                | Self::DracoyleEast
                | Self::Cue
                | Self::GoldenFull
                | Self::GoldenLocked
                | Self::Voice
                | Self::Cupcake
                | Self::GiftSantaDoll
                | Self::GiftPanda
                | Self::GiftPandaFar
                | Self::GiftDragon
                | Self::GiftDragonDeath
                | Self::GiftDragonDeathCrash
                | Self::GiftDragonDeathHigh
                | Self::GiftBankBreak
                | Self::GiftBankKeep
        )
    }
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
fn srv7_house_table_lamp_guest_uses_native_map_handle_and_replays() -> TestResult {
    run_case_items(
        Case::Placed,
        "oteryn:item.tibia.i2934",
        "oteryn:item.tibia.i2935",
    )
}

#[test]
fn srv7_house_cuckoo_clock_use_replays_and_tick_expires() -> TestResult {
    run_case_items(
        Case::Placed,
        "oteryn:item.tibia.i2662",
        "oteryn:item.tibia.i2663",
    )
}

#[test]
fn srv7_house_bookcase_rotation_preserves_native_container_instance() -> TestResult {
    run_case_items(
        Case::Rotate,
        "oteryn:item.tibia.i6109",
        "oteryn:item.tibia.i6110",
    )
}

#[test]
fn srv7_house_explicit_rotate_is_distinct_from_use_and_replays() -> TestResult {
    run_case_items(
        Case::Rotate,
        "oteryn:item.tibia.i23434",
        "oteryn:item.tibia.i23435",
    )
}

fn run_case(case: Case) -> TestResult {
    run_case_items(case, BEFORE, AFTER)
}

fn run_case_items(case: Case, before: &'static str, after: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "house_balloon",
                    !matches!(
                        case,
                        Case::GiftDragonDeath
                            | Case::GiftDragonDeathCrash
                            | Case::GiftDragonDeathHigh
                    ),
                    false,
                )
                .await?;
                let started = std::time::Instant::now();
                let result = scenario(&h, case, before, after).await;
                eprintln!("PG_MARKER srv7-house-item elapsed={:?}", started.elapsed());
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

#[path = "house_balloon/cupcakes.rs"]
mod cupcakes;
#[test]
fn srv7_house_blueberry_cupcake_native_use_replay_and_cooldown() -> TestResult {
    run_case_items(
        Case::Cupcake,
        "oteryn:item.tibia.i28484",
        "oteryn:item.tibia.i28484",
    )
}
#[test]
fn srv7_house_strawberry_cupcake_native_use_replay_and_cooldown() -> TestResult {
    run_case_items(
        Case::Cupcake,
        "oteryn:item.tibia.i28485",
        "oteryn:item.tibia.i28485",
    )
}
#[test]
fn srv7_house_lemon_cupcake_native_use_replay_and_cooldown() -> TestResult {
    run_case_items(
        Case::Cupcake,
        "oteryn:item.tibia.i28486",
        "oteryn:item.tibia.i28486",
    )
}

#[path = "house_balloon/cues.rs"]
mod cues;
#[path = "house_balloon/decoration_voices.rs"]
mod decoration_voices;
#[path = "house_balloon/native_move.rs"]
mod native_move;
#[test]
fn srv7_house_native_move9_refuses_unwrapped_nonpickupable_birdcage_take() -> TestResult {
    // Real Edvard acquisition is qualified independently by the authentic-world
    // BUY test. This fixture isolates the native House Take boundary.
    run_case_items(
        Case::Placed,
        "oteryn:item.tibia.i2976",
        "oteryn:item.tibia.i2976",
    )
}

#[test]
fn srv7_house_native_plain_move9_places_and_takes_same_furniture_with_replay() -> TestResult {
    run_case_items(
        Case::NativeMove,
        "oteryn:item.tibia.i2472",
        "oteryn:item.tibia.i2472",
    )
}

#[test]
fn srv7_house_dog_house_source_voice_and_replay() -> TestResult {
    run_case_items(
        Case::Voice,
        "oteryn:item.tibia.i23708",
        "oteryn:item.tibia.i23709",
    )
}
#[test]
fn srv7_house_baby_dragon_source_voice_and_replay() -> TestResult {
    run_case_items(
        Case::Voice,
        "oteryn:item.tibia.i23443",
        "oteryn:item.tibia.i23442",
    )
}

#[test]
fn srv7_house_fish_tank_committed_cue_and_replay() -> TestResult {
    run_case_items(
        Case::Cue,
        "oteryn:item.tibia.i23691",
        "oteryn:item.tibia.i23691",
    )
}
#[test]
fn srv7_house_harp_committed_cue_and_replay() -> TestResult {
    run_case_items(
        Case::Cue,
        "oteryn:item.tibia.i2963",
        "oteryn:item.tibia.i2963",
    )
}

#[path = "house_balloon/gifts.rs"]
mod gifts;
#[test]
fn srv7_house_santa_panda_preserves_source_and_replays() -> TestResult {
    run_case_items(
        Case::GiftPanda,
        "oteryn:item.tibia.i5080",
        "oteryn:item.tibia.i5080",
    )
}
#[test]
fn srv7_house_santa_dragon_damages_actor_once_and_replays() -> TestResult {
    run_case_items(
        Case::GiftDragon,
        "oteryn:item.tibia.i6566",
        "oteryn:item.tibia.i6566",
    )
}
#[test]
fn srv7_house_santa_piggy_breaks_in_place_and_mints_gold_once() -> TestResult {
    run_case_items(
        Case::GiftBankBreak,
        "oteryn:item.tibia.i2995",
        "oteryn:item.tibia.i2996",
    )
}
#[test]
fn srv7_house_santa_piggy_keeps_bank_and_mints_platinum_once() -> TestResult {
    run_case_items(
        Case::GiftBankKeep,
        "oteryn:item.tibia.i2995",
        "oteryn:item.tibia.i2995",
    )
}

#[path = "house_balloon/absolute_rotate.rs"]
mod absolute_rotate;
#[path = "house_balloon/legacy_clock_place.rs"]
mod legacy_clock_place;
#[test]
fn srv7_house_actual_charged_dracoyle_rotate_and_tick_preserve_original_clock() -> TestResult {
    run_case_items(
        Case::AbsoluteRotate,
        "oteryn:item.tibia.i9034",
        "oteryn:item.tibia.i9039",
    )
}

#[test]
fn srv7_house_santa_lethal_dragon_exits_through_ordinary_death() -> TestResult {
    run_case_items(
        Case::GiftDragonDeath,
        "oteryn:item.tibia.i6566",
        "oteryn:item.tibia.i6566",
    )
}

#[path = "house_balloon/carried_candle.rs"]
mod carried_candle;
#[test]
fn srv7_house_lit_carried_candle_exits_to_channel_and_expires_once() -> TestResult {
    run_case_items(
        Case::CarriedCandle,
        "oteryn:item.tibia.i2914",
        "oteryn:item.tibia.i2915",
    )
}

#[test]
fn srv7_house_santa_doll_native_use_preserves_toy_and_replays() -> TestResult {
    run_case_items(
        Case::GiftSantaDoll,
        "oteryn:item.tibia.i6511",
        "oteryn:item.tibia.i6511",
    )
}

#[test]
fn srv7_house_far_placed_santa_gift_requires_source_adjacency() -> TestResult {
    run_case_items(
        Case::GiftPandaFar,
        "oteryn:item.tibia.i5080",
        "oteryn:item.tibia.i5080",
    )
}

#[path = "house_balloon/gift_presentation.rs"]
mod gift_presentation;

#[path = "house_balloon/artist_easel.rs"]
mod artist_easel;
#[path = "house_balloon/static_base.rs"]
mod static_base;
#[test]
fn srv7_artist_easel_actual_map_pickup_then_house_rotate() -> TestResult {
    run_case_items(
        Case::ArtistEasel,
        "oteryn:item.tibia.i14687",
        "oteryn:item.tibia.i14688",
    )
}

#[path = "house_balloon/dracoyle.rs"]
mod dracoyle;
#[test]
fn srv7_house_dracoyle_west_native_gem_use_preserves_uuid_and_replays() -> TestResult {
    run_case_items(
        Case::DracoyleWest,
        "oteryn:item.tibia.i9035",
        "oteryn:item.tibia.i9034",
    )
}
#[test]
fn srv7_house_dracoyle_east_native_gem_use_preserves_uuid_and_replays() -> TestResult {
    run_case_items(
        Case::DracoyleEast,
        "oteryn:item.tibia.i9040",
        "oteryn:item.tibia.i9039",
    )
}

#[path = "house_balloon/gifts_live_death.rs"]
mod gifts_live_death;

#[path = "house_balloon/santa_toy_lifetime.rs"]
mod santa_toy_lifetime;

#[test]
fn srv7_house_santa_dragon_individual_native_place_starts_three_second_clock() -> TestResult {
    run_case_items(
        Case::NativeMove,
        "oteryn:item.tibia.i6566",
        "oteryn:item.tibia.i6566",
    )
}

#[test]
fn srv7_house_santa_santa_individual_native_place_starts_three_second_clock() -> TestResult {
    run_case_items(
        Case::NativeMove,
        "oteryn:item.tibia.i6567",
        "oteryn:item.tibia.i6567",
    )
}

#[test]
fn srv7_house_santa_panda_individual_native_place_starts_three_second_clock() -> TestResult {
    run_case_items(
        Case::NativeMove,
        "oteryn:item.tibia.i6568",
        "oteryn:item.tibia.i6568",
    )
}

#[test]
fn srv7_house_real_base_farm_torch_native_use_preserves_placement_and_replays() -> TestResult {
    run_case_items(
        Case::StaticBase,
        "oteryn:item.tibia.i2929",
        "oteryn:item.tibia.i2928",
    )
}

#[test]
fn srv7_house_legacy_root_place_preserves_actual_existing_absolute_clock_without_new_digest()
-> TestResult {
    run_case_items(
        Case::AbsoluteLegacyPlace,
        "oteryn:item.tibia.i9034",
        "oteryn:item.tibia.i9039",
    )
}

#[path = "house_balloon/death_crash.rs"]
mod death_crash;
#[test]
fn srv7_house_death_committed_exit_recovers_pending_temple_after_placement_crash() -> TestResult {
    run_case_items(
        Case::GiftDragonDeathCrash,
        "oteryn:item.tibia.i6566",
        "oteryn:item.tibia.i6566",
    )
}

#[path = "house_balloon/absolute_history.rs"]
mod absolute_history;
#[test]
fn srv7_house_paid_dracoyle_cycle_never_replays_completed_absolute_history() -> TestResult {
    run_case_items(
        Case::AbsoluteHistory,
        "oteryn:item.tibia.i9034",
        "oteryn:item.tibia.i9039",
    )
}

#[path = "house_balloon/gift_death.rs"]
mod gift_death;
#[test]
fn srv7_house_knight130_lethal_gift_returns_source_temple_and_recovers_timed_backpack() -> TestResult
{
    run_case_items(
        Case::GiftDragonDeathHigh,
        "oteryn:item.tibia.i6566",
        "oteryn:item.tibia.i6566",
    )
}

#[path = "house_balloon/death_ground.rs"]
mod death_ground;

#[path = "house_balloon/death_recovery_negatives.rs"]
mod death_recovery_negatives;

#[path = "house_balloon/golden_display.rs"]
mod golden_display;
#[test]
fn srv7_golden_display_full_outfit_native_house_use_changes_quality_and_publishes171() -> TestResult
{
    run_case_items(
        Case::GoldenFull,
        "oteryn:item.tibia.i31510",
        "oteryn:item.tibia.i31502",
    )
}
#[test]
fn srv7_golden_display_missing_outfit_native_house_use_preserves_item_and_publishes_poff()
-> TestResult {
    run_case_items(
        Case::GoldenLocked,
        "oteryn:item.tibia.i31510",
        "oteryn:item.tibia.i31502",
    )
}
