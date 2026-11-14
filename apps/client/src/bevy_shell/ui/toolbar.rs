//! Project shell controls into the in-game toolbar.
use super::{Control, ShellToolbar, Status};
use crate::bevy_shell::{game_ui, owner::HostOwner};
use bevy::prelude::*;

/// Keep admission controls outside the in-game HUD once play is installed.
// One disjoint query changes the toolbar, status and admission button nodes together.
#[allow(clippy::type_complexity)]
pub(in crate::bevy_shell) fn project_toolbar(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Query<Entity, With<game_ui::ToolbarRegion>>,
    mut nodes: Query<
        (
            Entity,
            &mut Node,
            Has<ShellToolbar>,
            Has<Status>,
            Option<&Control>,
            Option<&ChildOf>,
        ),
        Or<(With<ShellToolbar>, With<Status>, With<Control>)>,
    >,
) {
    let playing = owner.play.is_some();
    let staff = game_ui::staff_tools::toolbar_visible(&owner);
    for (entity, mut node, toolbar, status, control, parent) in &mut nodes {
        if toolbar {
            if playing {
                if let Ok(region) = regions.single() {
                    if parent.is_none_or(|parent| parent.parent() != region) {
                        commands.entity(entity).insert(ChildOf(region));
                    }
                    node.position_type = PositionType::Relative;
                    node.width = percent(100.);
                    node.max_width = percent(100.);
                    node.overflow = Overflow::clip();
                    node.top = Val::Auto;
                    node.bottom = Val::Auto;
                    node.left = Val::Auto;
                    node.right = Val::Auto;
                }
            } else {
                if parent.is_some() {
                    commands.entity(entity).remove::<ChildOf>();
                }
                node.position_type = PositionType::Absolute;
                node.width = Val::Auto;
                node.max_width = Val::Auto;
                node.top = px(8.);
                node.bottom = Val::Auto;
                node.left = px(8.);
                node.right = px(8.);
            }
        } else if matches!(
            control,
            Some(
                Control::AchievementNotices
                    | Control::Achievements
                    | Control::AnimusMastery
                    | Control::Appearance
                    | Control::Arena
                    | Control::BossDifficulty
                    | Control::BossRewardChest
                    | Control::Bosstiary(_)
                    | Control::BrowseField
                    | Control::Cast
                    | Control::CastChat
                    | Control::ContainerSorting
                    | Control::CyclopediaCharacter
                    | Control::CyclopediaMap
                    | Control::CyclopediaTrackers
                    | Control::Details(_)
                    | Control::ExivaSettings
                    | Control::ExpertPvpModes
                    | Control::FurnitureRotation
                    | Control::FurnitureWrapping
                    | Control::Guild(_)
                    | Control::GuildWars
                    | Control::GuildhallLeadership
                    | Control::HirelingManagement
                    | Control::HirelingServices
                    | Control::HouseBrowsing
                    | Control::HouseMoveOut
                    | Control::HousePodium
                    | Control::HouseTransfer
                    | Control::Market
                    | Control::Party
                    | Control::PlayerReport
                    | Control::Prey
                    | Control::Quests
                    | Control::RaidSchedule
                    | Control::SkillsConfig
                    | Control::Spells
                    | Control::StaffTools
                    | Control::StatusDetails
                    | Control::SupportEvidence
                    | Control::TaskBoard
                    | Control::TeamFinder
                    | Control::TouchControls
                    | Control::Training
                    | Control::WeaponProficiency
                    | Control::WorldEvent
            )
        ) {
            node.display = if playing && (staff || !matches!(control, Some(Control::StaffTools))) {
                Display::Flex
            } else {
                Display::None
            };
        } else if status || matches!(control, Some(Control::Login | Control::CancelLogin)) {
            node.display = if playing {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
}
