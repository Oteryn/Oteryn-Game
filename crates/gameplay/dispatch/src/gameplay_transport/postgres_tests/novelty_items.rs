//! W19 novelty actions enter through real ordinary USE and retain one durable outcome.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::novelty_items::{self, NoveltyEffects};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    ToyDragonMove,
    ToySantaMove,
    ToyPandaMove,
    PortableLamp,
    GroundPortableLamp,
    PortableMusic,
    RotateDracoyleWest,
    RotateDracoyleEast,
    DracoyleWest,
    DracoyleEast,
    SantaDoll,
    GroundSantaDoll,
    SantaPanda,
    GroundSantaPanda,
    Costume,
    Dice,
    DiceStreak,
    Trumpet,
    Rocket,
    Hat,
    HatInBag,
    VoodooSuccess,
    VoodooFailure,
    VoodooWithoutTarget,
    Present,
    GroundCupcakeBlueberry,
    GroundCupcakeStrawberry,
    GroundCupcakeLemon,
    GroundDice,
    GroundRocket,
    GroundTrumpet,
    NestedCostume,
    NestedTrumpet,
    GroundCostume,
    GroundPresent,
    GroundVoodoo,
    PendingLifecycle,
}
impl Case {
    fn key(self) -> &'static str {
        match self {
            Self::ToyDragonMove => "oteryn:item.tibia.i6566",
            Self::ToySantaMove => "oteryn:item.tibia.i6567",
            Self::ToyPandaMove => "oteryn:item.tibia.i6568",
            Self::PortableLamp | Self::GroundPortableLamp => "oteryn:item.tibia.i2914",
            Self::PortableMusic => "oteryn:item.tibia.i2948",
            Self::RotateDracoyleWest => "oteryn:item.tibia.i9035",
            Self::RotateDracoyleEast => "oteryn:item.tibia.i9040",
            Self::DracoyleWest => "oteryn:item.tibia.i9035",
            Self::DracoyleEast => "oteryn:item.tibia.i9040",
            Self::SantaDoll | Self::GroundSantaDoll => "oteryn:item.tibia.i6511",
            Self::SantaPanda | Self::GroundSantaPanda => "oteryn:item.tibia.i5080",
            Self::GroundCupcakeBlueberry => "oteryn:item.tibia.i28484",
            Self::GroundCupcakeStrawberry => "oteryn:item.tibia.i28485",
            Self::GroundCupcakeLemon => "oteryn:item.tibia.i28486",
            Self::Costume | Self::NestedCostume | Self::GroundCostume => "oteryn:item.tibia.i653",
            Self::Dice | Self::DiceStreak | Self::GroundDice => "oteryn:item.tibia.i5792",
            Self::Trumpet | Self::GroundTrumpet | Self::NestedTrumpet => "oteryn:item.tibia.i6572",
            Self::Rocket | Self::GroundRocket | Self::PendingLifecycle => "oteryn:item.tibia.i6576",
            Self::Hat | Self::HatInBag => "oteryn:item.tibia.i6578",
            Self::VoodooSuccess
            | Self::VoodooFailure
            | Self::VoodooWithoutTarget
            | Self::GroundVoodoo => "oteryn:item.tibia.i3002",
            Self::Present | Self::GroundPresent => "oteryn:item.tibia.i906",
        }
    }
    fn ground(self) -> bool {
        matches!(
            self,
            Self::GroundPortableLamp
                | Self::RotateDracoyleWest
                | Self::RotateDracoyleEast
                | Self::DracoyleWest
                | Self::DracoyleEast
                | Self::GroundSantaDoll
                | Self::GroundSantaPanda
                | Self::GroundCupcakeBlueberry
                | Self::GroundCupcakeStrawberry
                | Self::GroundCupcakeLemon
                | Self::GroundDice
                | Self::GroundRocket
                | Self::GroundTrumpet
                | Self::GroundCostume
                | Self::GroundPresent
                | Self::GroundVoodoo
        )
    }
    fn cupcake(self) -> Option<u32> {
        match self {
            Self::GroundCupcakeBlueberry => Some(28484),
            Self::GroundCupcakeStrawberry => Some(28485),
            Self::GroundCupcakeLemon => Some(28486),
            _ => None,
        }
    }
    fn nested(self) -> bool {
        matches!(self, Self::NestedCostume | Self::NestedTrumpet)
    }
    fn trumpet(self) -> bool {
        matches!(
            self,
            Self::Trumpet | Self::GroundTrumpet | Self::NestedTrumpet
        )
    }
    fn refused(self) -> bool {
        matches!(self, Self::HatInBag | Self::VoodooWithoutTarget)
    }
}
#[test]
fn novelty_costume_commits_condition_once() -> TestResult {
    run_case(Case::Costume)
}
#[test]
fn novelty_dice_commits_exact_face_once() -> TestResult {
    run_case(Case::Dice)
}
#[test]
fn novelty_dice_three_sixes_reset_and_replay_grants_once() -> TestResult {
    run_case(Case::DiceStreak)
}
#[test]
fn novelty_trumpet_commits_original_absolute_deadline() -> TestResult {
    run_case(Case::Trumpet)
}
#[test]
fn novelty_carried_rocket_commits_ten_health_loss_once() -> TestResult {
    run_case(Case::Rocket)
}
#[test]
fn novelty_equipped_hat_preserves_item_and_retains_donor_cues_once() -> TestResult {
    run_case(Case::Hat)
}
#[test]
fn novelty_hat_in_backpack_refuses_without_progress_or_effect() -> TestResult {
    run_case(Case::HatInBag)
}
#[test]
fn novelty_voodoo_success_targets_player_without_consumption() -> TestResult {
    run_case(Case::VoodooSuccess)
}
#[test]
fn novelty_voodoo_failure_is_a_retained_no_consume_outcome() -> TestResult {
    run_case(Case::VoodooFailure)
}
#[test]
fn novelty_voodoo_without_player_refuses_without_receipt() -> TestResult {
    run_case(Case::VoodooWithoutTarget)
}
#[test]
fn novelty_explosive_present_commits_consumption_and_donor_cues_once() -> TestResult {
    run_case(Case::Present)
}

#[test]
fn novelty_ground_dice_uses_map_handle_and_exact_source_position() -> TestResult {
    run_case(Case::GroundDice)
}
#[test]
fn novelty_ground_rocket_consumes_without_health_loss() -> TestResult {
    run_case(Case::GroundRocket)
}
#[test]
fn novelty_ground_trumpet_keeps_ground_order_and_original_deadline() -> TestResult {
    run_case(Case::GroundTrumpet)
}
#[test]
fn novelty_nested_costume_uses_open_container_handle_and_replays() -> TestResult {
    run_case(Case::NestedCostume)
}
#[test]
fn novelty_nested_trumpet_expires_in_its_actual_container() -> TestResult {
    run_case(Case::NestedTrumpet)
}

#[test]
fn novelty_ground_costume_changes_player_and_consumes_ground_bag() -> TestResult {
    run_case(Case::GroundCostume)
}
#[test]
fn novelty_ground_present_retains_player_cue_position() -> TestResult {
    run_case(Case::GroundPresent)
}
#[test]
fn novelty_ground_voodoo_preserves_source_and_targets_player() -> TestResult {
    run_case(Case::GroundVoodoo)
}

#[test]
fn novelty_cancelled_use_reconciles_committed_rollback_and_unknown_before_control_loss()
-> TestResult {
    run_case(Case::PendingLifecycle)
}

#[path = "novelty_items/pending.rs"]
mod pending;

fn run_case(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "novelty scenario thread")??;
    Ok(())
}
#[path = "novelty_items/scenario.rs"]
mod scenario;
use scenario::scenario;

#[path = "novelty_items/cupcakes.rs"]
mod cupcakes;
#[test]
fn srv7_ground_blueberry_cupcake_native_use_replay_and_cooldown() -> TestResult {
    run_case(Case::GroundCupcakeBlueberry)
}
#[test]
fn srv7_ground_strawberry_cupcake_native_use_replay_and_cooldown() -> TestResult {
    run_case(Case::GroundCupcakeStrawberry)
}
#[test]
fn srv7_ground_lemon_cupcake_native_use_replay_and_cooldown() -> TestResult {
    run_case(Case::GroundCupcakeLemon)
}

#[test]
fn srv7_santa_panda_carried_native_use_preserves_toy_and_replays() -> TestResult {
    run_case(Case::SantaPanda)
}
#[test]
fn srv7_santa_panda_ground_native_use_preserves_toy_and_replays() -> TestResult {
    run_case(Case::GroundSantaPanda)
}

#[test]
fn srv7_santa_doll_carried_native_use_preserves_toy_and_replays() -> TestResult {
    run_case(Case::SantaDoll)
}
#[test]
fn srv7_santa_doll_ground_native_use_preserves_toy_and_replays() -> TestResult {
    run_case(Case::GroundSantaDoll)
}

#[path = "novelty_items/dracoyle.rs"]
mod dracoyle;
#[test]
fn srv7_ground_dracoyle_west_native_gem_use_preserves_uuid_and_replays() -> TestResult {
    run_case(Case::DracoyleWest)
}
#[test]
fn srv7_ground_dracoyle_east_native_gem_use_preserves_uuid_and_replays() -> TestResult {
    run_case(Case::DracoyleEast)
}

#[path = "novelty_items/ground_rotate.rs"]
mod ground_rotate;
#[test]
fn srv7_ground_inactive_dracoyle_west_explicit_rotate_retains_custody_and_replays() -> TestResult {
    run_case(Case::RotateDracoyleWest)
}
#[test]
fn srv7_ground_inactive_dracoyle_east_explicit_rotate_retains_custody_and_replays() -> TestResult {
    run_case(Case::RotateDracoyleEast)
}

#[path = "novelty_items/portable_furniture.rs"]
mod portable_furniture;
#[test]
fn srv7_channel_portable_lamp_native_use_preserves_instance_and_starts_clock() -> TestResult {
    run_case(Case::PortableLamp)
}
#[test]
fn srv7_channel_all_fifteen_portable_instruments_publish_native_cues_once() -> TestResult {
    run_case(Case::PortableMusic)
}

#[path = "novelty_items/toy_first_move.rs"]
mod toy_first_move;

#[test]
fn srv7_santa_dragon_individual_native_move_starts_three_second_source_clock() -> TestResult {
    run_case(Case::ToyDragonMove)
}

#[test]
fn srv7_santa_santa_individual_native_move_starts_three_second_source_clock() -> TestResult {
    run_case(Case::ToySantaMove)
}

#[test]
fn srv7_santa_panda_individual_native_move_starts_three_second_source_clock() -> TestResult {
    run_case(Case::ToyPandaMove)
}

#[test]
fn srv7_ground_portable_lamp_native_use_preserves_uuid_custody_and_clock() -> TestResult {
    run_case(Case::GroundPortableLamp)
}
