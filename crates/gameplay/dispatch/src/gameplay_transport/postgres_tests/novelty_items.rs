//! W19 novelty actions enter through real ordinary USE and retain one durable outcome.
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use super::fixture::{Harness, debug, id, run};
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use super::{Dependencies, TestResult, prepare_current_session};
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::gameplay_transport::capabilities::SelectedCapabilities;
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::gameplay_transport::item_view::ItemKey;
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use oteryn_gameplay_items::novelty_items::{self, NoveltyEffects};
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
enum Case {
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
    VoodooFarTarget,
    Present,
    GroundDice,
    GroundRocket,
    GroundTrumpet,
    NestedCostume,
    NestedTrumpet,
    GroundCostume,
    GroundPresent,
    GroundVoodoo,
    PendingLifecycle,
    FeroxaTrap,
    FeroxaTrapOutside,
}
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
impl Case {
    #[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn key(self) -> &'static str {
        match self {
            Self::Costume | Self::NestedCostume | Self::GroundCostume => "oteryn:item.tibia.i653",
            Self::Dice | Self::DiceStreak | Self::GroundDice => "oteryn:item.tibia.i5792",
            Self::Trumpet | Self::GroundTrumpet | Self::NestedTrumpet => "oteryn:item.tibia.i6572",
            Self::Rocket | Self::GroundRocket | Self::PendingLifecycle => "oteryn:item.tibia.i6576",
            Self::Hat | Self::HatInBag => "oteryn:item.tibia.i6578",
            Self::VoodooSuccess
            | Self::VoodooFailure
            | Self::VoodooWithoutTarget
            | Self::VoodooFarTarget
            | Self::GroundVoodoo => "oteryn:item.tibia.i3002",
            Self::Present | Self::GroundPresent => "oteryn:item.tibia.i906",
            Self::FeroxaTrap | Self::FeroxaTrapOutside => "oteryn:item.tibia.i22074",
        }
    }
    #[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn ground(self) -> bool {
        matches!(
            self,
            Self::GroundDice
                | Self::GroundRocket
                | Self::GroundTrumpet
                | Self::GroundCostume
                | Self::GroundPresent
                | Self::GroundVoodoo
                | Self::FeroxaTrap
                | Self::FeroxaTrapOutside
        )
    }
    #[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn feroxa(self) -> bool { matches!(self, Self::FeroxaTrap | Self::FeroxaTrapOutside) }
    #[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn origin(self) -> MovementLocalPosition {
        match self {
            Self::FeroxaTrap => MovementLocalPosition { x: 33419, y: 31540, floor: 11 },
            Self::FeroxaTrapOutside => MovementLocalPosition { x: 33420, y: 31540, floor: 11 },
            _ => MovementLocalPosition { x: 6, y: 0, floor: 7 },
        }
    }
    #[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn nested(self) -> bool {
        matches!(self, Self::NestedCostume | Self::NestedTrumpet)
    }
    #[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn trumpet(self) -> bool {
        matches!(
            self,
            Self::Trumpet | Self::GroundTrumpet | Self::NestedTrumpet
        )
    }
    #[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn refused(self) -> bool {
        matches!(
            self,
            Self::HatInBag | Self::VoodooWithoutTarget | Self::VoodooFarTarget | Self::FeroxaTrapOutside
        )
    }
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn qn5_feroxa_trap_ground_use_is_fenced_and_replay_safe() -> TestResult { run_case(Case::FeroxaTrap) }
#[cfg(feature = "pg-tests-g2")]
#[test]
fn qn5_feroxa_trap_ground_use_checks_actor_room_boundary() -> TestResult { run_case(Case::FeroxaTrapOutside) }
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_costume_commits_condition_once() -> TestResult {
    run_case(Case::Costume)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_dice_commits_exact_face_once() -> TestResult {
    run_case(Case::Dice)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_dice_three_sixes_reset_and_replay_grants_once() -> TestResult {
    run_case(Case::DiceStreak)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_trumpet_commits_original_absolute_deadline() -> TestResult {
    run_case(Case::Trumpet)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_carried_rocket_commits_ten_health_loss_once() -> TestResult {
    run_case(Case::Rocket)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_equipped_hat_preserves_item_and_retains_donor_cues_once() -> TestResult {
    run_case(Case::Hat)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_hat_in_backpack_refuses_without_progress_or_effect() -> TestResult {
    run_case(Case::HatInBag)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_voodoo_success_targets_player_without_consumption() -> TestResult {
    run_case(Case::VoodooSuccess)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_voodoo_failure_is_a_retained_no_consume_outcome() -> TestResult {
    run_case(Case::VoodooFailure)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_voodoo_without_player_refuses_without_receipt() -> TestResult {
    run_case(Case::VoodooWithoutTarget)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_explosive_present_commits_consumption_and_donor_cues_once() -> TestResult {
    run_case(Case::Present)
}

#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_ground_dice_uses_map_handle_and_exact_source_position() -> TestResult {
    run_case(Case::GroundDice)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_ground_rocket_consumes_without_health_loss() -> TestResult {
    run_case(Case::GroundRocket)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_ground_trumpet_keeps_ground_order_and_original_deadline() -> TestResult {
    run_case(Case::GroundTrumpet)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_nested_costume_uses_open_container_handle_and_replays() -> TestResult {
    run_case(Case::NestedCostume)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_nested_trumpet_expires_in_its_actual_container() -> TestResult {
    run_case(Case::NestedTrumpet)
}

#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_ground_costume_changes_player_and_consumes_ground_bag() -> TestResult {
    run_case(Case::GroundCostume)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_ground_present_retains_player_cue_position() -> TestResult {
    run_case(Case::GroundPresent)
}
#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_ground_voodoo_preserves_source_and_targets_player() -> TestResult {
    run_case(Case::GroundVoodoo)
}

#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_cancelled_use_reconciles_committed_rollback_and_unknown_before_control_loss()
-> TestResult {
    run_case(Case::PendingLifecycle)
}

#[path = "novelty_items/pending.rs"]
mod pending;

#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
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
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use scenario::scenario;

#[cfg(feature = "pg-tests-g2")]
#[test]
fn novelty_voodoo_far_target_refuses_without_receipt_or_private_effect() -> TestResult {
    run_case(Case::VoodooFarTarget)
}
