//! Shell control identities and activation over retained window owners.
use crate::bevy_shell::{
    game_ui,
    owner::{GraphicsDraft, HostOwner},
};
use bevy::{prelude::*, ui_widgets::Activate};

#[derive(Component, Clone, Copy, Debug)]
pub(in crate::bevy_shell) enum Control {
    Appearance,
    CastChat,
    Cast,
    ExivaSettings,
    HouseMoveOut,
    GuildhallLeadership,
    CyclopediaTrackers,
    CyclopediaMap,
    BossDifficulty,
    BossRewardChest,
    AnimusMastery,
    BrowseField,
    ContainerSorting,
    Options,
    HirelingServices,
    FurnitureRotation,
    HouseTransfer,
    HouseBrowsing,
    GuildWars,
    Market,
    Party,
    PlayerReport,
    Quests,
    Spells,
    StatusDetails,
    StaffTools,
    WorldEvent,
    TeamFinder,
    Training,
    Store,
    Login,
    CancelLogin,
    Fullscreen,
    Vsync,
    IntegerScaling,
    Apply,
    Ok,
    Cancel,
}
impl Control {
    pub fn id(self) -> &'static str {
        match self {
            Self::Appearance => "shell.appearance",
            Self::CastChat => "shell.cast_chat",
            Self::Cast => "shell.cast_control",
            Self::ExivaSettings => "shell.exiva_settings",
            Self::HouseMoveOut => "shell.house_move_out",
            Self::GuildhallLeadership => "shell.guildhall_leadership",
            Self::CyclopediaTrackers => "shell.cyclopedia_trackers",
            Self::CyclopediaMap => "shell.cyclopedia_map",
            Self::BossDifficulty => "shell.boss_difficulty",
            Self::BossRewardChest => "shell.boss_reward_chest",
            Self::AnimusMastery => "shell.animus_mastery",
            Self::BrowseField => "shell.browse_field",
            Self::ContainerSorting => "shell.container_sorting",
            Self::Options => "shell.options",
            Self::HirelingServices => "shell.hireling_services",
            Self::FurnitureRotation => "shell.furniture_rotation",
            Self::HouseTransfer => "shell.house_transfer",
            Self::HouseBrowsing => "shell.house_browsing",
            Self::GuildWars => "shell.guild_wars",
            Self::Market => "shell.market",
            Self::Party => "shell.party",
            Self::PlayerReport => "shell.player_report",
            Self::Quests => "shell.quests",
            Self::Spells => "shell.spells",
            Self::StatusDetails => "shell.status_details",
            Self::StaffTools => "shell.staff_tools",
            Self::WorldEvent => "shell.world_event",
            Self::TeamFinder => "shell.team_finder",
            Self::Training => "shell.training",
            Self::Store => "shell.store",
            Self::Login => "shell.sign_in",
            Self::CancelLogin => "shell.cancel_sign_in",
            Self::Fullscreen => "graphics.fullscreen",
            Self::Vsync => "graphics.vsync",
            Self::IntegerScaling => "graphics.integer_scaling",
            Self::Apply => "graphics.apply",
            Self::Ok => "graphics.ok",
            Self::Cancel => "graphics.cancel",
        }
    }
    pub(super) fn checked(self, draft: GraphicsDraft) -> bool {
        match self {
            Self::Fullscreen => draft.fullscreen,
            Self::Vsync => draft.vsync,
            Self::IntegerScaling => draft.integer_scaling,
            _ => false,
        }
    }
}
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
            Control::Login => owner.request_login(),
            Control::CancelLogin => owner.cancel_login(),
            Control::Apply => owner.apply(false),
            Control::Ok => owner.apply(true),
            Control::Cancel => owner.cancel(),
            _ => {}
        }
    }
}
