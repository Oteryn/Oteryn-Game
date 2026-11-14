//! Shell control identities and activation over retained window owners.
use super::Control;
use crate::bevy_shell::{game_ui, owner::HostOwner};
use bevy::{prelude::*, ui_widgets::Activate};

pub(super) fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    modal: game_ui::modal::ModalGuard,
) {
    if !modal.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        if owner.modal_active()
            && matches!(
                control,
                Control::Appearance
                    | Control::CastChat
                    | Control::Cast
                    | Control::ExivaSettings
                    | Control::HirelingServices
                    | Control::FurnitureRotation
                    | Control::HouseMoveOut
                    | Control::HouseTransfer
                    | Control::HouseBrowsing
                    | Control::GuildhallLeadership
                    | Control::GuildWars
                    | Control::CyclopediaTrackers
                    | Control::CyclopediaMap
                    | Control::BossDifficulty
                    | Control::BossRewardChest
                    | Control::AnimusMastery
                    | Control::BrowseField
                    | Control::ContainerSorting
                    | Control::Options
                    | Control::Market
                    | Control::Party
                    | Control::PlayerReport
                    | Control::Quests
                    | Control::Spells
                    | Control::StatusDetails
                    | Control::StaffTools
                    | Control::WorldEvent
                    | Control::Training
                    | Control::Store
                    | Control::WeaponProficiency
                    | Control::Bosstiary(_)
                    | Control::AchievementNotices
                    | Control::ExpertPvpModes
                    | Control::Guild(_)
                    | Control::RaidSchedule
                    | Control::Arena
                    | Control::CyclopediaCharacter
                    | Control::FurnitureWrapping
                    | Control::Login
                    | Control::CancelLogin
            )
        {
            return;
        }
        match control {
            Control::Appearance => game_ui::appearance::open(&mut owner),
            Control::CastChat => game_ui::cast_chat::toggle(&mut owner),
            Control::Cast => game_ui::cast_control::open(&owner),
            Control::ExivaSettings => game_ui::open_exiva_settings(&owner),
            Control::HouseMoveOut => game_ui::house_move_out::open(&mut owner),
            Control::GuildhallLeadership => game_ui::guildhall_leadership::toggle(&mut owner),
            Control::CyclopediaTrackers => game_ui::cyclopedia_trackers::open(&owner),
            Control::CyclopediaMap => game_ui::cyclopedia_map::open(&mut owner),
            Control::BossDifficulty => game_ui::boss_difficulty::open(&owner),
            Control::BossRewardChest => game_ui::open_boss_reward_chest(&mut owner),
            Control::AnimusMastery => game_ui::animus_mastery::open(&owner),
            Control::BrowseField => game_ui::browse_field::open(&owner),
            Control::ContainerSorting => game_ui::open_container_sorting(&mut owner),
            Control::Options => owner.open_graphics(),
            Control::HirelingServices => game_ui::hireling_services::toggle(&owner),
            Control::FurnitureRotation => game_ui::toggle_furniture_rotation(&mut owner),
            Control::HouseTransfer => game_ui::house_transfer::open(&owner),
            Control::HouseBrowsing => game_ui::house_browsing::open(&mut owner),
            Control::GuildWars => game_ui::open_guild_wars(&owner),
            Control::Market => game_ui::open_market(&mut owner),
            Control::Party => game_ui::open_party(&mut owner),
            Control::PlayerReport => game_ui::player_report_report::open(&mut owner),
            Control::Quests => game_ui::open_quests(&mut owner),
            Control::Spells => game_ui::open_spells(&mut owner),
            Control::StatusDetails => game_ui::status_details::toggle(&owner),
            Control::StaffTools => game_ui::staff_tools::open(&mut owner),
            Control::WorldEvent => game_ui::world_event_window::open(&owner),
            Control::TeamFinder => game_ui::open_team_finder(&mut owner),
            Control::Training => game_ui::training::open(&owner),
            Control::Store => game_ui::store::open(&mut owner),
            Control::WeaponProficiency => game_ui::weapon_proficiency::open(&owner),
            Control::Bosstiary(cooldowns) => {
                game_ui::bosstiary::toggle(&mut owner, *cooldowns)
            }
            Control::AchievementNotices => game_ui::open_achievement_notices(&mut owner),
            Control::ExpertPvpModes => game_ui::expert_pvp_modes::toggle(&owner),
            Control::Guild(bank) => game_ui::guild::open(&mut owner, *bank),
            Control::RaidSchedule => game_ui::raid_schedule::open(&mut owner),
            Control::Arena => game_ui::arena::toggle(&owner),
            Control::CyclopediaCharacter => game_ui::cyclopedia_character::toggle(&owner),
            Control::FurnitureWrapping => game_ui::furniture_wrapping::toggle(&mut owner),
            Control::Login => owner.request_login(),
            Control::CancelLogin => owner.cancel_login(),
            Control::Apply => owner.apply(false),
            Control::Ok => owner.apply(true),
            Control::Cancel => owner.cancel(),
            _ => {}
        }
    }
}
